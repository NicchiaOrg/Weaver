//! Copying files dropped from outside into `.weaver/assets/` (`developer.DROP_ASSETS`).
//!
//! The dropped paths are the only absolute paths the Assets view accepts, as Tauri reports them. A
//! drop is all or nothing (`requirements/rules#asset-view`, **Drop from outside**):
//!
//! * every refusal is checked for every dropped path before anything is written: a path that is a
//!   folder, a symbolic link (read with `symlink_metadata`, never followed) or another kind of
//!   entry, a name that is not valid or starts with `.`, a name an asset already has (as the file
//!   system compares names), two dropped files with the same name (compared the same way);
//! * then the folder `.weaver/assets/` is created when it is missing (and only it), and each file
//!   is made with `create_new` and its bytes copied, so nothing is ever replaced and the originals
//!   are never touched. Each file made is recorded; a failure part way removes them all, and the
//!   folder too when this drop made it. When a removal fails, the error lists what is left;
//! * a drop whose files all lie in the canonical `.weaver/assets/` of the same project, dragged
//!   back onto its own view, is ignored: it returns at once and changes nothing.
//!
//! A file from elsewhere in the project, `.weaver/` included, may be copied in. The size limit
//! never refuses a file: the result says which copied files are over it.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use super::{assets_dir, check_name, exceeds_limit, same_name, sort, taken, Asset};
use crate::file_tab::path;
use crate::file_view::Refusal;

/// Why a drop was not copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyInError {
    /// Refused as a whole before anything was written, with the reason.
    Refused(Refusal),
    /// A step failed part way: `name` is the file that failed, `reason` why, and `not_removed`
    /// what could not be removed afterwards (empty when the project is as it was).
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

/// What a drop did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The files were copied.
    Copied(Copied),
    /// Every dropped file already is an asset of this project (dragged back onto its own view), or
    /// nothing was dropped: nothing was done.
    Ignored,
}

/// The assets a drop made, in the order of the grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copied {
    pub names: Vec<String>,
    /// The ones of them larger than the size limit.
    pub large: Vec<String>,
}

/// One dropped file that passed every check.
struct Item {
    name: String,
    source: PathBuf,
    dest: PathBuf,
}

fn not_a_file(name: &str, what: &str) -> Refusal {
    Refusal::new(format!("\"{name}\" is {what}: only files can be assets."))
}

/// Whether the dropped `source` lies in the canonical assets folder `assets`: its folder is the
/// folder or inside it. The file itself is never followed, so a link in the assets folder that
/// leads elsewhere still lies in it.
fn lies_in(assets: &Path, source: &Path) -> bool {
    let (Some(parent), Some(_)) = (source.parent(), source.file_name()) else {
        return false;
    };
    dunce::canonicalize(parent).is_ok_and(|parent| path::is_inside(assets, &parent))
}

/// Checks one dropped path.
fn check_source(dir: &Path, source: &Path) -> Result<Item, Refusal> {
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
    let kind = meta.file_type();
    if kind.is_symlink() {
        return Err(not_a_file(name, "a symbolic link"));
    }
    if kind.is_dir() {
        return Err(not_a_file(name, "a folder"));
    }
    if !kind.is_file() {
        return Err(not_a_file(name, "not a regular file"));
    }
    check_name(name)?;
    let dest = dir.join(name);
    if fs::symlink_metadata(&dest).is_ok() {
        return Err(taken(name));
    }
    Ok(Item {
        name: name.to_owned(),
        source: source.to_owned(),
        dest,
    })
}

/// Every refusal for the whole drop.
fn plan(root: &Path, sources: &[PathBuf]) -> Result<Vec<Item>, Refusal> {
    let dir = assets_dir(root);
    let mut items: Vec<Item> = Vec::with_capacity(sources.len());
    for source in sources {
        let item = check_source(&dir, source)?;
        if items.iter().any(|other| same_name(&other.name, &item.name)) {
            return Err(Refusal::new(format!(
                "Two dropped files are both named \"{}\".",
                item.name
            )));
        }
        items.push(item);
    }
    Ok(items)
}

