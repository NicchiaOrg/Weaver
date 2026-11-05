//! The watches of the File view: one `notify` watcher per project window, separate from the
//! project's own (`requirements/tech#file-view`, **Watching**).
//!
//! It holds one non-recursive watch per listed folder. What happens in a watched folder is
//! grouped by folder, and a group is handed over 150 ms after its last event ([`QUIET`]; the tests
//! give a shorter time), when the names the folder holds are not the ones it held before: a file
//! written, its metadata changed or a read are no news, whatever the system reports of them, and
//! neither are the temp files of atomic writes. Nothing here goes near the project picture, its
//! batcher or `workspace.FILES_CHANGED`.
//!
//! On Linux, when inotify has no watches left (`ENOSPC`), the watches move to the poll watcher,
//! as the project's do.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::event::ModifyKind;
use notify::{Config, EventKind, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};
use weaver_core::atomic;
use weaver_core::file_view::machine::Watches;

use crate::log_warn;

/// Quiet time after the last event of a folder before it is read again.
pub const QUIET: Duration = Duration::from_millis(150);

/// How often the poll watcher looks, after inotify ran out of watches.
const POLL_EVERY: Duration = Duration::from_secs(2);

/// Called, on the watcher's thread, with the folders (relative paths) to read again.
pub type FolderSink = Arc<dyn Fn(Vec<String>) + Send + Sync>;

enum Message {
    Event(notify::Result<notify::Event>),
    Stop,
}

/// A watched folder: its relative path, and the names it held when it was last looked at (`None`
/// until the first look, or when it could not be read: the next group is then always news).
struct Watched {
    rel: String,
    names: Option<BTreeSet<OsString>>,
}

/// The names in `folder`, leaving out the temp files of atomic writes; `None` when it cannot be read.
fn names_of(folder: &Path) -> Option<BTreeSet<OsString>> {
    let read = fs::read_dir(folder).ok()?;
    Some(
        read.filter_map(Result::ok)
            .map(|item| item.file_name())
            .filter(|name| !atomic::is_temp_name(&name.to_string_lossy()))
            .collect(),
    )
}

type WatchedFolders = Arc<Mutex<BTreeMap<PathBuf, Watched>>>;

/// A running watcher and the thread that groups its events; dropping it stops both.
struct Running {
    watcher: Box<dyn Watcher + Send>,
    stop: mpsc::Sender<Message>,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.stop.send(Message::Stop);
    }
}

/// The watches of one window's File view. Nothing runs until the first folder is watched, and
/// nothing is left once every folder is unwatched with [`Watches::unwatch_all`].
pub struct FolderWatcher {
    quiet: Duration,
    sink: FolderSink,
    /// The folders watched, by canonical path.
    folders: WatchedFolders,
    running: Option<Running>,
    polling: bool,
}

impl FolderWatcher {
    pub fn new(quiet: Duration, sink: FolderSink) -> Self {
        Self {
            quiet,
            sink,
            folders: Arc::new(Mutex::new(BTreeMap::new())),
            running: None,
            polling: false,
        }
    }

    fn build(&self, polling: bool) -> Result<Running, String> {
        let (sender, receiver) = mpsc::channel::<Message>();
        let events = sender.clone();
        let handler = move |event| {
            let _ = events.send(Message::Event(event));
        };
        let watcher: Box<dyn Watcher + Send> = if polling {
            let config = Config::default().with_poll_interval(POLL_EVERY);
            Box::new(PollWatcher::new(handler, config).map_err(|e| e.to_string())?)
        } else {
            Box::new(
                RecommendedWatcher::new(handler, Config::default()).map_err(|e| e.to_string())?,
            )
        };
        let folders = self.folders.clone();
        let sink = self.sink.clone();
        let quiet = self.quiet;
        std::thread::Builder::new()
            .name("file-view-watcher".to_owned())
            .spawn(move || run(&receiver, &folders, quiet, &sink))
            .map_err(|e| format!("cannot start the file watcher: {e}"))?;
        Ok(Running {
            watcher,
            stop: sender,
        })
    }

    /// Moves every watch to the poll watcher, after inotify ran out of watches.
    fn switch_to_polling(&mut self) -> Result<(), String> {
        log_warn!(
            "inotify has no watches left (see fs.inotify.max_user_watches): the File view's folders are polled every {} s",
            POLL_EVERY.as_secs()
        );
        let mut polling = self.build(true)?;
        for folder in self.folders.lock().expect("folders poisoned").keys() {
            polling
                .watcher
                .watch(folder, RecursiveMode::NonRecursive)
                .map_err(|e| e.to_string())?;
        }
        self.polling = true;
        self.running = Some(polling);
        Ok(())
    }
}

