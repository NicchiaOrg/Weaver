//! The actions of the Assets view that change or hand out assets: rename, delete to the system's
//! trash and the paths of a drag out (`developer.RENAME_ASSET`, `DELETE_ASSETS`,
//! `DRAG_ASSETS_OUT`).
//!
//! Each one takes asset names, never paths, and checks every refusal of
//! `requirements/rules#asset-view` again when it runs, against the disk as it is at that moment. A
//! refused action changes nothing. The operations of [`crate::file_view`] refuse `.weaver/` as a
//! whole and stay as they are; these are the assets' own, built on the same parts: the checks of
//! names, the rename that never replaces, the system trash.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{assets_dir, check_name, same_name, taken};
use crate::file_tab::path;
use crate::file_view::ops::same_entry;
use crate::file_view::rename::rename_no_replace;
use crate::file_view::Refusal;

pub use crate::file_view::ops::system_trash;

/// The asset `name` of the folder `dir`: not followed when it is a link, and refused when it is
/// gone or is anything but a regular file.
fn asset_in(dir: &Path, name: &str) -> Result<(PathBuf, fs::Metadata), Refusal> {
    check_name(name)?;
    let asset = dir.join(name);
    match fs::symlink_metadata(&asset) {
        Ok(meta) if meta.file_type().is_file() => Ok((asset, meta)),
        Ok(_) => Err(Refusal::new(format!("\"{name}\" is not an asset."))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(Refusal::new(format!("\"{name}\" no longer exists.")))
        }
        Err(error) => Err(Refusal::new(format!("\"{name}\" cannot be read: {error}"))),
    }
}

/// Gives the asset `name` the name `new_name`, in the assets folder of the project at `root` (the
/// canonical project root). It keeps its content and never replaces another asset; a rename that
/// only changes the case of the name is allowed where the file system compares names without
/// regard to case. Returns the new name.
pub fn rename(root: &Path, name: &str, new_name: &str) -> Result<String, Refusal> {
    let dir = assets_dir(root);
    let (source, source_meta) = asset_in(&dir, name)?;
    check_name(new_name)?;
    if new_name == name {
        return Ok(new_name.to_owned());
    }
    let target = dir.join(new_name);
    let failed = |error: io::Error| Refusal::new(format!("Could not rename \"{name}\": {error}"));
    match fs::symlink_metadata(&target) {
        Ok(existing) => {
            // Only the case of the name changes and the target is the asset itself: allowed.
            if same_name(name, new_name) && same_entry(&source_meta, &existing) {
                fs::rename(&source, &target).map_err(failed)?;
            } else {
                return Err(taken(new_name));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            match rename_no_replace(&source, &target) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    return Err(taken(new_name))
                }
                Err(error) => return Err(failed(error)),
            }
        }
        Err(error) => return Err(failed(error)),
    }
    Ok(new_name.to_owned())
}

/// An asset the trash refused (or that could not be sent there) and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub name: String,
    pub reason: String,
}

/// What a deletion did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Deleted {
    /// The assets that went to the trash, in the order asked.
    pub trashed: Vec<String>,
    /// The ones that stayed, with the reason. An asset that was already gone is neither.
    pub refused: Vec<Refused>,
}

/// Sends each of `names` to the trash on its own with `trash` (the `trash` crate in the app, a
/// fake in the tests): the ones the trash refuses stay, and the others are not put back.
pub fn delete(
    root: &Path,
    names: &[String],
    trash: &dyn Fn(&Path) -> Result<(), String>,
) -> Deleted {
    let dir = assets_dir(root);
    let mut done = Deleted::default();
    let mut seen: Vec<&str> = Vec::with_capacity(names.len());
    for name in names {
        if seen.contains(&name.as_str()) {
            continue;
        }
        seen.push(name);
        let refuse = |reason: String| Refused {
            name: name.clone(),
            reason,
        };
        if let Err(refusal) = check_name(name) {
            done.refused.push(refuse(refusal.reason().to_owned()));
            continue;
        }
        let asset = dir.join(name);
        match fs::symlink_metadata(&asset) {
            Ok(meta) if meta.file_type().is_file() => match trash(&asset) {
                Ok(()) => done.trashed.push(name.clone()),
                Err(reason) => done.refused.push(refuse(reason)),
            },
            Ok(_) => done
                .refused
                .push(refuse(format!("\"{name}\" is not an asset."))),
            // Gone since the list was read: it leaves the view with the next list.
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => done
                .refused
                .push(refuse(format!("it cannot be read: {error}"))),
        }
    }
    done
}

