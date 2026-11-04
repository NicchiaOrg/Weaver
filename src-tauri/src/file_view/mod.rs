//! The File view's tree in the app: the Machine `file-view` of each project window, in memory only.
//!
//! The rules, the listings and the actions are in [`weaver_core::file_view`]; this module keeps
//! one [`FileView`] per window with its watches ([`watch`]), tells the UI when a listed folder
//! changes (`files-tree-changed`, [`TreeChanged`]) and releases the view 10 s after it stopped
//! being shown. The commands the UI calls are in [`commands`].
//!
//! Nothing of this reaches the project's picture, its watcher or `project-changed`: the code
//! files are read only here, and only while the view is shown or released a short while ago.
//!
//! The tabs and the agent lock are another service ([`TabHooks`] is what the tree asks of it):
//! an action of the tree is refused for a locked file, and a tab follows a renamed or moved entry
//! and closes with a deleted one.

pub mod commands;
pub mod watch;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use serde::Serialize;
use ts_rs::TS;
use weaver_core::file_view::machine::{FileView, State, TreeSnapshot, TreeUpdate};
use weaver_core::file_view::{copy_in, ops, FolderListing, Refusal};
use weaver_core::project::clock::{Clock, SystemClock};

use self::watch::{FolderSink, FolderWatcher};

/// What the tree asks of the tabs and of the agent lock of the same window.
pub trait TabHooks: Send + Sync {
    /// Whether a file open in a tab, locked by the agent, is `rel` or inside it.
    fn is_locked_under(&self, project_id: &str, rel: &str) -> bool;
    /// The tab on `old`, or on a file inside it, follows `new` and keeps its unsaved changes.
    fn rename_under(&self, project_id: &str, old: &str, new: &str);
    /// Every tab on `rel`, or on a file inside it, closes.
    fn close_under(&self, project_id: &str, rel: &str);
}

/// No tabs, no locks.
pub struct NoTabs;

impl TabHooks for NoTabs {
    fn is_locked_under(&self, _: &str, _: &str) -> bool {
        false
    }

    fn rename_under(&self, _: &str, _: &str, _: &str) {}

    fn close_under(&self, _: &str, _: &str) {}
}

/// The tabs and the agent lock of the app, as the tree asks for them.
pub struct TabsAndLocks {
    pub tabs: Arc<crate::file_tabs::FileTabs>,
    pub locks: Arc<crate::file_lock::FileLockService>,
}

impl TabHooks for TabsAndLocks {
    fn is_locked_under(&self, project_id: &str, rel: &str) -> bool {
        self.locks.is_locked_under(project_id, rel)
    }

    fn rename_under(&self, project_id: &str, old: &str, new: &str) {
        self.tabs.rename_under(project_id, old, new);
    }

    fn close_under(&self, project_id: &str, rel: &str) {
        self.tabs.close_under(project_id, rel);
    }
}

/// `files-tree-changed`: listed folders of a window changed on disk, or by an action of the tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TreeChanged {
    /// The project hash of the window.
    pub project_id: String,
    #[serde(flatten)]
    #[ts(flatten)]
    pub update: TreeUpdate,
}

/// What a create, rename or move did: the path of the entry now, and what the tree must change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EntryChange {
    /// Relative to the project root; the entry's new path (the created one, the renamed one).
    pub path: String,
    pub update: TreeUpdate,
}

/// What a copy in did: the paths of the entries it made (relative to the project root, in the
/// tree's order, so the first one is the one to select), and what the tree must change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyInResult {
    pub paths: Vec<String>,
    pub update: TreeUpdate,
}

/// The one copy a window may run: held from the start of a drop's copy to its end, and gone with
/// the window.
struct CopyGuard<'a>(&'a AtomicBool);

impl<'a> CopyGuard<'a> {
    fn start(copying: &'a AtomicBool) -> Result<Self, Refusal> {
        copying
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self(copying))
            .map_err(|_| Refusal::new("Another copy is still running in this window."))
    }
}

impl Drop for CopyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Receives what changed in a listed folder of a window; the app sends it to the UI.
pub type TreeSink = Arc<dyn Fn(TreeChanged) + Send + Sync>;

