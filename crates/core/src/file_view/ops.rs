//! The actions of the tree that change the project's files: create, rename, move and delete
//! (`developer.CREATE_ENTRY`, `RENAME_ENTRY`, `MOVE_ENTRY`, `DELETE_ENTRY`).
//!
//! Each one checks every refusal of `requirements/rules#file-view` again when it runs, against
//! the disk as it is at that moment, and a refused action changes nothing:
//!
//! * a name that is not valid, or already taken in the target folder (compared as the file system
//!   compares names, so a rename that only changes the case of a name is allowed);
//! * creating inside `.weaver/`, renaming, moving or deleting `.weaver/` or anything inside it,
//!   moving anything into it;
//! * moving a folder into itself or into a folder inside it, and moving across volumes;
//! * renaming, moving or deleting a file whose agent lock is held, or a folder holding one (the
//!   `locked` argument says it for a relative path).
//!
//! `root` is the canonical project root and every path is relative to it, with `/`. What an action
//! returns is the new relative path of the entry it made or moved.

use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use super::rename::{is_cross_volume, rename_no_replace};
use super::{is_within, join_rel, names, same_name, split_rel, Refusal, WEAVER_FOLDER};
use crate::file_tab::path::{self, PathRefusal};

/// The reason shown for anything that would change `.weaver/`.
pub const WEAVER_READ_ONLY: &str = ".weaver is written by Weaver: read-only here.";

/// The reason shown for an entry the agent is changing.
pub const AGENT_LOCK: &str = "The agent is editing this file.";

/// The checked parts of `rel`, and whether they name `.weaver/` or something inside it.
fn checked(rel: &str) -> Result<(Vec<&str>, bool), PathRefusal> {
    let parts = path::parts(rel)?;
    let in_weaver = parts
        .first()
        .is_some_and(|first| same_name(first, WEAVER_FOLDER));
    Ok((parts, in_weaver))
}

/// The parts of a folder path joined again, so `a//b/` and `a/./b` are `a/b`.
fn normal(parts: &[&str]) -> String {
    parts.join("/")
}

/// The canonical folder `rel` of the project, which must be a folder.
fn folder_of(root: &Path, rel: &str) -> Result<PathBuf, Refusal> {
    let folder = path::resolve(root, rel)?;
    if folder.is_dir() {
        Ok(folder)
    } else {
        Err(Refusal::new(format!(
            "\"{}\" is not a folder of the project.",
            rel.trim_matches('/')
        )))
    }
}

/// Where the entry `name` of the folder `parent_rel` is, with its parent folder resolved (so an
/// entry reached through a link that leaves the project is refused) and the entry itself not
/// followed: a link is renamed, moved and deleted as a link. Refused when it is gone.
fn entry_in(root: &Path, parent_rel: &str, name: &str) -> Result<(PathBuf, fs::Metadata), Refusal> {
    let parent = folder_of(root, parent_rel)?;
    let entry = parent.join(name);
    match fs::symlink_metadata(&entry) {
        Ok(meta) => Ok((entry, meta)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(Refusal::new(format!("\"{name}\" no longer exists.")))
        }
        Err(error) => Err(Refusal::new(format!("\"{name}\" cannot be read: {error}"))),
    }
}

/// Whether two existing paths are the same entry (a case-only rename names the entry it renames).
fn same_entry(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        a.dev() == b.dev() && a.ino() == b.ino()
    }
    #[cfg(not(unix))]
    {
        // Windows compares names without regard to case: the same name in the same folder is the
        // same entry.
        let _ = (a, b);
        true
    }
}

/// An empty file (`create_new`) or a folder (`create_dir`) named `name` in the folder `parent`;
/// never replaces an existing entry. Returns the new entry's path.
pub fn create(root: &Path, parent: &str, name: &str, folder: bool) -> Result<String, Refusal> {
    let (parts, in_weaver) = checked(parent)?;
    if in_weaver {
        return Err(Refusal::new(WEAVER_READ_ONLY));
    }
    names::check(name)?;
    let dir = folder_of(root, parent)?;
    let target = dir.join(name);
    let made = if folder {
        fs::create_dir(&target)
    } else {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map(drop)
    };
    match made {
        Ok(()) => Ok(join_rel(&normal(&parts), name)),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(names::taken(name)),
        Err(error) => Err(Refusal::new(format!(
            "Could not create \"{name}\": {error}"
        ))),
    }
}

