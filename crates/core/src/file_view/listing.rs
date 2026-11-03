//! Listing one folder of the project (`requirements/tech#file-view`, **Listing**): names and kinds
//! from one `read_dir`, never the content of a file, never a recursion, an index or a search.
//!
//! `.git` is left out by name, and so is what git ignores, asked with one `git check-ignore -z
//! --stdin` per listed folder (`GIT_OPTIONAL_LOCKS=0`); without git, or outside a repository,
//! nothing is ignored. Other names starting with a dot are listed, `.weaver` included. Folders
//! come first, then the rest, each in name order without regard to case.

use std::fs;
use std::path::Path;

use super::{join_rel, FileEntry, FileEntryKind, Refusal};
use crate::file_tab::path;
use crate::project_files::folder_tree::git_ignored_paths;

/// The entries of the folder `rel` (empty for the project root) of the project at `root`, the
/// canonical project root. Refused when the path is not one the File view accepts or is not a
/// folder of the project.
pub fn list(root: &Path, rel: &str) -> Result<Vec<FileEntry>, Refusal> {
    let folder = path::resolve(root, rel)?;
    if !folder.is_dir() {
        return Err(Refusal::new(format!(
            "\"{}\" is not a folder of the project.",
            rel.trim_matches('/')
        )));
    }
    let mut entries: Vec<FileEntry> = Vec::new();
    let read = fs::read_dir(&folder).map_err(|error| {
        Refusal::new(format!(
            "The folder \"{}\" could not be read: {error}",
            rel.trim_matches('/')
        ))
    })?;
    for item in read.filter_map(Result::ok) {
        let Ok(name) = item.file_name().into_string() else {
            continue;
        };
        if name.eq_ignore_ascii_case(".git") {
            continue;
        }
        let Ok(kind) = item.file_type() else {
            continue;
        };
        let kind = if kind.is_symlink() {
            FileEntryKind::Link
        } else if kind.is_dir() {
            FileEntryKind::Folder
        } else {
            FileEntryKind::File
        };
        entries.push(FileEntry {
            path: join_rel(rel, &name),
            name,
            kind,
        });
    }
    let paths: Vec<&str> = entries.iter().map(|entry| entry.path.as_str()).collect();
    let ignored = git_ignored_paths(root, &paths);
    entries.retain(|entry| !ignored.contains(&entry.path));
    entries.sort_by(|a, b| {
        (b.kind == FileEntryKind::Folder)
            .cmp(&(a.kind == FileEntryKind::Folder))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(entries)
}

/// Only the folders of `rel`, for the folder tree of `Move to…`.
pub fn list_folders(root: &Path, rel: &str) -> Result<Vec<FileEntry>, Refusal> {
    let mut entries = list(root, rel)?;
    entries.retain(|entry| entry.kind == FileEntryKind::Folder);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::canonical_root;
    use crate::process;

    fn put(root: &Path, rel: &str) {
        let target = root.join(rel);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, "x").unwrap();
    }

    fn project() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = canonical_root(dir.path()).unwrap();
        (dir, root)
    }

    fn names(entries: &[FileEntry]) -> Vec<(&str, FileEntryKind)> {
        entries
            .iter()
            .map(|entry| (entry.name.as_str(), entry.kind))
            .collect()
    }

    fn git(root: &Path, args: &[&str]) -> bool {
        process::command("git", args.iter().copied())
            .current_dir(root)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    #[test]
    fn folders_come_first_then_files_each_by_name_without_regard_to_case() {
        let (_dir, root) = project();
        put(&root, "b.txt");
        put(&root, "A.txt");
        put(&root, "zeta/inner.txt");
        put(&root, "Alpha/inner.txt");
        let entries = list(&root, "").unwrap();
        assert_eq!(
            names(&entries),
            [
                ("Alpha", FileEntryKind::Folder),
                ("zeta", FileEntryKind::Folder),
                ("A.txt", FileEntryKind::File),
                ("b.txt", FileEntryKind::File),
            ]
        );
        assert_eq!(entries[0].path, "Alpha");
        let inner = list(&root, "Alpha").unwrap();
        assert_eq!(inner[0].path, "Alpha/inner.txt");
    }

    #[test]
    fn git_is_left_out_by_name_and_other_dot_names_are_listed() {
        let (_dir, root) = project();
        fs::create_dir_all(root.join(".git/objects")).unwrap();
        put(&root, ".gitignore");
        put(&root, ".weaver/system.md");
        put(&root, ".env");
        let listed = list(&root, "").unwrap();
        let listed: Vec<&str> = listed.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(listed, [".weaver", ".env", ".gitignore"]);
    }

    #[test]
    fn what_git_ignores_is_left_out_and_without_git_nothing_is() {
        let (_dir, root) = project();
        put(&root, ".gitignore");
        fs::write(root.join(".gitignore"), "target/\n*.log\n").unwrap();
        put(&root, "target/out.bin");
        put(&root, "build.log");
        put(&root, "src/main.rs");
        // Not a repository: git cannot say, so nothing is left out.
        let all = list(&root, "").unwrap();
        assert!(names(&all).contains(&("target", FileEntryKind::Folder)));
        assert!(names(&all).contains(&("build.log", FileEntryKind::File)));

        if !git(&root, &["init", "-q"]) {
            return; // no git here: the other half of the rule is what was just checked
        }
        let left = list(&root, "").unwrap();
        let left: Vec<&str> = left.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(left, ["src", ".gitignore"]);
    }

    #[test]
    fn a_path_outside_the_project_inside_git_or_not_a_folder_is_refused() {
        let (_dir, root) = project();
        put(&root, "a.txt");
        fs::create_dir_all(root.join(".git")).unwrap();
        assert!(list(&root, "..").is_err());
        assert!(list(&root, "/etc").is_err());
        assert!(list(&root, ".git").is_err());
        let not_folder = list(&root, "a.txt").unwrap_err();
        assert_eq!(
            not_folder.to_string(),
            "\"a.txt\" is not a folder of the project."
        );
        assert!(list(&root, "missing").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_is_listed_as_a_link_and_one_leaving_the_project_is_not_entered() {
        let (_dir, root) = project();
        let (_other, outside) = project();
        put(&outside, "secret.txt");
        std::os::unix::fs::symlink(&outside, root.join("out")).unwrap();
        put(&root, "real/file.txt");
        std::os::unix::fs::symlink(root.join("real"), root.join("inside")).unwrap();
        let entries = list(&root, "").unwrap();
        assert!(names(&entries).contains(&("out", FileEntryKind::Link)));
        assert!(names(&entries).contains(&("inside", FileEntryKind::Link)));
        assert!(list(&root, "out").is_err());
        assert_eq!(list(&root, "inside").unwrap()[0].path, "inside/file.txt");
    }

    #[test]
    fn only_folders_for_the_move_dialog() {
        let (_dir, root) = project();
        put(&root, "a.txt");
        put(&root, "dir/b.txt");
        let folders = list_folders(&root, "").unwrap();
        assert_eq!(names(&folders), [("dir", FileEntryKind::Folder)]);
    }
}