/// Sends `files-tree-changed` to the window of the project it is about.
pub fn emitter(app: tauri::AppHandle) -> TreeSink {
    use tauri::Emitter;
    Arc::new(move |changed: TreeChanged| {
        let label = crate::windows::registry::label_of(&changed.project_id);
        if let Err(error) = app.emit_to(
            &label,
            crate::events::EventName::FilesTreeChanged.as_str(),
            &changed,
        ) {
            crate::log_warn!("the window {label} was not told of a change in its tree: {error}");
        }
    })
}

/// One window's view.
struct Window {
    view: Mutex<FileView<FolderWatcher>>,
    /// A drop from outside is being copied.
    copying: AtomicBool,
}

/// The File view of every open project window.
pub struct FileViewService {
    clock: Arc<dyn Clock>,
    quiet: Duration,
    hooks: Arc<dyn TabHooks>,
    sink: Arc<OnceLock<TreeSink>>,
    windows: Mutex<HashMap<String, Arc<Window>>>,
}

/// The folder a relative path lies in (the root: empty).
fn parent_of(rel: &str) -> &str {
    weaver_core::file_view::split_rel(rel).0
}

impl FileViewService {
    pub fn new(hooks: Arc<dyn TabHooks>) -> Self {
        Self::with_timing(Arc::new(SystemClock), watch::QUIET, hooks)
    }

    /// With the clock that measures the release and the quiet time of the watches given, so the
    /// tests drive both.
    pub fn with_timing(clock: Arc<dyn Clock>, quiet: Duration, hooks: Arc<dyn TabHooks>) -> Self {
        Self {
            clock,
            quiet,
            hooks,
            sink: Arc::new(OnceLock::new()),
            windows: Mutex::new(HashMap::new()),
        }
    }

    /// Where `files-tree-changed` goes; only the first sink counts. The service exists before the
    /// app handle does, and nothing is watched before a window shows the view.
    pub fn attach(&self, sink: TreeSink) {
        let _ = self.sink.set(sink);
    }

    fn window(&self, project_id: &str, root: &Path) -> Arc<Window> {
        let mut windows = self.windows.lock().expect("file view windows poisoned");
        if let Some(window) = windows.get(project_id) {
            return window.clone();
        }
        let window = Arc::new_cyclic(|weak: &std::sync::Weak<Window>| {
            let weak = weak.clone();
            let sink = self.sink.clone();
            let project = project_id.to_owned();
            let on_folders: FolderSink = Arc::new(move |folders: Vec<String>| {
                let Some(window) = weak.upgrade() else {
                    return;
                };
                let mut view = window.view.lock().expect("file view poisoned");
                // The folders are read again in Open and in Releasing; the UI hears of it only
                // while it shows the view.
                let mut merged = TreeUpdate {
                    generation: view.generation(),
                    folders: Vec::new(),
                    removed: Vec::new(),
                };
                for folder in folders {
                    let update = view.folder_changed(&folder);
                    merged.generation = update.generation.max(merged.generation);
                    merged.folders.extend(update.folders);
                    merged.removed.extend(update.removed);
                }
                if view.state() == State::Open && !merged.is_empty() {
                    if let Some(sink) = sink.get() {
                        sink(TreeChanged {
                            project_id: project.clone(),
                            update: merged,
                        });
                    }
                }
            });
            Window {
                view: Mutex::new(FileView::new(
                    root.to_path_buf(),
                    self.clock.clone(),
                    FolderWatcher::new(self.quiet, on_folders),
                )),
                copying: AtomicBool::new(false),
            }
        });
        windows.insert(project_id.to_owned(), window.clone());
        window
    }

    fn existing(&self, project_id: &str) -> Option<Arc<Window>> {
        self.windows
            .lock()
            .expect("file view windows poisoned")
            .get(project_id)
            .cloned()
    }

    /// The window's view, which must be Open for an action of the tree.
    fn open_window(&self, project_id: &str) -> Result<Arc<Window>, Refusal> {
        self.existing(project_id)
            .filter(|window| window.view.lock().expect("file view poisoned").state() == State::Open)
            .ok_or_else(|| Refusal::new("The File view is not open."))
    }