/// Moves `from` to `to` without replacing anything, with the refusal for each way it can fail.
fn move_entry(from: &Path, to: &Path, name: &str, verb: &str) -> Result<(), Refusal> {
    match rename_no_replace(from, to) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(names::taken(
            to.file_name().and_then(|n| n.to_str()).unwrap_or(name),
        )),
        Err(error) if is_cross_volume(&error) => Err(Refusal::new(format!(
            "Could not move \"{name}\": the target folder is on another volume."
        ))),
        Err(error) => Err(Refusal::new(format!(
            "Could not {verb} \"{name}\": {error}"
        ))),
    }
}

/// Gives the entry `rel` the name `new_name` in the same folder. Returns the new path.
pub fn rename(
    root: &Path,
    rel: &str,
    new_name: &str,
    locked: &dyn Fn(&str) -> bool,
) -> Result<String, Refusal> {
    let (parts, in_weaver) = checked(rel)?;
    if parts.is_empty() {
        return Err(Refusal::new("The project folder cannot be renamed here."));
    }
    if in_weaver {
        return Err(Refusal::new(WEAVER_READ_ONLY));
    }
    names::check(new_name)?;
    let rel = normal(&parts);
    let (parent_rel, old_name) = split_rel(&rel);
    let (source, source_meta) = entry_in(root, parent_rel, old_name)?;
    if locked(&rel) {
        return Err(Refusal::new(AGENT_LOCK));
    }
    let new_rel = join_rel(parent_rel, new_name);
    if new_name == old_name {
        return Ok(new_rel);
    }
    let target = source.with_file_name(new_name);
    match fs::symlink_metadata(&target) {
        Ok(existing) => {
            // Only the case of the name changes and the target is the entry itself: allowed.
            if same_name(old_name, new_name) && same_entry(&source_meta, &existing) {
                fs::rename(&source, &target).map_err(|error| {
                    Refusal::new(format!("Could not rename \"{old_name}\": {error}"))
                })?;
            } else {
                return Err(names::taken(new_name));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            move_entry(&source, &target, old_name, "rename")?;
        }
        Err(error) => {
            return Err(Refusal::new(format!(
                "Could not rename \"{old_name}\": {error}"
            )))
        }
    }
    Ok(new_rel)
}

/// Moves the entry `rel` into the folder `target_folder` (empty: the project root). Returns the
/// new path.
pub fn move_into(
    root: &Path,
    rel: &str,
    target_folder: &str,
    locked: &dyn Fn(&str) -> bool,
) -> Result<String, Refusal> {
    let (parts, in_weaver) = checked(rel)?;
    if parts.is_empty() {
        return Err(Refusal::new("The project folder cannot be moved."));
    }
    let (target_parts, target_in_weaver) = checked(target_folder)?;
    if in_weaver || target_in_weaver {
        return Err(Refusal::new(WEAVER_READ_ONLY));
    }
    let rel = normal(&parts);
    let target_rel = normal(&target_parts);
    let (parent_rel, name) = split_rel(&rel);
    let (source, meta) = entry_in(root, parent_rel, name)?;
    if locked(&rel) {
        return Err(Refusal::new(AGENT_LOCK));
    }
    let target_dir = folder_of(root, &target_rel)?;
    if meta.is_dir() {
        // Into itself or a folder inside it: by name, and by where a link in the way leads.
        let inside = is_within(&rel, &target_rel)
            || dunce::canonicalize(&source)
                .is_ok_and(|canonical| path::is_inside(&canonical, &target_dir));
        if inside {
            return Err(Refusal::new(format!(
                "Could not move \"{name}\": a folder cannot be moved into itself or into a folder inside it."
            )));
        }
    }
    let target = target_dir.join(name);
    if fs::symlink_metadata(&target).is_ok() {
        return Err(names::taken(name));
    }
    move_entry(&source, &target, name, "move")?;
    Ok(join_rel(&target_rel, name))
}

/// Moves `path` to the system's trash, with the `trash` crate.
pub fn system_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|error| error.to_string())
}