/// Copies the bytes of `item` into a file made with `create_new`, which is recorded in `created`
/// as soon as it exists. Returns how many bytes were copied.
fn copy_file(
    item: &Item,
    step: &dyn Fn(&Path) -> io::Result<()>,
    created: &mut Vec<PathBuf>,
) -> io::Result<u64> {
    step(&item.dest)?;
    let mut from = File::open(&item.source)?;
    let mut to = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&item.dest)?;
    created.push(item.dest.clone());
    let bytes = io::copy(&mut from, &mut to)?;
    to.sync_all()?;
    Ok(bytes)
}

/// Removes what a failed drop made. Returns what could not be removed: the file's name and why.
/// A file that is already gone counts as removed.
fn remove_made(created: &[PathBuf]) -> Vec<String> {
    let mut left = Vec::new();
    for path in created.iter().rev() {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                let name = path
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
                left.push(format!("\"{name}\": {error}"));
            }
        }
    }
    left
}

/// Copies the dropped `sources` (absolute paths) into the assets folder of the project at `root`,
/// the canonical project root.
pub fn copy_in(root: &Path, sources: &[PathBuf]) -> Result<Outcome, CopyInError> {
    copy_in_with(root, sources, &|_| Ok(()))
}

/// [`copy_in`] with `step` told each destination before it is made; an error from it fails the
/// copy at that file. The tests inject a failure part way, and wait in it to keep a copy running.
pub fn copy_in_with(
    root: &Path,
    sources: &[PathBuf],
    step: &dyn Fn(&Path) -> io::Result<()>,
) -> Result<Outcome, CopyInError> {
    let dir = assets_dir(root);
    if sources.is_empty() {
        return Ok(Outcome::Ignored);
    }
    if let Ok(canonical) = dunce::canonicalize(&dir) {
        if sources.iter().all(|source| lies_in(&canonical, source)) {
            return Ok(Outcome::Ignored);
        }
    }
    let items = plan(root, sources)?;

    let fail = |name: &str, error: io::Error, not_removed: Vec<String>| CopyInError::Failed {
        name: name.to_owned(),
        reason: error.to_string(),
        not_removed,
    };
    let made_dir = match fs::create_dir(&dir) {
        Ok(()) => true,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => false,
        Err(error) => return Err(fail(&items[0].name, error, Vec::new())),
    };

    let mut created: Vec<PathBuf> = Vec::with_capacity(items.len());
    let mut copied: Vec<Asset> = Vec::with_capacity(items.len());
    for item in &items {
        match copy_file(item, step, &mut created) {
            Ok(size) => copied.push(Asset {
                name: item.name.clone(),
                size,
            }),
            Err(error) => {
                let not_removed = remove_made(&created);
                if made_dir && not_removed.is_empty() {
                    // Best effort: an empty folder left behind is the folder the first asset makes.
                    let _ = fs::remove_dir(&dir);
                }
                return Err(fail(&item.name, error, not_removed));
            }
        }
    }
    sort(&mut copied);
    let large = copied
        .iter()
        .filter(|asset| exceeds_limit(asset.size))
        .map(|asset| asset.name.clone())
        .collect();
    Ok(Outcome::Copied(Copied {
        names: copied.into_iter().map(|asset| asset.name).collect(),
        large,
    }))
}

#[cfg(test)]
mod tests {
    use super::super::list::list;
    use super::super::SIZE_LIMIT;
    use super::*;
    use crate::paths::canonical_root;

