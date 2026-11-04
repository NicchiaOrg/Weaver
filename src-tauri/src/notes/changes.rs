//! What the project watcher tells the Notes view (`workspace.NOTES_CHANGED`, the event
//! `notes-changed`).
//!
//! There is no watcher of its own: the project's watcher already sees `.weaver/notes/`. Its
//! batches (handed over 150 ms after the last event) go through [`split_batch`], which takes the
//! paths under `.weaver/notes/` out of the batch: they never reach the project's picture, its
//! rebuild, `project-changed` or `workspace.FILES_CHANGED`, whatever their names look like.
//!
//! [`NotesChanges::announce`] turns what was taken out into one [`NotesChanged`] per batch: the
//! project, a generation, whether the sheet changed with its new SHA-256 (none when the file is
//! missing) and the names of the images that changed. A sheet whose hash is the one Weaver last
//! wrote or read is a write of Weaver's own and is not sent.

use std::collections::{BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use ts_rs::TS;
use weaver_core::notes::{self, SHEET_FILE};

use crate::log_warn;
use crate::pictures::Rebuild;

/// `notes-changed`: the notes of a project changed on disk by anything other than Weaver's own
/// writing (`workspace.NOTES_CHANGED`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NotesChanged {
    /// The project hash of the window.
    pub project_id: String,
    /// Counts the events sent for the project since it opened.
    #[ts(type = "number")]
    pub generation: u64,
    /// Whether the sheet `notes.md` changed, appeared or disappeared.
    pub sheet_changed: bool,
    /// The new SHA-256 of the sheet (the hash of its bytes); `null` when the sheet did not change
    /// or the file is missing.
    pub hash: Option<String>,
    /// The names of the images that changed, appeared or disappeared, relative to the notes
    /// folder, sorted.
    pub images: Vec<String>,
}

/// Receives each event; the app sends it to the window of the project.
pub type NotesSink = Arc<dyn Fn(NotesChanged) + Send + Sync>;

/// Sends `notes-changed` to the window of the project it is about.
pub fn emitter(app: tauri::AppHandle) -> NotesSink {
    use tauri::Emitter;
    Arc::new(move |changed: NotesChanged| {
        let label = crate::windows::registry::label_of(&changed.project_id);
        if let Err(error) = app.emit_to(
            &label,
            crate::events::EventName::NotesChanged.as_str(),
            &changed,
        ) {
            log_warn!("the window {label} was not told of a change of its notes: {error}");
        }
    })
}

/// What a batch of watcher events says about the notes, from the names of the paths alone.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NotesBatch {
    /// The sheet may have changed.
    pub sheet: bool,
    /// Events were lost, or the folder itself changed: every image may have changed.
    pub all_images: bool,
    /// Paths relative to the notes folder, with `/`: images, or folders holding some.
    pub candidates: BTreeSet<String>,
}

/// A batch of watcher events of a project, split in what concerns the notes and what concerns
/// the rest of `.weaver/`.
#[derive(Debug, PartialEq, Eq)]
pub struct Split {
    /// What concerns the notes; `None` when the batch names nothing of them the view needs.
    pub notes: Option<NotesBatch>,
    /// What is left for the project's picture; `None` when nothing is.
    pub rest: Option<Rebuild>,
}

/// Takes the paths under `.weaver/notes/` of the project at `root` out of `batch`. Hidden names
/// there (the `.gitignore`, temp files) go too, but tell the view nothing. A batch that names no
/// path (events were lost, `.weaver/` appeared or went away) concerns both.
pub fn split_batch(root: &Path, batch: Rebuild) -> Split {
    match batch {
        Rebuild::All => Split {
            notes: Some(NotesBatch {
                sheet: true,
                all_images: true,
                candidates: BTreeSet::new(),
            }),
            rest: Some(Rebuild::All),
        },
        Rebuild::Paths(paths) => {
            let dir = notes::notes_dir(root);
            let (inside, rest): (Vec<PathBuf>, Vec<PathBuf>) =
                paths.into_iter().partition(|path| path.starts_with(&dir));
            let mut notes = NotesBatch::default();
            for path in &inside {
                let Ok(relative) = path.strip_prefix(&dir) else {
                    continue;
                };
                let parts: Vec<String> = relative
                    .components()
                    .filter_map(|part| match part {
                        Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
                        _ => None,
                    })
                    .collect();
                if parts.is_empty() {
                    // The folder itself appeared, went away or changed.
                    notes.sheet = true;
                    notes.all_images = true;
                } else if parts.iter().any(|part| part.starts_with('.')) {
                    // The `.gitignore` and the temp files of atomic writes.
                } else if parts.len() == 1 && parts[0] == SHEET_FILE {
                    notes.sheet = true;
                } else {
                    notes.candidates.insert(parts.join("/"));
                }
            }
            let concerns = notes.sheet || notes.all_images || !notes.candidates.is_empty();
            Split {
                notes: concerns.then_some(notes),
                rest: (!rest.is_empty()).then_some(Rebuild::Paths(rest)),
            }
        }
    }
}

