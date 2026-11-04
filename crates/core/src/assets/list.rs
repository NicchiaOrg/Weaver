//! Listing the assets of a project (`requirements/tech#asset-view`, **Asset list**): one
//! `read_dir` of `.weaver/assets/` and the metadata it returns, never opening a file and never
//! following a link.
//!
//! An asset is a regular file whose name does not start with `.`. Folders, symbolic links, other
//! kinds of entry and dot files put there by other means are left out; so is a name that is not
//! valid UTF-8, since no command could name it. A missing folder is an empty list.

use std::fs;
use std::io;
use std::path::Path;

use super::{assets_dir, sort, Asset};

/// The assets of the project at `root` (canonical), in the order of the grid.
pub fn list(root: &Path) -> io::Result<Vec<Asset>> {
    list_dir(&assets_dir(root))
}

/// The assets in the folder `dir`.
fn list_dir(dir: &Path) -> io::Result<Vec<Asset>> {
    let read = match fs::read_dir(dir) {
        Ok(read) => read,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut assets = Vec::new();
    for item in read {
        // An entry that vanished or cannot be read while the folder is listed is not an asset.
        let Ok(item) = item else {
            continue;
        };
        let Ok(name) = item.file_name().into_string() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        // `DirEntry::metadata` does not follow a link: a link is never a regular file here.
        let Ok(meta) = item.metadata() else {
            continue;
        };
        if !meta.file_type().is_file() {
            continue;
        }
        assets.push(Asset {
            name,
            size: meta.len(),
        });
    }
    sort(&mut assets);
    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::canonical_root;
    use std::path::PathBuf;

    fn project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = canonical_root(dir.path()).unwrap();
        fs::create_dir(root.join(".weaver")).unwrap();
        (dir, root)
    }

    #[test]
    fn a_project_without_the_folder_has_no_assets() {
        let (_dir, root) = project();
        assert_eq!(list(&root).unwrap(), []);
        assert!(
            !assets_dir(&root).exists(),
            "listing never makes the folder"
        );
    }

    #[test]
    fn the_assets_are_the_regular_files_with_their_sizes_in_the_order_of_the_grid() {
        let (_dir, root) = project();
        let dir = assets_dir(&root);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("b.txt"), "hello").unwrap();
        fs::write(dir.join("Apple.png"), "").unwrap();
        fs::write(dir.join("cherry.png"), vec![0u8; 2048]).unwrap();
        let assets = list(&root).unwrap();
        assert_eq!(
            assets,
            [
                Asset {
                    name: "Apple.png".into(),
                    size: 0
                },
                Asset {
                    name: "b.txt".into(),
                    size: 5
                },
                Asset {
                    name: "cherry.png".into(),
                    size: 2048
                },
            ]
        );
    }

    #[test]
    fn dot_names_folders_and_links_are_left_out() {
        let (_dir, root) = project();
        let dir = assets_dir(&root);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("kept.txt"), "x").unwrap();
        fs::write(dir.join(".hidden"), "x").unwrap();
        fs::write(dir.join(".gitignore"), "*").unwrap();
        fs::create_dir(dir.join("folder")).unwrap();
        fs::write(dir.join("folder/inside.txt"), "x").unwrap();
        fs::write(root.join("target.txt"), "x").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("target.txt"), dir.join("link.txt")).unwrap();
            std::os::unix::fs::symlink(root.join("missing"), dir.join("broken")).unwrap();
        }
        let names: Vec<String> = list(&root).unwrap().into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["kept.txt"]);
    }

    #[test]
    fn an_assets_path_that_is_a_file_is_an_error_not_an_empty_list() {
        let (_dir, root) = project();
        fs::write(assets_dir(&root), "not a folder").unwrap();
        assert!(list(&root).is_err());
    }
}