impl Watches for FolderWatcher {
    fn watch(&mut self, rel: &str, folder: &Path) -> Result<(), String> {
        if self.running.is_none() {
            self.running = Some(self.build(self.polling)?);
        }
        self.folders.lock().expect("folders poisoned").insert(
            folder.to_path_buf(),
            Watched {
                rel: rel.to_owned(),
                names: None,
            },
        );
        let running = self.running.as_mut().expect("started above");
        match running.watcher.watch(folder, RecursiveMode::NonRecursive) {
            Ok(()) => {
                // Looked at once the watch is on, so that nothing that happens next is missed.
                if let Some(watched) = self
                    .folders
                    .lock()
                    .expect("folders poisoned")
                    .get_mut(folder)
                {
                    watched.names = names_of(folder);
                }
                Ok(())
            }
            Err(error)
                if cfg!(target_os = "linux")
                    && !self.polling
                    && matches!(error.kind, notify::ErrorKind::MaxFilesWatch) =>
            {
                self.switch_to_polling().inspect_err(|_| {
                    self.folders
                        .lock()
                        .expect("folders poisoned")
                        .remove(folder);
                })
            }
            Err(error) => {
                self.folders
                    .lock()
                    .expect("folders poisoned")
                    .remove(folder);
                Err(error.to_string())
            }
        }
    }

    fn unwatch(&mut self, rel: &str) {
        let folder = {
            let mut folders = self.folders.lock().expect("folders poisoned");
            let found = folders
                .iter()
                .find(|(_, watched)| watched.rel == rel)
                .map(|(path, _)| path.clone());
            if let Some(path) = &found {
                folders.remove(path);
            }
            found
        };
        if let (Some(folder), Some(running)) = (folder, self.running.as_mut()) {
            let _ = running.watcher.unwatch(&folder);
            // On Windows the watch holds a handle on its folder that only the watcher's own
            // thread closes, after `unwatch` returned, and a folder with a handle open cannot be
            // renamed, moved or deleted. `configure` is answered by that thread once it worked
            // through every action sent before it, the unwatch included.
            #[cfg(windows)]
            let _ = running.watcher.configure(Config::default());
        }
    }

    fn unwatch_all(&mut self) {
        self.folders.lock().expect("folders poisoned").clear();
        self.running = None;
    }
}

/// Whether an event can change what a listing shows: an entry appeared, went, or was renamed.
fn changes_names(kind: &EventKind) -> bool {
    !matches!(
        kind,
        EventKind::Access(_) | EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Metadata(_))
    )
}