/// Moves the entry `rel` to the trash with `trash`, which is [`system_trash`] in the app and a
/// fake in the tests. When the trash refuses, nothing is deleted.
pub fn delete(
    root: &Path,
    rel: &str,
    locked: &dyn Fn(&str) -> bool,
    trash: &dyn Fn(&Path) -> Result<(), String>,
) -> Result<(), Refusal> {
    let (parts, in_weaver) = checked(rel)?;
    if parts.is_empty() {
        return Err(Refusal::new("The project folder cannot be deleted here."));
    }
    if in_weaver {
        return Err(Refusal::new(WEAVER_READ_ONLY));
    }
    let rel = normal(&parts);
    let (parent_rel, name) = split_rel(&rel);
    let (entry, _) = entry_in(root, parent_rel, name)?;
    if locked(&rel) {
        return Err(Refusal::new(AGENT_LOCK));
    }
    trash(&entry)
        .map_err(|reason| Refusal::new(format!("Could not move \"{name}\" to the trash: {reason}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::canonical_root;

    fn project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = canonical_root(dir.path()).unwrap();
        (dir, root)
    }

    fn put(root: &Path, rel: &str, text: &str) {
        let target = root.join(rel);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, text).unwrap();
    }

    fn read(root: &Path, rel: &str) -> String {
        fs::read_to_string(root.join(rel)).unwrap()
    }

    fn free(_: &str) -> bool {
        false
    }

    fn reason<T: std::fmt::Debug>(result: Result<T, Refusal>) -> String {
        result.unwrap_err().to_string()
    }

    // --- create ---------------------------------------------------------------------------

    #[test]
    fn a_new_file_is_empty_and_a_new_folder_is_empty() {
        let (_dir, root) = project();
        fs::create_dir(root.join("src")).unwrap();
        assert_eq!(create(&root, "src", "a.rs", false).unwrap(), "src/a.rs");
        assert_eq!(read(&root, "src/a.rs"), "");
        assert_eq!(create(&root, "", "docs", true).unwrap(), "docs");
        assert!(root.join("docs").is_dir());
        assert_eq!(create(&root, "src/", "b", true).unwrap(), "src/b");
    }

    #[test]
    fn creating_never_replaces_an_existing_entry() {
        let (_dir, root) = project();
        put(&root, "a.txt", "mine");
        fs::create_dir(root.join("d")).unwrap();
        assert_eq!(
            reason(create(&root, "", "a.txt", false)),
            "A file or folder named \"a.txt\" already exists here."
        );
        assert_eq!(read(&root, "a.txt"), "mine");
        assert!(create(&root, "", "d", false).is_err());
        assert!(create(&root, "", "a.txt", true).is_err());
        assert!(create(&root, "", "d", true).is_err());
        assert!(root.join("d").is_dir());
    }

    #[test]
    fn creating_refuses_a_bad_name_a_missing_parent_and_weaver() {
        let (_dir, root) = project();
        fs::create_dir_all(root.join(".weaver/tasks")).unwrap();
        for name in ["", ".", "..", "a/b", "a\\b", "a\0b"] {
            assert!(create(&root, "", name, false).is_err(), "{name:?}");
        }
        assert!(create(&root, "missing", "a", false).is_err());
        assert!(create(&root, "..", "a", false).is_err());
        assert!(create(&root, ".git", "a", false).is_err());
        assert_eq!(
            reason(create(&root, ".weaver", "a", false)),
            WEAVER_READ_ONLY
        );
        assert_eq!(
            reason(create(&root, ".weaver/tasks", "a", true)),
            WEAVER_READ_ONLY
        );
        assert!(!root.join(".weaver/a").exists());
    }

    // --- rename ---------------------------------------------------------------------------

    #[test]
    fn a_rename_keeps_the_content_and_the_folder() {
        let (_dir, root) = project();
        put(&root, "src/a.txt", "hello");
        assert_eq!(
            rename(&root, "src/a.txt", "b.txt", &free).unwrap(),
            "src/b.txt"
        );
        assert_eq!(read(&root, "src/b.txt"), "hello");
        assert!(!root.join("src/a.txt").exists());
        assert_eq!(rename(&root, "src", "lib", &free).unwrap(), "lib");
        assert_eq!(read(&root, "lib/b.txt"), "hello");
    }

    #[test]
    fn a_rename_never_replaces_an_entry_that_has_the_name() {
        let (_dir, root) = project();
        put(&root, "a.txt", "from");
        put(&root, "b.txt", "target");
        fs::create_dir(root.join("d")).unwrap();
        assert_eq!(
            reason(rename(&root, "a.txt", "b.txt", &free)),
            "A file or folder named \"b.txt\" already exists here."
        );
        assert!(rename(&root, "a.txt", "d", &free).is_err());
        assert_eq!(read(&root, "a.txt"), "from");
        assert_eq!(read(&root, "b.txt"), "target");
    }

    #[test]
    fn a_rename_that_only_changes_the_case_is_allowed() {
        let (_dir, root) = project();
        put(&root, "readme.md", "text");
        assert_eq!(
            rename(&root, "readme.md", "README.md", &free).unwrap(),
            "README.md"
        );
        let names: Vec<String> = fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["README.md"]);
        assert_eq!(read(&root, "README.md"), "text");
        // The same name is not a rename.
        assert_eq!(
            rename(&root, "README.md", "README.md", &free).unwrap(),
            "README.md"
        );
    }

    #[test]
    fn a_rename_refuses_a_bad_name_weaver_the_root_a_missing_entry_and_a_lock() {
        let (_dir, root) = project();
        put(&root, "a.txt", "x");
        put(&root, ".weaver/system.md", "x");
        for name in ["", ".", "..", "x/y"] {
            assert!(rename(&root, "a.txt", name, &free).is_err());
        }
        assert_eq!(
            reason(rename(&root, ".weaver", "w", &free)),
            WEAVER_READ_ONLY
        );
        assert_eq!(
            reason(rename(&root, ".weaver/system.md", "s.md", &free)),
            WEAVER_READ_ONLY
        );
        assert!(rename(&root, "", "x", &free).is_err());
        assert!(rename(&root, "missing", "x", &free).is_err());
        assert!(rename(&root, "../a.txt", "x", &free).is_err());
        let locked = |p: &str| p == "a.txt";
        assert_eq!(reason(rename(&root, "a.txt", "b.txt", &locked)), AGENT_LOCK);
        assert!(root.join("a.txt").exists() && !root.join("b.txt").exists());
    }

    #[test]
    fn a_folder_holding_a_locked_file_is_not_renamed_moved_or_deleted() {
        let (_dir, root) = project();
        put(&root, "src/deep/a.txt", "x");
        fs::create_dir(root.join("other")).unwrap();
        let locked = |p: &str| is_within(p, "src/deep/a.txt");
        assert_eq!(reason(rename(&root, "src", "lib", &locked)), AGENT_LOCK);
        assert_eq!(
            reason(move_into(&root, "src", "other", &locked)),
            AGENT_LOCK
        );
        let trash = |_: &Path| -> Result<(), String> { panic!("nothing is trashed") };
        assert_eq!(reason(delete(&root, "src", &locked, &trash)), AGENT_LOCK);
        assert!(root.join("src/deep/a.txt").exists());
    }

    // --- move -----------------------------------------------------------------------------

    #[test]
    fn a_move_goes_into_a_folder_and_into_the_root() {
        let (_dir, root) = project();
        put(&root, "a.txt", "one");
        put(&root, "src/b.txt", "two");
        fs::create_dir(root.join("dest")).unwrap();
        assert_eq!(
            move_into(&root, "a.txt", "dest", &free).unwrap(),
            "dest/a.txt"
        );
        assert_eq!(read(&root, "dest/a.txt"), "one");
        assert_eq!(move_into(&root, "src/b.txt", "", &free).unwrap(), "b.txt");
        assert_eq!(read(&root, "b.txt"), "two");
        assert_eq!(move_into(&root, "src", "dest", &free).unwrap(), "dest/src");
        assert!(root.join("dest/src").is_dir());
    }

    #[test]
    fn a_move_never_replaces_an_entry_that_has_the_name() {
        let (_dir, root) = project();
        put(&root, "a.txt", "from");
        put(&root, "dest/a.txt", "target");
        assert_eq!(
            reason(move_into(&root, "a.txt", "dest", &free)),
            "A file or folder named \"a.txt\" already exists here."
        );
        assert_eq!(read(&root, "a.txt"), "from");
        assert_eq!(read(&root, "dest/a.txt"), "target");
        // Into the folder it is already in.
        assert!(move_into(&root, "dest/a.txt", "dest", &free).is_err());
    }

    #[test]
    fn a_folder_cannot_go_into_itself_or_into_a_folder_inside_it() {
        let (_dir, root) = project();
        put(&root, "a/b/c.txt", "x");
        assert!(reason(move_into(&root, "a", "a", &free)).contains("into itself"));
        assert!(reason(move_into(&root, "a", "a/b", &free)).contains("into itself"));
        assert_eq!(read(&root, "a/b/c.txt"), "x");
        // A name that only starts like the folder is another folder.
        fs::create_dir(root.join("ab")).unwrap();
        assert_eq!(move_into(&root, "a", "ab", &free).unwrap(), "ab/a");
    }

    #[test]
    fn nothing_goes_into_weaver_and_nothing_leaves_it() {
        let (_dir, root) = project();
        put(&root, "a.txt", "x");
        put(&root, ".weaver/tasks/t.md", "x");
        assert_eq!(
            reason(move_into(&root, "a.txt", ".weaver", &free)),
            WEAVER_READ_ONLY
        );
        assert_eq!(
            reason(move_into(&root, "a.txt", ".weaver/tasks", &free)),
            WEAVER_READ_ONLY
        );
        assert_eq!(
            reason(move_into(&root, ".weaver/tasks/t.md", "", &free)),
            WEAVER_READ_ONLY
        );
        assert_eq!(
            reason(move_into(&root, ".weaver", "", &free)),
            WEAVER_READ_ONLY
        );
        assert!(root.join("a.txt").exists() && root.join(".weaver/tasks/t.md").exists());
    }

    #[test]
    fn a_move_refuses_a_target_that_is_not_a_folder_or_leaves_the_project() {
        let (_dir, root) = project();
        put(&root, "a.txt", "x");
        put(&root, "b.txt", "y");
        assert!(move_into(&root, "a.txt", "b.txt", &free).is_err());
        assert!(move_into(&root, "a.txt", "missing", &free).is_err());
        assert!(move_into(&root, "a.txt", "..", &free).is_err());
        assert!(move_into(&root, "a.txt", ".git", &free).is_err());
        assert!(move_into(&root, "missing", "", &free).is_err());
        assert!(move_into(&root, "", "", &free).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_leaving_the_project_cannot_be_moved_into() {
        let (_dir, root) = project();
        let (_other, outside) = project();
        std::os::unix::fs::symlink(&outside, root.join("out")).unwrap();
        put(&root, "a.txt", "x");
        assert!(move_into(&root, "a.txt", "out", &free).is_err());
        assert!(create(&root, "out", "n.txt", false).is_err());
        assert!(rename(&root, "out/x", "y", &free).is_err());
        assert!(!outside.join("a.txt").exists() && !outside.join("n.txt").exists());
        // The link itself is an entry of the project: it can be renamed.
        assert_eq!(
            rename(&root, "out", "elsewhere", &free).unwrap(),
            "elsewhere"
        );
    }

    #[test]
    fn a_move_across_volumes_has_its_own_reason() {
        let (_dir, root) = project();
        put(&root, "a.txt", "x");
        let error = if cfg!(windows) {
            io::Error::from_raw_os_error(17)
        } else {
            io::Error::from_raw_os_error(18)
        };
        assert!(is_cross_volume(&error));
        assert!(!is_cross_volume(&io::Error::from(io::ErrorKind::NotFound)));
        assert!(root.join("a.txt").exists());
    }

    // --- delete ---------------------------------------------------------------------------

    /// A trash that moves the entry into a folder of its own, as the system's trash does.
    fn fake_trash(bin: PathBuf) -> impl Fn(&Path) -> Result<(), String> {
        move |entry: &Path| {
            fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
            fs::rename(entry, bin.join(entry.file_name().unwrap())).map_err(|e| e.to_string())
        }
    }

    #[test]
    fn a_deleted_entry_goes_to_the_trash_and_leaves_the_tree() {
        let (_dir, root) = project();
        let (_bin_dir, bin) = project();
        put(&root, "src/a.txt", "x");
        put(&root, "b.txt", "y");
        let trash = fake_trash(bin.clone());
        delete(&root, "b.txt", &free, &trash).unwrap();
        delete(&root, "src", &free, &trash).unwrap();
        assert!(!root.join("b.txt").exists() && !root.join("src").exists());
        assert!(bin.join("b.txt").exists() && bin.join("src/a.txt").exists());
    }

    #[test]
    fn when_the_trash_refuses_nothing_is_deleted() {
        let (_dir, root) = project();
        put(&root, "a.txt", "x");
        let trash = |_: &Path| -> Result<(), String> { Err("there is no trash".to_owned()) };
        assert_eq!(
            reason(delete(&root, "a.txt", &free, &trash)),
            "Could not move \"a.txt\" to the trash: there is no trash"
        );
        assert_eq!(read(&root, "a.txt"), "x");
    }

    #[test]
    fn deleting_refuses_weaver_the_root_a_missing_entry_and_a_path_outside() {
        let (_dir, root) = project();
        put(&root, ".weaver/tasks/t.md", "x");
        let trash = |_: &Path| -> Result<(), String> { panic!("nothing is trashed") };
        assert_eq!(
            reason(delete(&root, ".weaver", &free, &trash)),
            WEAVER_READ_ONLY
        );
        assert_eq!(
            reason(delete(&root, ".weaver/tasks/t.md", &free, &trash)),
            WEAVER_READ_ONLY
        );
        assert!(delete(&root, "", &free, &trash).is_err());
        assert!(delete(&root, "missing", &free, &trash).is_err());
        assert!(delete(&root, "../x", &free, &trash).is_err());
        assert!(delete(&root, ".git/config", &free, &trash).is_err());
    }
}
