//! The files open in the tabs of the File view (Machine `file-tab`), on the backend side: what each
//! tab read, its watch, its saving, and what the agent lock does to it.
//!
//! [`FileTabs`] keeps, per project window and in memory only, the tabs the UI has: the path, the
//! SHA-256 of the raw bytes read, the byte order mark, the line ending chosen for saving and why
//! the text is read-only. The text of a tab and its undo history live in the UI, which renders what
//! the backend sends. A file is read only when it opens in a tab, after an event of its own watch,
//! or when the developer reloads it; closing the tab frees the watch at once. A save writes only
//! when the file on disk still has the hash the tab read ([`weaver_core::file_tab::save`]).
//!
//! Nothing here reads a folder, builds an index or reaches the project's picture: the events of the
//! tabs' watch never become `workspace.FILES_CHANGED`.

pub mod commands;
pub mod events;
pub mod watch;
pub mod wire;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use serde::Deserialize;
use weaver_core::file_tab::limit::{disk_event, make_room, DiskEvent, Room, TabInfo};
use weaver_core::file_tab::path;
use weaver_core::file_tab::read::{self, sha256_bytes, FileRead, LineEnding, ReadOnly};
pub use weaver_core::file_tab::save::SaveOutcome;
use weaver_core::file_tab::save::{self, PathLocks};

use self::events::FileEvents;
use self::watch::{ChangeSink, TabWatch, TabWatcher};
use self::wire::{
    FileContent, FileKind, FileOpenResult, FileReloadResult, ReadOnlyReason, TabChange,
};
use crate::file_lock::{FileLockService, LockObserver};
use crate::log_warn;
use crate::pictures::Rebuild;

/// How a save compares with the file on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SaveMode {
    /// `developer.SAVE_FILE`: writes only when the file is still the one the tab read.
    Check,
    /// `developer.OVERWRITE_FILE` and `Keep mine`: writes whatever the file holds now.
    Overwrite,
    /// `Save again` from Deleted: writes only when nothing exists at the path.
    Again,
}

struct Tab {
    /// The canonical file.
    canonical: PathBuf,
    /// The SHA-256 of the raw bytes the tab read, or wrote last.
    hash: String,
    bom: bool,
    line_ending: LineEnding,
    binary: bool,
    read_only: Option<ReadOnly>,
    /// The file is gone and the UI knows it.
    deleted: bool,
    /// The hash of the file as it was last reported changed, so that one change is told once.
    reported: Option<String>,
}

impl Tab {
    fn new(canonical: PathBuf, read: &FileRead) -> Self {
        let mut tab = Self {
            canonical,
            hash: String::new(),
            bom: false,
            line_ending: LineEnding::Lf,
            binary: false,
            read_only: None,
            deleted: false,
            reported: None,
        };
        tab.adopt(read);
        tab
    }

    /// The tab read the file again.
    fn adopt(&mut self, read: &FileRead) {
        self.hash.clone_from(&read.hash);
        self.bom = read.bom;
        self.line_ending = read.line_ending;
        self.binary = read.is_binary();
        self.read_only = read.read_only;
        self.deleted = false;
        self.reported = None;
    }
}

struct Project {
    /// The project id (the project hash).
    id: String,
    root: PathBuf,
    /// The tabs by path relative to the root (canonical form of the file, with `/`).
    tabs: BTreeMap<String, Tab>,
    watcher: Option<Box<dyn TabWatcher>>,
}

pub struct FileTabs {
    projects: Mutex<HashMap<String, Project>>,
    locks: Arc<FileLockService>,
    events: Arc<FileEvents>,
    /// One lock per file: a save and the reading of a watch event never interleave.
    paths: PathLocks,
    watch: Arc<dyn TabWatch>,
    me: Weak<FileTabs>,
}

