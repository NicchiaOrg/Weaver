//! The History view in the app: the entries of a project, newest first, with the Tasks they cite.
//!
//! What is in the entries and in which order comes from `trama_core::project::history`; the UI
//! groups them by local day and filters them by period (it knows the local time zone), and never
//! sorts. The UI asks for the snapshot when its window loads and again each time a
//! `project-changed` event names the slice `history`; the `revision` it carries lets it drop an
//! answer older than one it already shows.

use serde::Serialize;
use tauri::State;
use trama_core::project::cards::CardKind;
use trama_core::project::history::{self, Entry, TaskCopy};
use trama_core::project::image::ProjectImage;
use ts_rs::TS;

use crate::blocking;
use crate::error::AppError;
use crate::state::AppState;

/// A Task an entry cites, from the copy the entry keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTask {
    /// The permanent identity (a Task in History has no short number).
    pub id: String,
    pub title: String,
    /// The label of the Proposal the Task belonged to, when it had one.
    pub label: Option<String>,
    /// When the Task was finished (UTC ISO 8601); `None` when no copy of the Task is available.
    pub finished: Option<String>,
    /// What the Task referred to.
    pub refs: Vec<String>,
    /// What the Task said after its title, in Markdown.
    pub body: String,
}

/// A History entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub title: String,
    /// One line, inline Markdown.
    pub summary: String,
    /// The text after the title, in Markdown.
    pub text: String,
    /// When the entry was first written (UTC ISO 8601).
    pub created: String,
    /// When it was written or last updated: the date of the entry (UTC ISO 8601).
    pub updated: String,
    /// The Tasks it cites, in the order cited.
    pub tasks: Vec<HistoryTask>,
}

/// The History of one project at one revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HistorySnapshot {
    pub project_id: String,
    /// The revision of the picture this was worked out from.
    #[ts(type = "number")]
    pub revision: u64,
    /// Newest first by the date of the entry.
    pub entries: Vec<HistoryEntry>,
}

fn task_of(id: &str, copy: Option<TaskCopy>) -> HistoryTask {
    match copy {
        Some(copy) => HistoryTask {
            id: id.to_owned(),
            title: copy.title,
            label: copy.label,
            finished: Some(copy.finished),
            refs: copy.refs,
            body: copy.body,
        },
        None => HistoryTask {
            id: id.to_owned(),
            title: "Task not available".to_owned(),
            label: None,
            finished: None,
            refs: Vec::new(),
            body: String::new(),
        },
    }
}

fn entry_of(image: &ProjectImage, entry: Entry) -> HistoryEntry {
    let tasks = entry
        .tasks
        .iter()
        .map(|id| {
            // A cited Task still a file (its copy could not be made yet) is shown from the file.
            let copy = entry
                .cleared
                .get(id)
                .cloned()
                .or_else(|| history::copy_of(image, image.card(CardKind::Task, id)?).ok());
            task_of(id, copy)
        })
        .collect();
    HistoryEntry {
        id: entry.id,
        title: entry.title,
        summary: entry.summary,
        text: entry.text,
        created: entry.created,
        updated: entry.updated,
        tasks,
    }
}

/// The History of `image`, worked out whole.
pub fn snapshot(project_id: &str, revision: u64, image: &ProjectImage) -> HistorySnapshot {
    HistorySnapshot {
        project_id: project_id.to_owned(),
        revision,
        entries: history::entries(image)
            .into_iter()
            .map(|entry| entry_of(image, entry))
            .collect(),
    }
}

/// The whole History of an open project.
#[tauri::command]
pub async fn history_snapshot(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<HistorySnapshot, AppError> {
    let picture = state.pictures.get(&project_id)?;
    let name = picture
        .root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned());
    blocking::disk_named("history snapshot", name, move || {
        snapshot(&project_id, picture.revision, &picture.image)
    })
    .await
}

#[cfg(test)]
mod tests {
    use trama_core::project::changes::ChangeSet;

    use super::*;

    const TASK: &str = "---\ncolumn: done\ntechnical: false\nrefs:\n  - order\ncreated: 2026-11-02T07:00:01.000Z\nfinished: 2026-11-02T08:00:00.000Z\n---\n# A task\n\nDone when.\n";
    const ENTRY: &str = "---\ncreated: 2026-11-02T09:00:00.000Z\nupdated: 2026-11-02T09:30:00.000Z\nsummary: What was done\ntasks:\n  - bbbbbbbb\n  - aaaaaaaa\ncleared:\n  bbbbbbbb:\n    title: Cleared\n    refs: []\n    technical: true\n    label: Checkout\n    finished: 2026-11-02T08:30:00.000Z\n    body: The body.\n---\n# Checkout works\n\nThe text.\n";
    const OLDER: &str = "---\ncreated: 2026-11-01T09:00:00.000Z\nupdated: 2026-11-01T09:00:00.000Z\nsummary: Yesterday\ntasks:\n  - cccccccc\n---\n# Older\n";

    fn image() -> ProjectImage {
        let mut changes = ChangeSet::new();
        changes.write("tasks/aaaaaaaa-a-task.md", TASK);
        changes.write("history/hhhhhhhh-checkout-works.md", ENTRY);
        changes.write("history/gggggggg-older.md", OLDER);
        ProjectImage::empty(std::path::Path::new("/nonexistent-project")).with_changes(&changes)
    }

    #[test]
    fn entries_come_newest_first_with_their_tasks_in_the_order_cited() {
        let snapshot = snapshot("0123456789abcdef", 4, &image());
        assert_eq!(snapshot.revision, 4);
        let titles: Vec<&str> = snapshot.entries.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["Checkout works", "Older"]);
        let entry = &snapshot.entries[0];
        assert_eq!(
            (
                entry.summary.as_str(),
                entry.text.as_str(),
                entry.updated.as_str()
            ),
            ("What was done", "The text.", "2026-11-02T09:30:00.000Z")
        );
        let [first, second] = entry.tasks.as_slice() else {
            panic!("{:?}", entry.tasks)
        };
        assert_eq!(
            (
                first.id.as_str(),
                first.title.as_str(),
                first.label.as_deref()
            ),
            ("bbbbbbbb", "Cleared", Some("Checkout"))
        );
        assert_eq!(
            (first.finished.as_deref(), first.body.as_str()),
            (Some("2026-11-02T08:30:00.000Z"), "The body.")
        );
        // Still a file, not copied yet: shown from the file.
        assert_eq!(
            (
                second.id.as_str(),
                second.title.as_str(),
                second.refs.as_slice()
            ),
            ("aaaaaaaa", "A task", ["order".to_owned()].as_slice())
        );
        // Neither copied nor a file: only its identity.
        let missing = &snapshot.entries[1].tasks[0];
        assert_eq!(
            (missing.id.as_str(), missing.finished.as_deref()),
            ("cccccccc", None)
        );
    }

    #[test]
    fn the_wire_form_is_camel_case() {
        let json = serde_json::to_value(snapshot("0123456789abcdef", 1, &image())).unwrap();
        assert_eq!(json["projectId"], "0123456789abcdef");
        assert_eq!(
            json["entries"][0]["tasks"][0]["finished"],
            "2026-11-02T08:30:00.000Z"
        );
        assert_eq!(json["entries"][0]["updated"], "2026-11-02T09:30:00.000Z");
    }
}
