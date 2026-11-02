//! The arguments of a launch of Weaver (the first one, or a second one handed to the running
//! instance by the single-instance plugin).
//!
//! A folder as argument is `developer.OPEN_PROJECT`; anything else brings the most recently used
//! window forward.

use std::path::{Path, PathBuf};

/// The folder a launch asks to open: the first argument after the program name that is not an
/// option and names a folder (a relative one is taken from `cwd`). `None` when there is none.
pub fn folder_argument(args: &[String], cwd: &Path) -> Option<PathBuf> {
    args.iter()
        .skip(1)
        .filter(|arg| !arg.is_empty() && !arg.starts_with('-'))
        .map(|arg| {
            let path = Path::new(arg);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                cwd.join(path)
            }
        })
        .find(|path| path.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn an_absolute_folder_is_the_project_to_open() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().to_string_lossy().into_owned();
        assert_eq!(
            folder_argument(&args(&["weaver", &folder]), Path::new("/elsewhere")),
            Some(dir.path().to_path_buf())
        );
    }

    #[test]
    fn a_relative_folder_is_taken_from_the_working_folder_of_the_launch() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("my project")).unwrap();
        assert_eq!(
            folder_argument(&args(&["weaver", "my project"]), dir.path()),
            Some(dir.path().join("my project"))
        );
        assert_eq!(
            folder_argument(&args(&["weaver", "."]), dir.path()),
            Some(dir.path().join("."))
        );
    }

    #[test]
    fn no_folder_means_the_most_recent_window_comes_forward() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("file.txt"), "x").unwrap();
        for launch in [
            args(&["weaver"]),
            args(&["weaver", "--flag"]),
            args(&["weaver", "missing"]),
            args(&["weaver", "file.txt"]),
            args(&["weaver", ""]),
        ] {
            assert_eq!(folder_argument(&launch, dir.path()), None, "{launch:?}");
        }
    }

    #[test]
    fn the_program_name_is_never_a_folder_and_options_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().to_string_lossy().into_owned();
        assert_eq!(
            folder_argument(&args(&[&folder]), Path::new("/")),
            None,
            "the first argument is the program"
        );
        assert_eq!(
            folder_argument(&args(&["weaver", "--verbose", &folder]), Path::new("/")),
            Some(dir.path().to_path_buf())
        );
    }
}
