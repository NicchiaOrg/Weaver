//! The unsaved changes of the File view of a window, as far as closing needs them.
//!
//! The UI of each project window tells the backend, at every change, in which files its File view
//! has unsaved changes (`files_unsaved`). Closing the window or quitting asks a native confirmation
//! that lists them; with `Save all` the backend asks the window's UI for the texts of its tabs
//! (`files-flush`, answered by `files_flushed`, as `requests-flush` does for the request editor),
//! waits at most [`crate::requests::FLUSH_LIMIT`] and saves them as `developer.SAVE_FILE`. What
//! could not be saved goes back to the window as `files-save-failed`, so that its File view shows
//! the tab of each such file with the reason.
//!
//! Nothing here reads or writes a file: the texts and the paths travel, the saving is the
//! [`super::manager::Shell::save_tab`] of the File view's tabs.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;

use super::manager::WindowManager;
use super::registry::label_of;
use crate::error::AppError;
use crate::events::EventName;

/// Payload of `files-flush`: the window is closing and the File view must send the texts of its
/// tabs with unsaved changes (the answer is `files_flushed`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FilesFlush {
    pub project_id: String,
}

/// The text of one tab with unsaved changes: its path relative to the project, with `/`, and the
/// text as the editor holds it (no byte order mark, LF line endings).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TabText {
    pub path: String,
    pub text: String,
}

/// A file that `Save all` could not save, and why, in words the developer reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SaveFailure {
    pub path: String,
    pub reason: String,
}

/// A file `Save all` wrote: its path and the hash of what was written, which its tab holds from
/// then on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SavedFile {
    pub path: String,
    pub hash: String,
}

/// What a `Save all` did: the files it saved, and those it could not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveReport {
    pub saved: Vec<SavedFile>,
    pub failures: Vec<SaveFailure>,
}

/// Payload of `files-save-failed`: the window stays open on the tab of the first failed file; the
/// files that were saved meanwhile are listed, so that their tabs show no unsaved change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FilesSaveFailed {
    pub project_id: String,
    pub saved: Vec<SavedFile>,
    pub failures: Vec<SaveFailure>,
}

/// Payload of `files-close-tab`: the File view of the window closes its active tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FilesCloseTab {
    pub project_id: String,
}

/// Asks the File view of a project window for the texts of its tabs with unsaved changes; `false`
/// when the window cannot be reached.
pub fn emit_flush(app: &AppHandle, project_id: &str) -> bool {
    app.emit_to(
        label_of(project_id),
        EventName::FilesFlush.as_str(),
        &FilesFlush {
            project_id: project_id.to_owned(),
        },
    )
    .is_ok()
}

/// Tells a window that its `Save all` left files unsaved.
pub fn emit_save_failed(app: &AppHandle, event: &FilesSaveFailed) {
    let label = label_of(&event.project_id);
    if let Err(error) = app.emit_to(&label, EventName::FilesSaveFailed.as_str(), event) {
        crate::log_warn!("the window {label} was not told which files were not saved: {error}");
    }
}

/// **Close Window** was chosen in a window whose File view has a tab: it closes the active tab.
pub fn emit_close_tab(app: &AppHandle, project_id: &str) {
    let label = label_of(project_id);
    let event = FilesCloseTab {
        project_id: project_id.to_owned(),
    };
    if let Err(error) = app.emit_to(&label, EventName::FilesCloseTab.as_str(), &event) {
        crate::log_warn!("the window {label} was not told to close its tab: {error}");
    }
}

/// The File view of a window says in which files it has unsaved changes (none: it has none).
/// Said at every change, with the whole list.
#[tauri::command]
pub async fn files_unsaved(
    manager: State<'_, Arc<WindowManager>>,
    project_id: String,
    paths: Vec<String>,
) -> Result<(), AppError> {
    manager.set_unsaved(&project_id, paths);
    Ok(())
}

/// The answer to `files-flush`: the texts of the tabs with unsaved changes.
#[tauri::command]
pub async fn files_flushed(
    manager: State<'_, Arc<WindowManager>>,
    project_id: String,
    tabs: Vec<TabText>,
) -> Result<(), AppError> {
    manager.tabs_flushed(&project_id, tabs);
    Ok(())
}

/// The File view has a tab (`claimed`) or has none: while it has one, **Close Window**
/// (`Cmd+W` / `Ctrl+W`) closes the active tab instead of the window.
#[tauri::command]
pub async fn files_close_key(
    manager: State<'_, Arc<WindowManager>>,
    project_id: String,
    claimed: bool,
) -> Result<(), AppError> {
    manager.set_close_key(&project_id, claimed);
    Ok(())
}