    fn project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = canonical_root(dir.path()).unwrap();
        fs::create_dir(root.join(".weaver")).unwrap();
        (dir, root)
    }

    /// A folder outside the project with the files `names` (each holding its own name).
    fn outside(names: &[&str]) -> (tempfile::TempDir, Vec<PathBuf>) {
        let dir = tempfile::tempdir().unwrap();
        let base = canonical_root(dir.path()).unwrap();
        let paths = names
            .iter()
            .map(|name| {
                let path = base.join(name);
                fs::write(&path, name).unwrap();
                path
            })
            .collect();
        (dir, paths)
    }

    fn reason(result: Result<Outcome, CopyInError>) -> String {
        result.unwrap_err().to_string()
    }

    fn names(root: &Path) -> Vec<String> {
        list(root).unwrap().into_iter().map(|a| a.name).collect()
    }

    fn copied(outcome: Outcome) -> Copied {
        match outcome {
            Outcome::Copied(copied) => copied,
            Outcome::Ignored => panic!("the drop was ignored"),
        }
    }

    #[test]
    fn files_are_copied_the_originals_stay_and_the_folder_is_made_by_the_first_copy() {
        let (_dir, root) = project();
        let (_out, sources) = outside(&["b.txt", "A.png"]);
        assert!(!assets_dir(&root).exists());
        let done = copied(copy_in(&root, &sources).unwrap());
        assert_eq!(done.names, ["A.png", "b.txt"]);
        assert!(done.large.is_empty());
        assert_eq!(
            fs::read_to_string(assets_dir(&root).join("b.txt")).unwrap(),
            "b.txt"
        );
        assert_eq!(
            fs::read_to_string(&sources[1]).unwrap(),
            "A.png",
            "the original stays"
        );
        assert!(sources[0].exists());
        assert_eq!(names(&root), ["A.png", "b.txt"]);
        assert!(
            !assets_dir(&root).join(".gitignore").exists(),
            "the assets folder is versioned: no .gitignore"
        );
    }

    #[test]
    fn a_file_from_elsewhere_in_the_project_may_be_copied_in() {
        let (_dir, root) = project();
        fs::write(root.join("logo.png"), "png").unwrap();
        fs::write(root.join(".weaver/specs.yaml"), "x").unwrap();
        let done = copied(
            copy_in(
                &root,
                &[root.join("logo.png"), root.join(".weaver/specs.yaml")],
            )
            .unwrap(),
        );
        assert_eq!(done.names, ["logo.png", "specs.yaml"]);
        assert!(root.join("logo.png").exists());
    }

    #[test]
    fn a_file_over_the_limit_is_copied_and_named_one_at_the_limit_is_not() {
        let (_dir, root) = project();
        let (_out, sources) = outside(&["exact.bin", "over.bin", "small.txt"]);
        File::options()
            .write(true)
            .open(&sources[0])
            .unwrap()
            .set_len(SIZE_LIMIT)
            .unwrap();
        File::options()
            .write(true)
            .open(&sources[1])
            .unwrap()
            .set_len(SIZE_LIMIT + 1)
            .unwrap();
        let done = copied(copy_in(&root, &sources).unwrap());
        assert_eq!(done.names, ["exact.bin", "over.bin", "small.txt"]);
        assert_eq!(done.large, ["over.bin"]);
        let sizes: Vec<u64> = list(&root).unwrap().iter().map(|a| a.size).collect();
        assert_eq!(sizes, [SIZE_LIMIT, SIZE_LIMIT + 1, 9]);
    }

    #[test]
    fn a_folder_refuses_the_whole_drop_and_nothing_is_copied() {
        let (_dir, root) = project();
        let (out, mut sources) = outside(&["a.txt"]);
        fs::create_dir(out.path().join("pictures")).unwrap();
        sources.push(canonical_root(out.path()).unwrap().join("pictures"));
        assert_eq!(
            reason(copy_in(&root, &sources)),
            "\"pictures\" is a folder: only files can be assets."
        );
        assert!(!assets_dir(&root).exists(), "not even the folder is made");
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_refuses_the_whole_drop_even_when_it_leads_to_a_file() {
        let (_dir, root) = project();
        let (out, mut sources) = outside(&["a.txt"]);
        let link = canonical_root(out.path()).unwrap().join("shortcut.txt");
        std::os::unix::fs::symlink(&sources[0], &link).unwrap();
        sources.push(link);
        assert_eq!(
            reason(copy_in(&root, &sources)),
            "\"shortcut.txt\" is a symbolic link: only files can be assets."
        );
        assert!(!assets_dir(&root).exists());
    }

    #[test]
    fn a_name_that_is_not_valid_or_starts_with_a_dot_refuses_the_whole_drop() {
        let (_dir, root) = project();
        let (_out, sources) = outside(&["fine.txt", ".env"]);
        assert_eq!(
            reason(copy_in(&root, &sources)),
            "Names starting with \".\" are not allowed."
        );
        assert!(!assets_dir(&root).exists());
        // A name the Windows rules refuse is checked by `check_name`; here the system's own rules.
        if cfg!(windows) {
            let (_out, sources) = outside(&["CON.txt"]);
            assert!(copy_in(&root, &sources).is_err());
        }
    }

    #[test]
    fn a_name_an_asset_already_has_refuses_the_drop_and_the_asset_is_not_replaced() {
        let (_dir, root) = project();
        let (_out, sources) = outside(&["a.txt"]);
        copy_in(&root, &sources).unwrap();
        fs::write(assets_dir(&root).join("a.txt"), "mine").unwrap();
        let (_other, again) = outside(&["new.txt", "a.txt"]);
        assert_eq!(
            reason(copy_in(&root, &again)),
            "An asset named \"a.txt\" already exists."
        );
        assert_eq!(
            fs::read_to_string(assets_dir(&root).join("a.txt")).unwrap(),
            "mine"
        );
        assert_eq!(names(&root), ["a.txt"], "new.txt was not copied either");
    }

    #[test]
    fn names_are_compared_as_the_file_system_compares_them() {
        let (_dir, root) = project();
        let (_out, sources) = outside(&["Logo.png"]);
        copy_in(&root, &sources).unwrap();
        let (_other, lower) = outside(&["logo.png"]);
        let result = copy_in(&root, &lower);
        if cfg!(any(windows, target_os = "macos")) {
            assert_eq!(
                reason(result),
                "An asset named \"logo.png\" already exists."
            );
        } else {
            copied(result.unwrap());
            assert_eq!(names(&root), ["Logo.png", "logo.png"]);
        }
    }

    #[test]
    fn two_dropped_files_with_one_name_refuse_the_drop() {
        let (_dir, root) = project();
        let (_a, mut sources) = outside(&["same.txt"]);
        let (_b, more) = outside(&["same.txt"]);
        sources.extend(more);
        assert_eq!(
            reason(copy_in(&root, &sources)),
            "Two dropped files are both named \"same.txt\"."
        );
        assert!(!assets_dir(&root).exists());
        // Without regard to case where the file system does so.
        if cfg!(any(windows, target_os = "macos")) {
            let (_c, mut sources) = outside(&["Same.txt"]);
            let (_d, more) = outside(&["same.txt"]);
            sources.extend(more);
            assert!(reason(copy_in(&root, &sources)).starts_with("Two dropped files"));
        }
    }

    #[test]
    fn a_path_that_is_gone_or_relative_refuses_the_drop() {
        let (_dir, root) = project();
        let (out, _) = outside(&[]);
        let gone = canonical_root(out.path()).unwrap().join("gone.txt");
        assert_eq!(
            reason(copy_in(&root, &[gone])),
            "\"gone.txt\" no longer exists."
        );
        assert!(reason(copy_in(&root, &[PathBuf::from("relative.txt")])).contains("full path"));
    }

    #[test]
    fn dropping_the_assets_of_the_project_onto_its_own_view_does_nothing() {
        let (_dir, root) = project();
        let (_out, sources) = outside(&["a.txt", "b.txt"]);
        copy_in(&root, &sources).unwrap();
        let own: Vec<PathBuf> = ["a.txt", "b.txt"]
            .iter()
            .map(|name| assets_dir(&root).join(name))
            .collect();
        assert_eq!(copy_in(&root, &own).unwrap(), Outcome::Ignored);
        assert_eq!(copy_in(&root, &own[..1]).unwrap(), Outcome::Ignored);
        assert_eq!(copy_in(&root, &[]).unwrap(), Outcome::Ignored);
        assert_eq!(names(&root), ["a.txt", "b.txt"]);
        // Mixed with a file from outside, the asset's name is taken and the drop is refused.
        let (_other, extra) = outside(&["c.txt"]);
        let mixed = vec![own[0].clone(), extra[0].clone()];
        assert_eq!(
            reason(copy_in(&root, &mixed)),
            "An asset named \"a.txt\" already exists."
        );
        assert_eq!(names(&root), ["a.txt", "b.txt"]);
    }

    #[test]
    fn the_assets_of_another_project_are_copied_like_any_file() {
        let (_one, first) = project();
        let (_two, second) = project();
        let (_out, sources) = outside(&["a.txt"]);
        copy_in(&first, &sources).unwrap();
        let from_first = vec![assets_dir(&first).join("a.txt")];
        assert!(matches!(
            copy_in(&second, &from_first).unwrap(),
            Outcome::Copied(_)
        ));
        assert_eq!(names(&second), ["a.txt"]);
    }

    #[test]
    fn a_failure_part_way_removes_everything_the_drop_copied_and_the_folder_it_made() {
        let (_dir, root) = project();
        let (_out, sources) = outside(&["a.txt", "b.txt", "c.txt"]);
        let fail_on_c = |dest: &Path| {
            if dest.ends_with("c.txt") {
                Err(io::Error::other("the disk is full"))
            } else {
                Ok(())
            }
        };
        let error = copy_in_with(&root, &sources, &fail_on_c).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Could not copy \"c.txt\": the disk is full. Nothing was copied."
        );
        assert!(
            !assets_dir(&root).exists(),
            "the folder this drop made goes too"
        );
        assert!(
            sources.iter().all(|source| source.exists()),
            "the originals stay"
        );
    }

    #[test]
    fn a_failure_part_way_leaves_the_assets_that_were_there_untouched() {
        let (_dir, root) = project();
        let (_out, first) = outside(&["old.txt"]);
        copy_in(&root, &first).unwrap();
        let (_more, sources) = outside(&["a.txt", "b.txt"]);
        let fail_on_b = |dest: &Path| {
            if dest.ends_with("b.txt") {
                Err(io::Error::other("no room"))
            } else {
                Ok(())
            }
        };
        assert!(copy_in_with(&root, &sources, &fail_on_b).is_err());
        assert_eq!(names(&root), ["old.txt"]);
    }

    #[test]
    fn a_file_that_appears_in_the_meantime_is_never_replaced() {
        let (_dir, root) = project();
        let (_out, sources) = outside(&["a.txt", "b.txt"]);
        let race = |dest: &Path| {
            if dest.ends_with("b.txt") {
                fs::write(dest, "somebody else's").unwrap();
            }
            Ok(())
        };
        let error = copy_in_with(&root, &sources, &race).unwrap_err();
        assert!(error.to_string().starts_with("Could not copy \"b.txt\""));
        assert_eq!(
            fs::read_to_string(assets_dir(&root).join("b.txt")).unwrap(),
            "somebody else's"
        );
        assert_eq!(names(&root), ["b.txt"], "a.txt, copied before, was removed");
    }

    #[test]
    fn when_the_removal_fails_the_error_lists_what_is_left() {
        let (_dir, root) = project();
        let (_out, sources) = outside(&["a.txt", "b.txt"]);
        // On the second file, the first copy is swapped for a folder with something in it, which
        // `remove_file` cannot remove.
        let swap = |dest: &Path| {
            if dest.ends_with("b.txt") {
                let first = dest.with_file_name("a.txt");
                fs::remove_file(&first)?;
                fs::create_dir(&first)?;
                fs::write(first.join("inside"), "x")?;
                return Err(io::Error::other("it failed"));
            }
            Ok(())
        };
        let message = copy_in_with(&root, &sources, &swap)
            .unwrap_err()
            .to_string();
        assert!(
            message.starts_with(
                "Could not copy \"b.txt\": it failed. What was already copied could not be removed: \"a.txt\":"
            ),
            "{message}"
        );
        assert!(assets_dir(&root).join("a.txt").exists());
    }

    #[test]
    fn without_a_weaver_folder_the_copy_fails_and_makes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let root = canonical_root(dir.path()).unwrap();
        let (_out, sources) = outside(&["a.txt"]);
        let message = reason(copy_in(&root, &sources));
        assert!(message.starts_with("Could not copy \"a.txt\""), "{message}");
        assert!(message.ends_with("Nothing was copied."));
        assert!(!root.join(".weaver").exists());
    }
}