/// The thread that groups events by folder and hands each group over after `quiet`.
fn run(
    receiver: &mpsc::Receiver<Message>,
    folders: &Mutex<BTreeMap<PathBuf, Watched>>,
    quiet: Duration,
    sink: &FolderSink,
) {
    let mut pending: BTreeSet<PathBuf> = BTreeSet::new();
    let mut everything = false;
    let mut last: Option<Instant> = None;
    loop {
        let message = match last {
            Some(at) => {
                receiver.recv_timeout((at + quiet).saturating_duration_since(Instant::now()))
            }
            None => receiver.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match message {
            Ok(Message::Stop) | Err(RecvTimeoutError::Disconnected) => return,
            Ok(Message::Event(Ok(event))) => {
                if !changes_names(&event.kind) {
                    continue;
                }
                let folders = folders.lock().expect("folders poisoned");
                for path in &event.paths {
                    let is_temp = path
                        .file_name()
                        .is_some_and(|name| atomic::is_temp_name(&name.to_string_lossy()));
                    if is_temp {
                        continue;
                    }
                    if let Some(parent) =
                        path.parent().filter(|parent| folders.contains_key(*parent))
                    {
                        pending.insert(parent.to_path_buf());
                        last = Some(Instant::now());
                    }
                }
            }
            Ok(Message::Event(Err(error))) => {
                // Events may have been lost: every watched folder is read again.
                log_warn!("the File view's watcher reported an error: {error}");
                everything = true;
                last = Some(Instant::now());
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
        if last.is_some_and(|at| at.elapsed() >= quiet) {
            last = None;
            let mut due: Vec<String> = Vec::new();
            {
                let mut folders = folders.lock().expect("folders poisoned");
                let looked: Vec<PathBuf> = if std::mem::take(&mut everything) {
                    folders.keys().cloned().collect()
                } else {
                    std::mem::take(&mut pending).into_iter().collect()
                };
                pending.clear();
                for folder in looked {
                    let Some(watched) = folders.get_mut(&folder) else {
                        continue;
                    };
                    let names = names_of(&folder);
                    if names.is_none() || names != watched.names {
                        watched.names = names;
                        due.push(watched.rel.clone());
                    }
                }
            }
            due.sort();
            if !due.is_empty() {
                sink(due);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    const TEST_QUIET: Duration = Duration::from_millis(40);

    struct Fixture {
        _dir: tempfile::TempDir,
        root: PathBuf,
        received: mpsc::Receiver<Vec<String>>,
        watcher: FolderWatcher,
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let root = weaver_core::paths::canonical_root(dir.path()).unwrap();
        fs::create_dir_all(root.join("sub")).unwrap();
        let (sender, received) = mpsc::channel();
        let sender = Mutex::new(sender);
        let sink: FolderSink = Arc::new(move |folders| {
            let _ = sender.lock().unwrap().send(folders);
        });
        let mut watcher = FolderWatcher::new(TEST_QUIET, sink);
        watcher.watch("", &root).unwrap();
        // What the system still reports about the fixture is not what the tests look for.
        std::thread::sleep(Duration::from_millis(400));
        while received.try_recv().is_ok() {}
        Fixture {
            _dir: dir,
            root,
            received,
            watcher,
        }
    }

    fn next(f: &Fixture) -> Vec<String> {
        f.received
            .recv_timeout(Duration::from_secs(20))
            .expect("a group of folders arrives")
    }

    fn nothing_for(f: &Fixture, wait: Duration) -> bool {
        f.received.recv_timeout(wait).is_err()
    }

    #[test]
    fn a_new_entry_in_a_watched_folder_names_that_folder_after_the_quiet_time() {
        let f = fixture();
        fs::write(f.root.join("new.txt"), "x").unwrap();
        assert_eq!(next(&f), [""]);
    }

    #[test]
    fn many_events_of_one_folder_come_as_one_group() {
        let f = fixture();
        for n in 0..5 {
            fs::write(f.root.join(format!("f{n}.txt")), "x").unwrap();
        }
        assert_eq!(next(&f), [""]);
        assert!(nothing_for(&f, Duration::from_millis(500)));
    }

    #[test]
    fn a_folder_is_reported_only_while_it_is_watched() {
        let mut f = fixture();
        let sub = f.root.join("sub");
        fs::write(sub.join("early.txt"), "x").unwrap();
        assert!(
            nothing_for(&f, Duration::from_millis(500)),
            "sub is not watched yet"
        );

        f.watcher.watch("sub", &sub).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        while f.received.try_recv().is_ok() {}
        fs::write(sub.join("late.txt"), "x").unwrap();
        assert_eq!(next(&f), ["sub"]);

        f.watcher.unwatch("sub");
        std::thread::sleep(Duration::from_millis(300));
        while f.received.try_recv().is_ok() {}
        fs::write(sub.join("later.txt"), "x").unwrap();
        assert!(
            nothing_for(&f, Duration::from_millis(600)),
            "sub is not watched any more"
        );
    }

    #[test]
    fn a_write_to_an_existing_file_and_the_temp_file_of_an_atomic_write_are_not_news() {
        let f = fixture();
        fs::write(f.root.join("a.txt"), "one").unwrap();
        assert_eq!(next(&f), [""]);
        // The system may report the rewrite of a file it just reported as created as created again.
        fs::write(f.root.join("a.txt"), "two").unwrap();
        fs::write(f.root.join(".a.txt.0123456789ab.tmp"), "x").unwrap();
        fs::remove_file(f.root.join(".a.txt.0123456789ab.tmp")).unwrap();
        assert!(
            nothing_for(&f, Duration::from_millis(900)),
            "the names are the ones the folder held"
        );
        // A name that appears is news again.
        fs::write(f.root.join("b.txt"), "x").unwrap();
        assert_eq!(next(&f), [""]);
    }

    #[test]
    fn only_the_events_that_can_change_a_listing_count() {
        use notify::event::{
            AccessKind, CreateKind, DataChange, MetadataKind, RemoveKind, RenameMode,
        };
        assert!(changes_names(&EventKind::Create(CreateKind::File)));
        assert!(changes_names(&EventKind::Remove(RemoveKind::Folder)));
        assert!(changes_names(&EventKind::Modify(ModifyKind::Name(
            RenameMode::Both
        ))));
        assert!(changes_names(&EventKind::Any));
        assert!(!changes_names(&EventKind::Modify(ModifyKind::Data(
            DataChange::Content
        ))));
        assert!(!changes_names(&EventKind::Modify(ModifyKind::Metadata(
            MetadataKind::Any
        ))));
        assert!(!changes_names(&EventKind::Access(AccessKind::Any)));
    }

    #[test]
    fn nothing_is_reported_once_every_watch_is_dropped() {
        let mut f = fixture();
        f.watcher.unwatch_all();
        fs::write(f.root.join("x.txt"), "x").unwrap();
        assert!(nothing_for(&f, Duration::from_millis(600)));
        assert!(f.watcher.running.is_none());
        assert!(f.watcher.folders.lock().unwrap().is_empty());
    }

    #[test]
    fn a_watcher_that_was_dropped_stops_its_thread() {
        let f = fixture();
        let Fixture {
            received,
            watcher,
            root,
            _dir,
            ..
        } = f;
        drop(watcher);
        fs::write(root.join("y.txt"), "x").unwrap();
        assert!(received.recv_timeout(Duration::from_millis(600)).is_err());
    }
}
