//! The Machine `file-lock` in the app: which files open in a tab of the File view an agent of the
//! project is changing.
//!
//! [`FileLockService`] keeps one [`FileLocks`] per project, in memory only, and tells the UI when
//! the lock of a file open in a tab is taken or released (`file-lock-changed`). It is fed by the
//! live `session/update` notifications of every job, of General and of every Ask more
//! conversation ([`tools`] reads them), never by replayed ones and never by the transcripts of
//! sub-agents, and it is told when a conversation ends. It keeps the running tools of a project
//! even while no window has the File view open, so that a file opened during a tool opens locked.
//! It reads and writes nothing on disk, apart from asking for the canonical form of the paths a
//! tool names.

pub mod tools;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock, Weak};

use serde_json::Value;
use weaver_core::file_lock::{FileLocks, LockChange};
use weaver_core::file_tab::path;

use crate::file_tabs::events::FileEvents;

/// Who follows a file the agent moved.
pub trait LockObserver: Send + Sync {
    /// The lock of the file open at `path` was released, and omp says the agent moved it to
    /// `moved_to` (both canonical). Called before the UI hears of it.
    fn file_moved(&self, project_id: &str, path: &Path, moved_to: &Path);
}

pub struct FileLockService {
    projects: Mutex<HashMap<String, FileLocks>>,
    events: Arc<FileEvents>,
    observers: RwLock<Vec<Weak<dyn LockObserver>>>,
}

impl FileLockService {
    pub fn new(events: Arc<FileEvents>) -> Arc<Self> {
        Arc::new(Self {
            projects: Mutex::new(HashMap::new()),
            events,
            observers: RwLock::new(Vec::new()),
        })
    }

    pub fn observe(&self, observer: Weak<dyn LockObserver>) {
        self.observers
            .write()
            .expect("observers poisoned")
            .push(observer);
    }

