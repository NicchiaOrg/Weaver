//! The OS advisory lock of a project (`flock` on Unix, `LockFileEx` on Windows, via `fd-lock`).
//!
//! The app holds it for every mutation; a CLI that writes in-process takes the same lock on
//! `<app local data>/locks/<project hash>.lock`. A lock not obtained within [`LOCK_TIMEOUT`]
//! is the error "project busy". The lock is released when the closure returns or panics,
//! and by the OS if the process dies.

use std::fs::OpenOptions;
use std::io;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use fd_lock::RwLock;

use crate::paths::AppDirs;

/// How long a mutation waits for the project lock.
pub const LOCK_TIMEOUT: Duration = Duration::from_secs(10);

const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    /// The lock was not obtained in time.
    #[error("project busy")]
    Busy,
    #[error("lock file: {0}")]
    Io(#[from] io::Error),
}

/// Runs `f` while holding the lock of the project, waiting up to [`LOCK_TIMEOUT`].
pub fn with_project_lock<T>(
    dirs: &AppDirs,
    project_hash: &str,
    f: impl FnOnce() -> T,
) -> Result<T, LockError> {
    with_lock_file(&dirs.lock_file(project_hash), LOCK_TIMEOUT, f)
}

/// Runs `f` while holding the exclusive lock of `path` (created with its folder when missing),
/// waiting up to `timeout`.
pub fn with_lock_file<T>(
    path: &Path,
    timeout: Duration,
    f: impl FnOnce() -> T,
) -> Result<T, LockError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    let mut lock = RwLock::new(file);
    let deadline = Instant::now() + timeout;
    loop {
        match lock.try_write() {
            Ok(_guard) => return Ok(f()),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(LockError::Busy);
                }
                thread::sleep(POLL_INTERVAL);
            }
            Err(e) => return Err(LockError::Io(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc};

    use super::*;

    #[test]
    fn lock_not_obtained_in_time_is_project_busy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("locks").join("p.lock");
        let (locked_tx, locked_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let holder_path = path.clone();
        let holder = thread::spawn(move || {
            with_lock_file(&holder_path, LOCK_TIMEOUT, || {
                locked_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            })
            .unwrap();
        });
        locked_rx.recv().unwrap();

        let outcome = with_lock_file(&path, Duration::from_millis(100), || {
            unreachable!("must not run")
        });
        match outcome {
            Err(LockError::Busy) => assert_eq!(LockError::Busy.to_string(), "project busy"),
            other => panic!("expected Busy, got {other:?}"),
        }

        release_tx.send(()).unwrap();
        holder.join().unwrap();
        // Once released the lock is available again.
        assert_eq!(
            with_lock_file(&path, Duration::from_millis(100), || 7).unwrap(),
            7
        );
    }

    /// Two writers, standing for the app and a CLI, each opening the lock file on their own:
    /// their read-modify-write sections never overlap.
    #[test]
    fn concurrent_mutations_of_the_same_project_are_serialised() {
        let dir = tempfile::tempdir().unwrap();
        let dirs = AppDirs::from_bases(dir.path(), dir.path(), None);
        let counter_file = dir.path().join("counter");
        std::fs::write(&counter_file, "0").unwrap();
        let inside = Arc::new(AtomicUsize::new(0));
        let overlaps = Arc::new(AtomicUsize::new(0));

        let writers: Vec<_> = (0..2)
            .map(|_| {
                let dirs = dirs.clone();
                let counter_file = counter_file.clone();
                let inside = inside.clone();
                let overlaps = overlaps.clone();
                thread::spawn(move || {
                    for _ in 0..25 {
                        with_project_lock(&dirs, "0123456789abcdef", || {
                            if inside.fetch_add(1, Ordering::SeqCst) != 0 {
                                overlaps.fetch_add(1, Ordering::SeqCst);
                            }
                            let n: u32 = std::fs::read_to_string(&counter_file)
                                .unwrap()
                                .parse()
                                .unwrap();
                            thread::yield_now();
                            std::fs::write(&counter_file, (n + 1).to_string()).unwrap();
                            inside.fetch_sub(1, Ordering::SeqCst);
                        })
                        .unwrap();
                    }
                })
            })
            .collect();
        for writer in writers {
            writer.join().unwrap();
        }
        assert_eq!(overlaps.load(Ordering::SeqCst), 0);
        assert_eq!(std::fs::read_to_string(&counter_file).unwrap(), "50");
    }

    #[test]
    fn different_projects_do_not_block_each_other() {
        let dir = tempfile::tempdir().unwrap();
        let dirs = AppDirs::from_bases(dir.path(), dir.path(), None);
        let outcome = with_project_lock(&dirs, "aaaaaaaaaaaaaaaa", || {
            with_lock_file(
                &dirs.lock_file("bbbbbbbbbbbbbbbb"),
                Duration::from_millis(50),
                || 1,
            )
        });
        assert_eq!(outcome.unwrap().unwrap(), 1);
    }
}
