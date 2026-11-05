//! Saving the text of a tab, with the check against what the tab read.
//!
//! The caller takes the per-file lock ([`PathLocks`]) around [`save`], so that two saves of one
//! file never interleave. The write is the atomic one of [`crate::atomic`], which gives the
//! temp file the permission bits of the file it replaces. [`save`] takes the canonical path, with
//! symbolic links resolved (see [`super::path::resolve`]): a save through a link writes the
//! target, never replaces the link.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use super::read::{sha256_bytes, to_disk_bytes, LineEnding};
use crate::atomic;

/// What the save compares with, before it writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveMode<'a> {
    /// `developer.SAVE_FILE`: writes only when the file on disk still has this SHA-256 (of its
    /// raw bytes), the one the tab read.
    Check(&'a str),
    /// `developer.OVERWRITE_FILE` and `Keep mine`: writes whatever the file holds now.
    Overwrite,
    /// `Save again` from Deleted: writes only when nothing exists at the path.
    Again,
}

/// How a save ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    /// Written; the SHA-256 of the bytes written, which the tab keeps.
    Saved { hash: String },
    /// The file on disk is not the one the tab read (or, for [`SaveMode::Again`], something exists
    /// at the path): nothing was written.
    Changed,
    /// The file no longer exists ([`SaveMode::Check`]): nothing was written.
    Gone,
    /// The write failed: nothing changed on disk. The reason.
    Failed(String),
}

