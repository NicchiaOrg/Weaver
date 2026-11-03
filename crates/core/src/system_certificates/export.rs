//! The thin layer of I/O of System Certificates: reading the certificates the system trusts
//! (30 s at most), writing the bundle atomically, reading it back.
//!
//! No child process and no network: `rustls-native-certs` reads the user's and the machine's
//! stores only, and the public roots are compiled into Weaver (`webpki-root-certs`).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, UNIX_EPOCH};

use super::bundle::{build_bundle, count_certificates};
use super::{BundleInfo, EXPORT_LIMIT, TIMED_OUT};

/// The DER certificates the system trusts, as the reader of the export returns them.
pub type Certificates = Vec<Vec<u8>>;

/// Reads the certificates of the system and writes the bundle at `bundle`: the system's first,
/// then Mozilla's public roots. Blocks up to [`EXPORT_LIMIT`]: call it from the blocking pool,
/// never from the IPC or UI thread.
pub fn export(bundle: &Path) -> Result<BundleInfo, String> {
    let public: Vec<&[u8]> = webpki_root_certs::TLS_SERVER_ROOT_CERTS
        .iter()
        .map(|der| der.as_ref())
        .collect();
    export_with(bundle, read_system_certificates, &public, EXPORT_LIMIT)
}

/// [`export`] with its sources injected: `read` reads the system (on a thread of its own, so
/// that a slow reader can be abandoned), `public` are the public roots, `limit` how long to wait
/// for `read`. A failure is the reason in the words of `requirements/tech#system-certificates`;
/// it leaves the earlier bundle whole.
pub fn export_with<R>(
    bundle: &Path,
    read: R,
    public: &[&[u8]],
    limit: Duration,
) -> Result<BundleInfo, String>
where
    R: FnOnce() -> Result<Certificates, String> + Send + 'static,
{
    let system = read_within(read, limit)?;
    let built = build_bundle(&system, public)?;
    let written_ms = write_bundle(bundle, &built.pem)?;
    Ok(BundleInfo {
        count: u32::try_from(built.count).unwrap_or(u32::MAX),
        written_ms,
    })
}

/// The certificates the operating system trusts (`rustls-native-certs`).
pub fn read_system_certificates() -> Result<Certificates, String> {
    from_native(rustls_native_certs::load_native_certs())
}

/// Errors on single certificates are skipped while at least one loads. When none loads and there
/// was an error, the first error is the reason.
fn from_native(result: rustls_native_certs::CertificateResult) -> Result<Certificates, String> {
    if result.certs.is_empty() {
        if let Some(error) = result.errors.first() {
            return Err(format!("could not read the system certificates: {error}"));
        }
    }
    Ok(result
        .certs
        .iter()
        .map(|der| <[u8]>::to_vec(der.as_ref()))
        .collect())
}

/// Runs `read` on a thread of its own and waits for it at most `limit`. Past the limit the result
/// is [`TIMED_OUT`] and the late result of the thread is dropped.
pub fn read_within<R>(read: R, limit: Duration) -> Result<Certificates, String>
where
    R: FnOnce() -> Result<Certificates, String> + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .name("system-certificates-read".to_owned())
        .spawn(move || {
            // The receiver is gone after the limit: the late result goes nowhere.
            let _ = sender.send(read());
        })
        .map_err(|error| format!("could not read the system certificates: {error}"))?;
    match receiver.recv_timeout(limit) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(TIMED_OUT.to_owned()),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(
            "could not read the system certificates: the reader stopped unexpectedly".to_owned(),
        ),
    }
}

/// Writes `pem` to `path` atomically (`system-certificates.pem.<pid>.tmp` in the same folder,
/// then a rename over it) and returns the modification time of the result, in milliseconds since
/// the epoch. A failure is `could not write the bundle: <message>`; the earlier bundle stays
/// whole and the temp file is removed.
pub fn write_bundle(path: &Path, pem: &str) -> Result<i64, String> {
    write_bundle_with(
        path,
        pem,
        std::process::id(),
        &crate::atomic::rename_with_retry,
    )
}

