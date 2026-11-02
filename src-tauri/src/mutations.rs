//! One serial queue of mutations per project.
//!
//! Every mutation of a project, from the UI or forwarded by the CLI, goes through
//! [`MutationQueues::run`]: it waits its turn in the project's queue (first come, first served),
//! then runs on the blocking pool holding the OS advisory lock of the project
//! (`<app local data>/locks/<project hash>.lock`), the same lock a CLI writing in-process takes.
//! Under the lock the closure re-reads its files from disk, validates the result and then
//! writes; a refusal (`Err`) writes nothing. A lock not obtained in time is "project busy".

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use trama_core::lock::{self, LOCK_TIMEOUT};
use trama_core::paths::AppDirs;

use crate::blocking;
use crate::error::AppError;

pub struct MutationQueues {
    dirs: AppDirs,
    lock_timeout: Duration,
    queues: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl MutationQueues {
    pub fn new(dirs: AppDirs) -> Self {
        Self::with_lock_timeout(dirs, LOCK_TIMEOUT)
    }

    /// Same as [`MutationQueues::new`] with another lock timeout (tests).
    pub fn with_lock_timeout(dirs: AppDirs, lock_timeout: Duration) -> Self {
        Self {
            dirs,
            lock_timeout,
            queues: Mutex::new(HashMap::new()),
        }
    }

    fn queue(&self, project_hash: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut queues = self.queues.lock().expect("queue table poisoned");
        queues.entry(project_hash.to_owned()).or_default().clone()
    }

    /// Runs one mutation of the project `project_hash`.
    pub async fn run<T, F>(&self, project_hash: &str, mutation: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, AppError> + Send + 'static,
    {
        let queue = self.queue(project_hash);
        let _turn = queue.lock().await;
        let path = self.dirs.lock_file(project_hash);
        let timeout = self.lock_timeout;
        blocking::disk(move || lock::with_lock_file(&path, timeout, mutation)).await??
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::thread;

    use super::*;
    use crate::error::ErrorCode;

    const HASH: &str = "0123456789abcdef";

    fn dirs(base: &std::path::Path) -> AppDirs {
        AppDirs::from_bases(base, base, None)
    }

    #[tokio::test]
    async fn mutations_of_one_project_run_one_at_a_time() {
        let base = tempfile::tempdir().unwrap();
        let queues = Arc::new(MutationQueues::new(dirs(base.path())));
        let inside = Arc::new(AtomicUsize::new(0));
        let overlaps = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(Mutex::new(Vec::new()));

        let mut tasks = Vec::new();
        for n in 0..8 {
            let (queues, inside, overlaps, order) = (
                queues.clone(),
                inside.clone(),
                overlaps.clone(),
                order.clone(),
            );
            tasks.push(tokio::spawn(async move {
                queues
                    .run(HASH, move || {
                        if inside.fetch_add(1, Ordering::SeqCst) != 0 {
                            overlaps.fetch_add(1, Ordering::SeqCst);
                        }
                        thread::yield_now();
                        order.lock().unwrap().push(n);
                        inside.fetch_sub(1, Ordering::SeqCst);
                        Ok(())
                    })
                    .await
            }));
        }
        for task in tasks {
            task.await.unwrap().unwrap();
        }
        assert_eq!(overlaps.load(Ordering::SeqCst), 0);
        assert_eq!(order.lock().unwrap().len(), 8);
    }

    /// The app queue and a CLI writing in-process share the OS lock, so their sections never overlap.
    #[tokio::test]
    async fn app_and_cli_mutations_of_the_same_project_are_serialised() {
        let base = tempfile::tempdir().unwrap();
        let app_dirs = dirs(base.path());
        let queues = Arc::new(MutationQueues::new(app_dirs.clone()));
        let inside = Arc::new(AtomicUsize::new(0));
        let overlaps = Arc::new(AtomicUsize::new(0));
        let counter = base.path().join("counter");
        std::fs::write(&counter, "0").unwrap();

        let bump = {
            let (inside, overlaps, counter) = (inside.clone(), overlaps.clone(), counter.clone());
            move || {
                if inside.fetch_add(1, Ordering::SeqCst) != 0 {
                    overlaps.fetch_add(1, Ordering::SeqCst);
                }
                let n: u32 = std::fs::read_to_string(&counter).unwrap().parse().unwrap();
                thread::yield_now();
                std::fs::write(&counter, (n + 1).to_string()).unwrap();
                inside.fetch_sub(1, Ordering::SeqCst);
            }
        };

        let cli = {
            let (dirs, bump) = (app_dirs.clone(), bump.clone());
            thread::spawn(move || {
                for _ in 0..20 {
                    lock::with_project_lock(&dirs, HASH, &bump).unwrap();
                }
            })
        };
        for _ in 0..20 {
            let bump = bump.clone();
            queues
                .run(HASH, move || {
                    bump();
                    Ok(())
                })
                .await
                .unwrap();
        }
        cli.join().unwrap();
        assert_eq!(overlaps.load(Ordering::SeqCst), 0);
        assert_eq!(std::fs::read_to_string(&counter).unwrap(), "40");
    }

    #[tokio::test]
    async fn a_lock_held_elsewhere_makes_the_mutation_fail_with_project_busy_and_write_nothing() {
        let base = tempfile::tempdir().unwrap();
        let app_dirs = dirs(base.path());
        let queues =
            MutationQueues::with_lock_timeout(app_dirs.clone(), Duration::from_millis(100));
        let target = base.path().join("file");

        let (locked_tx, locked_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let holder = {
            let dirs = app_dirs.clone();
            thread::spawn(move || {
                lock::with_project_lock(&dirs, HASH, || {
                    locked_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                })
                .unwrap();
            })
        };
        locked_rx.recv().unwrap();

        let written = target.clone();
        let error = queues
            .run(HASH, move || {
                std::fs::write(&written, "changed").unwrap();
                Ok(())
            })
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::ProjectBusy);
        assert_eq!(error.message, "project busy");
        assert!(!target.exists());

        release_tx.send(()).unwrap();
        holder.join().unwrap();
        queues.run(HASH, || Ok(())).await.unwrap();
    }

    #[tokio::test]
    async fn a_refusal_is_returned_as_is() {
        let base = tempfile::tempdir().unwrap();
        let queues = MutationQueues::new(dirs(base.path()));
        let error = queues
            .run(HASH, || {
                Err::<(), _>(AppError::new(ErrorCode::InvalidInput, "refused"))
            })
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }
}
