//! Copying files and folders dropped from outside into a folder of the project
//! (`developer.DROP_EXTERNAL_ENTRIES`).
//!
//! The dropped paths are the only absolute paths the File view accepts, as Tauri reports them. A
//! drop is all or nothing:
//!
//! * every refusal of `requirements/rules#file-view` is checked for every dropped entry before
//!   anything is written: an entry whose canonical path lies inside the project, a name that is not
//!   valid, a name already taken in the target folder, two dropped entries with the same name, a
//!   target inside `.weaver/` (the target follows the normal rules of a path of the File view);
//! * then each entry is copied without ever following a link (`symlink_metadata`): a file is created
//!   with `create_new` and its bytes copied, keeping its permission bits; a folder with
//!   `create_dir`, then its entries one by one, depth first; a symbolic link is made again with the
//!   same target. Nothing is ever replaced and the originals are never touched;
//! * each entry is copied directly into the target folder; the top-level entries the copy created
//!   are recorded (only an entry whose creation succeeded is recorded, so a name that someone took
//!   meanwhile is never removed). A step that fails part way removes exactly the recorded entries,
//!   for good (not to the trash, since they are the drop's own copies). The target folder shows the
//!   partial entries for a moment, and nothing is written anywhere else, `.weaver/` included. When
//!   the removal itself fails, the error lists what is left.
//!
//! Reading a whole dropped folder is the one recursive read the File view makes.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use super::ops::{checked, folder_of, WEAVER_READ_ONLY};
use super::{join_rel, names, same_name, Refusal};
use crate::file_tab::path;

/// Why a drop was not copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyInError {
    /// Refused as a whole before anything was written, with the reason.
    Refused(Refusal),
    /// A step failed part way: `name` is the entry that failed (its path inside the dropped
    /// entry), `reason` why, and `not_removed` what could not be removed afterwards (empty when
    /// the project is as it was).
    Failed {
        name: String,
        reason: String,
        not_removed: Vec<String>,
    },
}

impl fmt::Display for CopyInError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(refusal) => write!(f, "{refusal}"),
            Self::Failed {
                name,
                reason,
                not_removed,
            } if not_removed.is_empty() => {
                write!(f, "Could not copy \"{name}\": {reason}. Nothing was copied.")
            }
            Self::Failed {
                name,
                reason,
                not_removed,
            } => write!(
                f,
                "Could not copy \"{name}\": {reason}. What was already copied could not be removed: {}.",
                not_removed.join("; ")
            ),
        }
    }
}

impl std::error::Error for CopyInError {}

impl From<Refusal> for CopyInError {
    fn from(refusal: Refusal) -> Self {
        Self::Refused(refusal)
    }
}

/// One dropped entry that passed every check.
struct Item {
    name: String,
    source: PathBuf,
    dest: PathBuf,
    /// A real folder, not a link to one: the tree lists folders first.
    folder: bool,
}

/// The canonical form of a dropped path: links resolved. A link that leads nowhere is the
/// canonical form of its folder plus its name.
fn canonical_of(source: &Path, name: &str, is_link: bool) -> Result<PathBuf, Refusal> {
    match dunce::canonicalize(source) {
        Ok(canonical) => Ok(canonical),
        Err(_) if is_link => source
            .parent()
            .and_then(|parent| dunce::canonicalize(parent).ok())
            .map(|parent| parent.join(name))
            .ok_or_else(|| Refusal::new(format!("\"{name}\" cannot be read."))),
        Err(error) => Err(Refusal::new(format!("\"{name}\" cannot be read: {error}"))),
    }
}

/// Checks one dropped path against the project and the target folder.
fn check_source(root: &Path, target_dir: &Path, source: &Path) -> Result<Item, Refusal> {
    if !source.is_absolute() {
        return Err(Refusal::new(format!(
            "\"{}\" was not dropped with its full path.",
            source.display()
        )));
    }
    let Some(name) = source.file_name().and_then(|name| name.to_str()) else {
        return Err(Refusal::new(format!(
            "\"{}\" cannot be copied: it has no valid name.",
            source.display()
        )));
    };
    let meta = match fs::symlink_metadata(source) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(Refusal::new(format!("\"{name}\" no longer exists.")));
        }
        Err(error) => return Err(Refusal::new(format!("\"{name}\" cannot be read: {error}"))),
    };
    let canonical = canonical_of(source, name, meta.file_type().is_symlink())?;
    if path::is_inside(root, &canonical) {
        return Err(Refusal::new(format!(
            "\"{name}\" is already in the project: drag it inside the tree to move it."
        )));
    }
    if path::is_inside(&canonical, root) {
        // Copying a folder that holds the project would copy the copy, for ever.
        return Err(Refusal::new(format!(
            "\"{name}\" holds the project and cannot be copied into it."
        )));
    }
    names::check(name)?;
    if fs::symlink_metadata(target_dir.join(name)).is_ok() {
        return Err(names::taken(name));
    }
    Ok(Item {
        name: name.to_owned(),
        source: source.to_owned(),
        dest: target_dir.join(name),
        folder: meta.is_dir(),
    })
}

