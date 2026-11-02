//! Opening the system terminal for omp's installer and login (`requirements/tech#omp-setup`).
//!
//! The command runs where the developer sees it and the terminal stays open at its end. These
//! launches do pass through a shell, so [`quote_posix`], [`quote_powershell`] and
//! [`applescript_string`] are the one place text is quoted for a target; the only text inserted
//! into a command is the absolute path of omp and the id of a provider of the fixed list
//! ([`Provider`]).
//!
//! | System  | Launch |
//! |---------|--------|
//! | macOS   | `/usr/bin/osascript` with fixed `-e` lines: Terminal runs `do script "<command>"` and comes forward |
//! | Windows | `powershell.exe -NoExit -NoProfile -ExecutionPolicy Bypass -Command <command>` in a new console |
//! | Linux   | the first available of `x-terminal-emulator -e`, `gnome-terminal --`, `konsole -e`, `xfce4-terminal -x`, `xterm -e`, running `sh -c '<command>; printf "\nPress Enter to close"; read _'` |

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::lookup::Os;
use super::providers::Provider;

/// The official installer for macOS and Linux.
pub const INSTALLER_UNIX: &str = "curl -fsSL https://omp.sh/install | sh";
/// The official installer for Windows.
pub const INSTALLER_WINDOWS: &str = "irm https://omp.sh/install.ps1 | iex";

/// The emulators tried on Linux, in order, with the flag that precedes the command.
pub const LINUX_TERMINALS: [(&str, &str); 5] = [
    ("x-terminal-emulator", "-e"),
    ("gnome-terminal", "--"),
    ("konsole", "-e"),
    ("xfce4-terminal", "-x"),
    ("xterm", "-e"),
];

/// How long `osascript` may take to open Terminal.
const OSASCRIPT_LIMIT: Duration = Duration::from_secs(15);

#[cfg(windows)]
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TerminalError {
    #[error("the path of omp is not absolute")]
    NotAbsolute,
    #[error("the path of omp holds a line break or a quote that cannot be passed to a terminal")]
    UnsafePath,
    #[error("no terminal emulator was found")]
    NoTerminal,
    #[error("{0}")]
    Launch(String),
}

/// `text` in single quotes for `sh`, `bash`, `zsh`, `dash` and `fish`: a quote inside becomes
/// `'\''`.
pub fn quote_posix(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// `text` in single quotes for PowerShell: a quote inside is doubled. PowerShell also reads the
/// typographic single quotes as quotes, so they are doubled too.
pub fn quote_powershell(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('\'');
    for c in text.chars() {
        quoted.push(c);
        if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}') {
            quoted.push(c);
        }
    }
    quoted.push('\'');
    quoted
}

/// `text` as an AppleScript string literal: `\` and `"` are escaped.
pub fn applescript_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', r"\\").replace('"', r#"\""#))
}

/// The installer command for `os`.
pub fn installer_command(os: Os) -> &'static str {
    if os == Os::Windows {
        INSTALLER_WINDOWS
    } else {
        INSTALLER_UNIX
    }
}

/// `omp login <provider>` for `os`, with the absolute path of omp quoted for the shell of the
/// terminal (PowerShell on Windows, a POSIX-like shell elsewhere).
pub fn login_command(os: Os, omp: &Path, provider: &Provider) -> Result<String, TerminalError> {
    let text = omp.to_str().ok_or(TerminalError::UnsafePath)?;
    if !is_absolute_on(os, text) {
        return Err(TerminalError::NotAbsolute);
    }
    if text
        .chars()
        .any(|c| c.is_control() || (os == Os::Windows && c == '"'))
    {
        return Err(TerminalError::UnsafePath);
    }
    Ok(match os {
        Os::Windows => format!("& {} login {}", quote_powershell(text), provider.id),
        Os::MacOs | Os::Linux => format!("{} login {}", quote_posix(text), provider.id),
    })
}

/// Whether `text` is an absolute path on `os` (a drive letter or a UNC prefix on Windows),
/// whichever system Weaver runs on.
fn is_absolute_on(os: Os, text: &str) -> bool {
    match os {
        Os::Windows => {
            let bytes = text.as_bytes();
            let drive = bytes.len() >= 3
                && bytes[0].is_ascii_alphabetic()
                && bytes[1] == b':'
                && matches!(bytes[2], b'\\' | b'/');
            drive || text.starts_with("\\\\")
        }
        Os::MacOs | Os::Linux => text.starts_with('/'),
    }
}