/// [`write_bundle`] with the process id and the rename injected.
pub fn write_bundle_with(
    path: &Path,
    pem: &str,
    pid: u32,
    rename: &dyn Fn(&Path, &Path) -> io::Result<()>,
) -> Result<i64, String> {
    let fail = |error: io::Error| format!("could not write the bundle: {error}");
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| fail(io::Error::new(io::ErrorKind::InvalidInput, "no folder")))?;
    let name = path
        .file_name()
        .ok_or_else(|| fail(io::Error::new(io::ErrorKind::InvalidInput, "no file name")))?;
    let mut temp_name = name.to_os_string();
    temp_name.push(format!(".{pid}.tmp"));
    let temp = parent.join(temp_name);
    let written = fs::create_dir_all(parent)
        .and_then(|()| write_temp(&temp, pem))
        .and_then(|()| rename(&temp, path));
    if let Err(error) = written {
        let _ = fs::remove_file(&temp);
        return Err(fail(error));
    }
    modified_ms(path).map_err(fail)
}

fn write_temp(temp: &Path, pem: &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(temp)?;
    file.write_all(pem.as_bytes())?;
    file.sync_all()
}

/// The bundle at `path` as the app sees it at start: its certificate count read back from the
/// file and its modification time. `None` when the file is missing, unreadable or holds no
/// certificate.
pub fn read_bundle_info(path: &Path) -> Option<BundleInfo> {
    let text = fs::read_to_string(path).ok()?;
    let count = count_certificates(&text);
    if count == 0 {
        return None;
    }
    Some(BundleInfo {
        count: u32::try_from(count).unwrap_or(u32::MAX),
        written_ms: modified_ms(path).ok()?,
    })
}

