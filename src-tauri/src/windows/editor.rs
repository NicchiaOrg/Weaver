//! `Open in editor`: opens a file at a line in the editor of the settings (`app` feature).
//!
//! The argument list comes from [`Settings::editor_command`] (preset or custom command, `{file}`
//! and `{line}` filled, never a shell). The program is looked for on the `PATH` and, because an
//! app started from the Dock or the Start menu does not get the `PATH` of a terminal, in the folders
//! where the editors' command line tools are usually installed. It is started through
//! [`weaver_core::process::command`], with no console and no input; a failure comes back as the
//! reason the UI shows in a notice.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use weaver_core::process;
use weaver_core::settings::Settings;

use crate::log_warn;

/// How long a freshly started editor is watched for an immediate failure (a command that exits at
/// once with an error). An editor still running after this started well.
pub const GRACE: Duration = Duration::from_millis(1500);

/// Where programs are looked for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProgramSearch {
    /// The value of `PATH`.
    pub path: Option<OsString>,
    /// Folders tried after `PATH`.
    pub extra_dirs: Vec<PathBuf>,
    /// Extensions tried after the bare name (Windows `PATHEXT`, lowercase, with the dot).
    pub extensions: Vec<String>,
}

impl ProgramSearch {
    /// The search of this process on this system.
    pub fn of_process() -> Self {
        let home =
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
        let mut extra: Vec<PathBuf> = Vec::new();
        if cfg!(target_os = "macos") {
            extra.extend(
                [
                    "/opt/homebrew/bin",
                    "/usr/local/bin",
                    "/Applications/Visual Studio Code.app/Contents/Resources/app/bin",
                    "/Applications/Cursor.app/Contents/Resources/app/bin",
                    "/Applications/Sublime Text.app/Contents/SharedSupport/bin",
                ]
                .map(PathBuf::from),
            );
            if let Some(home) = &home {
                extra.push(home.join("Library/Application Support/JetBrains/Toolbox/scripts"));
            }
        } else if cfg!(unix) {
            extra.extend(["/usr/local/bin", "/snap/bin"].map(PathBuf::from));
        }
        if cfg!(unix) {
            if let Some(home) = &home {
                extra.push(home.join(".local/bin"));
            }
        }
        let extensions = if cfg!(windows) {
            std::env::var("PATHEXT")
                .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned())
                .split(';')
                .filter(|ext| !ext.is_empty())
                .map(str::to_ascii_lowercase)
                .collect()
        } else {
            Vec::new()
        };
        Self {
            path: std::env::var_os("PATH"),
            extra_dirs: extra,
            extensions,
        }
    }

    /// The file `program` names: itself when it has a folder part, else the first match in `PATH`
    /// and then in the extra folders.
    pub fn resolve(&self, program: &str) -> Option<PathBuf> {
        let given = Path::new(program);
        if given.components().count() > 1 || given.is_absolute() {
            return given.is_file().then(|| given.to_path_buf());
        }
        let from_path = self.path.iter().flat_map(std::env::split_paths);
        for dir in from_path.chain(self.extra_dirs.iter().cloned()) {
            if dir.as_os_str().is_empty() {
                continue;
            }
            let plain = dir.join(program);
            if is_launchable(&plain) {
                return Some(plain);
            }
            for extension in &self.extensions {
                let with = dir.join(format!("{program}{extension}"));
                if with.is_file() {
                    return Some(with);
                }
            }
        }
        None
    }
}

#[cfg(unix)]
fn is_launchable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_launchable(path: &Path) -> bool {
    path.is_file()
}

