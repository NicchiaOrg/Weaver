//! The Notes view in the app: the backend of the project's one Markdown sheet and of its images
//! (`requirements/tech#notes-view`).
//!
//! The rules and the disk work are in [`weaver_core::notes`]; this module keeps, per open project,
//! what the app must remember between commands, in memory only:
//!
//! * the hash of the last sheet Weaver wrote or read, so that the watcher does not send its own
//!   write as `notes-changed` ([`NotesService::last_hash`], [`NotesService::remember_hash`]; the
//!   events are in [`changes`]);
//! * the writes of the sheet: at most one runs per window at a time, and the texts that arrive
//!   meanwhile coalesce into one more write of the latest ([`NotesService::write`]). Writing runs
//!   on the blocking pool and never blocks the other commands of the window;
//! * the window that is closing and waits for its Notes view to answer `notes-flush` with the
//!   text still waiting and the images to put in the trash ([`flush_window`]).
//!
//! The commands the UI calls are in [`commands`]. They take image names, never paths; the only
//! absolute paths are the sources of a drop from outside, as Tauri reports them.
//!
//! Logs name projects, files and paths; never the text of the sheet, the bytes of an image or its
//! `data:` URL.

pub mod changes;
pub mod commands;

use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::oneshot;
use ts_rs::TS;
use weaver_core::notes::images::{Outcome, Why};
use weaver_core::notes::sheet;

use crate::events::EventName;
use crate::log_warn;
use crate::requests::FLUSH_LIMIT;
use crate::state::AppState;
use crate::windows::registry::label_of;

/// What `notes_read` answers: the text of the sheet, no byte order mark and LF line endings, with
/// the SHA-256 of the bytes of the file; both are none when the file is missing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NotesSheet {
    pub text: Option<String>,
    pub hash: Option<String>,
}

/// What `notes_write` answers: the hash of the bytes written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NotesWritten {
    pub hash: String,
}

/// One pasted file: its own name when the clipboard gives one, and its bytes in base64.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PastedImage {
    pub name: Option<String>,
    pub data: String,
}

/// Why an image was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum NotesRefusalReason {
    /// Larger than 10 MB.
    TooLarge,
    /// Not a PNG, JPEG, GIF or WebP.
    NotAccepted,
}

/// What became of one file given to a copy, in the order given. `source` is the file's own name,
/// or `pasted image` when it has none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum NotesImageOutcome {
    /// Copied into the notes folder under this name.
    Added { name: String },
    Refused {
        source: String,
        reason: NotesRefusalReason,
    },
    /// The copy failed with this reason.
    Failed { source: String, message: String },
    /// Not an image: nothing happened.
    LeftOut,
}

impl From<Outcome> for NotesImageOutcome {
    fn from(outcome: Outcome) -> Self {
        match outcome {
            Outcome::Added { name } => Self::Added { name },
            Outcome::Refused { source, why } => Self::Refused {
                source,
                reason: match why {
                    Why::TooLarge => NotesRefusalReason::TooLarge,
                    Why::NotAccepted => NotesRefusalReason::NotAccepted,
                },
            },
            Outcome::Failed { source, message } => Self::Failed { source, message },
            Outcome::LeftOut => Self::LeftOut,
        }
    }
}

/// What a copy of images did: one outcome per file given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NotesImagesAdded {
    pub files: Vec<NotesImageOutcome>,
}

/// An image the trash refused, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NotesTrashRefusal {
    pub name: String,
    pub reason: String,
}

/// What moving images to the trash did: the ones that stayed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NotesTrashed {
    pub refused: Vec<NotesTrashRefusal>,
}

/// Payload of `notes-flush`: the window is closing and its Notes view must answer with
/// `notes_flushed` (the text still waiting, the images to put in the trash).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NotesFlush {
    pub project_id: String,
}

/// The result of one write: the hash of the bytes written, or the reason it failed.
pub type WriteResult = Result<String, String>;

/// Writes the sheet of the project at the root with the text; the hash of what it wrote.
pub type Store = dyn Fn(&Path, &str) -> io::Result<String> + Send + Sync;