/// Every refusal for the whole drop; the folder it goes to (relative to the root, normal) and
/// the entries to copy.
fn plan(
    root: &Path,
    target_folder: &str,
    sources: &[PathBuf],
) -> Result<(String, Vec<Item>), Refusal> {
    let (parts, in_weaver) = checked(target_folder)?;
    if in_weaver {
        return Err(Refusal::new(WEAVER_READ_ONLY));
    }
    let target_rel = parts.join("/");
    let target_dir = folder_of(root, &target_rel)?;
    let mut items: Vec<Item> = Vec::with_capacity(sources.len());
    for source in sources {
        let item = check_source(root, &target_dir, source)?;
        if items.iter().any(|other| same_name(&other.name, &item.name)) {
            return Err(Refusal::new(format!(
                "Two dropped items are both named \"{}\".",
                item.name
            )));
        }
        items.push(item);
    }
    Ok((target_rel, items))
}

/// A step that failed: the entry's path inside the dropped one, and why.
struct StepError {
    name: String,
    error: io::Error,
}

/// A symbolic link at `dest` with the target of the link `source`.
fn make_link(source: &Path, dest: &Path) -> io::Result<()> {
    let target = fs::read_link(source)?;
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, dest)
    }
    #[cfg(windows)]
    {
        // Which kind of link the target needs; a refusal (no privilege, say) fails the copy.
        if fs::metadata(source).is_ok_and(|meta| meta.is_dir()) {
            std::os::windows::fs::symlink_dir(target, dest)
        } else {
            std::os::windows::fs::symlink_file(target, dest)
        }
    }
}

/// Copies the entry `source` to `dest`, which must not exist. `rel` is its path inside the dropped
/// entry (for the reason of a failure); `step` is told each destination before it is made, and
/// an error from it fails the copy there (the tests inject their failures with it). `made` is set
/// as soon as `dest` itself was created by this call, so that a failure after that point still
/// finds it recorded, and a failure before it (a name taken meanwhile, say) never does.
fn copy_entry(
    source: &Path,
    dest: &Path,
    rel: &str,
    step: &dyn Fn(&Path) -> io::Result<()>,
    made: &mut bool,
) -> Result<(), StepError> {
    let fail = |error: io::Error| StepError {
        name: rel.to_owned(),
        error,
    };
    step(dest).map_err(fail)?;
    let meta = fs::symlink_metadata(source).map_err(fail)?;
    let kind = meta.file_type();
    if kind.is_symlink() {
        make_link(source, dest).map_err(fail)?;
        *made = true;
    } else if kind.is_dir() {
        fs::create_dir(dest).map_err(fail)?;
        *made = true;
        for item in fs::read_dir(source).map_err(fail)? {
            let name = item.map_err(fail)?.file_name();
            let child = format!("{rel}/{}", name.to_string_lossy());
            // Inside a folder this call made, nothing needs recording: removing it removes them.
            copy_entry(
                &source.join(&name),
                &dest.join(&name),
                &child,
                step,
                &mut false,
            )?;
        }
    } else if kind.is_file() {
        let mut from = File::open(source).map_err(fail)?;
        let mut to = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dest)
            .map_err(fail)?;
        *made = true;
        io::copy(&mut from, &mut to).map_err(fail)?;
        drop(to);
        fs::set_permissions(dest, meta.permissions()).map_err(fail)?;
    } else {
        return Err(fail(io::Error::other(
            "it is not a file, a folder or a link",
        )));
    }
    Ok(())
}

