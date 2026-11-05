//! Changes to project files that happen all together or not at all.
//!
//! A change of cards runs under the project's write lock (held by the caller: the app's
//! mutation queue, or [`apply_locked`] for a command line that writes in-process). Under the
//! lock it reads the project, validates everything against the state *after* the change, and
//! only then writes. A refusal writes nothing: no file, no folder, no cache and no lock
//! leftover. If a write fails half-way, the files already written go back to their previous
//! content and the folders created for them are removed.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::atomic;
use crate::lock::{self, LockError};
use crate::paths::{self, AppDirs};

use super::image::ProjectImage;
use super::layout;
use super::text;

/// What happens to one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// The file gets this content (normalized and written with LF and a final newline).
    Write(String),
    Delete,
}

/// The changes of one operation, by path relative to `.weaver/`; written in path order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeSet {
    changes: BTreeMap<String, Change>,
}

impl ChangeSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the content of `rel` (`/` or `\` separators); a later call for the same path wins.
    pub fn write(&mut self, rel: &str, content: impl Into<String>) {
        self.changes
            .insert(layout::normalize_rel(rel), Change::Write(content.into()));
    }

    /// Deletes the file `rel`; deleting a file that is not there is not an error.
    pub fn delete(&mut self, rel: &str) {
        self.changes
            .insert(layout::normalize_rel(rel), Change::Delete);
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.changes.len()
    }

    pub fn get(&self, rel: &str) -> Option<&Change> {
        self.changes.get(&layout::normalize_rel(rel))
    }

    /// The changes in path order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Change)> {
        self.changes.iter()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MutationError<E> {
    /// The validation refused the change; nothing was written.
    #[error("refused")]
    Refused(E),
    /// A file could not be written; `restore_failures` lists the paths that could not be put
    /// back as they were (normally empty).
    #[error("cannot write {path}: {source}")]
    Write {
        path: String,
        source: io::Error,
        restore_failures: Vec<String>,
    },
    /// A file could not be read before the change (the previous content could not be kept).
    #[error("cannot read {path}: {source}")]
    Read { path: String, source: io::Error },
    #[error(transparent)]
    Lock(LockError),
    #[error("cannot canonicalise the project root: {0}")]
    Root(io::Error),
}

/// Reads the project, lets `plan` build the changes from it, validates the project after them
/// with `validate`, then writes. Must run under the project's write lock.
///
/// Returns the image of the project after the change and what `plan` returned. `plan` and
/// `validate` refuse with `Err`, which writes nothing.
pub fn mutate<T, E>(
    project_root: &Path,
    plan: impl FnOnce(&ProjectImage) -> Result<(ChangeSet, T), E>,
    validate: impl FnOnce(&ProjectImage) -> Result<(), E>,
) -> Result<(ProjectImage, T), MutationError<E>> {
    let before = ProjectImage::load(project_root);
    let (changes, value) = plan(&before).map_err(MutationError::Refused)?;
    let after = before.with_changes(&changes);
    validate(&after).map_err(MutationError::Refused)?;
    write_all(project_root, &changes)?;
    Ok((after, value))
}

/// Same as [`mutate`] for changes known beforehand.
pub fn apply<E>(
    project_root: &Path,
    changes: &ChangeSet,
    validate: impl FnOnce(&ProjectImage) -> Result<(), E>,
) -> Result<ProjectImage, MutationError<E>> {
    mutate(project_root, |_| Ok((changes.clone(), ())), validate).map(|(image, ())| image)
}

/// Same as [`mutate`], holding the project lock itself (waiting up to the lock timeout): for
/// a process that writes on its own, like the command line with the app closed.
pub fn mutate_locked<T, E>(
    dirs: &AppDirs,
    project_root: &Path,
    plan: impl FnOnce(&ProjectImage) -> Result<(ChangeSet, T), E>,
    validate: impl FnOnce(&ProjectImage) -> Result<(), E>,
) -> Result<(ProjectImage, T), MutationError<E>> {
    let hash = paths::project_hash(project_root).map_err(MutationError::Root)?;
    lock::with_project_lock(dirs, &hash, || mutate(project_root, plan, validate))
        .map_err(MutationError::Lock)?
}

/// Same as [`apply`], holding the project lock itself.
pub fn apply_locked<E>(
    dirs: &AppDirs,
    project_root: &Path,
    changes: &ChangeSet,
    validate: impl FnOnce(&ProjectImage) -> Result<(), E>,
) -> Result<ProjectImage, MutationError<E>> {
    mutate_locked(dirs, project_root, |_| Ok((changes.clone(), ())), validate)
        .map(|(image, ())| image)
}

/// What a path held before the change.
struct Previous {
    path: PathBuf,
    rel: String,
    bytes: Option<Vec<u8>>,
}

fn write_all<E>(project_root: &Path, changes: &ChangeSet) -> Result<(), MutationError<E>> {
    // Keep the previous content of every file first: if that fails, nothing has been touched.
    let mut previous: Vec<Previous> = Vec::new();
    for (rel, _) in changes.iter() {
        let path = layout::absolute(project_root, rel);
        let bytes = match fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
            {
                None
            }
            Err(source) => {
                return Err(MutationError::Read {
                    path: rel.clone(),
                    source,
                })
            }
        };
        previous.push(Previous {
            path,
            rel: rel.clone(),
            bytes,
        });
    }

    let mut created_dirs: Vec<PathBuf> = Vec::new();
    for (done, ((rel, change), before)) in changes.iter().zip(&previous).enumerate() {
        let result = match change {
            Change::Write(content) => {
                note_missing_dirs(&before.path, &mut created_dirs);
                text::write_text(&before.path, content)
            }
            Change::Delete => remove_if_present(&before.path),
        };
        if let Err(source) = result {
            let restore_failures = roll_back(&previous[..=done], &created_dirs);
            return Err(MutationError::Write {
                path: rel.clone(),
                source,
                restore_failures,
            });
        }
    }
    Ok(())
}

/// Removes a file; one that is not there (or whose folder is not a folder) counts as removed.
fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            Ok(())
        }
        other => other,
    }
}

/// Records the folders above `path` that do not exist yet, so that they can be removed again.
fn note_missing_dirs(path: &Path, created: &mut Vec<PathBuf>) {
    let mut missing = Vec::new();
    let mut dir = path.parent();
    while let Some(d) = dir {
        if d.exists() {
            break;
        }
        missing.push(d.to_path_buf());
        dir = d.parent();
    }
    for d in missing.into_iter().rev() {
        if !created.contains(&d) {
            created.push(d);
        }
    }
}

/// Puts the files back as they were, newest first, then removes the folders that were
/// created. Returns the paths that could not be restored.
fn roll_back(attempted: &[Previous], created_dirs: &[PathBuf]) -> Vec<String> {
    let mut failures = Vec::new();
    for before in attempted.iter().rev() {
        let result = match &before.bytes {
            Some(bytes) => atomic::write(&before.path, bytes, None),
            None => remove_if_present(&before.path),
        };
        if result.is_err() {
            failures.push(before.rel.clone());
        }
    }
    for dir in created_dirs.iter().rev() {
        let _ = fs::remove_dir(dir);
    }
    failures
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::cards::CardKind;
    use crate::project::testing::TASK;

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let tasks = layout::weaver_dir(dir.path()).join("tasks");
        fs::create_dir_all(&tasks).unwrap();
        fs::write(tasks.join("vxdhoylk-first.md"), TASK).unwrap();
        dir
    }

    fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
        fn walk(dir: &Path, base: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            let Ok(listing) = fs::read_dir(dir) else {
                return;
            };
            for item in listing {
                let path = item.unwrap().path();
                let key = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                if path.is_dir() {
                    out.insert(format!("{key}/"), Vec::new());
                    walk(&path, base, out);
                } else {
                    out.insert(key, fs::read(&path).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    #[test]
    fn a_valid_change_is_written_with_lf_and_a_final_newline() {
        let dir = project();
        let mut changes = ChangeSet::new();
        changes.write(
            "tasks/aaaaaaaa-new.md",
            TASK.replace('\n', "\r\n").trim_end(),
        );
        changes.write(
            "history\\hhhhhhhh-entry.md",
            "---\ncreated: 2026-11-02T07:00:01.000Z\n---\n# H\n",
        );
        changes.delete("tasks/vxdhoylk-first.md");
        let after = apply::<String>(dir.path(), &changes, |_| Ok(())).unwrap();
        assert!(after.card(CardKind::Task, "aaaaaaaa").is_some());
        let written = fs::read(layout::absolute(dir.path(), "tasks/aaaaaaaa-new.md")).unwrap();
        assert_eq!(written, TASK.as_bytes());
        assert!(layout::absolute(dir.path(), "history/hhhhhhhh-entry.md").exists());
        assert!(!layout::absolute(dir.path(), "tasks/vxdhoylk-first.md").exists());
        // What the validation saw is what the next read finds.
        let reread = ProjectImage::load(dir.path());
        assert_eq!(
            reread.file_hash("tasks/aaaaaaaa-new.md"),
            after.file_hash("tasks/aaaaaaaa-new.md")
        );
    }

    #[test]
    fn validation_runs_against_the_state_after_the_change_and_a_refusal_writes_nothing() {
        let dir = project();
        let before = snapshot(dir.path());
        let mut changes = ChangeSet::new();
        changes.write("tasks/aaaaaaaa-new.md", TASK);
        changes.write(
            "history/hhhhhhhh-entry.md",
            "---\ncreated: 2026-11-02T07:00:01.000Z\n---\n# H\n",
        );
        changes.write("brand-new-folder/model-file.md", "x");
        let result = apply(dir.path(), &changes, |after| {
            // Sees the new Task, which is not on disk yet.
            assert!(after.card(CardKind::Task, "aaaaaaaa").is_some());
            assert!(!layout::absolute(dir.path(), "tasks/aaaaaaaa-new.md").exists());
            Err("two Tasks are not allowed".to_owned())
        });
        assert!(matches!(result, Err(MutationError::Refused(m)) if m.contains("not allowed")));
        assert_eq!(snapshot(dir.path()), before);
    }

    #[test]
    fn a_refusal_while_planning_writes_nothing_either() {
        let dir = project();
        let before = snapshot(dir.path());
        let result = mutate::<(), String>(dir.path(), |_| Err("no".to_owned()), |_| Ok(()));
        assert!(matches!(result, Err(MutationError::Refused(_))));
        assert_eq!(snapshot(dir.path()), before);
    }

    #[test]
    fn plan_reads_the_project_and_returns_a_value() {
        let dir = project();
        let (after, count) = mutate::<usize, String>(
            dir.path(),
            |image| {
                let n = image.cards_of(CardKind::Task).count();
                let mut changes = ChangeSet::new();
                changes.write("tasks/aaaaaaaa-new.md", TASK);
                Ok((changes, n))
            },
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(count, 1);
        assert_eq!(after.cards_of(CardKind::Task).count(), 2);
    }

    #[test]
    fn a_multi_file_write_that_fails_is_undone() {
        let dir = project();
        // `tasks/zzzzzzzz-blocker` is a file, so nothing can be written below it; its path
        // sorts last, after every other change.
        let blocker = layout::absolute(dir.path(), "tasks/zzzzzzzz-blocker");
        fs::write(&blocker, b"i am a file").unwrap();
        let before = snapshot(dir.path());

        let mut changes = ChangeSet::new();
        changes.write("tasks/aaaaaaaa-new.md", TASK); // written first (path order)
        changes.write(
            "tasks/vxdhoylk-first.md",
            "---\ncolumn: done\ncreated: 2026-11-02T07:00:01.000Z\n---\n# Changed\n",
        ); // rewritten
        changes.write(
            "history/hhhhhhhh-entry.md",
            "---\ncreated: 2026-11-02T07:00:01.000Z\n---\n# H\n",
        ); // new folder
        changes.write("tasks/zzzzzzzz-blocker/x.md", "never"); // fails
        let result = apply::<()>(dir.path(), &changes, |_| Ok(()));
        match result {
            Err(MutationError::Write {
                path,
                restore_failures,
                ..
            }) => {
                assert_eq!(path, "tasks/zzzzzzzz-blocker/x.md");
                assert!(restore_failures.is_empty());
            }
            other => panic!("expected a write failure, got {:?}", other.map(|_| ())),
        }
        // Every file is back to its previous content, the new ones and their folders are gone.
        assert_eq!(snapshot(dir.path()), before);
        assert!(!layout::absolute(dir.path(), "history").exists());
        assert_eq!(
            fs::read(layout::absolute(dir.path(), "tasks/vxdhoylk-first.md")).unwrap(),
            TASK.as_bytes()
        );
    }

    #[test]
    fn deleting_a_file_that_is_not_there_is_not_an_error_and_a_failed_delete_is_undone() {
        let dir = project();
        let mut changes = ChangeSet::new();
        changes.delete("tasks/gone0000-x.md");
        apply::<()>(dir.path(), &changes, |_| Ok(())).unwrap();

        // A folder where a file is expected cannot be deleted as a file: nothing changes.
        let folder = layout::absolute(dir.path(), "tasks/zzzzzzzz-folder.md");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("inner"), b"x").unwrap();
        let before = snapshot(dir.path());
        let mut changes = ChangeSet::new();
        changes.delete("tasks/vxdhoylk-first.md");
        changes.delete("tasks/zzzzzzzz-folder.md");
        let result = apply::<()>(dir.path(), &changes, |_| Ok(()));
        assert!(matches!(
            result,
            Err(MutationError::Read { .. } | MutationError::Write { .. })
        ));
        assert_eq!(snapshot(dir.path()), before);
    }

    #[test]
    fn the_locked_variant_takes_the_project_lock_and_leaves_none_behind() {
        let dir = project();
        let dirs = AppDirs::from_bases(&dir.path().join("cfg"), &dir.path().join("data"), None);
        let mut changes = ChangeSet::new();
        changes.write("tasks/aaaaaaaa-new.md", TASK);
        apply_locked::<()>(&dirs, dir.path(), &changes, |_| Ok(())).unwrap();
        assert!(layout::absolute(dir.path(), "tasks/aaaaaaaa-new.md").exists());
        // The lock is free again: another locked change goes through.
        let mut again = ChangeSet::new();
        again.delete("tasks/aaaaaaaa-new.md");
        apply_locked::<()>(&dirs, dir.path(), &again, |_| Ok(())).unwrap();
        // A refusal under the lock releases it too.
        let refused = apply_locked(&dirs, dir.path(), &changes, |_| Err("no")).map(|_| ());
        assert!(matches!(refused, Err(MutationError::Refused("no"))));
        apply_locked::<()>(&dirs, dir.path(), &changes, |_| Ok(())).unwrap();
    }

    #[test]
    fn change_sets_normalize_windows_paths() {
        let mut changes = ChangeSet::new();
        changes.write("model\\shop\\ordine così.md", "x");
        assert!(changes.get("model/shop/ordine così.md").is_some());
        assert_eq!(changes.len(), 1);
        changes.delete("model/shop/ordine così.md");
        assert_eq!(
            changes.get("model\\shop\\ordine così.md"),
            Some(&Change::Delete)
        );
    }
}
