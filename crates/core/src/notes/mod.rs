//! The developer's notes in `.weaver/notes/`: the sheet `notes.md` and the images pasted in it.
//!
//! The folder is local only (its own `.gitignore` with `*`, see
//! [`layout::ensure_local_folder`](crate::project::layout::ensure_local_folder)) and is created
//! when the sheet or an image is first written, never at initialisation. Its files are never
//! Model, Requirement, Specs or card files and never part of the project image; only the Notes
//! view writes in it.
//!
//! This module holds what the watcher needs to tell the Notes view what changed on disk: the
//! hash of the sheet and the names of the images. Everything is synchronous `std` I/O on the
//! canonical project root.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`sheet`] | Reading and writing `notes.md`: LF text, the hash of the bytes, the atomic write |
//! | [`images`] | Naming, accepting and copying images in, reading one for display, the trash |

pub mod images;
pub mod sheet;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::file_tab::read::sha256_bytes;
use crate::project::layout::{self, NOTES_DIR};

/// The sheet, directly in `.weaver/notes/`.
pub const SHEET_FILE: &str = "notes.md";

/// Folders nested deeper than this are not listed (protects against symlink loops).
const MAX_DEPTH: usize = 8;

/// `<project root>/.weaver/notes`.
pub fn notes_dir(root: &Path) -> PathBuf {
    layout::weaver_dir(root).join(NOTES_DIR)
}

/// `<project root>/.weaver/notes/notes.md`.
pub fn sheet_path(root: &Path) -> PathBuf {
    notes_dir(root).join(SHEET_FILE)
}

/// The SHA-256 (lowercase hex) of the bytes of the sheet, the hash every read and write of the
/// sheet uses (`file_tab::read::sha256_bytes`). `None` when the file is missing (or the notes
/// folder is).
pub fn sheet_hash(root: &Path) -> io::Result<Option<String>> {
    match fs::read(sheet_path(root)) {
        Ok(bytes) => Ok(Some(sha256_bytes(&bytes))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        // `notes` is a file, not a folder: there is no sheet.
        Err(e) if e.kind() == io::ErrorKind::NotADirectory => Ok(None),
        Err(e) => Err(e),
    }
}

/// The names of the images in `.weaver/notes/` now: every file but the sheet, as a path relative
/// to the folder with `/` separators, sorted. Names starting with `.` (the `.gitignore` and the
/// temp files of atomic writes) are left out. Empty when the folder does not exist.
pub fn image_names(root: &Path) -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    collect(&notes_dir(root), "", 0, &mut names)?;
    names.retain(|name| name != SHEET_FILE);
    names.sort();
    Ok(names)
}

fn collect(dir: &Path, rel: &str, depth: usize, names: &mut Vec<String>) -> io::Result<()> {
    let listing = match fs::read_dir(dir) {
        Ok(listing) => listing,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotADirectory => return Ok(()),
        Err(e) => return Err(e),
    };
    for item in listing {
        let item = item?;
        let name = item.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let child = if rel.is_empty() {
            name
        } else {
            format!("{rel}/{name}")
        };
        let kind = item.file_type()?;
        if kind.is_dir() {
            if depth < MAX_DEPTH {
                collect(&item.path(), &child, depth + 1, names)?;
            }
        } else {
            names.push(child);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        (dir, root)
    }

    fn put(root: &Path, name: &str, bytes: &[u8]) {
        let path = notes_dir(root).join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn a_missing_sheet_has_no_hash_and_reading_never_makes_the_folder() {
        let (_dir, root) = root();
        assert_eq!(sheet_hash(&root).unwrap(), None);
        assert_eq!(image_names(&root).unwrap(), Vec::<String>::new());
        assert!(!notes_dir(&root).exists());
    }

    #[test]
    fn the_hash_is_of_the_bytes_of_the_file() {
        let (_dir, root) = root();
        put(&root, SHEET_FILE, b"one\ntwo\n");
        assert_eq!(
            sheet_hash(&root).unwrap(),
            Some(sha256_bytes(b"one\ntwo\n"))
        );
        // Another editor's line endings and BOM are another content.
        put(&root, SHEET_FILE, b"\xef\xbb\xbfone\r\ntwo\r\n");
        assert_eq!(
            sheet_hash(&root).unwrap(),
            Some(sha256_bytes(b"\xef\xbb\xbfone\r\ntwo\r\n"))
        );
    }

    #[test]
    fn the_images_are_every_file_but_the_sheet_and_the_hidden_ones() {
        let (_dir, root) = root();
        put(&root, SHEET_FILE, b"x");
        put(&root, ".gitignore", b"*\n");
        put(&root, ".notes.md.0123.tmp", b"x");
        put(&root, "b.png", b"x");
        put(&root, "a.png", b"x");
        put(&root, "pasted/c.png", b"x");
        assert_eq!(
            image_names(&root).unwrap(),
            ["a.png", "b.png", "pasted/c.png"]
        );
    }
}