/// Removes a copy the drop made, for good. A link is removed as a link, never followed.
fn remove_copy(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() {
        for item in fs::read_dir(path)? {
            remove_copy(&item?.path())?;
        }
        return fs::remove_dir(path);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        if meta.file_type().is_symlink_dir() {
            return fs::remove_dir(path);
        }
        let mut permissions = meta.permissions();
        if permissions.readonly() {
            permissions.set_readonly(false);
            fs::set_permissions(path, permissions)?;
        }
    }
    fs::remove_file(path)
}

/// Copies the dropped `sources` (absolute paths) into the folder `target_folder` of the project at
/// `root`, the canonical project root (empty: the root itself). Returns the paths of the entries
/// made, relative to the root, in the tree's order (folders first, then the rest, each by name).
pub fn copy_in(
    root: &Path,
    target_folder: &str,
    sources: &[PathBuf],
) -> Result<Vec<String>, CopyInError> {
    copy_in_with(root, target_folder, sources, &|_| Ok(()))
}

/// Describes what could not be removed for the error: the entry's name and why.
fn left_over(path: &Path, error: &io::Error) -> String {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    format!("\"{name}\": {error}")
}

/// Copies every item into its destination. `created` receives the destinations this copy made,
/// each recorded as soon as it exists, whether or not the copy of what is inside it went on.
fn copy_items(
    items: &[Item],
    step: &dyn Fn(&Path) -> io::Result<()>,
    created: &mut Vec<PathBuf>,
) -> Result<(), StepError> {
    for item in items {
        let mut made = false;
        let result = copy_entry(&item.source, &item.dest, &item.name, step, &mut made);
        if made {
            created.push(item.dest.clone());
        }
        result?;
    }
    Ok(())
}

