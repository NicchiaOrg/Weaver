//! The file watcher of an open project: changes under `.trama/` become batches that feed
//! [`Pictures::rebuild`](crate::pictures::Pictures::rebuild).
//!
//! * [`Batcher`] is the pure rule of `requirements/tech#General`: a batch is handed over 150 ms
//!   after the last event, and at most 1 s after the first one when changes never stop. It takes
//!   the time as an argument, so tests drive it without sleeping.
//! * [`Watch`] starts a watcher for a project; [`NotifyWatch`] is the one over the `notify` crate.
//!   It watches `.trama/` recursively, and the project folder itself without recursion to notice
//!   `.trama/` appearing (Get Started) or going away, which asks for a rebuild of everything.
//!   Paths are reported under the root it was given, which is the canonical root.
//! * The watcher never reads or parses anything: it only names paths. Each project has its own
//!   watcher, batcher and thread; nothing is shared between projects.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{EventKind, RecursiveMode, Watcher};
use trama_core::atomic;
use trama_core::project::layout;

use crate::pictures::Rebuild;
use crate::{log_debug, log_warn};

/// Quiet time after the last event of a batch.
pub const QUIET: Duration = Duration::from_millis(150);

/// Longest wait after the first event of a batch, under continuous changes.
pub const LONGEST: Duration = Duration::from_secs(1);

/// Collects changes into batches.
#[derive(Debug)]
pub struct Batcher {
    quiet: Duration,
    longest: Duration,
    first: Option<Duration>,
    last: Duration,
    everything: bool,
    paths: BTreeSet<PathBuf>,
}

impl Default for Batcher {
    fn default() -> Self {
        Self::new(QUIET, LONGEST)
    }
}

impl Batcher {
    pub fn new(quiet: Duration, longest: Duration) -> Self {
        Self {
            quiet,
            longest,
            first: None,
            last: Duration::ZERO,
            everything: false,
            paths: BTreeSet::new(),
        }
    }

    fn touch(&mut self, now: Duration) {
        self.first.get_or_insert(now);
        self.last = now;
    }

    /// A file or folder changed (`now` is any monotonic time).
    pub fn push(&mut self, now: Duration, path: PathBuf) {
        self.touch(now);
        self.paths.insert(path);
    }

    /// Something changed that names no path (events were lost, `.trama/` appeared): the batch
    /// re-reads everything.
    pub fn push_everything(&mut self, now: Duration) {
        self.touch(now);
        self.everything = true;
    }

    /// When the batch is due, or `None` when it is empty.
    pub fn deadline(&self) -> Option<Duration> {
        let first = self.first?;
        Some((self.last + self.quiet).min(first + self.longest))
    }

    /// The batch when it is due at `now`, which empties it.
    pub fn take_due(&mut self, now: Duration) -> Option<Rebuild> {
        if self.deadline()? > now {
            return None;
        }
        self.first = None;
        let everything = std::mem::take(&mut self.everything);
        let paths = std::mem::take(&mut self.paths);
        Some(if everything {
            Rebuild::All
        } else {
            Rebuild::Paths(paths.into_iter().collect())
        })
    }
}

/// Receives each batch of a project, on the watcher's own thread.
pub type BatchSink = Arc<dyn Fn(Rebuild) + Send + Sync>;