/// Opens `file` (absolute) at `line` (the first line when unknown) in the editor of `settings`.
/// `Err` is the sentence to show the developer.
pub fn open(
    settings: &Settings,
    file: &Path,
    line: Option<u32>,
    search: &ProgramSearch,
    grace: Duration,
) -> Result<(), String> {
    let argv = settings
        .editor_command(&file.to_string_lossy(), line)
        .map_err(|error| format!("The editor setting cannot be used: {error}"))?;
    let (program, arguments) = argv
        .split_first()
        .ok_or_else(|| "The editor setting has no program.".to_owned())?;
    let resolved = search.resolve(program).ok_or_else(|| {
        format!("The editor `{program}` was not found. Choose another editor in the settings, or a command with the full path of the program.")
    })?;

    let mut child = process::command(&resolved, arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("The editor `{program}` could not be started: {error}"))?;

    let deadline = Instant::now() + grace;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                return Err(match status.code() {
                    Some(code) => format!("The editor `{program}` exited with code {code}."),
                    None => format!("The editor `{program}` was stopped."),
                });
            }
            Ok(None) if Instant::now() >= deadline => break,
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                return Err(format!(
                    "The editor `{program}` could not be watched: {error}"
                ))
            }
        }
    }
    // Still running: it started. Reap it when it ends, so no zombie stays behind.
    let program = program.clone();
    let reaper = std::thread::Builder::new()
        .name("editor-reaper".to_owned())
        .spawn(move || {
            if let Ok(status) = child.wait() {
                if !status.success() {
                    log_warn!("the editor `{program}` ended with {status}");
                }
            }
        });
    if let Err(error) = reaper {
        log_warn!("the editor process is not reaped: {error}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use weaver_core::settings::EditorPreset;

    fn search_in(dir: &Path) -> ProgramSearch {
        ProgramSearch {
            path: Some(std::env::join_paths([dir]).unwrap()),
            ..ProgramSearch::default()
        }
    }

    #[test]
    fn a_program_is_found_on_the_path_then_in_the_extra_folders() {
        let on_path = tempfile::tempdir().unwrap();
        let extra = tempfile::tempdir().unwrap();
        write_program(&on_path.path().join("here"), "exit 0");
        write_program(&extra.path().join("there"), "exit 0");
        write_program(&on_path.path().join("both"), "exit 0");
        write_program(&extra.path().join("both"), "exit 0");
        let search = ProgramSearch {
            extra_dirs: vec![extra.path().to_path_buf()],
            ..search_in(on_path.path())
        };
        assert_eq!(search.resolve("here"), Some(on_path.path().join("here")));
        assert_eq!(search.resolve("there"), Some(extra.path().join("there")));
        assert_eq!(
            search.resolve("both"),
            Some(on_path.path().join("both")),
            "the PATH comes first"
        );
        assert_eq!(search.resolve("missing"), None);
    }

    #[test]
    fn an_extension_of_the_system_is_tried_after_the_bare_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("code.cmd"), "").unwrap();
        let search = ProgramSearch {
            extensions: vec![".exe".into(), ".cmd".into()],
            ..search_in(dir.path())
        };
        assert_eq!(search.resolve("code"), Some(dir.path().join("code.cmd")));
        assert_eq!(
            ProgramSearch {
                extensions: vec![],
                ..search
            }
            .resolve("code"),
            None
        );
    }

    #[test]
    fn a_program_with_a_folder_part_is_used_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("my editor");
        write_program(&program, "exit 0");
        let search = ProgramSearch::default();
        assert_eq!(search.resolve(&program.to_string_lossy()), Some(program));
        assert_eq!(
            search.resolve(&dir.path().join("nope").to_string_lossy()),
            None
        );
    }

    #[test]
    fn a_setting_that_cannot_be_used_says_why() {
        let mut settings = Settings::default();
        settings.editor.preset = EditorPreset::Custom;
        settings.editor.command = String::new();
        let error = open(
            &settings,
            Path::new("/a.rs"),
            None,
            &ProgramSearch::default(),
            Duration::ZERO,
        )
        .unwrap_err();
        assert_eq!(
            error,
            "The editor setting cannot be used: The command must contain {file}."
        );
    }

    #[test]
    fn an_editor_that_is_not_installed_is_named() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = Settings::default();
        settings.editor.preset = EditorPreset::Zed;
        let error = open(
            &settings,
            Path::new("/a.rs"),
            Some(3),
            &search_in(dir.path()),
            Duration::ZERO,
        )
        .unwrap_err();
        assert!(
            error.starts_with("The editor `zed` was not found."),
            "{error}"
        );
    }

    /// A script that is the "editor": it writes its arguments, one per line, to `args.txt` next to it.
    #[cfg(unix)]
    fn write_program(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(not(unix))]
    fn write_program(path: &Path, _body: &str) {
        std::fs::write(path, "").unwrap();
    }

    // These tests run a shell script as the editor: Windows has no equivalent without a shell, and
    // the argument construction is covered by the settings tests of `weaver-core` on every OS.
    #[cfg(unix)]
    mod launching {
        use super::*;

        fn recording_editor(dir: &Path, name: &str, tail: &str) -> PathBuf {
            let program = dir.join(name);
            write_program(
                &program,
                &format!(
                    "printf '%s\\n' \"$@\" > \"{}\"\n{tail}",
                    dir.join("args.txt").display()
                ),
            );
            program
        }

        #[test]
        fn each_preset_is_launched_with_the_file_and_the_line() {
            for (preset, program, expected) in [
                (EditorPreset::Vscode, "code", vec!["--goto", "FILE:12"]),
                (EditorPreset::Cursor, "cursor", vec!["--goto", "FILE:12"]),
                (EditorPreset::Zed, "zed", vec!["FILE:12"]),
                (
                    EditorPreset::Jetbrains,
                    "idea",
                    vec!["--line", "12", "FILE"],
                ),
                (EditorPreset::Sublime, "subl", vec!["FILE:12"]),
            ] {
                let dir = tempfile::tempdir().unwrap();
                recording_editor(dir.path(), program, "");
                let mut settings = Settings::default();
                settings.editor.preset = preset;
                let file = dir.path().join("my project").join("src file.rs");
                open(&settings, &file, Some(12), &search_in(dir.path()), GRACE).unwrap();

                let written = std::fs::read_to_string(dir.path().join("args.txt")).unwrap();
                let file = file.to_string_lossy().into_owned();
                let expected: Vec<String> = expected
                    .iter()
                    .map(|arg| arg.replace("FILE", &file))
                    .collect();
                assert_eq!(written.lines().collect::<Vec<_>>(), expected, "{preset:?}");
            }
        }

        #[test]
        fn a_custom_command_is_launched_without_a_shell() {
            let dir = tempfile::tempdir().unwrap();
            let program = recording_editor(dir.path(), "my editor", "");
            let mut settings = Settings::default();
            settings.editor.preset = EditorPreset::Custom;
            settings.editor.command = format!(
                "'{}' --at={{file}}#{{line}} $HOME;touch /tmp/never",
                program.display()
            );
            let file = dir.path().join("a b.md");
            open(&settings, &file, None, &ProgramSearch::default(), GRACE).unwrap();
            let written = std::fs::read_to_string(dir.path().join("args.txt")).unwrap();
            assert_eq!(
                written.lines().collect::<Vec<_>>(),
                [
                    format!("--at={}#1", file.display()),
                    "$HOME;touch".to_owned(),
                    "/tmp/never".to_owned()
                ],
                "a missing line is line 1 and nothing is expanded"
            );
        }

        #[test]
        fn an_editor_that_fails_at_once_gives_its_exit_code_as_the_reason() {
            let dir = tempfile::tempdir().unwrap();
            recording_editor(dir.path(), "code", "exit 3");
            let mut settings = Settings::default();
            settings.editor.preset = EditorPreset::Vscode;
            let error = open(
                &settings,
                &dir.path().join("a.rs"),
                Some(1),
                &search_in(dir.path()),
                GRACE,
            )
            .unwrap_err();
            assert_eq!(error, "The editor `code` exited with code 3.");
        }

        #[test]
        fn an_editor_that_keeps_running_is_a_success() {
            let dir = tempfile::tempdir().unwrap();
            recording_editor(dir.path(), "code", "sleep 1");
            let mut settings = Settings::default();
            settings.editor.preset = EditorPreset::Vscode;
            open(
                &settings,
                &dir.path().join("a.rs"),
                Some(1),
                &search_in(dir.path()),
                Duration::from_millis(100),
            )
            .unwrap();
        }
    }
}
