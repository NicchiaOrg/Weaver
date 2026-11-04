//! The PATH omp is looked up and run with (`requirements/tech#omp`, "PATH of a GUI app").
//!
//! * macOS and Linux: at start Weaver reads the login PATH once with
//!   `$SHELL -l -c 'printf %s "$PATH"'` and a 5 s limit. This is the only shell Weaver starts on
//!   its own. On failure it keeps the inherited PATH.
//! * Windows: every lookup reads the current user and machine `Path` from the registry, so a
//!   fresh install is seen without restarting Weaver.
//! * Child PATH: `<app local data>/bin`, omp's folder and bun's bin folder, then the PATH above.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use super::run::{limits, Exec};

/// The arguments that make a login shell print its PATH.
pub const LOGIN_PATH_ARGS: [&str; 3] = ["-l", "-c", "printf %s \"$PATH\""];

/// The PATH in the output of the login shell: the last line, because a profile may print a banner
/// before the command does. `None` when it is empty.
pub fn parse_login_path(stdout: &str) -> Option<OsString> {
    let line = stdout.lines().rfind(|line| !line.trim().is_empty())?.trim();
    Some(OsString::from(line))
}

/// Reads the login PATH of `shell` (`$SHELL`); `None` on any failure (the inherited PATH stays).
pub fn read_login_path(exec: &dyn Exec, shell: &Path) -> Option<OsString> {
    let output = exec.run(shell, &LOGIN_PATH_ARGS, limits::LOGIN_PATH).ok()?;
    parse_login_path(&output.stdout)
}

/// The PATH of this process.
pub fn inherited_path() -> OsString {
    std::env::var_os("PATH").unwrap_or_default()
}

