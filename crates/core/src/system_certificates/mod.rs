//! System Certificates (`requirements/tech#system-certificates`, `requirements/rules#system-certificates`):
//! omp trusts the certificate authorities the system trusts, for proxies that inspect HTTPS.
//!
//! Pure functions plus a thin layer of I/O; the app backend runs them, keeps the state and tells
//! the windows. The UI never reads the bundle or the certificates.
//!
//! | Module      | Responsibility |
//! |-------------|----------------|
//! | this one    | The names, the 24 h age rule, the variables of an omp process |
//! | [`bundle`]  | The pure build of the PEM text from DER certificates |
//! | [`export`]  | Reading the system (30 s limit), the atomic write, reading the bundle back |
//! | [`machine`] | The Machine `system-certificates`: states, one export at a time, the status |
//! | [`zscaler`] | Whether Zscaler is installed, with an injected file system (file metadata only) |
//!
//! # For the Tasks that start omp processes
//!
//! Every omp process (the commands of `requirements/tech#omp`, jobs, General and their resumes, the
//! terminals of the installer and of the logins) gets [`variables`] when it is spawned, after the
//! inherited environment, so that they win over inherited values of the same names. In the app,
//! `crate::system_certificates::variables(&state)` reads the setting and the bundle path for you.
//! The login shell and `reg.exe`, which only read `PATH`, never get them; neither does anything
//! register them with a job.
//!
//! Failure texts: [`crate::omp::failure::classify`] and [`crate::omp::failure::explain`].

pub mod bundle;
pub mod export;
pub mod machine;
pub mod zscaler;

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

pub use bundle::{build_bundle, count_certificates, Bundle};
pub use export::{export, export_with, read_bundle_info, write_bundle};
pub use machine::{ExportEnded, Machine, Phase, Request, Status, Ticket};

/// The bundle's file name in the local data folder (`AppDirs::system_certificates_file`).
pub const BUNDLE_FILE_NAME: &str = "system-certificates.pem";

/// A bundle older than this is stale.
pub const BUNDLE_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// While the setting is on, the next export is due this long after the last one ended.
pub const REFRESH_EVERY: Duration = BUNDLE_MAX_AGE;

/// An export ends within this time.
pub const EXPORT_LIMIT: Duration = Duration::from_secs(30);

/// Why an export fails when the system gives no certificate authority.
pub const NO_AUTHORITY: &str = "no certificate authority found";

/// Why an export fails when the limit passed.
pub const TIMED_OUT: &str = "the export did not finish within 30 s";

/// The bundle in use: how many certificates it holds and when it was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundleInfo {
    pub count: u32,
    /// Milliseconds since 1970-01-01T00:00:00Z: the modification time of the file.
    pub written_ms: i64,
}

/// Whether a bundle is stale: missing (`written_ms` is `None`) or written more than 24 h before
/// `now_ms`. The clock is the caller's, injected in tests.
pub fn is_stale(written_ms: Option<i64>, now_ms: i64) -> bool {
    let max_age = i64::try_from(BUNDLE_MAX_AGE.as_millis()).unwrap_or(i64::MAX);
    match written_ms {
        None => true,
        Some(written) => now_ms.saturating_sub(written) > max_age,
    }
}

/// The variables that tell a program to trust the bundle, each with the bundle's path.
pub const BUNDLE_VARIABLES: [&str; 9] = [
    "NODE_EXTRA_CA_CERTS",
    "SSL_CERT_FILE",
    "REQUESTS_CA_BUNDLE",
    "CURL_CA_BUNDLE",
    "GIT_SSL_CAINFO",
    "PIP_CERT",
    "CARGO_HTTP_CAINFO",
    "AWS_CA_BUNDLE",
    "DENO_CERT",
];

/// The variable that turns off the compression of omp's requests, which some proxies break.
pub const REQUEST_COMPRESSION: (&str, &str) = ("PI_REQUEST_COMPRESSION", "0");

