//! The folder tree of `Import from Code` (`requirements/ui#project`): the entries of one folder of
//! the project, leaving out names that start with a dot and everything git ignores.
//!
//! The tree shows the first level and reads the children of a folder when it is opened, so the
//! app asks for one folder at a time with [`list`]. Paths travel relative to the project folder,
//! with `/` on every system; they are the same text the import job receives as excluded paths.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Serialize;
use ts_rs::TS;

use crate::process;

/// One entry of a folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
    pub name: String,
    /// Relative to the project folder, with `/`.
    pub path: String,
    /// A folder (it has children to read when opened); otherwise a file.
    pub folder: bool,
}

/// The folder `rel` of the project at `root` made of its parts; `Err` when `rel` leaves the project
/// or names something that is never listed.
fn folder_of(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let mut folder = root.to_path_buf();
    for part in rel.split('/').filter(|part| !part.is_empty()) {
        if part.starts_with('.') || part.contains('\\') || part.contains(':') {
            return Err(format!("`{rel}` is not a folder of the project"));
        }
        folder.push(part);
    }
    Ok(folder)
}

/// The entries of the folder `rel` (empty for the project folder), folders first, then by name
/// (case-insensitive). Names starting with a dot and what git ignores are left out; outside a git
/// repository, or with no git, nothing is ignored.
pub fn list(root: &Path, rel: &str) -> Result<Vec<TreeEntry>, String> {
    let folder = folder_of(root, rel)?;
    let canonical_root =
        fs::canonicalize(root).map_err(|e| format!("the project folder cannot be read: {e}"))?;
    let canonical =
        fs::canonicalize(&folder).map_err(|e| format!("`{rel}` cannot be read: {e}"))?;
    if !canonical.starts_with(&canonical_root) || !canonical.is_dir() {
        return Err(format!("`{rel}` is not a folder of the project"));
    }
    let base = rel.trim_matches('/');
    let mut entries: Vec<TreeEntry> = Vec::new();
    for item in fs::read_dir(&folder)
        .map_err(|e| format!("`{rel}` cannot be read: {e}"))?
        .filter_map(Result::ok)
    {
        let Ok(name) = item.file_name().into_string() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let folder = item.path().is_dir();
        let path = if base.is_empty() {
            name.clone()
        } else {
            format!("{base}/{name}")
        };
        entries.push(TreeEntry { name, path, folder });
    }
    let ignored = git_ignored(root, &entries);
    entries.retain(|entry| !ignored.contains(&entry.path));
    entries.sort_by(|a, b| {
        b.folder
            .cmp(&a.folder)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(entries)
}

/// The paths among `entries` that git ignores; none when git cannot say (not a repository, no git).
fn git_ignored(root: &Path, entries: &[TreeEntry]) -> Vec<String> {
    let paths: Vec<&str> = entries.iter().map(|entry| entry.path.as_str()).collect();
    git_ignored_paths(root, &paths)
}

/// The paths among `paths` (relative to `root`, with `/`) that git ignores, with one
/// `git check-ignore -z --stdin`; none when git cannot say (not a repository, no git). Also what
/// the File view's listing leaves out.
pub(crate) fn git_ignored_paths(root: &Path, paths: &[&str]) -> Vec<String> {
    if paths.is_empty() {
        return Vec::new();
    }
    let mut input: Vec<u8> = Vec::new();
    for path in paths {
        input.extend_from_slice(path.as_bytes());
        input.push(0);
    }
    let Ok(mut child) = process::command("git", ["check-ignore", "-z", "--stdin"])
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Vec::new();
    };
    let Some(mut stdin) = child.stdin.take() else {
        return Vec::new();
    };
    // Written on its own thread: a long list must not fill the pipe while the answer is unread.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
    let output = child.wait_with_output();
    let _ = writer.join();
    match output {
        // 0: some path is ignored; 1: none is; anything else (128): git cannot say.
        Ok(output) if matches!(output.status.code(), Some(0)) => output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|part| !part.is_empty())
            .filter_map(|part| String::from_utf8(part.to_vec()).ok())
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(root: &Path, rel: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "x").unwrap();
    }

    fn names(entries: &[TreeEntry]) -> Vec<(&str, bool)> {
        entries
            .iter()
            .map(|e| (e.name.as_str(), e.folder))
            .collect()
    }

    fn git_is_available() -> bool {
        process::command("git", ["--version"])
            .output()
            .is_ok_and(|o| o.status.success())
    }

    #[test]
    fn the_first_level_lists_folders_then_files_without_dot_names() {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), "src/main.rs");
        put(dir.path(), "docs/a.md");
        put(dir.path(), ".weaver/model/system.md");
        put(dir.path(), ".env");
        put(dir.path(), "README.md");
        put(dir.path(), "Zeta.txt");
        put(dir.path(), "alpha.txt");
        let entries = list(dir.path(), "").unwrap();
        assert_eq!(
            names(&entries),
            [
                ("docs", true),
                ("src", true),
                ("alpha.txt", false),
                ("README.md", false),
                ("Zeta.txt", false)
            ]
        );
        assert_eq!(entries[0].path, "docs");
    }

    #[test]
    fn children_are_read_when_a_folder_is_opened_with_paths_from_the_project_folder() {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), "src/ui/app.tsx");
        put(dir.path(), "src/lib.rs");
        let entries = list(dir.path(), "src").unwrap();
        assert_eq!(
            entries.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(),
            ["src/ui", "src/lib.rs"]
        );
        assert_eq!(
            list(dir.path(), "src/ui/").unwrap()[0].path,
            "src/ui/app.tsx"
        );
    }

    #[test]
    fn a_path_that_leaves_the_project_or_names_a_dot_folder_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), ".git/config");
        put(dir.path(), "a/b.txt");
        for bad in [
            "..",
            "a/../..",
            ".git",
            "a/.hidden",
            "missing",
            "a/b.txt",
            "C:\\x",
            "a\\b",
        ] {
            assert!(list(dir.path(), bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn what_git_ignores_is_left_out() {
        if !git_is_available() {
            eprintln!("skipped: git is not installed");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        assert!(process::command("git", ["init", "-q"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success());
        fs::write(
            dir.path().join(".gitignore"),
            "node_modules/\n*.log\nbuild/out\n",
        )
        .unwrap();
        put(dir.path(), "node_modules/pkg/index.js");
        put(dir.path(), "src/main.rs");
        put(dir.path(), "debug.log");
        put(dir.path(), "build/out/a.o");
        put(dir.path(), "build/keep.txt");
        assert_eq!(
            names(&list(dir.path(), "").unwrap()),
            [("build", true), ("src", true)]
        );
        assert_eq!(
            names(&list(dir.path(), "build").unwrap()),
            [("keep.txt", false)]
        );
    }

    #[test]
    fn a_folder_that_is_no_repository_ignores_nothing() {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), "node_modules/a.js");
        assert_eq!(
            names(&list(dir.path(), "").unwrap()),
            [("node_modules", true)]
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_folder_outside_the_project_is_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        put(outside.path(), "secret.txt");
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();
        assert!(list(dir.path(), "link").is_err());
    }
}
