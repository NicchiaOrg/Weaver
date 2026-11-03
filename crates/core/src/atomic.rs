//! Atomic file writes: the content goes to a temp file `.<name>.<random>.tmp` in the same
//! folder, is flushed to disk, then renamed over the target.
//!
//! On Windows a rename refused for sharing or access reasons (an editor, an antivirus or a
//! watcher holding the target) is retried every [`RENAME_RETRY_INTERVAL`] for at most
//! [`RENAME_RETRY_WINDOW`], then fails.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use crate::endpoint::random_hex;

/// Pause between two attempts of a refused rename (Windows).
pub const RENAME_RETRY_INTERVAL: Duration = Duration::from_millis(50);

/// How long a refused rename is retried before it fails (Windows).
pub const RENAME_RETRY_WINDOW: Duration = Duration::from_secs(1);

const TEMP_SUFFIX: &str = ".tmp";

/// Writes `bytes` to `path` atomically, creating the parent folder when missing.
///
/// `unix_mode` (for example `0o600`) is applied at creation on Unix and ignored elsewhere.
pub fn write(path: &Path, bytes: &[u8], unix_mode: Option<u32>) -> io::Result<()> {
    write_with(path, bytes, unix_mode, None)
}

/// Like [`write`], for a file that is replaced and keeps its permission bits: the temp file gets
/// the permissions of the file at `path` (the executable bit included, whatever the umask),
/// when there is one; a new file gets the default ones. Where permissions are not copied
/// (outside Unix) this is [`write`].
pub fn write_keeping_permissions(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let permissions = fs::metadata(path).ok().map(|meta| meta.permissions());
    write_with(path, bytes, None, permissions)
}

fn write_with(
    path: &Path,
    bytes: &[u8],
    unix_mode: Option<u32>,
    permissions: Option<fs::Permissions>,
) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent folder"))?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    fs::create_dir_all(parent)?;
    let mut temp_name = std::ffi::OsString::from(".");
    temp_name.push(name);
    temp_name.push(format!(".{}{TEMP_SUFFIX}", random_hex(12)));
    let temp = parent.join(temp_name);
    let result = write_temp(&temp, bytes, unix_mode, permissions)
        .and_then(|()| rename_with_retry(&temp, path));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Whether `name` is the name of a temp file of [`write`] (`.<name>.<random>.tmp`);
/// watchers ignore these.
pub fn is_temp_name(name: &str) -> bool {
    name.len() > 1 + TEMP_SUFFIX.len() && name.starts_with('.') && name.ends_with(TEMP_SUFFIX)
}

fn write_temp(
    temp: &Path,
    bytes: &[u8],
    unix_mode: Option<u32>,
    permissions: Option<fs::Permissions>,
) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if let Some(mode) = unix_mode {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = unix_mode;
    let mut file: File = options.open(temp)?;
    file.write_all(bytes)?;
    // Copied from the replaced file with `fchmod`, which the umask does not touch.
    #[cfg(unix)]
    if let Some(permissions) = permissions {
        file.set_permissions(permissions)?;
    }
    #[cfg(not(unix))]
    let _ = permissions;
    file.sync_all()
}

/// Renames `from` over `to`, retrying a refusal for sharing reasons on Windows (see the module).
pub fn rename_with_retry(from: &Path, to: &Path) -> io::Result<()> {
    let deadline = Instant::now() + RENAME_RETRY_WINDOW;
    loop {
        match fs::rename(from, to) {
            Err(e) if cfg!(windows) && is_sharing_refusal(&e) && Instant::now() < deadline => {
                thread::sleep(RENAME_RETRY_INTERVAL);
            }
            other => return other,
        }
    }
}

/// A rename refused because the target is open elsewhere or access was denied
/// (`ERROR_ACCESS_DENIED` 5, `ERROR_SHARING_VIOLATION` 32, `ERROR_LOCK_VIOLATION` 33).
fn is_sharing_refusal(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::PermissionDenied
        || matches!(error.raw_os_error(), Some(5 | 32 | 33))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_the_content_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("file.txt");
        write(&path, b"one", None).unwrap();
        write(&path, b"two", None).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"two");
        let names: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("file.txt")]);
    }

    #[test]
    fn temp_files_are_dot_name_random_tmp_in_the_same_folder() {
        assert!(is_temp_name(".file.txt.0123456789ab.tmp"));
        assert!(!is_temp_name("file.txt"));
        assert!(!is_temp_name(".gitignore"));
        assert!(!is_temp_name(".tmp"));
    }

    #[test]
    fn a_failed_write_leaves_the_target_and_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        write(&path, b"old", None).unwrap();
        // A folder cannot be replaced by a file: the rename fails.
        let blocked = dir.path().join("blocked");
        fs::create_dir(&blocked).unwrap();
        fs::write(blocked.join("inner"), b"x").unwrap();
        assert!(write(&blocked, b"new", None).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"old");
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| is_temp_name(n))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn sharing_and_access_refusals_are_retryable() {
        assert!(is_sharing_refusal(&io::Error::from(
            io::ErrorKind::PermissionDenied
        )));
        assert!(is_sharing_refusal(&io::Error::from_raw_os_error(32)));
        assert!(!is_sharing_refusal(&io::Error::from(
            io::ErrorKind::NotFound
        )));
    }

    #[cfg(unix)]
    #[test]
    fn applies_the_unix_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret");
        write(&path, b"x", Some(0o600)).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