/// The text waiting for the write that follows the running one, and who waits for it.
struct Queued {
    text: String,
    waiters: Vec<oneshot::Sender<WriteResult>>,
}

/// The writes of one project's sheet.
#[derive(Default)]
struct Writer {
    running: bool,
    queued: Option<Queued>,
}

/// One open project's notes state.
struct Project {
    /// The hash of the last sheet written or read; none when unknown or when it was missing.
    hash: Mutex<Option<String>>,
    writer: Mutex<Writer>,
}

/// Ends the running write of a project when the thread leaves for any reason. Waiters of a write
/// that never ran are dropped, so their commands fail instead of waiting for ever.
struct Release<'a> {
    project: &'a Project,
    armed: bool,
}

impl Drop for Release<'_> {
    fn drop(&mut self) {
        if self.armed {
            let mut writer = self.project.writer.lock().expect("notes writer poisoned");
            writer.running = false;
            writer.queued = None;
        }
    }
}

/// The notes of every open project.
pub struct NotesService {
    projects: Mutex<HashMap<String, Arc<Project>>>,
    /// The windows closing, by project id, waiting for their Notes view to answer.
    flushing: Mutex<HashMap<String, oneshot::Sender<()>>>,
    store: Arc<Store>,
}

impl Default for NotesService {
    fn default() -> Self {
        Self::new()
    }
}

impl NotesService {
    pub fn new() -> Self {
        Self::with_store(Arc::new(|root: &Path, text: &str| sheet::write(root, text)))
    }

    /// A service that writes the sheet through `store`, for the tests.
    pub fn with_store(store: Arc<Store>) -> Self {
        Self {
            projects: Mutex::new(HashMap::new()),
            flushing: Mutex::new(HashMap::new()),
            store,
        }
    }

    fn project(&self, project_id: &str) -> Arc<Project> {
        self.projects
            .lock()
            .expect("notes projects poisoned")
            .entry(project_id.to_owned())
            .or_insert_with(|| {
                Arc::new(Project {
                    hash: Mutex::new(None),
                    writer: Mutex::new(Writer::default()),
                })
            })
            .clone()
    }

    fn existing(&self, project_id: &str) -> Option<Arc<Project>> {
        self.projects
            .lock()
            .expect("notes projects poisoned")
            .get(project_id)
            .cloned()
    }

    /// The hash of the last sheet Weaver wrote or read in the project: none when it is unknown or
    /// the sheet was missing. A change of the sheet with this hash is Weaver's own and is not
    /// sent as `notes-changed`.
    pub fn last_hash(&self, project_id: &str) -> Option<String> {
        self.existing(project_id)
            .and_then(|project| project.hash.lock().expect("notes hash poisoned").clone())
    }

    /// Remembers the hash of the sheet Weaver just read (none: it was missing) or is about to
    /// write.
    pub fn remember_hash(&self, project_id: &str, hash: Option<String>) {
        *self
            .project(project_id)
            .hash
            .lock()
            .expect("notes hash poisoned") = hash;
    }

    /// `window.CLOSED`: nothing is kept for the project. A write still running goes on to its end.
    pub fn close_project(&self, project_id: &str) {
        self.projects
            .lock()
            .expect("notes projects poisoned")
            .remove(project_id);
    }

    /// Writes `text` as the whole sheet of the project at `root`. The answer comes when the text
    /// is written, or, when a write is already running, when the one more write that follows it
    /// is: every text that arrives while one runs is replaced by the latest, which is written
    /// once, and all of their answers are the result of that write.
    ///
    /// The hash of what is written is remembered before the write (the watcher sees the file at
    /// least 150 ms later) and the previous one is put back when the write fails.
    pub fn write(
        &self,
        project_id: &str,
        root: &Path,
        text: String,
    ) -> oneshot::Receiver<WriteResult> {
        let project = self.project(project_id);
        let (answer, receiver) = oneshot::channel();
        {
            let mut writer = project.writer.lock().expect("notes writer poisoned");
            if writer.running {
                match &mut writer.queued {
                    Some(queued) => {
                        queued.text = text;
                        queued.waiters.push(answer);
                    }
                    None => {
                        writer.queued = Some(Queued {
                            text,
                            waiters: vec![answer],
                        });
                    }
                }
                return receiver;
            }
            writer.running = true;
        }
        let store = self.store.clone();
        let root = root.to_path_buf();
        tauri::async_runtime::spawn_blocking(move || {
            run_writes(&project, &root, &*store, text, answer);
        });
        receiver
    }