/// [`copy_in`] with `step` told each destination before it is made (in the target folder); an
/// error from it fails the copy at that entry. The tests inject a failure part way with it, and
/// wait in it to keep a copy running.
pub fn copy_in_with(
    root: &Path,
    target_folder: &str,
    sources: &[PathBuf],
    step: &dyn Fn(&Path) -> io::Result<()>,
) -> Result<Vec<String>, CopyInError> {
    let (target_rel, mut items) = plan(root, target_folder, sources)?;
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let mut created: Vec<PathBuf> = Vec::new();
    if let Err(failed) = copy_items(&items, step, &mut created) {
        let not_removed: Vec<String> = created
            .iter()
            .rev()
            .filter_map(|copy| remove_copy(copy).err().map(|error| left_over(copy, &error)))
            .collect();
        return Err(CopyInError::Failed {
            name: failed.name,
            reason: failed.error.to_string(),
            not_removed,
        });
    }
    items.sort_by(|a, b| {
        b.folder
            .cmp(&a.folder)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(items
        .iter()
        .map(|item| join_rel(&target_rel, &item.name))
        .collect())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::file_view::case_insensitive_names;
    use crate::paths::canonical_root;

    /// A project folder and, next to it, the folder the dropped things come from.
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("project");
        let outside = dir.path().join("outside");
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join(".weaver")).unwrap();
        fs::create_dir(&outside).unwrap();
        let root = canonical_root(&root).unwrap();
        let outside = canonical_root(&outside).unwrap();
        (dir, root, outside)
    }

    fn put(base: &Path, rel: &str, text: &str) -> PathBuf {
        let target = base.join(rel);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, text).unwrap();
        target
    }

    fn reason<T: fmt::Debug>(result: Result<T, CopyInError>) -> String {
        result.unwrap_err().to_string()
    }

    /// What lies under `base`, relative, sorted: to see that a refusal or a failure changed
    /// nothing. The project's `.weaver/` folder itself is not listed, but whatever it holds is, so
    /// that a copy writing anything there is seen.
    fn tree(base: &Path) -> Vec<String> {
        fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
            for item in fs::read_dir(dir).unwrap() {
                let path = item.unwrap().path();
                let rel = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if rel != ".weaver" {
                    out.push(rel);
                }
                if fs::symlink_metadata(&path).unwrap().is_dir() {
                    walk(&path, base, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(base, base, &mut out);
        out.sort();
        out
    }

    #[test]
    fn files_and_nested_folders_are_copied_and_the_originals_stay() {
        let (_dir, root, outside) = fixture();
        fs::create_dir(root.join("src")).unwrap();
        let file = put(&outside, "notes.txt", "hello");
        put(&outside, "assets/logo.png", "png");
        put(&outside, "assets/icons/a.svg", "svg");
        fs::create_dir(outside.join("assets/empty")).unwrap();
        let folder = outside.join("assets");

        let made = copy_in(&root, "src", &[file.clone(), folder.clone()]).unwrap();

        assert_eq!(made, vec!["src/assets", "src/notes.txt"]);
        assert_eq!(
            fs::read_to_string(root.join("src/notes.txt")).unwrap(),
            "hello"
        );
        assert_eq!(
            fs::read_to_string(root.join("src/assets/icons/a.svg")).unwrap(),
            "svg"
        );
        assert!(root.join("src/assets/empty").is_dir());
        assert_eq!(fs::read_to_string(file).unwrap(), "hello");
        assert!(folder.join("logo.png").is_file());
    }

    #[test]
    fn a_drop_on_the_root_goes_to_the_root() {
        let (_dir, root, outside) = fixture();
        let file = put(&outside, "a.txt", "a");
        assert_eq!(
            copy_in(&root, "", std::slice::from_ref(&file)).unwrap(),
            vec!["a.txt"]
        );
        assert_eq!(
            reason(copy_in(&root, "", &[file])),
            "A file or folder named \"a.txt\" already exists here."
        );
        assert!(root.join("a.txt").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn permission_bits_of_files_are_kept() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, root, outside) = fixture();
        let script = put(&outside, "run.sh", "#!/bin/sh");
        fs::set_permissions(&script, fs::Permissions::from_mode(0o750)).unwrap();
        let locked = put(&outside, "dir/read-only.txt", "x");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o440)).unwrap();

        copy_in(&root, "", &[script, outside.join("dir")]).unwrap();

        let mode = |rel: &str| fs::metadata(root.join(rel)).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode("run.sh"), 0o750);
        assert_eq!(mode("dir/read-only.txt"), 0o440);
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_are_copied_as_links_and_never_followed() {
        use std::os::unix::fs::symlink;
        let (dir, root, outside) = fixture();
        let elsewhere = dir.path().join("elsewhere");
        fs::create_dir(&elsewhere).unwrap();
        put(&elsewhere, "big.txt", "big");
        put(&outside, "folder/real.txt", "real");
        symlink("real.txt", outside.join("folder/inner-link")).unwrap();
        symlink(&elsewhere, outside.join("folder/dir-link")).unwrap();
        symlink("nowhere", outside.join("dangling")).unwrap();

        let made = copy_in(
            &root,
            "",
            &[outside.join("folder"), outside.join("dangling")],
        )
        .unwrap();

        assert_eq!(made, vec!["folder", "dangling"]);
        let link = |rel: &str| fs::read_link(root.join(rel)).unwrap();
        assert_eq!(link("folder/inner-link"), PathBuf::from("real.txt"));
        assert_eq!(link("folder/dir-link"), elsewhere);
        assert_eq!(link("dangling"), PathBuf::from("nowhere"));
        assert!(fs::symlink_metadata(root.join("folder/dir-link"))
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            fs::read_to_string(elsewhere.join("big.txt")).unwrap(),
            "big"
        );
    }

    #[test]
    fn an_entry_already_in_the_project_refuses_the_whole_drop() {
        let (_dir, root, outside) = fixture();
        let inside = put(&root, "src/main.rs", "fn main() {}");
        let other = put(&outside, "other.txt", "o");
        let before = tree(&root);

        assert_eq!(
            reason(copy_in(&root, "", &[other.clone(), inside])),
            "\"main.rs\" is already in the project: drag it inside the tree to move it."
        );
        assert_eq!(
            reason(copy_in(&root, "", &[root.join("src")])),
            "\"src\" is already in the project: drag it inside the tree to move it."
        );
        assert_eq!(tree(&root), before);
        assert!(other.is_file());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_that_leads_into_the_project_counts_as_inside() {
        let (_dir, root, outside) = fixture();
        put(&root, "a.txt", "a");
        std::os::unix::fs::symlink(root.join("a.txt"), outside.join("to-a")).unwrap();
        assert!(
            reason(copy_in(&root, "", &[outside.join("to-a")])).contains("already in the project")
        );
        assert!(!root.join("to-a").exists());
    }

    #[test]
    fn a_folder_that_holds_the_project_is_refused() {
        let (dir, root, _outside) = fixture();
        assert!(reason(copy_in(&root, "", &[dir.path().to_owned()])).contains("holds the project"));
        assert_eq!(tree(&root), Vec::<String>::new());
    }

    #[test]
    fn a_name_taken_in_the_target_folder_refuses_the_whole_drop() {
        let (_dir, root, outside) = fixture();
        put(&root, "src/a.txt", "mine");
        let a = put(&outside, "a.txt", "theirs");
        let b = put(&outside, "b.txt", "b");

        assert_eq!(
            reason(copy_in(&root, "src", &[b, a])),
            "A file or folder named \"a.txt\" already exists here."
        );
        assert_eq!(fs::read_to_string(root.join("src/a.txt")).unwrap(), "mine");
        assert!(!root.join("src/b.txt").exists());
    }

    #[test]
    fn a_name_that_differs_only_in_case_is_taken_as_the_file_system_compares_names() {
        let (_dir, root, outside) = fixture();
        put(&root, "Readme.md", "mine");
        let upper = put(&outside, "README.md", "theirs");
        let result = copy_in(&root, "", &[upper]);
        if case_insensitive_names() {
            assert_eq!(
                result.unwrap_err().to_string(),
                "A file or folder named \"README.md\" already exists here."
            );
            assert_eq!(fs::read_to_string(root.join("Readme.md")).unwrap(), "mine");
        } else {
            assert_eq!(result.unwrap(), vec!["README.md"]);
        }
    }

    #[test]
    fn two_dropped_entries_with_one_name_refuse_the_whole_drop() {
        let (_dir, root, outside) = fixture();
        let first = put(&outside, "one/same.txt", "1");
        let second = put(&outside, "two/same.txt", "2");
        let case = put(&outside, "three/SAME.TXT", "3");

        assert_eq!(
            reason(copy_in(&root, "", &[first.clone(), second])),
            "Two dropped items are both named \"same.txt\"."
        );
        let result = copy_in(&root, "", &[first, case]);
        if case_insensitive_names() {
            assert_eq!(
                result.unwrap_err().to_string(),
                "Two dropped items are both named \"SAME.TXT\"."
            );
        } else {
            assert!(result.is_ok());
        }
        if case_insensitive_names() {
            assert_eq!(tree(&root), Vec::<String>::new());
        }
    }

    #[test]
    fn a_target_inside_weaver_a_missing_target_and_a_bad_target_are_refused() {
        let (_dir, root, outside) = fixture();
        fs::create_dir_all(root.join(".weaver/tasks")).unwrap();
        put(&root, "plain.txt", "p");
        let file = put(&outside, "a.txt", "a");

        assert_eq!(
            reason(copy_in(&root, ".weaver", std::slice::from_ref(&file))),
            WEAVER_READ_ONLY
        );
        assert_eq!(
            reason(copy_in(&root, ".weaver/tasks", std::slice::from_ref(&file))),
            WEAVER_READ_ONLY
        );
        assert!(copy_in(&root, "missing", std::slice::from_ref(&file)).is_err());
        assert!(copy_in(&root, "plain.txt", std::slice::from_ref(&file)).is_err());
        assert!(copy_in(&root, "..", std::slice::from_ref(&file)).is_err());
        assert!(copy_in(&root, ".git", &[file]).is_err());
        assert!(!root.join(".weaver/a.txt").exists());
        assert!(!root.join(".weaver/tasks/a.txt").exists());
    }

    #[test]
    fn a_name_that_is_not_valid_refuses_the_whole_drop() {
        let (_dir, root, outside) = fixture();
        let good = put(&outside, "good.txt", "g");
        // A relative path, and a path with no name, are not what Tauri reports.
        assert!(copy_in(&root, "", &[good.clone(), PathBuf::from("relative.txt")]).is_err());
        assert!(copy_in(&root, "", &[good.clone(), PathBuf::from("/")]).is_err());
        assert!(
            copy_in(&root, "", &[good.clone(), outside.join("gone.txt")])
                .unwrap_err()
                .to_string()
                .contains("no longer exists")
        );
        #[cfg(unix)]
        {
            let bad = put(&outside, "back\\slash.txt", "b");
            assert_eq!(
                reason(copy_in(&root, "", &[good, bad])),
                "The name cannot contain \"/\", \"\\\" or a NUL character."
            );
        }
        assert_eq!(tree(&root), Vec::<String>::new());
    }

    #[test]
    fn a_failure_part_way_removes_everything_the_drop_copied() {
        let (_dir, root, outside) = fixture();
        fs::create_dir(root.join("target")).unwrap();
        put(&root, "target/keep.txt", "keep");
        let first = put(&outside, "first.txt", "1");
        put(&outside, "tree/a.txt", "a");
        put(&outside, "tree/deep/b.txt", "b");
        put(&outside, "tree/deep/c.txt", "c");
        let last = put(&outside, "last.txt", "l");
        let before = tree(&root);
        let calls = Cell::new(0);

        // The fifth destination fails: `first.txt` and part of `tree` exist by then.
        let result = copy_in_with(
            &root,
            "target",
            &[first.clone(), outside.join("tree"), last.clone()],
            &|_| {
                calls.set(calls.get() + 1);
                if calls.get() == 5 {
                    Err(io::Error::other("disk on fire"))
                } else {
                    Ok(())
                }
            },
        );

        let message = result.unwrap_err().to_string();
        assert!(
            message.starts_with("Could not copy \"tree/")
                && message.ends_with(": disk on fire. Nothing was copied."),
            "{message}"
        );
        assert_eq!(calls.get(), 5);
        assert_eq!(tree(&root), before);
        assert_eq!(
            fs::read_to_string(root.join("target/keep.txt")).unwrap(),
            "keep"
        );
        assert_eq!(fs::read_to_string(first).unwrap(), "1");
        assert!(outside.join("tree/deep/c.txt").is_file());
        assert!(last.is_file());
    }

    #[cfg(unix)]
    #[test]
    fn a_failure_removes_read_only_copies_and_links_too() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let (_dir, root, outside) = fixture();
        let locked = put(&outside, "a-locked.txt", "x");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o400)).unwrap();
        symlink("a-locked.txt", outside.join("b-link")).unwrap();
        let last = put(&outside, "c.txt", "c");
        let calls = Cell::new(0);

        let result = copy_in_with(&root, "", &[locked, outside.join("b-link"), last], &|_| {
            calls.set(calls.get() + 1);
            if calls.get() == 3 {
                Err(io::Error::other("stop"))
            } else {
                Ok(())
            }
        });

        assert_eq!(
            result.unwrap_err().to_string(),
            "Could not copy \"c.txt\": stop. Nothing was copied."
        );
        assert_eq!(tree(&root), Vec::<String>::new());
    }

    #[test]
    fn nothing_is_ever_replaced_when_the_name_appears_while_copying() {
        let (_dir, root, outside) = fixture();
        let file = put(&outside, "late.txt", "theirs");
        // Someone creates the name between the checks and the copy.
        let result = copy_in_with(&root, "", &[file], &|_| {
            fs::write(root.join("late.txt"), "mine").unwrap();
            Ok(())
        });
        assert!(matches!(result, Err(CopyInError::Failed { .. })));
        // The entry that was not the drop's own stays: it was never recorded, so it is never removed.
        assert_eq!(fs::read_to_string(root.join("late.txt")).unwrap(), "mine");
        assert_eq!(tree(&root), ["late.txt"]);
    }

    #[test]
    fn a_name_that_appears_before_a_later_entry_removes_the_entries_already_copied() {
        let (_dir, root, outside) = fixture();
        fs::create_dir(root.join("target")).unwrap();
        let first = put(&outside, "a.txt", "a");
        let folder = put(&outside, "dir/inner.txt", "i");
        let last = put(&outside, "z.txt", "z");
        let before = tree(&root);

        // The last name is taken after the entries before it were copied, just before it is made.
        let result = copy_in_with(
            &root,
            "target",
            &[first, outside.join("dir"), last],
            &|dest| {
                if dest.file_name().is_some_and(|name| name == "z.txt") {
                    fs::write(root.join("target/z.txt"), "mine").unwrap();
                }
                Ok(())
            },
        );

        let message = result.unwrap_err().to_string();
        assert!(
            message.starts_with("Could not copy \"z.txt\": ")
                && message.ends_with(". Nothing was copied."),
            "{message}"
        );
        let mut expected = before;
        expected.push("target/z.txt".to_owned());
        expected.sort();
        assert_eq!(tree(&root), expected);
        assert_eq!(
            fs::read_to_string(root.join("target/z.txt")).unwrap(),
            "mine"
        );
        assert!(folder.is_file());
    }

    #[test]
    fn the_copy_goes_straight_into_the_target_and_writes_nothing_in_weaver() {
        let (_dir, root, outside) = fixture();
        fs::create_dir(root.join("target")).unwrap();
        let file = put(&outside, "one.txt", "1");
        put(&outside, "tree/deep/b.txt", "b");
        let last = put(&outside, "two.txt", "2");
        let seen: std::cell::RefCell<Vec<Vec<String>>> = std::cell::RefCell::new(Vec::new());

        let made = copy_in_with(
            &root,
            "target",
            &[file, outside.join("tree"), last],
            &|_| {
                // `tree()` lists what `.weaver/` holds: nothing, at every step.
                let weaver: Vec<String> = tree(&root)
                    .into_iter()
                    .filter(|rel| rel.starts_with(".weaver"))
                    .collect();
                seen.borrow_mut().push(weaver);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(
            made,
            vec!["target/tree", "target/one.txt", "target/two.txt"]
        );
        let seen = seen.into_inner();
        assert!(!seen.is_empty());
        for weaver in &seen {
            assert!(weaver.is_empty(), "{weaver:?}");
        }
        assert_eq!(
            tree(&root),
            [
                "target",
                "target/one.txt",
                "target/tree",
                "target/tree/deep",
                "target/tree/deep/b.txt",
                "target/two.txt"
            ]
        );
    }

    #[test]
    fn a_project_without_weaver_is_copied_into_and_no_weaver_is_made() {
        let (_dir, root, outside) = fixture();
        fs::remove_dir(root.join(".weaver")).unwrap();
        let file = put(&outside, "a.txt", "a");

        let made = copy_in(&root, "", &[file]).unwrap();

        assert_eq!(made, vec!["a.txt"]);
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "a");
        assert!(!root.join(".weaver").exists());
    }

    #[test]
    fn a_failure_leaves_the_tree_and_weaver_as_they_were() {
        let (_dir, root, outside) = fixture();
        put(&outside, "tree/deep/b.txt", "b");
        let last = put(&outside, "last.txt", "l");

        let message = reason(copy_in_with(
            &root,
            "",
            &[outside.join("tree"), last],
            &|dest| {
                if dest.file_name().is_some_and(|name| name == "last.txt") {
                    return Err(io::Error::other("disk on fire"));
                }
                Ok(())
            },
        ));

        assert_eq!(
            message,
            "Could not copy \"last.txt\": disk on fire. Nothing was copied."
        );
        assert_eq!(tree(&root), Vec::<String>::new());
    }

    #[cfg(unix)]
    #[test]
    fn when_a_copy_cannot_be_removed_the_error_lists_what_is_left() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, root, outside) = fixture();
        put(&outside, "tree/deep/b.txt", "b");
        let last = put(&outside, "last.txt", "l");
        // A folder that cannot be written is no obstacle to root: nothing to test then.
        let probe = root.join("probe");
        fs::create_dir(&probe).unwrap();
        fs::set_permissions(&probe, fs::Permissions::from_mode(0o500)).unwrap();
        let enforced = fs::write(probe.join("x"), "x").is_err();
        fs::set_permissions(&probe, fs::Permissions::from_mode(0o700)).unwrap();
        fs::remove_dir(&probe).unwrap();
        if !enforced {
            return;
        }

        let locked = Cell::new(None::<PathBuf>);
        let result = copy_in_with(&root, "", &[outside.join("tree"), last], &|dest| {
            if dest.file_name().is_some_and(|name| name == "last.txt") {
                // The first entry is complete: lock its inner folder, then fail.
                let deep = dest.parent().unwrap().join("tree/deep");
                fs::set_permissions(&deep, fs::Permissions::from_mode(0o500)).unwrap();
                locked.set(Some(deep));
                return Err(io::Error::other("disk on fire"));
            }
            Ok(())
        });

        let message = result.unwrap_err().to_string();
        let deep = locked.take().unwrap();
        let in_project = tree(&root);
        fs::set_permissions(&deep, fs::Permissions::from_mode(0o700)).unwrap();
        // What the drop copied and could not be removed is left in the target, listed by name,
        // and nothing else was written anywhere.
        assert_eq!(in_project, ["tree", "tree/deep", "tree/deep/b.txt"]);
        assert!(
            message.starts_with("Could not copy \"last.txt\": disk on fire. What was already copied could not be removed: \"tree\": "),
            "{message}"
        );
    }
}