    fn projects(&self) -> std::sync::MutexGuard<'_, HashMap<String, FileLocks>> {
        self.projects.lock().expect("file locks poisoned")
    }

    // -----------------------------------------------------------------------------------------
    // What the agents say
    // -----------------------------------------------------------------------------------------

    /// A live `update` of a `session/update` of `conversation` (see
    /// [`weaver_core::file_lock::job_conversation`] and its siblings) in the project at `root`
    /// (canonical). Updates that are not about a tool that changes files do nothing.
    pub fn tool_update(&self, project_id: &str, root: &Path, conversation: &str, update: &Value) {
        let Some(update) = tools::read_update(update) else {
            return;
        };
        let changes = {
            let mut projects = self.projects();
            let locks = projects
                .entry(project_id.to_owned())
                .or_insert_with(|| FileLocks::new(root.to_path_buf()));
            let changes = tools::apply(locks, conversation, &update);
            if locks.is_idle() {
                projects.remove(project_id);
            }
            changes
        };
        self.report(project_id, root, changes);
    }

    /// `omp.TURN_ENDED`, `omp.TURN_FAILED`, `omp.EXITED`, `job.ENDED`, `developer.CLEAR_GENERAL`
    /// and `developer.CLOSE_ASK`: every tool of the conversation stops counting. Calling it again
    /// for a conversation that ended is harmless.
    pub fn conversation_ended(&self, project_id: &str, conversation: &str) {
        self.end(project_id, |locks| locks.conversation_ended(conversation));
    }

    /// Like [`conversation_ended`](Self::conversation_ended) for General and an Ask more
    /// conversation, whose tools are told apart by the omp that ran them: every omp of `family`.
    pub fn family_ended(&self, project_id: &str, family: &str) {
        self.end(project_id, |locks| locks.conversation_family_ended(family));
    }

    fn end(&self, project_id: &str, end: impl FnOnce(&mut FileLocks) -> Vec<LockChange>) {
        let (root, changes) = {
            let mut projects = self.projects();
            let Some(locks) = projects.get_mut(project_id) else {
                return;
            };
            let root = locks.root().to_path_buf();
            let changes = end(locks);
            if locks.is_idle() {
                projects.remove(project_id);
            }
            (root, changes)
        };
        self.report(project_id, &root, changes);
    }

    // -----------------------------------------------------------------------------------------
    // What the tabs say
    // -----------------------------------------------------------------------------------------

    /// `developer.OPEN_FILE`: the canonical `file` opens in a tab of the project at `root`.
    /// Returns whether an agent is already changing it.
    pub fn file_opened(&self, project_id: &str, root: &Path, file: PathBuf) -> bool {
        self.projects()
            .entry(project_id.to_owned())
            .or_insert_with(|| FileLocks::new(root.to_path_buf()))
            .file_opened(file)
    }

    /// `file-tab.CLOSED`.
    pub fn file_closed(&self, project_id: &str, file: &Path) {
        let mut projects = self.projects();
        if let Some(locks) = projects.get_mut(project_id) {
            locks.file_closed(file);
            if locks.is_idle() {
                projects.remove(project_id);
            }
        }
    }

    /// A tab follows its file to another path (the tree renamed or moved it).
    pub fn file_renamed(&self, project_id: &str, from: &Path, to: PathBuf) {
        if let Some(locks) = self.projects().get_mut(project_id) {
            locks.file_renamed(from, to);
        }
    }

    /// The window of the project closed: its tabs are gone. The tools stay while they run.
    pub fn close_project(&self, project_id: &str) {
        {
            let mut projects = self.projects();
            if let Some(locks) = projects.get_mut(project_id) {
                locks.forget_open_files();
                if locks.is_idle() {
                    projects.remove(project_id);
                }
            }
        }
        self.events.forget(project_id);
    }

    /// Whether the lock of the file open at `file` (canonical) is held.
    pub fn is_locked(&self, project_id: &str, file: &Path) -> bool {
        self.projects()
            .get(project_id)
            .is_some_and(|locks| locks.is_held(file))
    }

    /// Whether a file open in a tab and locked is `rel` or lies inside it: the tree refuses to
    /// rename, move or delete it.
    pub fn is_locked_under(&self, project_id: &str, rel: &str) -> bool {
        let projects = self.projects();
        let Some(locks) = projects.get(project_id) else {
            return false;
        };
        match path::resolve(locks.root(), rel) {
            Ok(canonical) => locks.is_held_under(&canonical),
            Err(_) => false,
        }
    }

    // -----------------------------------------------------------------------------------------
    // Reporting
    // -----------------------------------------------------------------------------------------

    fn report(&self, project_id: &str, root: &Path, changes: Vec<LockChange>) {
        for change in changes {
            match change {
                LockChange::Locked(file) => {
                    if let Some(rel) = path::relative(root, &file) {
                        self.events.lock_changed(project_id, &rel, true, None);
                    }
                }
                LockChange::Released {
                    path: file,
                    moved_to,
                } => {
                    if let Some(to) = &moved_to {
                        self.follow(project_id, &file, to);
                    }
                    let Some(rel) = path::relative(root, &file) else {
                        continue;
                    };
                    let moved = moved_to.as_deref().and_then(|to| path::relative(root, to));
                    self.events
                        .lock_changed(project_id, &rel, false, moved.as_deref());
                }
            }
        }
    }

    /// The agent moved the file: its tab and its lock state follow it before the UI is told.
    fn follow(&self, project_id: &str, from: &Path, to: &Path) {
        self.file_renamed(project_id, from, to.to_path_buf());
        let observers: Vec<Arc<dyn LockObserver>> = self
            .observers
            .read()
            .expect("observers poisoned")
            .iter()
            .filter_map(Weak::upgrade)
            .collect();
        for observer in observers {
            observer.file_moved(project_id, from, to);
        }
    }
}

/// What the actor of one omp feeds the lock with: its project, its root and the name of its
/// conversation, so that the actors only say what omp said and when it ended.
pub struct ConversationTools {
    service: Arc<FileLockService>,
    project_id: String,
    root: PathBuf,
    conversation: String,
}

impl ConversationTools {
    pub fn new(
        service: Arc<FileLockService>,
        project_id: String,
        root: PathBuf,
        conversation: String,
    ) -> Self {
        Self {
            service,
            project_id,
            root,
            conversation,
        }
    }

    /// A live update of the conversation (never a replayed one).
    pub fn update(&self, update: &Value) {
        self.service
            .tool_update(&self.project_id, &self.root, &self.conversation, update);
    }

    /// The turn ended or failed, omp exited or the actor is gone: its tools stop counting.
    pub fn end(&self) {
        self.service
            .conversation_ended(&self.project_id, &self.conversation);
    }
}

#[cfg(test)]
mod tests;
