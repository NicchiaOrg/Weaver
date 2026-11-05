//! The sheet `.weaver/notes/notes.md`: reading it for the Notes view and writing it back
//! (`requirements/tech#notes-view`, Reading and Writing).
//!
//! Reading returns the text as the view shows it (no byte order mark, LF line endings) with the
//! SHA-256 of the raw bytes of the file. Writing takes the whole text and writes it atomically
//! with the shared writer, in UTF-8 with LF, and returns the hash of the bytes it wrote. The
//! folder, with its `.gitignore`, is created by the first write and never before.

use std::fs;
use std::io;
use std::path::Path;

use super::sheet_path;
use crate::atomic;
use crate::file_tab::read::{sha256_bytes, to_ui_text};
use crate::project::layout::{self, NOTES_DIR};

/// The sheet as the view gets it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheet {
    /// No byte order mark, LF line endings.
    pub text: String,
    /// SHA-256 (lowercase hex) of the bytes of the file as they are on disk.
    pub hash: String,
}

/// The sheet of the project at `root`, `None` when the file does not exist. A file that is not
/// valid UTF-8 is an error: reading it lossily would let the next write destroy its bytes.
pub fn read(root: &Path) -> io::Result<Option<Sheet>> {
    let bytes = match fs::read(sheet_path(root)) {
        Ok(bytes) => bytes,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(None)
        }
        Err(error) => return Err(error),
    };
    let hash = sha256_bytes(&bytes);
    let text = String::from_utf8(bytes).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "the sheet is not valid UTF-8 text",
        )
    })?;
    Ok(Some(Sheet {
        text: to_ui_text(&text),
        hash,
    }))
}

/// The bytes written for `text`: UTF-8 with LF line endings.
pub fn disk_bytes(text: &str) -> Vec<u8> {
    text.replace("\r\n", "\n").into_bytes()
}

/// The hash a write of `text` leaves in the file.
pub fn hash_of(text: &str) -> String {
    sha256_bytes(&disk_bytes(text))
}

/// Writes `text` as the whole sheet of the project at `root`, atomically, creating
/// `.weaver/notes/` with its `.gitignore` first when it is missing. Returns the hash of the bytes
/// written.
pub fn write(root: &Path, text: &str) -> io::Result<String> {
    let bytes = disk_bytes(text);
    layout::ensure_local_folder(root, NOTES_DIR)?;
    atomic::write(&sheet_path(root), &bytes, None)?;
    Ok(sha256_bytes(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::notes_dir;
    use crate::paths::canonical_root;

    fn project() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = canonical_root(dir.path()).unwrap();
        (dir, root)
    }

    #[test]
    fn a_missing_sheet_reads_as_none_and_creates_nothing() {
        let (_dir, root) = project();
        assert_eq!(read(&root).unwrap(), None);
        fs::create_dir_all(layout::weaver_dir(&root)).unwrap();
        assert_eq!(read(&root).unwrap(), None);
        assert!(!notes_dir(&root).exists());
    }

    #[test]
    fn the_first_write_creates_the_folder_with_its_gitignore() {
        let (_dir, root) = project();
        let hash = write(&root, "# Notes\n").unwrap();
        assert_eq!(fs::read(sheet_path(&root)).unwrap(), b"# Notes\n");
        assert_eq!(
            fs::read(notes_dir(&root).join(".gitignore")).unwrap(),
            b"*\n"
        );
        assert_eq!(hash, sha256_bytes(b"# Notes\n"));
        let sheet = read(&root).unwrap().unwrap();
        assert_eq!(sheet.text, "# Notes\n");
        assert_eq!(sheet.hash, hash);
    }

    #[test]
    fn a_write_replaces_the_whole_sheet_and_leaves_no_temp_file() {
        let (_dir, root) = project();
        write(&root, "first").unwrap();
        write(&root, "second").unwrap();
        assert_eq!(fs::read_to_string(sheet_path(&root)).unwrap(), "second");
        let mut names: Vec<_> = fs::read_dir(notes_dir(&root))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, [".gitignore", "notes.md"]);
    }

    #[test]
    fn a_write_is_utf8_with_lf() {
        let (_dir, root) = project();
        let hash = write(&root, "one\r\ntwo\r\nÈ\n").unwrap();
        assert_eq!(
            fs::read(sheet_path(&root)).unwrap(),
            "one\ntwo\nÈ\n".as_bytes()
        );
        assert_eq!(hash, hash_of("one\r\ntwo\r\nÈ\n"));
        assert_eq!(hash, hash_of("one\ntwo\nÈ\n"));
    }

    #[test]
    fn a_crlf_file_reads_as_lf_with_the_hash_of_its_own_bytes() {
        let (_dir, root) = project();
        fs::create_dir_all(notes_dir(&root)).unwrap();
        let bytes = b"\xEF\xBB\xBFone\r\ntwo\r\n";
        fs::write(sheet_path(&root), bytes).unwrap();
        let sheet = read(&root).unwrap().unwrap();
        assert_eq!(sheet.text, "one\ntwo\n");
        assert_eq!(sheet.hash, sha256_bytes(bytes));
    }

    #[test]
    fn a_sheet_that_is_not_utf8_is_an_error_and_is_not_changed_by_reading() {
        let (_dir, root) = project();
        fs::create_dir_all(notes_dir(&root)).unwrap();
        fs::write(sheet_path(&root), [0xFF, 0xFE, 0x00]).unwrap();
        let error = read(&root).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(fs::read(sheet_path(&root)).unwrap(), [0xFF, 0xFE, 0x00]);
    }

    #[test]
    fn the_gitignore_is_written_again_by_the_next_write_when_it_is_missing() {
        let (_dir, root) = project();
        write(&root, "a").unwrap();
        fs::remove_file(notes_dir(&root).join(".gitignore")).unwrap();
        write(&root, "b").unwrap();
        assert_eq!(
            fs::read(notes_dir(&root).join(".gitignore")).unwrap(),
            b"*\n"
        );
    }
}