    /// Moves the images `names` to the trash with `trash`, each on its own.
    pub fn trash(
        &self,
        root: &Path,
        names: &[String],
        trash: &dyn Fn(&Path) -> Result<(), String>,
    ) -> NotesTrashed {
        let done = weaver_core::notes::images::trash(root, names, trash);
        for name in &done.trashed {
            crate::log_info!(
                "the image {name} of the notes of {} went to the trash",
                root.display()
            );
        }
        NotesTrashed {
            refused: done
                .refused
                .into_iter()
                .map(|refused| NotesTrashRefusal {
                    name: refused.name,
                    reason: refused.reason,
                })
                .collect(),
        }
    }

    /// The Notes view of a closing window answered `notes-flush` and its work is done.
    fn finish_flush(&self, project_id: &str) {
        if let Some(waiting) = self
            .flushing
            .lock()
            .expect("notes flushing poisoned")
            .remove(project_id)
        {
            let _ = waiting.send(());
        }
    }
}

/// The write thread of a project: writes `text`, then the latest queued text for as long as
/// there is one, and answers every waiter with the result of its write.
fn run_writes(
    project: &Project,
    root: &Path,
    store: &Store,
    text: String,
    answer: oneshot::Sender<WriteResult>,
) {
    let mut release = Release {
        project,
        armed: true,
    };
    let mut job = Some((text, vec![answer]));
    while let Some((text, waiters)) = job.take() {
        let result = write_now(project, root, store, &text);
        for waiter in waiters {
            let _ = waiter.send(result.clone());
        }
        let mut writer = project.writer.lock().expect("notes writer poisoned");
        job = writer
            .queued
            .take()
            .map(|queued| (queued.text, queued.waiters));
        if job.is_none() {
            writer.running = false;
        }
    }
    release.armed = false;
}

fn write_now(project: &Project, root: &Path, store: &Store, text: &str) -> WriteResult {
    let expected = sheet::hash_of(text);
    let previous = project
        .hash
        .lock()
        .expect("notes hash poisoned")
        .replace(expected);
    match store(root, text) {
        Ok(hash) => {
            *project.hash.lock().expect("notes hash poisoned") = Some(hash.clone());
            Ok(hash)
        }
        Err(error) => {
            *project.hash.lock().expect("notes hash poisoned") = previous;
            log_warn!(
                "the notes of {} could not be written: {error}",
                root.display()
            );
            Err(format!("The notes could not be saved: {error}"))
        }
    }
}

/// Asks the Notes view of a project window for the text still waiting and the images to put in
/// the trash, and waits (at most [`FLUSH_LIMIT`]) until it answered and the app wrote and trashed
/// them. Called before a window closes and when the app quits, after the request editor's flush.
pub async fn flush_window(app: &AppHandle, project_id: &str) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let service = state.notes.clone();
    let (answer, receiver) = oneshot::channel();
    service
        .flushing
        .lock()
        .expect("notes flushing poisoned")
        .insert(project_id.to_owned(), answer);
    let event = NotesFlush {
        project_id: project_id.to_owned(),
    };
    if app
        .emit_to(label_of(project_id), EventName::NotesFlush.as_str(), &event)
        .is_ok()
    {
        let _ = tokio::time::timeout(FLUSH_LIMIT, receiver).await;
    }
    service
        .flushing
        .lock()
        .expect("notes flushing poisoned")
        .remove(project_id);
}

#[cfg(test)]
mod tests;