impl FileTabs {
    pub fn new(
        locks: Arc<FileLockService>,
        events: Arc<FileEvents>,
        watch: Arc<dyn TabWatch>,
    ) -> Arc<Self> {
        let tabs = Arc::new_cyclic(|me| FileTabs {
            projects: Mutex::new(HashMap::new()),
            locks: locks.clone(),
            events,
            paths: PathLocks::default(),
            watch,
            me: me.clone(),
        });
        let weak: Weak<FileTabs> = Arc::downgrade(&tabs);
        let observer: Weak<dyn LockObserver> = weak;
        locks.observe(observer);
        tabs
    }

    fn projects(&self) -> MutexGuard<'_, HashMap<String, Project>> {
        self.projects.lock().expect("file tabs poisoned")
    }

    // -----------------------------------------------------------------------------------------
    // Opening, reading again, closing
    // -----------------------------------------------------------------------------------------

    /// `developer.OPEN_FILE`: reads `rel` (relative to `root`, the canonical project root) and
    /// gives it a tab. `shown` are the tabs the UI has, the one shown least recently first: the
    /// limit of 10 closes the first without unsaved changes, or refuses when every one has some.
    pub fn open(
        &self,
        project_id: &str,
        root: &Path,
        rel: &str,
        shown: &[TabInfo],
    ) -> Result<FileOpenResult, String> {
        let canonical = path::resolve(root, rel).map_err(|error| error.to_string())?;
        let key = path::relative(root, &canonical)
            .filter(|key| !key.is_empty())
            .ok_or_else(|| "This path is not a file inside the project.".to_owned())?;
        let closing = {
            let projects = self.projects();
            let project = projects.get(project_id);
            if let Some(existing) = project.and_then(|p| existing_tab(p, &canonical)) {
                return Ok(FileOpenResult::AlreadyOpen { path: existing });
            }
            let known: Vec<TabInfo> = shown
                .iter()
                .filter(|tab| project.is_some_and(|p| p.tabs.contains_key(&tab.path)))
                .cloned()
                .collect();
            match make_room(&known) {
                Room::Fits => None,
                Room::Close(tab) => Some(tab),
                Room::Refused => return Ok(FileOpenResult::Full),
            }
        };

        let read = self.read_file(&canonical, is_inside_weaver(&key))?;
        let content = content_of(&read);

        let mut closed = None;
        {
            let mut projects = self.projects();
            let project = projects
                .entry(project_id.to_owned())
                .or_insert_with(|| Project {
                    id: project_id.to_owned(),
                    root: root.to_path_buf(),
                    tabs: BTreeMap::new(),
                    watcher: None,
                });
            if let Some(existing) = existing_tab(project, &canonical) {
                return Ok(FileOpenResult::AlreadyOpen { path: existing });
            }
            if let Some(tab) = closing {
                if self.remove_tab(project_id, project, &tab) {
                    closed = Some(tab);
                }
            }
            project
                .tabs
                .insert(key.clone(), Tab::new(canonical.clone(), &read));
            self.watch_folder_of(project, &canonical);
        }
        let locked = self.locks.file_opened(project_id, root, canonical);
        Ok(FileOpenResult::Opened {
            path: key,
            content,
            locked,
            closed,
        })
    }

    /// Reads the file again, for `developer.RELOAD_FILE`, for a tab that reads the file again after
    /// a change on disk, and after the lock of its file was released.
    pub fn reload(&self, project_id: &str, rel: &str) -> Result<FileReloadResult, String> {
        let canonical = self.tab_file(project_id, rel)?;
        let lock = self.paths.of(&canonical);
        let _held = save::hold(&lock);
        let inside = is_inside_weaver(rel);
        let read = match read::read(&canonical, inside) {
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if let Some(tab) = self
                    .projects()
                    .get_mut(project_id)
                    .and_then(|p| p.tabs.get_mut(rel))
                {
                    tab.deleted = true;
                }
                return Ok(FileReloadResult::Gone);
            }
            Err(error) => return Err(read_failure(&error)),
        };
        if let Some(tab) = self
            .projects()
            .get_mut(project_id)
            .and_then(|project| project.tabs.get_mut(rel))
        {
            tab.adopt(&read);
        }
        Ok(FileReloadResult::Read {
            content: content_of(&read),
        })
    }

    /// `file-tab.CLOSED`: frees the tab, its lock tracking and its watch at once.
    pub fn close(&self, project_id: &str, rel: &str) {
        let mut projects = self.projects();
        if let Some(project) = projects.get_mut(project_id) {
            self.remove_tab(project_id, project, rel);
            if project.tabs.is_empty() {
                projects.remove(project_id);
            }
        }
    }

    /// The window closed: every tab and every watch of the project goes.
    pub fn close_project(&self, project_id: &str) {
        let project = self.projects().remove(project_id);
        if let Some(mut project) = project {
            let paths: Vec<String> = project.tabs.keys().cloned().collect();
            for rel in paths {
                self.remove_tab(project_id, &mut project, &rel);
            }
        }
        self.locks.close_project(project_id);
    }

    /// Closes the tabs on `rel` or inside it (the tree deleted it).
    pub fn close_under(&self, project_id: &str, rel: &str) {
        let mut projects = self.projects();
        let Some(project) = projects.get_mut(project_id) else {
            return;
        };
        let under: Vec<String> = project
            .tabs
            .keys()
            .filter(|key| is_under(key, rel))
            .cloned()
            .collect();
        for key in under {
            self.remove_tab(project_id, project, &key);
        }
        if project.tabs.is_empty() {
            projects.remove(project_id);
        }
    }

    /// A tab on `old` or inside it follows the entry to `new` (the tree renamed or moved it); the
    /// rename already happened on disk.
    pub fn rename_under(&self, project_id: &str, old: &str, new: &str) {
        let mut projects = self.projects();
        let Some(project) = projects.get_mut(project_id) else {
            return;
        };
        let moving: Vec<String> = project
            .tabs
            .keys()
            .filter(|key| is_under(key, old))
            .cloned()
            .collect();
        for key in moving {
            let new_key = format!("{new}{}", &key[old.len()..]);
            let Some(mut tab) = project.tabs.remove(&key) else {
                continue;
            };
            let new_file = path::resolve(&project.root, &new_key)
                .unwrap_or_else(|_| project.root.join(&new_key));
            self.unwatch_folder_of(project, &tab.canonical);
            self.locks
                .file_renamed(project_id, &tab.canonical, new_file.clone());
            tab.canonical = new_file;
            self.watch_folder_of(project, &tab.canonical);
            project.tabs.insert(new_key, tab);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Saving
    // -----------------------------------------------------------------------------------------

    /// `developer.SAVE_FILE`, `developer.OVERWRITE_FILE` and `Save again`: saves `text` as the tab
    /// at `rel` says, checking against the hash the tab holds. Blocking.
    pub fn save(&self, project_id: &str, rel: &str, text: &str, mode: SaveMode) -> SaveOutcome {
        self.save_read(project_id, rel, text, mode, None)
    }

    /// As [`Self::save`], checking against `read`, the hash the tab read, when the command carries it.
    pub fn save_read(
        &self,
        project_id: &str,
        rel: &str,
        text: &str,
        mode: SaveMode,
        read: Option<&str>,
    ) -> SaveOutcome {
        let (canonical, hash, line_ending, bom, writable) = {
            let projects = self.projects();
            let Some(tab) = projects.get(project_id).and_then(|p| p.tabs.get(rel)) else {
                return SaveOutcome::Failed("This file is not open in a tab.".to_owned());
            };
            (
                tab.canonical.clone(),
                tab.hash.clone(),
                tab.line_ending,
                tab.bom,
                tab.read_only.is_none() && !tab.binary,
            )
        };
        if !writable {
            return SaveOutcome::Failed("This file is read-only here.".to_owned());
        }
        let lock = self.paths.of(&canonical);
        let _held = save::hold(&lock);
        let compare = match mode {
            SaveMode::Check => save::SaveMode::Check(read.unwrap_or(&hash)),
            SaveMode::Overwrite => save::SaveMode::Overwrite,
            SaveMode::Again => save::SaveMode::Again,
        };
        let outcome = save::save(&canonical, text, line_ending, bom, compare);
        if let Some(tab) = self
            .projects()
            .get_mut(project_id)
            .and_then(|project| project.tabs.get_mut(rel))
        {
            match &outcome {
                // The tab keeps the hash it wrote: the watch event of this write is its own.
                SaveOutcome::Saved { hash } => {
                    tab.hash.clone_from(hash);
                    tab.deleted = false;
                    tab.reported = None;
                }
                SaveOutcome::Gone => tab.deleted = true,
                SaveOutcome::Changed | SaveOutcome::Failed(_) => {}
            }
        }
        outcome
    }

    // -----------------------------------------------------------------------------------------
    // The watch
    // -----------------------------------------------------------------------------------------

    /// A batch of the watch of a project's open files: the tabs it names look at their file again.
    fn disk_changed(&self, project_id: &str, batch: &Rebuild) {
        let tabs: Vec<(String, PathBuf)> = {
            let projects = self.projects();
            let Some(project) = projects.get(project_id) else {
                return;
            };
            project
                .tabs
                .iter()
                .filter(|(_, tab)| match batch {
                    Rebuild::All => true,
                    Rebuild::Paths(paths) => paths
                        .iter()
                        .any(|path| path::same_file(path, &tab.canonical)),
                })
                .map(|(key, tab)| (key.clone(), tab.canonical.clone()))
                .collect()
        };
        for (rel, canonical) in tabs {
            self.check_tab(project_id, &rel, &canonical);
        }
    }

    /// Reads the file of a tab after an event of its watch and tells the UI what it means.
    fn check_tab(&self, project_id: &str, rel: &str, canonical: &Path) {
        let lock = self.paths.of(canonical);
        let _held = save::hold(&lock);
        let on_disk = match fs::read(canonical) {
            Ok(bytes) => Some(sha256_bytes(&bytes)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            // Unreadable for now: the next event looks again.
            Err(_) => return,
        };
        let change = {
            let mut projects = self.projects();
            let Some(tab) = projects
                .get_mut(project_id)
                .and_then(|project| project.tabs.get_mut(rel))
                .filter(|tab| tab.canonical == canonical)
            else {
                return;
            };
            match disk_event(on_disk.as_deref(), &tab.hash, tab.deleted) {
                None => {
                    tab.reported = None;
                    return;
                }
                Some(DiskEvent::Deleted) => {
                    tab.deleted = true;
                    tab.reported = None;
                    TabChange::Deleted
                }
                Some(DiskEvent::Changed) => {
                    if !tab.deleted && tab.reported == on_disk {
                        return;
                    }
                    tab.deleted = false;
                    tab.reported = on_disk;
                    TabChange::Changed
                }
            }
        };
        self.events.tab_changed(project_id, rel, change);
    }

    fn sink(&self, project_id: &str) -> ChangeSink {
        let (me, project_id) = (self.me.clone(), project_id.to_owned());
        Arc::new(move |batch| {
            if let Some(tabs) = me.upgrade() {
                tabs.disk_changed(&project_id, &batch);
            }
        })
    }

    /// Watches the folder of `file`, starting the project's watcher with its first tab.
    fn watch_folder_of(&self, project: &mut Project, file: &Path) {
        let Some(folder) = file.parent() else { return };
        if project.watcher.is_none() {
            match self.watch.start(self.sink(&project.id)) {
                Ok(watcher) => project.watcher = Some(watcher),
                Err(error) => {
                    log_warn!(
                        "the files open in tabs of {} are not watched: {error}",
                        project.root.display()
                    );
                    return;
                }
            }
        }
        if let Some(watcher) = project.watcher.as_mut() {
            watcher.watch(folder);
        }
    }

    fn unwatch_folder_of(&self, project: &mut Project, file: &Path) {
        if let (Some(folder), Some(watcher)) = (file.parent(), project.watcher.as_mut()) {
            watcher.unwatch(folder);
        }
    }

    /// Takes the tab at `rel` out of the project: its watch, its lock tracking and its state go.
    /// Returns whether there was one.
    fn remove_tab(&self, project_id: &str, project: &mut Project, rel: &str) -> bool {
        let Some(tab) = project.tabs.remove(rel) else {
            return false;
        };
        self.unwatch_folder_of(project, &tab.canonical);
        if project.tabs.is_empty() {
            project.watcher = None;
        }
        self.locks.file_closed(project_id, &tab.canonical);
        true
    }

    fn tab_file(&self, project_id: &str, rel: &str) -> Result<PathBuf, String> {
        self.projects()
            .get(project_id)
            .and_then(|project| project.tabs.get(rel))
            .map(|tab| tab.canonical.clone())
            .ok_or_else(|| "This file is not open in a tab.".to_owned())
    }

    fn read_file(&self, canonical: &Path, inside_weaver: bool) -> Result<FileRead, String> {
        let lock = self.paths.of(canonical);
        let _held = save::hold(&lock);
        match fs::metadata(canonical) {
            Ok(meta) if !meta.is_file() => {
                return Err("This is a folder, not a file.".to_owned());
            }
            Ok(_) => {}
            Err(error) => return Err(read_failure(&error)),
        }
        read::read(canonical, inside_weaver).map_err(|error| read_failure(&error))
    }
}

impl LockObserver for FileTabs {
    /// The agent moved the file: the tab takes the new path and its watch moves with it. Its text
    /// is read again by the UI at the new path.
    fn file_moved(&self, project_id: &str, from: &Path, to: &Path) {
        let mut projects = self.projects();
        let Some(project) = projects.get_mut(project_id) else {
            return;
        };
        let Some(key) = project
            .tabs
            .iter()
            .find(|(_, tab)| path::same_file(&tab.canonical, from))
            .map(|(key, _)| key.clone())
        else {
            return;
        };
        let Some(mut tab) = project.tabs.remove(&key) else {
            return;
        };
        self.unwatch_folder_of(project, &tab.canonical);
        tab.canonical = to.to_path_buf();
        tab.deleted = false;
        tab.reported = None;
        self.watch_folder_of(project, to);
        let new_key = path::relative(&project.root, to).unwrap_or(key);
        project.tabs.insert(new_key, tab);
    }
}

/// The path of the tab that already holds `canonical`, if any.
fn existing_tab(project: &Project, canonical: &Path) -> Option<String> {
    project
        .tabs
        .iter()
        .find(|(_, tab)| path::same_file(&tab.canonical, canonical))
        .map(|(key, _)| key.clone())
}

/// Whether `key` is `dir` or lies inside it.
fn is_under(key: &str, dir: &str) -> bool {
    key == dir
        || key
            .strip_prefix(dir)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Whether a path relative to the root lies inside `.weaver/`.
fn is_inside_weaver(rel: &str) -> bool {
    rel.split('/')
        .next()
        .is_some_and(|first| first.eq_ignore_ascii_case(".weaver"))
}

fn content_of(read: &FileRead) -> FileContent {
    FileContent {
        kind: if read.is_binary() {
            FileKind::Binary
        } else {
            FileKind::Text
        },
        text: read.text.clone(),
        hash: read.hash.clone(),
        read_only: read.read_only.map(|reason| match reason {
            ReadOnly::TooLarge => ReadOnlyReason::TooLarge,
            ReadOnly::Weaver => ReadOnlyReason::Weaver,
        }),
    }
}

/// Why a file could not be read, for the developer.
fn read_failure(error: &io::Error) -> String {
    match error.kind() {
        io::ErrorKind::NotFound => "The file does not exist.".to_owned(),
        io::ErrorKind::PermissionDenied => "Weaver is not allowed to read this file.".to_owned(),
        _ => format!("Could not read the file: {error}"),
    }
}