/// The canonical paths of the assets `names` for a drag out, each checked to be an asset of the
/// project at `root` and to lie directly in its canonical assets folder.
pub fn drag_paths(root: &Path, names: &[String]) -> Result<Vec<PathBuf>, Refusal> {
    let dir = assets_dir(root);
    let canonical_dir =
        dunce::canonicalize(&dir).map_err(|_| Refusal::new("The project has no assets."))?;
    let mut paths: Vec<PathBuf> = Vec::with_capacity(names.len());
    for name in names {
        let (asset, _) = asset_in(&dir, name)?;
        let canonical = dunce::canonicalize(&asset)
            .map_err(|error| Refusal::new(format!("\"{name}\" cannot be read: {error}")))?;
        let direct = canonical
            .parent()
            .is_some_and(|parent| path::same_file(parent, &canonical_dir));
        if !direct {
            return Err(Refusal::new(format!("\"{name}\" is not an asset.")));
        }
        if !paths.contains(&canonical) {
            paths.push(canonical);
        }
    }
    if paths.is_empty() {
        return Err(Refusal::new("There is no asset to drag."));
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::super::list::list;
    use super::*;
    use crate::paths::canonical_root;

    fn project_with(files: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = canonical_root(dir.path()).unwrap();
        fs::create_dir_all(assets_dir(&root)).unwrap();
        for (name, text) in files {
            fs::write(assets_dir(&root).join(name), text).unwrap();
        }
        (dir, root)
    }

    fn names(root: &Path) -> Vec<String> {
        list(root).unwrap().into_iter().map(|a| a.name).collect()
    }

    fn reason<T: std::fmt::Debug>(result: Result<T, Refusal>) -> String {
        result.unwrap_err().to_string()
    }

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    // --- rename -------------------------------------------------------------------------------

    #[test]
    fn a_rename_keeps_the_content_under_the_new_name() {
        let (_dir, root) = project_with(&[("a.txt", "content")]);
        assert_eq!(rename(&root, "a.txt", "b.txt").unwrap(), "b.txt");
        assert_eq!(names(&root), ["b.txt"]);
        assert_eq!(
            fs::read_to_string(assets_dir(&root).join("b.txt")).unwrap(),
            "content"
        );
    }

    #[test]
    fn a_rename_never_replaces_another_asset() {
        let (_dir, root) = project_with(&[("a.txt", "first"), ("b.txt", "second")]);
        assert_eq!(
            reason(rename(&root, "a.txt", "b.txt")),
            "An asset named \"b.txt\" already exists."
        );
        assert_eq!(
            fs::read_to_string(assets_dir(&root).join("a.txt")).unwrap(),
            "first"
        );
        assert_eq!(
            fs::read_to_string(assets_dir(&root).join("b.txt")).unwrap(),
            "second"
        );
    }

    #[test]
    fn a_rename_that_only_changes_the_case_is_allowed_and_a_different_name_in_another_case_is_not()
    {
        let (_dir, root) = project_with(&[("logo.png", "logo"), ("Photo.png", "photo")]);
        rename(&root, "logo.png", "Logo.png").unwrap();
        assert_eq!(names(&root), ["Logo.png", "Photo.png"]);
        assert_eq!(
            fs::read_to_string(assets_dir(&root).join("Logo.png")).unwrap(),
            "logo"
        );
        let other = rename(&root, "Logo.png", "photo.png");
        if cfg!(any(windows, target_os = "macos")) {
            assert_eq!(
                reason(other),
                "An asset named \"photo.png\" already exists."
            );
            assert_eq!(names(&root), ["Logo.png", "Photo.png"]);
        } else {
            other.unwrap();
            assert_eq!(names(&root), ["Photo.png", "photo.png"]);
        }
    }

    #[test]
    fn a_rename_refuses_a_name_that_is_not_valid() {
        let (_dir, root) = project_with(&[("a.txt", "x")]);
        assert_eq!(
            reason(rename(&root, "a.txt", ".hidden")),
            "Names starting with \".\" are not allowed."
        );
        assert_eq!(
            reason(rename(&root, "a.txt", "")),
            "The name cannot be empty."
        );
        assert!(rename(&root, "a.txt", "folder/b.txt").is_err());
        assert!(rename(&root, "a.txt", "..").is_err());
        assert_eq!(names(&root), ["a.txt"]);
    }

    #[test]
    fn a_rename_takes_names_not_paths_and_only_assets() {
        let (_dir, root) = project_with(&[("a.txt", "x")]);
        fs::create_dir(assets_dir(&root).join("folder")).unwrap();
        fs::write(root.join("outside.txt"), "x").unwrap();
        assert!(rename(&root, "../../outside.txt", "in.txt").is_err());
        assert!(root.join("outside.txt").exists());
        assert_eq!(
            reason(rename(&root, "folder", "other")),
            "\"folder\" is not an asset."
        );
        assert_eq!(
            reason(rename(&root, "missing.txt", "x.txt")),
            "\"missing.txt\" no longer exists."
        );
        assert_eq!(rename(&root, "a.txt", "a.txt").unwrap(), "a.txt");
    }

    // --- delete -------------------------------------------------------------------------------

    /// A trash that moves the asset into a folder of its own, as the system's trash does.
    fn fake_trash(bin: PathBuf) -> impl Fn(&Path) -> Result<(), String> {
        move |path: &Path| {
            fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
            fs::rename(path, bin.join(path.file_name().unwrap())).map_err(|e| e.to_string())
        }
    }

    #[test]
    fn the_assets_deleted_go_to_the_trash_and_leave_the_list() {
        let (dir, root) = project_with(&[("a.txt", "a"), ("b.txt", "b"), ("c.txt", "c")]);
        let bin = canonical_root(dir.path()).unwrap().join("bin");
        let done = delete(
            &root,
            &strings(&["a.txt", "c.txt"]),
            &fake_trash(bin.clone()),
        );
        assert_eq!(done.trashed, ["a.txt", "c.txt"]);
        assert!(done.refused.is_empty());
        assert_eq!(names(&root), ["b.txt"]);
        assert_eq!(fs::read_to_string(bin.join("a.txt")).unwrap(), "a");
    }

    #[test]
    fn one_the_trash_refuses_stays_and_the_others_are_not_put_back() {
        let (dir, root) = project_with(&[("a.txt", "a"), ("b.txt", "b"), ("c.txt", "c")]);
        let bin = canonical_root(dir.path()).unwrap().join("bin");
        let inner = fake_trash(bin);
        let trash = |path: &Path| {
            if path.ends_with("b.txt") {
                Err("there is no trash".to_owned())
            } else {
                inner(path)
            }
        };
        let done = delete(&root, &strings(&["a.txt", "b.txt", "c.txt"]), &trash);
        assert_eq!(done.trashed, ["a.txt", "c.txt"]);
        assert_eq!(
            done.refused,
            [Refused {
                name: "b.txt".into(),
                reason: "there is no trash".into()
            }]
        );
        assert_eq!(names(&root), ["b.txt"]);
    }

    #[test]
    fn deleting_asks_the_trash_for_assets_only_once_each() {
        let (_dir, root) = project_with(&[("a.txt", "a")]);
        fs::create_dir(assets_dir(&root).join("folder")).unwrap();
        let asked = std::cell::RefCell::new(Vec::new());
        let trash = |path: &Path| {
            asked.borrow_mut().push(path.to_path_buf());
            Ok(())
        };
        let done = delete(
            &root,
            &strings(&["a.txt", "a.txt", "folder", "missing.txt", "../x", ".hidden"]),
            &trash,
        );
        assert_eq!(done.trashed, ["a.txt"]);
        assert_eq!(asked.borrow().len(), 1, "only the asset reached the trash");
        let refused: Vec<(&str, &str)> = done
            .refused
            .iter()
            .map(|r| (r.name.as_str(), r.reason.as_str()))
            .collect();
        assert_eq!(
            refused,
            [
                ("folder", "\"folder\" is not an asset."),
                (
                    "../x",
                    "The name cannot contain \"/\", \"\\\" or a NUL character."
                ),
                (".hidden", "Names starting with \".\" are not allowed."),
            ]
        );
    }

    // --- drag out -----------------------------------------------------------------------------

    #[test]
    fn a_drag_out_resolves_the_canonical_paths_of_the_assets() {
        let (_dir, root) = project_with(&[("a.txt", "a"), ("b.txt", "b")]);
        let paths = drag_paths(&root, &strings(&["b.txt", "a.txt", "b.txt"])).unwrap();
        let canonical = dunce::canonicalize(assets_dir(&root)).unwrap();
        assert_eq!(paths, [canonical.join("b.txt"), canonical.join("a.txt")]);
    }

    #[test]
    fn a_drag_out_refuses_what_is_not_an_asset() {
        let (_dir, root) = project_with(&[("a.txt", "a")]);
        fs::create_dir(assets_dir(&root).join("folder")).unwrap();
        fs::write(root.join("outside.txt"), "x").unwrap();
        for bad in [
            "folder",
            "missing.txt",
            "../../outside.txt",
            ".hidden",
            "a/b",
        ] {
            assert!(
                drag_paths(&root, &strings(&["a.txt", bad])).is_err(),
                "{bad}"
            );
        }
        assert!(drag_paths(&root, &[]).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                root.join("outside.txt"),
                assets_dir(&root).join("link.txt"),
            )
            .unwrap();
            assert_eq!(
                reason(drag_paths(&root, &strings(&["link.txt"]))),
                "\"link.txt\" is not an asset."
            );
        }
        let (_other, empty) = {
            let dir = tempfile::tempdir().unwrap();
            let root = canonical_root(dir.path()).unwrap();
            (dir, root)
        };
        assert_eq!(
            reason(drag_paths(&empty, &strings(&["a.txt"]))),
            "The project has no assets."
        );
    }
}
