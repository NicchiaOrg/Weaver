//! The Tauri commands of the File view's tabs: open a file, read it again, save it, close its tab.
//! The UI never touches a file; every path is relative to the project root, with `/` separators,
//! and is checked in the backend. Disk work runs on the blocking pool.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;
use weaver_core::file_tab::limit::TabInfo;

use super::wire::{FileOpenResult, FileReloadResult, FileSaveResult};
use super::{FileTabs, SaveMode, SaveOutcome};
use crate::blocking;
use crate::error::{AppError, ErrorCode};
use crate::state::AppState;

fn refused(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::Refused, message)
}

/// The canonical root of an open project.
fn root_of(state: &AppState, project_id: &str) -> Result<PathBuf, AppError> {
    Ok(state.pictures.get(project_id)?.root)
}

fn tabs_of(state: &AppState) -> Arc<FileTabs> {
    state.file_tabs.clone()
}

/// `developer.OPEN_FILE`: reads the file once and gives it a tab. `tabs` are the tabs the UI has,
/// the one shown least recently first, with whether each has unsaved changes: the limit of 10
/// tabs closes the first one without any and refuses when every one has some.
#[tauri::command]
pub async fn file_open(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
    tabs: Vec<TabInfo>,
) -> Result<FileOpenResult, AppError> {
    let root = root_of(&state, &project_id)?;
    let files = tabs_of(&state);
    blocking::disk_named("open file", None, move || {
        files
            .open(&project_id, &root, &path, &tabs)
            .map_err(refused)
    })
    .await?
}

/// The file of a tab as it is on disk now, for `developer.RELOAD_FILE` and for a tab that reads
/// its file again after the disk changed or the agent's lock was released.
#[tauri::command]
pub async fn file_reload(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<FileReloadResult, AppError> {
    let files = tabs_of(&state);
    blocking::disk_named("reload file", None, move || {
        files.reload(&project_id, &path).map_err(refused)
    })
    .await?
}

/// `developer.SAVE_FILE`, `developer.OVERWRITE_FILE` and `Save again`: writes the text of a tab; the
/// command carries the hash the tab read, which only the check of `SAVE_FILE` uses.
#[tauri::command]
pub async fn file_save(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
    text: String,
    hash: String,
    mode: SaveMode,
) -> Result<FileSaveResult, AppError> {
    let files = tabs_of(&state);
    let outcome = blocking::disk_named("save file", None, move || {
        files.save_read(&project_id, &path, &text, mode, Some(&hash))
    })
    .await?;
    Ok(match outcome {
        SaveOutcome::Saved { hash } => FileSaveResult::Saved { hash },
        SaveOutcome::Changed => FileSaveResult::Changed,
        SaveOutcome::Gone => FileSaveResult::Gone,
        SaveOutcome::Failed(reason) => FileSaveResult::Failed { reason },
    })
}

/// `file-tab.CLOSED`: frees the tab, its lock tracking and its watch at once.
#[tauri::command]
pub async fn file_close(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<(), AppError> {
    tabs_of(&state).close(&project_id, &path);
    Ok(())
}