/// The value of `Path` in the output of `reg query <key> /v Path`.
pub fn parse_reg_query_path(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let line = line.trim();
        let rest = line
            .strip_prefix("Path")
            .or_else(|| line.strip_prefix("PATH"))?;
        let rest = rest.trim_start();
        let rest = rest
            .strip_prefix("REG_EXPAND_SZ")
            .or_else(|| rest.strip_prefix("REG_SZ"))?;
        let value = rest.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

/// `value` with each `%NAME%` replaced by what `lookup` says; an unknown name stays as it is.
pub fn expand_percent(value: &str, lookup: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => match lookup(&after[..end]) {
                Some(replacement) => {
                    out.push_str(&replacement);
                    rest = &after[end + 1..];
                }
                None => {
                    out.push('%');
                    rest = after;
                }
            },
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The registry keys of the machine and the current user `Path`, in the order Windows composes
/// them (machine first).
const WINDOWS_KEYS: [&str; 2] = [
    r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
    r"HKCU\Environment",
];

/// The machine and user `Path` of Windows, read through `reg query` (no shell). Falls back to the
/// inherited PATH for what cannot be read.
pub fn read_windows_path(exec: &dyn Exec, system_root: &Path) -> OsString {
    let reg = system_root.join("System32").join("reg.exe");
    let mut parts: Vec<String> = Vec::new();
    for key in WINDOWS_KEYS {
        if let Ok(output) = exec.run(&reg, &["query", key, "/v", "Path"], limits::LOGIN_PATH) {
            if let Some(value) = parse_reg_query_path(&output.stdout) {
                parts.push(expand_percent(&value, &|name| std::env::var(name).ok()));
            }
        }
    }
    if parts.is_empty() {
        return inherited_path();
    }
    OsString::from(parts.join(";"))
}

/// `<app local data>/bin`, omp's folder and bun's bin folder, then `base`.
pub fn child_path(
    local_bin: &Path,
    omp_dir: Option<&Path>,
    bun_bin: Option<&Path>,
    base: &OsStr,
) -> OsString {
    let mut folders: Vec<PathBuf> = vec![local_bin.to_path_buf()];
    folders.extend(omp_dir.map(Path::to_path_buf));
    folders.extend(bun_bin.map(Path::to_path_buf));
    folders.extend(std::env::split_paths(base));
    // A folder with the list separator in its name cannot be put in a PATH; leave it out.
    std::env::join_paths(folders.iter().filter(|f| std::env::join_paths([f]).is_ok()))
        .unwrap_or_else(|_| base.to_owned())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::process::timed::Failure;
    use crate::process::timed::Output;

    use super::*;

    type Call = (PathBuf, Vec<String>, Duration);
    type Answers = Vec<Result<Output, Failure>>;

    struct FakeExec(std::sync::Mutex<(Answers, Vec<Call>)>);

    impl FakeExec {
        fn new(answers: Vec<Result<Output, Failure>>) -> Self {
            Self(std::sync::Mutex::new((answers, Vec::new())))
        }
    }

    impl Exec for FakeExec {
        fn run(&self, program: &Path, args: &[&str], limit: Duration) -> Result<Output, Failure> {
            let mut state = self.0.lock().unwrap();
            state.1.push((
                program.to_path_buf(),
                args.iter().map(|a| a.to_string()).collect(),
                limit,
            ));
            state.0.remove(0)
        }
    }

    fn out(stdout: &str) -> Result<Output, Failure> {
        Ok(Output {
            stdout: stdout.to_owned(),
            stderr: String::new(),
        })
    }

    #[test]
    fn the_login_path_is_the_last_line_of_the_shell_output() {
        assert_eq!(
            parse_login_path("/usr/bin:/bin"),
            Some("/usr/bin:/bin".into())
        );
        assert_eq!(
            parse_login_path("Welcome back\nlast login today\n/opt/bin:/usr/bin"),
            Some("/opt/bin:/usr/bin".into())
        );
        assert_eq!(parse_login_path("  \n"), None);
        assert_eq!(parse_login_path(""), None);
    }

    #[test]
    fn the_login_path_is_read_with_the_fixed_arguments_and_a_five_second_limit() {
        let exec = FakeExec::new(vec![out("/a:/b")]);
        assert_eq!(
            read_login_path(&exec, Path::new("/bin/zsh")),
            Some("/a:/b".into())
        );
        let calls = exec.0.lock().unwrap().1.clone();
        assert_eq!(
            calls,
            [(
                PathBuf::from("/bin/zsh"),
                vec!["-l".into(), "-c".into(), "printf %s \"$PATH\"".into()],
                Duration::from_secs(5)
            )]
        );
    }

    #[test]
    fn a_failed_read_keeps_the_inherited_path() {
        let exec = FakeExec::new(vec![Err(Failure::TimedOut {
            limit: Duration::from_secs(5),
        })]);
        assert_eq!(read_login_path(&exec, Path::new("/bin/zsh")), None);
        let exec = FakeExec::new(vec![out("")]);
        assert_eq!(read_login_path(&exec, Path::new("/bin/zsh")), None);
    }

    #[test]
    fn the_registry_output_is_parsed() {
        let text = "\r\nHKEY_CURRENT_USER\\Environment\r\n    Path    REG_EXPAND_SZ    %USERPROFILE%\\bin;C:\\Tools\r\n\r\n";
        assert_eq!(
            parse_reg_query_path(text),
            Some("%USERPROFILE%\\bin;C:\\Tools".to_owned())
        );
        assert_eq!(
            parse_reg_query_path("ERROR: The system was unable to find the specified registry key"),
            None
        );
        assert_eq!(parse_reg_query_path("    Other    REG_SZ    x"), None);
    }

    #[test]
    fn percent_variables_are_expanded_and_unknown_ones_stay() {
        let lookup = |name: &str| (name == "USERPROFILE").then(|| r"C:\Users\ada".to_owned());
        assert_eq!(
            expand_percent(r"%USERPROFILE%\bin;%NOPE%\x;100%", &lookup),
            r"C:\Users\ada\bin;%NOPE%\x;100%"
        );
        assert_eq!(expand_percent("plain", &lookup), "plain");
        assert_eq!(expand_percent("%%", &lookup), "%%");
    }

    #[test]
    fn the_windows_path_joins_machine_then_user() {
        let exec = FakeExec::new(vec![
            out("    Path    REG_EXPAND_SZ    C:\\Windows\r\n"),
            out("    Path    REG_SZ    C:\\Users\\ada\\bin\r\n"),
        ]);
        let path = read_windows_path(&exec, Path::new(r"C:\Windows"));
        assert_eq!(path, OsString::from(r"C:\Windows;C:\Users\ada\bin"));
        let calls = exec.0.lock().unwrap().1.clone();
        assert!(calls[0].1[1].starts_with("HKLM"));
        assert!(calls[1].1[1].starts_with("HKCU"));
    }

    #[test]
    fn the_child_path_orders_weavers_folder_omps_buns_then_the_base() {
        let path = child_path(
            Path::new("/w/bin"),
            Some(Path::new("/o")),
            Some(Path::new("/b")),
            OsStr::new("/x:/y"),
        );
        let folders: Vec<PathBuf> = std::env::split_paths(&path).collect();
        assert_eq!(
            folders,
            ["/w/bin", "/o", "/b", "/x", "/y"].map(PathBuf::from)
        );
        let bare = child_path(Path::new("/w/bin"), None, None, OsStr::new(""));
        assert_eq!(
            std::env::split_paths(&bare).next(),
            Some(PathBuf::from("/w/bin"))
        );
    }
}