/// A terminal to start: the program, its arguments and whether it needs its own console.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    /// Windows: the process is created with `CREATE_NEW_CONSOLE`.
    pub new_console: bool,
}

fn os_args(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

/// How to run `command` in a terminal of `os`. `which` finds a program of the Linux list by name.
pub fn launch_plan(
    os: Os,
    command: &str,
    which: &dyn Fn(&str) -> Option<PathBuf>,
) -> Result<Launch, TerminalError> {
    match os {
        Os::MacOs => {
            let run = format!("do script {}", applescript_string(command));
            Ok(Launch {
                program: PathBuf::from("/usr/bin/osascript"),
                args: os_args(&[
                    "-e",
                    "tell application \"Terminal\"",
                    "-e",
                    "activate",
                    "-e",
                    &run,
                    "-e",
                    "end tell",
                ]),
                new_console: false,
            })
        }
        Os::Windows => Ok(Launch {
            program: PathBuf::from("powershell.exe"),
            args: os_args(&[
                "-NoExit",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                command,
            ]),
            new_console: true,
        }),
        Os::Linux => {
            let script = format!("{command}; printf \"\\nPress Enter to close\"; read _");
            let (program, flag) = LINUX_TERMINALS
                .iter()
                .find_map(|(name, flag)| which(name).map(|found| (found, *flag)))
                .ok_or(TerminalError::NoTerminal)?;
            Ok(Launch {
                program,
                args: os_args(&[flag, "sh", "-c", &script]),
                new_console: false,
            })
        }
    }
}

/// Starts the terminal. On macOS it waits for `osascript` (which fails when the developer has not
/// allowed Weaver to control Terminal); elsewhere the terminal is left running and reaped in a
/// thread.
pub fn open(os: Os, launch: &Launch) -> Result<(), TerminalError> {
    let mut command = crate::process::command(&launch.program, &launch.args);
    #[cfg(windows)]
    if launch.new_console {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NEW_CONSOLE);
    }
    if os == Os::MacOs {
        return super::run::run_command(command, OSASCRIPT_LIMIT)
            .map(|_| ())
            .map_err(|failure| TerminalError::Launch(failure.reason()));
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|e| TerminalError::Launch(e.to_string()))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::omp::providers::provider;

    fn anthropic() -> &'static Provider {
        provider("anthropic").unwrap()
    }

    fn linux_with(available: &'static [&'static str]) -> impl Fn(&str) -> Option<PathBuf> {
        move |name| {
            available
                .contains(&name)
                .then(|| PathBuf::from("/usr/bin").join(name))
        }
    }

    fn strings(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn posix_quoting_wraps_in_single_quotes_and_escapes_quotes() {
        assert_eq!(quote_posix("/usr/bin/omp"), "'/usr/bin/omp'");
        assert_eq!(quote_posix("/Users/o'neil/omp"), r"'/Users/o'\''neil/omp'");
        assert_eq!(
            quote_posix("/a b/$HOME/`id`/\"x\"/;rm"),
            "'/a b/$HOME/`id`/\"x\"/;rm'"
        );
        assert_eq!(quote_posix(""), "''");
        assert_eq!(quote_posix("''"), r"''\'''\'''");
    }

    #[test]
    fn powershell_quoting_doubles_every_kind_of_single_quote() {
        assert_eq!(
            quote_powershell(r"C:\Program Files\omp\omp.exe"),
            r"'C:\Program Files\omp\omp.exe'"
        );
        assert_eq!(
            quote_powershell("C:\\o'neil\\omp.exe"),
            "'C:\\o''neil\\omp.exe'"
        );
        assert_eq!(quote_powershell("a\u{2019}b"), "'a\u{2019}\u{2019}b'");
        assert_eq!(quote_powershell("$env:X `n $(calc)"), "'$env:X `n $(calc)'");
    }

    #[test]
    fn applescript_quoting_escapes_backslash_and_double_quote() {
        assert_eq!(applescript_string("echo hi"), "\"echo hi\"");
        assert_eq!(
            applescript_string(r#"say "hi" \ done"#),
            r#""say \"hi\" \\ done""#
        );
    }

    #[test]
    fn the_installers_are_the_official_ones() {
        assert_eq!(
            installer_command(Os::MacOs),
            "curl -fsSL https://omp.sh/install | sh"
        );
        assert_eq!(
            installer_command(Os::Linux),
            "curl -fsSL https://omp.sh/install | sh"
        );
        assert_eq!(
            installer_command(Os::Windows),
            "irm https://omp.sh/install.ps1 | iex"
        );
    }

    #[test]
    fn the_login_command_quotes_the_path_for_each_shell() {
        assert_eq!(
            login_command(Os::MacOs, Path::new("/Users/o'neil/bin/omp"), anthropic()).unwrap(),
            r"'/Users/o'\''neil/bin/omp' login anthropic"
        );
        assert_eq!(
            login_command(
                Os::Linux,
                Path::new("/home/a b/omp"),
                provider("openai-codex").unwrap()
            )
            .unwrap(),
            "'/home/a b/omp' login openai-codex"
        );
        assert_eq!(
            login_command(
                Os::Windows,
                Path::new(r"C:\Users\o'neil\omp.exe"),
                provider("cursor").unwrap()
            )
            .unwrap(),
            r"& 'C:\Users\o''neil\omp.exe' login cursor"
        );
    }

    #[test]
    fn a_path_that_is_not_absolute_or_holds_a_line_break_is_refused() {
        assert_eq!(
            login_command(Os::Linux, Path::new("omp"), anthropic()),
            Err(TerminalError::NotAbsolute)
        );
        assert_eq!(
            login_command(
                Os::Linux,
                Path::new("/a\n; touch /tmp/x\n/omp"),
                anthropic()
            ),
            Err(TerminalError::UnsafePath)
        );
        assert_eq!(
            login_command(Os::MacOs, Path::new("/a\r/omp"), anthropic()),
            Err(TerminalError::UnsafePath)
        );
    }

    #[test]
    fn macos_runs_the_command_in_terminal_through_osascript_with_fixed_lines() {
        let launch = launch_plan(Os::MacOs, r#"'/o"mp' login anthropic"#, &|_| None).unwrap();
        assert_eq!(launch.program, PathBuf::from("/usr/bin/osascript"));
        assert_eq!(
            strings(&launch.args),
            [
                "-e",
                "tell application \"Terminal\"",
                "-e",
                "activate",
                "-e",
                r#"do script "'/o\"mp' login anthropic""#,
                "-e",
                "end tell"
            ]
        );
        assert!(!launch.new_console);
    }

    #[test]
    fn windows_runs_powershell_51_in_a_new_console_that_stays_open() {
        let launch = launch_plan(Os::Windows, "irm https://omp.sh/install.ps1 | iex", &|_| {
            None
        })
        .unwrap();
        assert_eq!(launch.program, PathBuf::from("powershell.exe"));
        assert_eq!(
            strings(&launch.args),
            [
                "-NoExit",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "irm https://omp.sh/install.ps1 | iex"
            ]
        );
        assert!(launch.new_console);
    }

    #[test]
    fn linux_uses_the_first_emulator_of_the_list_that_exists() {
        let command = "'/home/a/omp' login anthropic";
        let launch = launch_plan(Os::Linux, command, &linux_with(&["xterm", "konsole"])).unwrap();
        assert_eq!(launch.program, PathBuf::from("/usr/bin/konsole"));
        assert_eq!(
            strings(&launch.args),
            [
                "-e",
                "sh",
                "-c",
                "'/home/a/omp' login anthropic; printf \"\\nPress Enter to close\"; read _"
            ]
        );
        for (name, flag) in LINUX_TERMINALS {
            let only: &'static [&'static str] = Box::leak(Box::new([name]));
            let launch = launch_plan(Os::Linux, command, &linux_with(only)).unwrap();
            assert_eq!(launch.program, PathBuf::from("/usr/bin").join(name));
            assert_eq!(strings(&launch.args)[0], flag);
        }
    }

    #[test]
    fn linux_without_a_terminal_cannot_open_one() {
        assert_eq!(
            launch_plan(Os::Linux, "x", &|_| None),
            Err(TerminalError::NoTerminal)
        );
    }
}
