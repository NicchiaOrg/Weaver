//! Listing one folder of the project (`requirements/tech#file-view`, **Listing**): names and kinds
//! from one `read_dir`, never the content of a file, never a recursion, an index or a search.
//!
//! `.git` is left out by name, and so is what git ignores, asked with one `git check-ignore -v -z
//! --stdin` per listed folder (`GIT_OPTIONAL_LOCKS=0`), except what only a line of a Solo mode block
//! in the local exclusions file ignores; without git, or outside a repository, nothing is ignored.
//! Other names starting with a dot are listed, `.weaver` included. Folders come first, then the
//! rest, each in name order without regard to case.

use std::collections::HashMap;
use std::fs;
use std::ops::RangeInclusive;
use std::path::Path;

use super::{join_rel, FileEntry, FileEntryKind, Refusal};
use crate::file_tab::path;
use crate::project_files::folder_tree::git_ignored_by;
use crate::solo_mode::block_lines_of_source;

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
    let ignored = ignored_outside_solo_blocks(root, &paths);
    entries.retain(|entry| !ignored.contains(&entry.path));
    entries.sort_by(|a, b| {
        (b.kind == FileEntryKind::Folder)
            .cmp(&(a.kind == FileEntryKind::Folder))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(entries)
}

/// The paths among `paths` that git ignores for a reason other than a line of a Solo mode block of
/// the local exclusions file, from one `git check-ignore -v -z --stdin`. The exclusions file is read
/// once, when some path is ignored by it; a path whose deciding rule lies in a Solo mode block (of
/// any working copy) is not in the answer, so it is listed.
fn ignored_outside_solo_blocks(root: &Path, paths: &[&str]) -> Vec<String> {
    let mut blocks: HashMap<String, Vec<RangeInclusive<usize>>> = HashMap::new();
    git_ignored_by(root, paths)
        .into_iter()
        .filter(|ignored| {
            let lines = blocks
                .entry(ignored.source.clone())
                .or_insert_with(|| block_lines_of_source(root, &ignored.source));
            !lines.iter().any(|range| range.contains(&ignored.line))
        })
        .map(|ignored| ignored.path)
        .collect()
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

    /// The tests against real repositories: Solo mode blocks written by the module that owns them.
    mod solo {
        use super::*;
        use crate::solo_mode::testing::{commit, git as run_git, put as put_text, Sandbox};
        use crate::solo_mode::{locate, Git, Mode};

        fn have_git() -> bool {
            process::command("git", ["--version"])
                .output()
                .is_ok_and(|output| output.status.success())
        }

        fn listed(root: &Path, rel: &str) -> Vec<String> {
            list(root, rel)
                .unwrap()
                .into_iter()
                .map(|entry| entry.name)
                .collect()
        }

        /// Writes the block of the working copy at `root` for `mode`.
        fn block(root: &Path, mode: Mode) {
            locate(&Git::system(), root)
                .unwrap()
                .write_block(mode)
                .unwrap();
        }

        /// Appends `text` to the local exclusions file of the repository at `repo`.
        fn exclude(repo: &Path, text: &str) {
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(repo.join(".git/info/exclude"))
                .unwrap();
            file.write_all(text.as_bytes()).unwrap();
        }

        fn weaver_files(root: &Path) {
            put_text(root, ".weaver/system.md", "x");
            put_text(root, ".agents/skills/weaver-a/SKILL.md", "x");
            put_text(root, ".omp/agents/weaver-a.md", "x");
            put_text(root, "src/main.rs", "x");
        }

        #[test]
        fn what_a_solo_mode_block_ignores_is_listed_at_every_depth() {
            if !have_git() {
                return;
            }
            let sandbox = Sandbox::new();
            let root = sandbox.repo("project");
            weaver_files(&root);
            block(&root, Mode::Folders);
            assert_eq!(listed(&root, ""), [".agents", ".omp", ".weaver", "src"]);
            assert_eq!(listed(&root, ".agents"), ["skills"]);
            assert_eq!(listed(&root, ".agents/skills"), ["weaver-a"]);
            assert_eq!(listed(&root, ".weaver"), ["system.md"]);
            assert_eq!(listed(&root, ".omp/agents"), ["weaver-a.md"]);
        }

        #[test]
        fn the_block_of_files_mode_is_listed_too() {
            if !have_git() {
                return;
            }
            let sandbox = Sandbox::new();
            let root = sandbox.repo("project");
            weaver_files(&root);
            put_text(&root, ".omp/WATCHDOG.yml", "x");
            block(&root, Mode::Files);
            assert_eq!(listed(&root, ""), [".agents", ".omp", ".weaver", "src"]);
            assert_eq!(listed(&root, ".agents/skills"), ["weaver-a"]);
            assert_eq!(listed(&root, ".omp"), ["agents", "WATCHDOG.yml"]);
            assert_eq!(listed(&root, ".omp/agents"), ["weaver-a.md"]);
        }

        #[test]
        fn a_project_in_a_subfolder_of_the_repository_lists_its_block() {
            if !have_git() {
                return;
            }
            let sandbox = Sandbox::new();
            sandbox.repo("repo");
            let root = sandbox.folder("repo/app");
            weaver_files(&root);
            block(&root, Mode::Folders);
            assert_eq!(listed(&root, ""), [".agents", ".omp", ".weaver", "src"]);
            assert_eq!(listed(&root, ".omp/agents"), ["weaver-a.md"]);
        }

        #[test]
        fn a_linked_worktree_lists_what_the_block_in_the_main_repository_ignores() {
            if !have_git() {
                return;
            }
            let sandbox = Sandbox::new();
            let main = sandbox.repo("main");
            commit(&main);
            run_git(&main, &["worktree", "add", "-q", "../linked"]);
            let linked = sandbox.folder("linked");
            weaver_files(&linked);
            // The block of the linked working copy lands in the main repository's file.
            block(&linked, Mode::Folders);
            let exclusions = fs::read_to_string(main.join(".git/info/exclude")).unwrap();
            assert!(exclusions.contains("weaver solo mode"));
            assert_eq!(listed(&linked, ""), [".agents", ".omp", ".weaver", "src"]);
        }

        #[test]
        fn the_block_of_another_working_copy_is_listed_too() {
            if !have_git() {
                return;
            }
            let sandbox = Sandbox::new();
            let root = sandbox.repo("project");
            put_text(&root, "generated/out.txt", "x");
            put_text(&root, "src/main.rs", "x");
            exclude(
                &root,
                "# >>> weaver solo mode: /elsewhere\n/generated/\n# <<< weaver solo mode: /elsewhere\n",
            );
            assert_eq!(listed(&root, ""), ["generated", "src"]);
        }

        #[test]
        fn what_other_rules_ignore_stays_out_even_beside_a_block() {
            if !have_git() {
                return;
            }
            let sandbox = Sandbox::new();
            let root = sandbox.repo("project");
            weaver_files(&root);
            put_text(&root, "build.log", "x");
            put_text(&root, "secret.txt", "x");
            put_text(&root, "target/out.bin", "x");
            put_text(&root, ".gitignore", "*.log\ntarget/\n");
            // A line of the exclusions outside any block, before and after the block.
            exclude(&root, "secret.txt\n");
            block(&root, Mode::Folders);
            exclude(&root, "/src/\n");
            assert_eq!(
                listed(&root, ""),
                [".agents", ".omp", ".weaver", ".gitignore"]
            );
        }

        #[test]
        fn a_path_covered_by_the_block_and_by_gitignore_stays_out() {
            if !have_git() {
                return;
            }
            let sandbox = Sandbox::new();
            let root = sandbox.repo("project");
            weaver_files(&root);
            put_text(&root, ".gitignore", ".omp/\n");
            block(&root, Mode::Folders);
            // `.gitignore` outranks the exclusions file, so it decides `.omp`; the other folders of
            // the block are decided by the block.
            assert_eq!(
                listed(&root, ""),
                [".agents", ".weaver", "src", ".gitignore"]
            );
        }

        #[test]
        fn lines_of_a_begin_with_no_end_or_outside_a_block_do_not_count() {
            if !have_git() {
                return;
            }
            let sandbox = Sandbox::new();
            let root = sandbox.repo("project");
            for name in ["a.txt", "b.txt", "c.txt", "d.txt"] {
                put_text(&root, name, "x");
            }
            // a: after a begin line that never ends; b: after a begin line whose end is another
            // root's; c: between blocks; d: inside a complete block.
            exclude(&root, "# >>> weaver solo mode: /one\n/a.txt\n");
            exclude(
                &root,
                "# >>> weaver solo mode: /two\n/b.txt\n# <<< weaver solo mode: /other\n",
            );
            exclude(&root, "/c.txt\n");
            exclude(
                &root,
                "# >>> weaver solo mode: /three\n/d.txt\n# <<< weaver solo mode: /three\n",
            );
            assert_eq!(listed(&root, ""), ["d.txt"]);
        }

        #[test]
        fn a_path_a_later_line_takes_back_from_the_ignored_is_listed() {
            if !have_git() {
                return;
            }
            let sandbox = Sandbox::new();
            let root = sandbox.repo("project");
            put_text(&root, "keep.log", "x");
            put_text(&root, "drop.log", "x");
            put_text(&root, ".gitignore", "*.log\n!keep.log\n");
            assert_eq!(listed(&root, ""), [".gitignore", "keep.log"]);
        }
    }
}
