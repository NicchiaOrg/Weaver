//! Runs an omp command without a shell, with a time limit: the process is killed on timeout.
//!
//! [`Exec`] is the seam the probe, the update and the logout go through, so their tests use a
//! fake and never start a real omp.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use super::failure::Failure;
use crate::system_certificates::Variables;

/// Limits of the omp commands (`requirements/tech#omp`).
pub mod limits {
    use std::time::Duration;

    pub const VERSION: Duration = Duration::from_secs(15);
    pub const MODELS: Duration = Duration::from_secs(15);
    pub const UPDATE_CHECK: Duration = Duration::from_secs(30);
    pub const UPDATE: Duration = Duration::from_secs(5 * 60);
    pub const LOGOUT: Duration = Duration::from_secs(30);
    pub const USAGE: Duration = Duration::from_secs(30);
    /// The read of the login PATH at start.
    pub const LOGIN_PATH: Duration = Duration::from_secs(5);
}

/// The most stdout kept (a list of models is a few hundred KB).
const MAX_STDOUT: u64 = 16 * 1024 * 1024;
/// The most stderr kept, from its start (the last line of a longer text is still near the end of
/// what omp prints in practice).
const MAX_STDERR: u64 = 256 * 1024;
/// How long to wait for the pipes to close after the process was killed.
const DRAIN_AFTER_KILL: Duration = Duration::from_secs(1);
const POLL: Duration = Duration::from_millis(15);

/// What a command that ended with success printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
}

/// What System Certificates gives an omp process right now: whether the setting is on, and the
/// variables to set (empty with the setting off or without a bundle).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Certificates {
    pub enabled: bool,
    pub variables: Variables,
}

/// Reads [`Certificates`] at every call: the setting and the bundle of the moment, never once.
pub type CertificateSource = Arc<dyn Fn() -> Certificates + Send + Sync>;

/// Runs `program` with `args` and waits at most `limit`.
pub trait Exec: Send + Sync {
    fn run(&self, program: &Path, args: &[&str], limit: Duration) -> Result<Output, Failure>;

    /// Whether System Certificates is on now. Asked right after a failure, so that its
    /// explanation is chosen with the setting as it is then. Commands that are not omp's say
    /// `false`.
    fn system_certificates_on(&self) -> bool {
        false
    }
}

/// The operating system. For omp (a source of [`Certificates`] is given) it sets the environment
/// of an omp process: `PATH` is `<app local data>/bin`, omp's folder and bun's bin folder, then
/// the base PATH, and the System Certificates variables over the inherited ones. Without a source
/// (the login shell and `reg.exe`, which only read `PATH`) no certificate variable is set.
#[derive(Clone)]
pub struct SystemExec {
    local_bin: PathBuf,
    bun_bin: Option<PathBuf>,
    base_path: OsString,
    certificates: Option<CertificateSource>,
}

impl SystemExec {
    pub fn new(local_bin: PathBuf, bun_bin: Option<PathBuf>, base_path: OsString) -> Self {
        Self {
            local_bin,
            bun_bin,
            base_path,
            certificates: None,
        }
    }

    /// The same, for omp's own commands: they get the certificates of the moment.
    pub fn with_certificates(mut self, certificates: CertificateSource) -> Self {
        self.certificates = Some(certificates);
        self
    }

    /// The `PATH` of a child that runs `program`.
    pub fn child_path(&self, program: &Path) -> OsString {
        super::path_env::child_path(
            &self.local_bin,
            program.parent(),
            self.bun_bin.as_deref(),
            &self.base_path,
        )
    }
}

impl Exec for SystemExec {
    fn run(&self, program: &Path, args: &[&str], limit: Duration) -> Result<Output, Failure> {
        let mut command = crate::process::command(program, args);
        command.env("PATH", self.child_path(program));
        if let Some(source) = &self.certificates {
            crate::system_certificates::apply(&mut command, &source().variables);
        }
        run_command(command, limit)
    }

    fn system_certificates_on(&self) -> bool {
        self.certificates
            .as_ref()
            .is_some_and(|source| source().enabled)
    }
}

/// Runs an already built command (stdin closed, stdout and stderr captured) for at most `limit`.
pub fn run_command(mut command: std::process::Command, limit: Duration) -> Result<Output, Failure> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|_| Failure::NotStarted)?;
    let stdout = child.stdout.take().map(|pipe| drain(pipe, MAX_STDOUT));
    let stderr = child.stderr.take().map(|pipe| drain(pipe, MAX_STDERR));

    let deadline = Instant::now() + limit;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() >= deadline => break None,
            Ok(None) => std::thread::sleep(POLL),
            Err(_) => break None,
        }
    };
    let Some(status) = status else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(Failure::TimedOut { limit });
    };

    let stdout = collect(stdout);
    let stderr = collect(stderr);
    if status.success() {
        Ok(Output { stdout, stderr })
    } else {
        Err(Failure::Exit {
            code: status.code(),
            stderr,
        })
    }
}

