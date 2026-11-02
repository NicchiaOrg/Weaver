//! The single function every child process of Weaver and of the `weaver` CLI is started with.
//!
//! A child starts without a shell, from a program path and an array of arguments; no user text
//! is ever interpolated into a command line. On Windows it gets `CREATE_NO_WINDOW`.
//! The few commands that must pass through a shell (terminal launches of the omp setup) quote
//! their text with their own tested function and still start through [`command`].

use std::ffi::OsStr;
use std::process::Command;

/// `CREATE_NO_WINDOW` process creation flag of Windows.
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Builds the command for `program` with `args`. Callers add environment, working folder and
/// stdio, then spawn it; nothing here goes through a shell.
pub fn command<P, I, A>(program: P, args: I) -> Command
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = A>,
    A: AsRef<OsStr>,
{
    let mut command = Command::new(program);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unix only: it needs /bin/echo. Windows has no equivalent binary with the same argument
    // semantics, and the shell-free guarantee is the same `Command` code path.
    #[cfg(unix)]
    #[test]
    fn arguments_are_passed_literally_without_a_shell() {
        let hostile = "$(touch /tmp/should-not-exist); `id` && echo *";
        let output = command("/bin/echo", ["-n", hostile]).output().unwrap();
        assert_eq!(String::from_utf8(output.stdout).unwrap(), hostile);
    }

    #[test]
    fn a_missing_program_fails_to_start_instead_of_falling_back_to_a_shell() {
        assert!(command("/nonexistent/weaver-program", ["x"])
            .output()
            .is_err());
    }
}