    /// The state of the window's view; Closed for a window that never showed it.
    pub fn state(&self, project_id: &str) -> State {
        self.existing(project_id).map_or(State::Closed, |window| {
            window.view.lock().expect("file view poisoned").state()
        })
    }

    // --- the events of the Machine ----------------------------------------------------------

    /// `developer.OPEN_FILE_VIEW` for the project at `root` (canonical).
    pub fn open(&self, project_id: &str, root: &Path) -> Result<TreeSnapshot, Refusal> {
        let window = self.window(project_id, root);
        let snapshot = window.view.lock().expect("file view poisoned").open();
        snapshot
    }

    /// `developer.EXPAND_FOLDER`.
    pub fn expand(&self, project_id: &str, rel: &str) -> Result<TreeUpdate, Refusal> {
        let window = self.open_window(project_id)?;
        let mut view = window.view.lock().expect("file view poisoned");
        let folder = view.expand(rel)?;
        Ok(TreeUpdate {
            generation: view.generation(),
            folders: vec![folder],
            removed: Vec::new(),
        })
    }

    /// `developer.COLLAPSE_FOLDER`.
    pub fn collapse(&self, project_id: &str, rel: &str) -> Result<(), Refusal> {
        let window = self.open_window(project_id)?;
        let result = window
            .view
            .lock()
            .expect("file view poisoned")
            .collapse(rel);
        result
    }

    /// The folders of `rel` for the tree of `Move to…`: listed once, not held and not watched.
    pub fn folders(
        &self,
        project_id: &str,
        rel: &str,
    ) -> Result<Vec<weaver_core::file_view::FileEntry>, Refusal> {
        let window = self.open_window(project_id)?;
        let root = window
            .view
            .lock()
            .expect("file view poisoned")
            .root()
            .to_path_buf();
        weaver_core::file_view::listing::list_folders(&root, rel)
    }

    /// `developer.LEAVE_FILE_VIEW`, once the tabs closed: Open → Releasing. The time left until
    /// the release is due, for the caller to wait; `None` when the view was not Open.
    pub fn leave(&self, project_id: &str) -> Option<Duration> {
        let window = self.existing(project_id)?;
        let mut view = window.view.lock().expect("file view poisoned");
        if view.state() != State::Open {
            return None;
        }
        view.leave();
        view.release_in()
    }

    /// `time.FILE_VIEW_RELEASE_DUE`: releases the view when its delay passed and it was not shown
    /// again. `true` when it released.
    pub fn release_due(&self, project_id: &str) -> bool {
        self.existing(project_id).is_some_and(|window| {
            window
                .view
                .lock()
                .expect("file view poisoned")
                .release_due()
        })
    }

    /// One step of the release timer, `time.FILE_VIEW_RELEASE_DUE`: releases the view when its
    /// delay passed; otherwise the time left, for the timer to wait again. `None` when the view
    /// was released or is not waiting to be (it was shown again, or the window closed).
    pub fn release_step(&self, project_id: &str) -> Option<Duration> {
        let window = self.existing(project_id)?;
        let mut view = window.view.lock().expect("file view poisoned");
        if view.release_due() {
            return None;
        }
        view.release_in()
    }

    /// `window.CLOSED`: every watch and listing of the window is dropped at once.
    pub fn close_project(&self, project_id: &str) {
        let window = self
            .windows
            .lock()
            .expect("file view windows poisoned")
            .remove(project_id);
        if let Some(window) = window {
            window.view.lock().expect("file view poisoned").close();
        }
    }

    // --- the actions of the tree ------------------------------------------------------------

    fn change(
        &self,
        window: &Window,
        old: Option<&str>,
        new: Option<&str>,
        touched: &[&str],
    ) -> TreeUpdate {
        window
            .view
            .lock()
            .expect("file view poisoned")
            .after_change(old, new, touched)
    }

    fn root_of(window: &Window) -> PathBuf {
        window
            .view
            .lock()
            .expect("file view poisoned")
            .root()
            .to_path_buf()
    }

    /// `developer.CREATE_ENTRY`: an empty file or a folder named `name` in the folder `parent`.
    pub fn create(
        &self,
        project_id: &str,
        parent: &str,
        name: &str,
        folder: bool,
    ) -> Result<EntryChange, Refusal> {
        let window = self.open_window(project_id)?;
        let root = Self::root_of(&window);
        let path = ops::create(&root, parent, name, folder)?;
        let update = self.change(&window, None, None, &[parent_of(&path)]);
        Ok(EntryChange { path, update })
    }