/// The modification time of `path` in milliseconds since the epoch.
pub fn modified_ms(path: &Path) -> io::Result<i64> {
    let modified = fs::metadata(path)?.modified()?;
    Ok(match modified.duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_millis()).unwrap_or(i64::MAX),
        Err(before) => -i64::try_from(before.duration().as_millis()).unwrap_or(i64::MAX),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Instant, SystemTime};

    use super::super::bundle::tests::certificate;
    use super::super::{BUNDLE_FILE_NAME, NO_AUTHORITY};
    use super::*;

    fn temp_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn now_ms() -> i64 {
        i64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap()
    }

    #[test]
    fn an_export_writes_the_bundle_and_says_how_many_certificates_it_holds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data").join(BUNDLE_FILE_NAME);
        let system = vec![
            certificate("System", Some(true)),
            certificate("Leaf", Some(false)),
        ];
        let public = certificate("Public", Some(true));
        let before = now_ms();
        let info = export_with(
            &path,
            move || Ok(system),
            &[&public],
            Duration::from_secs(5),
        )
        .unwrap();
        assert_eq!(info.count, 2);
        assert!(info.written_ms >= before - 2_000 && info.written_ms <= now_ms() + 2_000);
        // Read back from the file.
        assert_eq!(read_bundle_info(&path), Some(info));
        assert_eq!(count_certificates(&fs::read_to_string(&path).unwrap()), 2);
        assert_eq!(temp_names(path.parent().unwrap()), [BUNDLE_FILE_NAME]);
    }

    #[test]
    fn an_export_without_a_system_authority_fails_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(BUNDLE_FILE_NAME);
        let public = certificate("Public", Some(true));
        let failure =
            export_with(&path, || Ok(Vec::new()), &[&public], Duration::from_secs(5)).unwrap_err();
        assert_eq!(failure, NO_AUTHORITY);
        assert!(!path.exists());
    }

    #[test]
    fn a_failure_of_the_reader_is_the_reason() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(BUNDLE_FILE_NAME);
        let failure = export_with(
            &path,
            || Err("could not read the system certificates: boom".to_owned()),
            &[],
            Duration::from_secs(5),
        )
        .unwrap_err();
        assert_eq!(failure, "could not read the system certificates: boom");
        assert!(!path.exists());
    }

    #[test]
    fn a_panic_of_the_reader_is_a_failure_not_a_hang() {
        let failure =
            read_within(|| panic!("reader exploded"), Duration::from_secs(5)).unwrap_err();
        assert!(
            failure.starts_with("could not read the system certificates:"),
            "{failure}"
        );
    }

    #[test]
    fn a_slow_reader_ends_the_export_at_the_limit_and_its_late_result_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(BUNDLE_FILE_NAME);
        let (release, wait) = mpsc::channel::<()>();
        let (finished, finished_reader) = mpsc::channel::<()>();
        let started = Instant::now();
        let failure = export_with(
            &path,
            move || {
                let _ = wait.recv();
                let _ = finished.send(());
                Ok(vec![certificate("Late", Some(true))])
            },
            &[],
            Duration::from_millis(100),
        )
        .unwrap_err();
        assert_eq!(failure, "the export did not finish within 30 s");
        assert!(started.elapsed() < Duration::from_secs(5));
        // The reader finishes later; nothing is written for it.
        release.send(()).unwrap();
        finished_reader
            .recv_timeout(Duration::from_secs(5))
            .expect("the reader ended");
        thread::sleep(Duration::from_millis(50));
        assert!(!path.exists());
    }

    #[test]
    fn the_limit_of_the_export_is_thirty_seconds() {
        assert_eq!(EXPORT_LIMIT, Duration::from_secs(30));
    }

    #[test]
    fn a_failing_rename_leaves_the_earlier_bundle_whole_and_removes_the_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(BUNDLE_FILE_NAME);
        fs::write(&path, "earlier bundle").unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_by_rename = Arc::clone(&seen);
        let failure = write_bundle_with(&path, "new bundle", 4242, &move |from, to| {
            seen_by_rename.lock().unwrap().push((
                from.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(from).unwrap(),
                to.to_path_buf(),
            ));
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "refused"))
        })
        .unwrap_err();
        assert_eq!(failure, "could not write the bundle: refused");
        assert_eq!(fs::read_to_string(&path).unwrap(), "earlier bundle");
        assert_eq!(temp_names(dir.path()), [BUNDLE_FILE_NAME]);
        // The temp file had the documented name and the whole new content when the rename ran.
        assert_eq!(
            *seen.lock().unwrap(),
            [(
                "system-certificates.pem.4242.tmp".to_owned(),
                "new bundle".to_owned(),
                path
            )]
        );
    }

    #[test]
    fn a_successful_write_replaces_the_bundle_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(BUNDLE_FILE_NAME);
        fs::write(&path, "earlier bundle").unwrap();
        let written_ms = write_bundle(&path, "new bundle").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new bundle");
        assert_eq!(temp_names(dir.path()), [BUNDLE_FILE_NAME]);
        assert_eq!(written_ms, modified_ms(&path).unwrap());
    }

    #[test]
    fn a_stale_temp_file_of_an_earlier_run_does_not_stop_the_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(BUNDLE_FILE_NAME);
        let stale = dir.path().join("system-certificates.pem.7.tmp");
        fs::write(&stale, "left over").unwrap();
        write_bundle_with(&path, "new", 7, &|from, to| fs::rename(from, to)).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(temp_names(dir.path()), [BUNDLE_FILE_NAME]);
    }

    #[test]
    fn a_write_into_a_place_that_cannot_hold_it_reports_the_message() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("file");
        fs::write(&blocker, "x").unwrap();
        let failure = write_bundle(&blocker.join(BUNDLE_FILE_NAME), "pem").unwrap_err();
        assert!(
            failure.starts_with("could not write the bundle: "),
            "{failure}"
        );
    }

    #[test]
    fn a_missing_or_empty_bundle_has_no_info() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(BUNDLE_FILE_NAME);
        assert_eq!(read_bundle_info(&path), None);
        fs::write(&path, "").unwrap();
        assert_eq!(read_bundle_info(&path), None);
    }

    #[test]
    fn errors_on_single_certificates_are_skipped_while_one_loads() {
        let one = webpki_root_certs::TLS_SERVER_ROOT_CERTS[0].clone();
        let mut result = rustls_native_certs::CertificateResult::default();
        result.certs.push(one.clone());
        result.errors.push(native_error("first"));
        assert_eq!(from_native(result).unwrap(), [one.as_ref().to_vec()]);
    }

    #[test]
    fn when_none_loads_the_first_error_is_the_reason() {
        let mut result = rustls_native_certs::CertificateResult::default();
        result.errors.push(native_error("first"));
        result.errors.push(native_error("second"));
        assert_eq!(
            from_native(result).unwrap_err(),
            "could not read the system certificates: test: first"
        );
    }

    #[test]
    fn nothing_loaded_and_no_error_is_an_empty_list() {
        let result = rustls_native_certs::CertificateResult::default();
        assert_eq!(from_native(result).unwrap(), Certificates::new());
    }

    fn native_error(text: &str) -> rustls_native_certs::Error {
        rustls_native_certs::Error {
            context: "test",
            kind: rustls_native_certs::ErrorKind::Os(text.into()),
        }
    }
}