/// The environment variables of an omp process, as name and value.
pub type Variables = Vec<(&'static str, OsString)>;

/// The variables an omp process gets, read when the process is spawned: with the setting on and
/// the bundle file present, [`BUNDLE_VARIABLES`] with the absolute path of `bundle` and
/// `PI_REQUEST_COMPRESSION=0`; with the setting off or no bundle, nothing, so the environment is
/// exactly as without System Certificates.
///
/// Weaver never turns TLS checking off: no variable here, and none that does, is ever added.
pub fn variables(enabled: bool, bundle: &Path) -> Variables {
    if !enabled || !bundle.is_file() {
        return Vec::new();
    }
    let path = std::path::absolute(bundle).unwrap_or_else(|_| bundle.to_path_buf());
    let mut variables: Variables = BUNDLE_VARIABLES
        .iter()
        .map(|name| (*name, path.clone().into_os_string()))
        .collect();
    variables.push((REQUEST_COMPRESSION.0, OsString::from(REQUEST_COMPRESSION.1)));
    variables
}

/// Puts `variables` in the environment of `command`, over the inherited values of the same
/// names. `PATH` is not touched.
pub fn apply(command: &mut Command, variables: &Variables) {
    for (name, value) in variables {
        command.env(name, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR_MS: i64 = 60 * 60 * 1000;

    #[test]
    fn a_missing_bundle_is_stale() {
        assert!(is_stale(None, 1_000_000));
    }

    #[test]
    fn a_bundle_is_stale_only_when_more_than_24_hours_old() {
        let now = 1_800_000_000_000;
        assert!(!is_stale(Some(now), now));
        assert!(!is_stale(Some(now - 23 * HOUR_MS), now));
        assert!(!is_stale(Some(now - 24 * HOUR_MS), now));
        assert!(is_stale(Some(now - 24 * HOUR_MS - 1), now));
        assert!(is_stale(Some(now - 48 * HOUR_MS), now));
        // A modification time in the future is not older than a day.
        assert!(!is_stale(Some(now + HOUR_MS), now));
    }

    fn folder_with_bundle() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join(BUNDLE_FILE_NAME);
        std::fs::write(&bundle, "pem").unwrap();
        (dir, bundle)
    }

    #[test]
    fn with_the_setting_on_and_a_bundle_every_variable_gets_the_absolute_path() {
        let (_dir, bundle) = folder_with_bundle();
        let variables = variables(true, &bundle);
        let names: Vec<&str> = variables.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            names,
            [
                "NODE_EXTRA_CA_CERTS",
                "SSL_CERT_FILE",
                "REQUESTS_CA_BUNDLE",
                "CURL_CA_BUNDLE",
                "GIT_SSL_CAINFO",
                "PIP_CERT",
                "CARGO_HTTP_CAINFO",
                "AWS_CA_BUNDLE",
                "DENO_CERT",
                "PI_REQUEST_COMPRESSION",
            ]
        );
        for (name, value) in &variables[..9] {
            assert_eq!(value, bundle.as_os_str(), "{name}");
            assert!(Path::new(value).is_absolute(), "{name}");
        }
        assert_eq!(variables[9].1, "0");
    }

    #[test]
    fn with_the_setting_off_nothing_is_set() {
        let (_dir, bundle) = folder_with_bundle();
        assert!(variables(false, &bundle).is_empty());
    }

    #[test]
    fn without_a_bundle_nothing_is_set() {
        let dir = tempfile::tempdir().unwrap();
        assert!(variables(true, &dir.path().join(BUNDLE_FILE_NAME)).is_empty());
        // A folder with the name of the bundle is not a bundle.
        std::fs::create_dir(dir.path().join(BUNDLE_FILE_NAME)).unwrap();
        assert!(variables(true, &dir.path().join(BUNDLE_FILE_NAME)).is_empty());
    }

    #[test]
    fn no_variable_turns_tls_checking_off() {
        let (_dir, bundle) = folder_with_bundle();
        let forbidden = [
            "NODE_TLS_REJECT_UNAUTHORIZED",
            "GIT_SSL_NO_VERIFY",
            "PYTHONHTTPSVERIFY",
        ];
        for (name, value) in variables(true, &bundle) {
            assert!(!forbidden.contains(&name), "{name}");
            assert_ne!(value, OsString::from("--insecure"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn the_variables_win_over_inherited_values_and_leave_path_alone() {
        let (_dir, bundle) = folder_with_bundle();
        let mut command = crate::process::command("/usr/bin/env", Vec::<&str>::new());
        command
            .env("NODE_EXTRA_CA_CERTS", "/inherited/ca.pem")
            .env("SSL_CERT_FILE", "/inherited/other.pem")
            .env("PI_REQUEST_COMPRESSION", "1")
            .env("PATH", "/usr/bin:/bin")
            .env("UNRELATED", "kept");
        apply(&mut command, &variables(true, &bundle));
        let output = command.output().unwrap();
        let text = String::from_utf8(output.stdout).unwrap();
        let line = |name: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(&format!("{name}=")))
                .map(str::to_owned)
        };
        let path = bundle.to_str().unwrap();
        assert_eq!(line("NODE_EXTRA_CA_CERTS").as_deref(), Some(path));
        assert_eq!(line("SSL_CERT_FILE").as_deref(), Some(path));
        assert_eq!(line("DENO_CERT").as_deref(), Some(path));
        assert_eq!(line("PI_REQUEST_COMPRESSION").as_deref(), Some("0"));
        assert_eq!(line("PATH").as_deref(), Some("/usr/bin:/bin"));
        assert_eq!(line("UNRELATED").as_deref(), Some("kept"));
    }

    #[cfg(unix)]
    #[test]
    fn without_variables_the_environment_is_left_as_it_was() {
        let mut command = crate::process::command("/usr/bin/env", Vec::<&str>::new());
        command
            .env_clear()
            .env("SSL_CERT_FILE", "/inherited/other.pem");
        apply(&mut command, &Vec::new());
        let text = String::from_utf8(command.output().unwrap().stdout).unwrap();
        assert_eq!(text.trim(), "SSL_CERT_FILE=/inherited/other.pem");
    }
}