    /// `developer.RENAME_ENTRY`.
    pub fn rename(&self, project_id: &str, rel: &str, name: &str) -> Result<EntryChange, Refusal> {
        let window = self.open_window(project_id)?;
        let root = Self::root_of(&window);
        let locked = |path: &str| self.hooks.is_locked_under(project_id, path);
        let path = ops::rename(&root, rel, name, &locked)?;
        let old = rel.trim_matches('/');
        if path != old {
            self.hooks.rename_under(project_id, old, &path);
        }
        let update = self.change(&window, Some(old), Some(&path), &[parent_of(old)]);
        Ok(EntryChange { path, update })
    }

    /// `developer.MOVE_ENTRY`: into the folder `target` (empty: the root).
    pub fn move_into(
        &self,
        project_id: &str,
        rel: &str,
        target: &str,
    ) -> Result<EntryChange, Refusal> {
        let window = self.open_window(project_id)?;
        let root = Self::root_of(&window);
        let locked = |path: &str| self.hooks.is_locked_under(project_id, path);
        let path = ops::move_into(&root, rel, target, &locked)?;
        let old = rel.trim_matches('/');
        self.hooks.rename_under(project_id, old, &path);
        let update = self.change(
            &window,
            Some(old),
            Some(&path),
            &[parent_of(old), parent_of(&path)],
        );
        Ok(EntryChange { path, update })
    }

    /// `developer.DROP_EXTERNAL_ENTRIES`: copies the dropped `sources` (absolute paths, as Tauri
    /// reports them) into the folder `target` (empty: the root). One copy at a time per window;
    /// the copy runs to its end even when the view is left or the window closes.
    pub fn copy_in(
        &self,
        project_id: &str,
        target: &str,
        sources: &[PathBuf],
    ) -> Result<CopyInResult, Refusal> {
        self.copy_in_with(project_id, target, sources, &|_| Ok(()))
    }

    /// [`Self::copy_in`] with `step` told each destination before it is made (see
    /// [`copy_in::copy_in_with`]), for the tests.
    pub fn copy_in_with(
        &self,
        project_id: &str,
        target: &str,
        sources: &[PathBuf],
        step: &dyn Fn(&Path) -> io::Result<()>,
    ) -> Result<CopyInResult, Refusal> {
        let window = self.open_window(project_id)?;
        let _running = CopyGuard::start(&window.copying)?;
        let root = Self::root_of(&window);
        let paths = copy_in::copy_in_with(&root, target, sources, step)
            .map_err(|error| Refusal::new(error.to_string()))?;
        let touched: Vec<&str> = paths
            .first()
            .map(|path| parent_of(path))
            .into_iter()
            .collect();
        let update = self.change(&window, None, None, &touched);
        Ok(CopyInResult { paths, update })
    }

    /// `developer.DELETE_ENTRY`: to the system's trash, through `trash`.
    pub fn delete_with(
        &self,
        project_id: &str,
        rel: &str,
        trash: &dyn Fn(&Path) -> Result<(), String>,
    ) -> Result<TreeUpdate, Refusal> {
        let window = self.open_window(project_id)?;
        let root = Self::root_of(&window);
        let locked = |path: &str| self.hooks.is_locked_under(project_id, path);
        ops::delete(&root, rel, &locked, trash)?;
        let old = rel.trim_matches('/');
        self.hooks.close_under(project_id, old);
        Ok(self.change(&window, Some(old), None, &[parent_of(old)]))
    }

    /// [`Self::delete_with`] the system's trash.
    pub fn delete(&self, project_id: &str, rel: &str) -> Result<TreeUpdate, Refusal> {
        self.delete_with(project_id, rel, &ops::system_trash)
    }
}

/// The listing of one folder as the UI event carries it (used by the tests of the commands).
pub fn listing_of(update: &TreeUpdate, path: &str) -> Option<FolderListing> {
    update
        .folders
        .iter()
        .find(|folder| folder.path == path)
        .cloned()
}

#[cfg(test)]
mod tests;