/// Reads a pipe to its end in a thread; the receiver gets the text once the pipe closes.
fn drain<R: Read + Send + 'static>(pipe: R, cap: u64) -> mpsc::Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.take(cap).read_to_end(&mut bytes);
        let _ = sender.send(String::from_utf8_lossy(&bytes).into_owned());
    });
    receiver
}

/// The text of a drained pipe; a grandchild that keeps the pipe open cannot block Weaver.
fn collect(receiver: Option<mpsc::Receiver<String>>) -> String {
    receiver
        .and_then(|r| r.recv_timeout(DRAIN_AFTER_KILL).ok())
        .unwrap_or_default()
}

/// Whether `name` (a program) is found in the folders of `path`; the first match wins.
pub fn which(name: &str, path: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .map(|dir| dir.join(name))
        .find(|candidate| super::lookup::is_executable(candidate))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn sh(script: &str) -> std::process::Command {
        crate::process::command("/bin/sh", ["-c", script])
    }

    #[test]
    fn success_returns_stdout_and_stderr() {
        let output = run_command(sh("echo out; echo err >&2"), Duration::from_secs(10)).unwrap();
        assert_eq!(output.stdout, "out\n");
        assert_eq!(output.stderr, "err\n");
    }

    #[test]
    fn a_failure_status_carries_the_code_and_stderr() {
        let failure =
            run_command(sh("echo broken >&2; exit 7"), Duration::from_secs(10)).unwrap_err();
        assert_eq!(
            failure,
            Failure::Exit {
                code: Some(7),
                stderr: "broken\n".to_owned()
            }
        );
    }

    #[test]
    fn a_program_that_does_not_exist_is_not_started() {
        let failure = run_command(
            crate::process::command("/nonexistent/omp", ["x"]),
            Duration::from_secs(10),
        )
        .unwrap_err();
        assert_eq!(failure, Failure::NotStarted);
    }

    #[test]
    fn a_process_over_its_limit_is_killed() {
        let started = Instant::now();
        let failure = run_command(sh("sleep 30"), Duration::from_millis(100)).unwrap_err();
        assert_eq!(
            failure,
            Failure::TimedOut {
                limit: Duration::from_millis(100)
            }
        );
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn the_child_path_starts_with_weavers_folder_then_omps_and_buns() {
        let exec = SystemExec::new(
            "/data/bin".into(),
            Some("/home/u/.bun/bin".into()),
            "/usr/bin:/bin".into(),
        );
        let path = exec.child_path(Path::new("/opt/omp/bin/omp"));
        let folders: Vec<PathBuf> = std::env::split_paths(&path).collect();
        assert_eq!(
            folders,
            [
                PathBuf::from("/data/bin"),
                PathBuf::from("/opt/omp/bin"),
                PathBuf::from("/home/u/.bun/bin"),
                PathBuf::from("/usr/bin"),
                PathBuf::from("/bin"),
            ]
        );
    }
}

#[cfg(all(test, unix))]
mod certificate_tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;
    use crate::system_certificates::variables;

    const PRINT: [&str; 2] = [
        "-c",
        "printf '%s|%s|%s' \"$NODE_EXTRA_CA_CERTS\" \"$PI_REQUEST_COMPRESSION\" \"$SSL_CERT_FILE\"",
    ];

    fn exec(source: Option<CertificateSource>) -> SystemExec {
        let exec = SystemExec::new(PathBuf::new(), None, "/usr/bin:/bin".into());
        match source {
            Some(source) => exec.with_certificates(source),
            None => exec,
        }
    }

    #[test]
    fn an_omp_command_gets_the_variables_of_the_moment_over_the_inherited_ones() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("system-certificates.pem");
        std::fs::write(&bundle, "x").unwrap();
        let on = Arc::new(AtomicBool::new(true));
        let source: CertificateSource = {
            let (on, bundle) = (Arc::clone(&on), bundle.clone());
            Arc::new(move || {
                let enabled = on.load(Ordering::SeqCst);
                Certificates {
                    enabled,
                    variables: variables(enabled, &bundle),
                }
            })
        };
        let exec = exec(Some(source));
        let run = |exec: &SystemExec| {
            exec.run(Path::new("/bin/sh"), &PRINT, Duration::from_secs(10))
                .unwrap()
                .stdout
        };
        let bundle_text = bundle.to_str().unwrap();
        assert_eq!(
            run(&exec),
            format!("{bundle_text}|0|{bundle_text}"),
            "the setting is on"
        );
        assert!(exec.system_certificates_on());

        // Read at each start: turned off, the next command gets nothing.
        on.store(false, Ordering::SeqCst);
        assert_eq!(run(&exec), "||");
        assert!(!exec.system_certificates_on());
    }

    #[test]
    fn the_commands_that_are_not_omps_get_no_certificate_variable() {
        let exec = exec(None);
        let out = exec
            .run(Path::new("/bin/sh"), &PRINT, Duration::from_secs(10))
            .unwrap();
        assert_eq!(out.stdout.matches('|').count(), 2);
        assert!(!exec.system_certificates_on());
    }
}