/// Saves `text` over the file at `path` (canonical) as `mode` says, with the line ending and
/// byte order mark of the tab.
pub fn save(
    path: &Path,
    text: &str,
    line_ending: LineEnding,
    bom: bool,
    mode: SaveMode<'_>,
) -> SaveOutcome {
    match mode {
        SaveMode::Check(expected) => match fs::read(path) {
            Ok(bytes) => {
                if sha256_bytes(&bytes) != expected {
                    return SaveOutcome::Changed;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => return SaveOutcome::Gone,
            Err(error) => return SaveOutcome::Failed(reason(&error)),
        },
        SaveMode::Again => {
            if fs::symlink_metadata(path).is_ok() {
                return SaveOutcome::Changed;
            }
        }
        SaveMode::Overwrite => {}
    }
    let bytes = to_disk_bytes(text, line_ending, bom);
    match atomic::write_keeping_permissions(path, &bytes) {
        Ok(()) => SaveOutcome::Saved {
            hash: sha256_bytes(&bytes),
        },
        Err(error) => SaveOutcome::Failed(reason(&error)),
    }
}

/// The reason of a failed write, for the developer: the message of the operating system.
fn reason(error: &io::Error) -> String {
    error.to_string()
}

/// One lock per file path (canonical), so that the read-compare-write of a save is not
/// interleaved with another save of the same file. The locks of files nobody holds are dropped
/// as they are taken.
#[derive(Debug, Default)]
pub struct PathLocks {
    held: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
}

impl PathLocks {
    /// The lock of `path`.
    pub fn of(&self, path: &Path) -> Arc<Mutex<()>> {
        let mut held = self.held.lock().expect("path locks poisoned");
        held.retain(|_, lock| Arc::strong_count(lock) > 1);
        held.entry(path.to_path_buf()).or_default().clone()
    }
}

/// Takes `lock`, ignoring a poisoned one: the data it guards is `()`.
pub fn hold(lock: &Mutex<()>) -> MutexGuard<'_, ()> {
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(dir: &Path, bytes: &[u8]) -> (PathBuf, String) {
        let path = dir.join("f.txt");
        fs::write(&path, bytes).unwrap();
        (path, sha256_bytes(bytes))
    }

    fn saved(outcome: SaveOutcome) -> String {
        match outcome {
            SaveOutcome::Saved { hash } => hash,
            other => panic!("not saved: {other:?}"),
        }
    }

    #[test]
    fn a_save_with_lf_writes_the_text_and_returns_the_hash_written() {
        let dir = tempfile::tempdir().unwrap();
        let (path, hash) = file(dir.path(), b"one\ntwo\n");
        let written = saved(save(
            &path,
            "one\nTWO\n",
            LineEnding::Lf,
            false,
            SaveMode::Check(&hash),
        ));
        assert_eq!(fs::read(&path).unwrap(), b"one\nTWO\n");
        assert_eq!(written, sha256_bytes(b"one\nTWO\n"));
    }

    #[test]
    fn a_crlf_file_is_written_back_with_crlf() {
        let dir = tempfile::tempdir().unwrap();
        let (path, hash) = file(dir.path(), b"one\r\ntwo\r\n");
        saved(save(
            &path,
            "one\ntwo\nthree\n",
            LineEnding::CrLf,
            false,
            SaveMode::Check(&hash),
        ));
        assert_eq!(fs::read(&path).unwrap(), b"one\r\ntwo\r\nthree\r\n");
    }

    #[test]
    fn a_file_with_mixed_endings_is_written_with_the_one_used_most() {
        let dir = tempfile::tempdir().unwrap();
        let original = "a\r\nb\r\nc\nd\r\n";
        let (path, hash) = file(dir.path(), original.as_bytes());
        let read = super::super::read::from_bytes(original.as_bytes(), false);
        saved(save(
            &path,
            read.text.as_deref().unwrap(),
            read.line_ending,
            read.bom,
            SaveMode::Check(&hash),
        ));
        assert_eq!(fs::read(&path).unwrap(), b"a\r\nb\r\nc\r\nd\r\n");
    }

    #[test]
    fn a_byte_order_mark_is_written_back_and_never_added() {
        let dir = tempfile::tempdir().unwrap();
        let (path, hash) = file(dir.path(), b"\xEF\xBB\xBFhi\n");
        let hash = saved(save(
            &path,
            "hi!\n",
            LineEnding::Lf,
            true,
            SaveMode::Check(&hash),
        ));
        assert_eq!(fs::read(&path).unwrap(), b"\xEF\xBB\xBFhi!\n");
        saved(save(
            &path,
            "x\n",
            LineEnding::Lf,
            false,
            SaveMode::Check(&hash),
        ));
        assert_eq!(
            fs::read(&path).unwrap(),
            b"x\n",
            "none is added when the file had none"
        );
    }

    #[test]
    fn the_final_line_break_stays_or_stays_out_as_the_text_has_it() {
        let dir = tempfile::tempdir().unwrap();
        let (path, hash) = file(dir.path(), b"a\nb");
        let hash = saved(save(
            &path,
            "a\nb",
            LineEnding::Lf,
            false,
            SaveMode::Check(&hash),
        ));
        assert_eq!(fs::read(&path).unwrap(), b"a\nb");
        saved(save(
            &path,
            "a\nb\n",
            LineEnding::Lf,
            false,
            SaveMode::Check(&hash),
        ));
        assert_eq!(fs::read(&path).unwrap(), b"a\nb\n");
    }

    #[test]
    fn a_save_when_the_file_changed_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (path, hash) = file(dir.path(), b"mine\n");
        fs::write(&path, b"theirs\n").unwrap();
        assert_eq!(
            save(
                &path,
                "mine!\n",
                LineEnding::Lf,
                false,
                SaveMode::Check(&hash)
            ),
            SaveOutcome::Changed
        );
        assert_eq!(fs::read(&path).unwrap(), b"theirs\n");
    }

    #[test]
    fn a_save_when_the_file_is_gone_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (path, hash) = file(dir.path(), b"x\n");
        fs::remove_file(&path).unwrap();
        assert_eq!(
            save(&path, "y\n", LineEnding::Lf, false, SaveMode::Check(&hash)),
            SaveOutcome::Gone
        );
        assert!(!path.exists());
    }

    #[test]
    fn overwrite_skips_the_comparison() {
        let dir = tempfile::tempdir().unwrap();
        let (path, _) = file(dir.path(), b"mine\n");
        fs::write(&path, b"theirs\n").unwrap();
        saved(save(
            &path,
            "mine!\n",
            LineEnding::Lf,
            false,
            SaveMode::Overwrite,
        ));
        assert_eq!(fs::read(&path).unwrap(), b"mine!\n");
        fs::remove_file(&path).unwrap();
        saved(save(
            &path,
            "again\n",
            LineEnding::Lf,
            false,
            SaveMode::Overwrite,
        ));
        assert_eq!(fs::read(&path).unwrap(), b"again\n");
    }

    #[test]
    fn save_again_writes_only_when_nothing_exists_at_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let (path, _) = file(dir.path(), b"x\n");
        fs::remove_file(&path).unwrap();
        saved(save(
            &path,
            "back\n",
            LineEnding::Lf,
            false,
            SaveMode::Again,
        ));
        assert_eq!(fs::read(&path).unwrap(), b"back\n");
        assert_eq!(
            save(&path, "other\n", LineEnding::Lf, false, SaveMode::Again),
            SaveOutcome::Changed
        );
        assert_eq!(fs::read(&path).unwrap(), b"back\n");
    }

    #[test]
    fn a_failed_write_changes_nothing_and_gives_the_reason() {
        let dir = tempfile::tempdir().unwrap();
        // A folder with a file inside cannot be replaced by a file.
        let folder = dir.path().join("folder");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("inner"), b"x").unwrap();
        let outcome = save(
            &folder,
            "text\n",
            LineEnding::Lf,
            false,
            SaveMode::Overwrite,
        );
        let SaveOutcome::Failed(reason) = outcome else {
            panic!("{outcome:?}")
        };
        assert!(!reason.is_empty());
        assert_eq!(fs::read(folder.join("inner")).unwrap(), b"x");
        let leftovers = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| atomic::is_temp_name(&e.file_name().to_string_lossy()))
            .count();
        assert_eq!(leftovers, 0, "no temp file stays");
    }

    #[cfg(unix)]
    #[test]
    fn the_permission_bits_of_the_file_stay() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let (path, hash) = file(dir.path(), b"#!/bin/sh\n");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        saved(save(
            &path,
            "#!/bin/sh\necho\n",
            LineEnding::Lf,
            false,
            SaveMode::Check(&hash),
        ));
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_save_through_a_symbolic_link_writes_its_target_and_keeps_the_link() {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let target = root.join("real.txt");
        fs::write(&target, b"old\n").unwrap();
        std::os::unix::fs::symlink(&target, root.join("link.txt")).unwrap();
        let resolved = super::super::path::resolve(&root, "link.txt").unwrap();
        assert_eq!(resolved, target);
        saved(save(
            &resolved,
            "new\n",
            LineEnding::Lf,
            false,
            SaveMode::Check(&sha256_bytes(b"old\n")),
        ));
        assert_eq!(fs::read(&target).unwrap(), b"new\n");
        assert!(fs::symlink_metadata(root.join("link.txt"))
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn the_lock_of_a_path_is_shared_and_dropped_when_nobody_holds_it() {
        let locks = PathLocks::default();
        let a = locks.of(Path::new("/a"));
        let again = locks.of(Path::new("/a"));
        assert!(Arc::ptr_eq(&a, &again));
        drop((a, again));
        let _other = locks.of(Path::new("/b"));
        assert_eq!(locks.held.lock().unwrap().len(), 1);
    }
}