/// The events of the notes of every open project.
pub struct NotesChanges {
    sink: OnceLock<NotesSink>,
    /// The generation of each project, held while an event is made and sent so that the events of
    /// a project leave in the order of their generations.
    projects: Mutex<HashMap<String, Arc<Mutex<u64>>>>,
}

impl Default for NotesChanges {
    fn default() -> Self {
        Self::new()
    }
}

impl NotesChanges {
    pub fn new() -> Self {
        Self {
            sink: OnceLock::new(),
            projects: Mutex::new(HashMap::new()),
        }
    }

    /// Where `notes-changed` goes; only the first sink counts. The service exists before the app
    /// handle does.
    pub fn attach(&self, sink: NotesSink) {
        let _ = self.sink.set(sink);
    }

    /// `window.CLOSED`: the generation of the project is forgotten.
    pub fn close_project(&self, project_id: &str) {
        self.projects
            .lock()
            .expect("notes projects poisoned")
            .remove(project_id);
    }

    /// Reads what `batch` names (the sheet, the folder) and, when something the view must know
    /// changed, sends the event and returns it. `known` is the hash of the sheet Weaver last
    /// wrote or read (`None` when it never did, or the file was missing): a sheet that hashes to
    /// it is Weaver's own writing. Runs disk work: call it on the blocking pool.
    pub fn announce(
        &self,
        project_id: &str,
        root: &Path,
        batch: &NotesBatch,
        known: &dyn Fn() -> Option<String>,
    ) -> Option<NotesChanged> {
        let generation = self
            .projects
            .lock()
            .expect("notes projects poisoned")
            .entry(project_id.to_owned())
            .or_default()
            .clone();
        let mut generation = generation.lock().expect("notes generation poisoned");

        let mut hash = None;
        let mut sheet_changed = false;
        if batch.sheet {
            match notes::sheet_hash(root) {
                Ok(now) => {
                    if now != known() {
                        sheet_changed = true;
                        hash = now;
                    }
                }
                Err(error) => log_warn!(
                    "the notes sheet of {} could not be read: {error}",
                    root.display()
                ),
            }
        }
        let images = changed_images(root, batch);
        if !sheet_changed && images.is_empty() {
            return None;
        }
        *generation += 1;
        let event = NotesChanged {
            project_id: project_id.to_owned(),
            generation: *generation,
            sheet_changed,
            hash,
            images,
        };
        if let Some(sink) = self.sink.get() {
            sink(event.clone());
        }
        Some(event)
    }
}

/// The names of the images `batch` names: the paths it gives, a folder replaced by the files in
/// it, or all of them when events were lost.
fn changed_images(root: &Path, batch: &NotesBatch) -> Vec<String> {
    let listed = |root: &Path| match notes::image_names(root) {
        Ok(names) => names,
        Err(error) => {
            log_warn!(
                "the notes images of {} could not be listed: {error}",
                root.display()
            );
            Vec::new()
        }
    };
    if batch.all_images {
        return listed(root);
    }
    let dir = notes::notes_dir(root);
    let mut names = BTreeSet::new();
    let mut inside_folders: Option<Vec<String>> = None;
    for candidate in &batch.candidates {
        if dir.join(candidate).is_dir() {
            let all = inside_folders.get_or_insert_with(|| listed(root));
            let prefix = format!("{candidate}/");
            names.extend(all.iter().filter(|name| name.starts_with(&prefix)).cloned());
        } else {
            names.insert(candidate.clone());
        }
    }
    names.into_iter().collect()
}

#[cfg(test)]
mod tests;