/// A running watcher; dropping it stops it.
pub struct WatchGuard(#[allow(dead_code)] Box<dyn Send>);

impl WatchGuard {
    pub fn new(inner: impl Send + 'static) -> Self {
        Self(Box::new(inner))
    }
}

/// Starts watchers.
pub trait Watch: Send + Sync {
    /// Watches the project at `root` (canonical) and calls `sink` with every batch of changes.
    fn start(&self, root: &Path, sink: BatchSink) -> Result<WatchGuard, String>;
}

enum Message {
    Event(notify::Result<notify::Event>),
    Stop,
}

struct Stopper(mpsc::Sender<Message>);

impl Drop for Stopper {
    fn drop(&mut self) {
        let _ = self.0.send(Message::Stop);
    }
}

/// The real watcher: `notify`'s recommended backend of the platform.
pub struct NotifyWatch;

impl Watch for NotifyWatch {
    fn start(&self, root: &Path, sink: BatchSink) -> Result<WatchGuard, String> {
        let (sender, receiver) = mpsc::channel::<Message>();
        let events = sender.clone();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = events.send(Message::Event(event));
        })
        .map_err(|e| format!("cannot start the file watcher: {e}"))?;

        let trama = layout::trama_dir(root);
        let mut watching_trama = false;
        if trama.is_dir() {
            watcher
                .watch(&trama, RecursiveMode::Recursive)
                .map_err(|e| format!("cannot watch .trama/: {e}"))?;
            watching_trama = true;
        }
        watcher
            .watch(root, RecursiveMode::NonRecursive)
            .map_err(|e| format!("cannot watch the project folder: {e}"))?;

        let root = root.to_path_buf();
        std::thread::Builder::new()
            .name("project-watcher".to_owned())
            .spawn(move || run(watcher, receiver, &root, watching_trama, &sink))
            .map_err(|e| format!("cannot start the file watcher: {e}"))?;
        Ok(WatchGuard::new(Stopper(sender)))
    }
}

