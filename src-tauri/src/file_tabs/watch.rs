//! The watch of the files open in tabs: one watcher per project window, started with its first tab
//! and dropped with its last one.
//!
//! A tab's file is watched through its folder, without recursion: editors and git replace files by
//! renaming a new one over them, which a watch on the file's own inode would lose. Only the events
//! that name an open file are reported, in batches (150 ms after the last event); the temp files of
//! atomic writes are ignored, and nothing else of the folder is ever read. The events never go near
//! the project's picture, its batcher or `workspace.FILES_CHANGED`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::time::Instant;

use notify::{EventKind, RecursiveMode, Watcher};
use weaver_core::atomic;

use crate::log_warn;
use crate::pictures::Rebuild;
use crate::windows::watch::Batcher;

/// What a batch names: the paths that changed, or `Rebuild::All` when events may have been lost.
pub type ChangeSink = Arc<dyn Fn(Rebuild) + Send + Sync>;

/// The watch of one project's open files; dropping it stops it.
pub trait TabWatcher: Send {
    /// Starts watching `folder` (the folder of an open file), without recursion.
    fn watch(&mut self, folder: &Path);
    /// Stops watching `folder`.
    fn unwatch(&mut self, folder: &Path);
}

/// Starts the watch of a project's open files.
pub trait TabWatch: Send + Sync {
    fn start(&self, sink: ChangeSink) -> Result<Box<dyn TabWatcher>, String>;
}

/// The real one, over the `notify` crate.
pub struct NotifyTabWatch;

enum Message {
    Event(notify::Result<notify::Event>),
}

struct Notify {
    watcher: notify::RecommendedWatcher,
    /// How many open files each watched folder serves.
    folders: BTreeMap<PathBuf, usize>,
}

impl TabWatcher for Notify {
    fn watch(&mut self, folder: &Path) {
        let count = self.folders.entry(folder.to_path_buf()).or_insert(0);
        *count += 1;
        if *count == 1 {
            if let Err(error) = self.watcher.watch(folder, RecursiveMode::NonRecursive) {
                log_warn!("cannot watch {}: {error}", folder.display());
            }
        }
    }

    fn unwatch(&mut self, folder: &Path) {
        let Some(count) = self.folders.get_mut(folder) else {
            return;
        };
        *count -= 1;
        if *count == 0 {
            self.folders.remove(folder);
            let _ = self.watcher.unwatch(folder);
        }
    }
}

impl TabWatch for NotifyTabWatch {
    fn start(&self, sink: ChangeSink) -> Result<Box<dyn TabWatcher>, String> {
        let (sender, receiver) = mpsc::channel::<Message>();
        let watcher = notify::recommended_watcher(move |event| {
            let _ = sender.send(Message::Event(event));
        })
        .map_err(|error| format!("cannot start the file watcher: {error}"))?;
        std::thread::Builder::new()
            .name("file-tabs-watcher".to_owned())
            .spawn(move || run(receiver, &sink))
            .map_err(|error| format!("cannot start the file watcher: {error}"))?;
        Ok(Box::new(Notify {
            watcher,
            folders: BTreeMap::new(),
        }))
    }
}

/// Batches the events until the watcher is dropped, which closes the channel.
fn run(receiver: mpsc::Receiver<Message>, sink: &ChangeSink) {
    let started = Instant::now();
    let mut batcher = Batcher::default();
    loop {
        let message = match batcher.deadline() {
            Some(deadline) => receiver.recv_timeout(deadline.saturating_sub(started.elapsed())),
            None => receiver.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match message {
            Err(RecvTimeoutError::Disconnected) => return,
            Err(RecvTimeoutError::Timeout) => {}
            Ok(Message::Event(Ok(event))) => {
                if matches!(event.kind, EventKind::Access(_)) {
                    continue;
                }
                let now = started.elapsed();
                for path in event.paths {
                    let is_temp = path
                        .file_name()
                        .is_some_and(|name| atomic::is_temp_name(&name.to_string_lossy()));
                    if !is_temp {
                        batcher.push(now, path);
                    }
                }
            }
            Ok(Message::Event(Err(error))) => {
                // Events may have been lost: every open file is looked at again.
                log_warn!("the watcher of the open files reported an error: {error}");
                batcher.push_everything(started.elapsed());
            }
        }
        if let Some(batch) = batcher.take_due(started.elapsed()) {
            sink(batch);
        }
    }
}