fn run(
    mut watcher: notify::RecommendedWatcher,
    receiver: mpsc::Receiver<Message>,
    root: &Path,
    mut watching_trama: bool,
    sink: &BatchSink,
) {
    let trama = layout::trama_dir(root);
    let started = Instant::now();
    let mut batcher = Batcher::default();
    loop {
        let message = match batcher.deadline() {
            Some(deadline) => receiver.recv_timeout(deadline.saturating_sub(started.elapsed())),
            None => receiver.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match message {
            Ok(Message::Stop) | Err(RecvTimeoutError::Disconnected) => return,
            Ok(Message::Event(Ok(event))) => {
                if matches!(event.kind, EventKind::Access(_)) {
                    continue;
                }
                let now = started.elapsed();
                for path in event.paths {
                    if path == trama {
                        // `.trama/` appeared, went away or changed itself.
                        let present = trama.is_dir();
                        if present && !watching_trama {
                            match watcher.watch(&trama, RecursiveMode::Recursive) {
                                Ok(()) => watching_trama = true,
                                Err(error) => {
                                    log_warn!("cannot watch .trama/ of {}: {error}", root.display())
                                }
                            }
                        } else if !present {
                            watching_trama = false;
                        }
                        batcher.push_everything(now);
                    } else if path.starts_with(&trama) {
                        let is_temp = path
                            .file_name()
                            .is_some_and(|name| atomic::is_temp_name(&name.to_string_lossy()));
                        if !is_temp {
                            batcher.push(now, path);
                        }
                    }
                }
            }
            Ok(Message::Event(Err(error))) => {
                // Events may have been lost: read everything again.
                log_warn!(
                    "the file watcher of {} reported an error: {error}",
                    root.display()
                );
                batcher.push_everything(started.elapsed());
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
        if let Some(batch) = batcher.take_due(started.elapsed()) {
            log_debug!("a batch of changes in {} is ready", root.display());
            sink(batch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn a_batch_is_due_150_ms_after_its_last_event() {
        let mut batcher = Batcher::default();
        assert_eq!(batcher.deadline(), None);
        batcher.push(ms(0), "a".into());
        batcher.push(ms(100), "b".into());
        assert_eq!(batcher.deadline(), Some(ms(250)));
        assert_eq!(batcher.take_due(ms(249)), None);
        assert_eq!(
            batcher.take_due(ms(250)),
            Some(Rebuild::Paths(vec!["a".into(), "b".into()]))
        );
        assert_eq!(batcher.deadline(), None, "handing a batch over empties it");
        assert_eq!(batcher.take_due(ms(10_000)), None);
    }

    #[test]
    fn under_continuous_changes_a_batch_is_due_one_second_after_its_first_event() {
        let mut batcher = Batcher::default();
        for t in (0..=1200).step_by(100) {
            batcher.push(ms(t), format!("f{t}").into());
            if t < 900 {
                assert_eq!(batcher.take_due(ms(t)), None, "at {t} ms");
            }
        }
        let due = batcher.deadline().unwrap();
        assert_eq!(due, ms(1000), "never later than 1 s after the first event");
        assert!(
            matches!(batcher.take_due(ms(1000)), Some(Rebuild::Paths(paths)) if paths.len() == 13)
        );
    }

    #[test]
    fn the_same_path_many_times_is_one_path() {
        let mut batcher = Batcher::default();
        for t in 0..5 {
            batcher.push(ms(t), "same".into());
        }
        assert_eq!(
            batcher.take_due(ms(500)),
            Some(Rebuild::Paths(vec!["same".into()]))
        );
    }

    #[test]
    fn a_change_that_names_no_path_rereads_everything() {
        let mut batcher = Batcher::default();
        batcher.push(ms(0), "a".into());
        batcher.push_everything(ms(10));
        assert_eq!(batcher.take_due(ms(200)), Some(Rebuild::All));
        // The next batch starts clean.
        batcher.push(ms(300), "b".into());
        assert_eq!(
            batcher.take_due(ms(500)),
            Some(Rebuild::Paths(vec!["b".into()]))
        );
    }

    fn collect_until(
        receiver: &mpsc::Receiver<Rebuild>,
        done: impl Fn(&[Rebuild]) -> bool,
    ) -> Vec<Rebuild> {
        let mut seen = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(20);
        while !done(&seen) {
            let left = deadline.saturating_duration_since(Instant::now());
            match receiver.recv_timeout(left) {
                Ok(batch) => seen.push(batch),
                Err(_) => panic!("no batch arrived in time; seen so far: {seen:?}"),
            }
        }
        seen
    }

    fn sink_to(sender: mpsc::Sender<Rebuild>) -> BatchSink {
        let sender = std::sync::Mutex::new(sender);
        Arc::new(move |batch| {
            let _ = sender.lock().unwrap().send(batch);
        })
    }

    #[test]
    fn the_watcher_names_the_files_that_changed_under_the_canonical_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = trama_core::paths::canonical_root(dir.path()).unwrap();
        std::fs::create_dir_all(root.join(".trama").join("tasks")).unwrap();
        let (sender, receiver) = mpsc::channel();
        let _guard = NotifyWatch.start(&root, sink_to(sender)).unwrap();

        let file = root.join(".trama").join("tasks").join("abcd1234-a-task.md");
        std::fs::write(&file, "---\n---\n").unwrap();
        // Something outside `.trama/` and a temporary file of an atomic write are not named.
        std::fs::write(root.join("outside.txt"), "x").unwrap();
        std::fs::write(
            root.join(".trama")
                .join("tasks")
                .join(".abcd1234-a-task.md.0123456789ab.tmp"),
            "x",
        )
        .unwrap();

        let seen = collect_until(&receiver, |seen| {
            seen.iter().any(|batch| {
                matches!(batch, Rebuild::Paths(paths) if paths.contains(&file))
                    || *batch == Rebuild::All
            })
        });
        for batch in &seen {
            if let Rebuild::Paths(paths) = batch {
                assert!(
                    paths.iter().all(|p| p.starts_with(root.join(".trama"))),
                    "{paths:?}"
                );
                assert!(
                    paths.iter().all(|p| !p.to_string_lossy().ends_with(".tmp")),
                    "{paths:?}"
                );
            }
        }
    }

    #[test]
    fn a_trama_folder_that_appears_later_is_watched_and_read_whole() {
        let dir = tempfile::tempdir().unwrap();
        let root = trama_core::paths::canonical_root(dir.path()).unwrap();
        let (sender, receiver) = mpsc::channel();
        let _guard = NotifyWatch.start(&root, sink_to(sender)).unwrap();

        std::fs::create_dir_all(root.join(".trama").join("tasks")).unwrap();
        collect_until(&receiver, |seen| seen.contains(&Rebuild::All));

        let file = root.join(".trama").join("tasks").join("abcd1234-late.md");
        std::fs::write(&file, "x").unwrap();
        collect_until(&receiver, |seen| {
            seen.iter()
                .any(|batch| matches!(batch, Rebuild::Paths(paths) if paths.contains(&file)))
        });
    }
}
