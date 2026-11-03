//! The Tauri commands of the File view's tree. Each runs on the blocking pool, since it reads or
//! changes files, and answers what the UI must change in its tree. A refused action is an
//! [`ErrorCode::Refused`] error whose message is the reason the UI shows in a toast; nothing
//! changes when it is refused.

use std::sync::Arc;

use tauri::State;
use weaver_core::file_view::machine::{TreeSnapshot, TreeUpdate};
use weaver_core::file_view::{FileEntry, Refusal};

use super::{EntryChange, FileViewService};
use crate::blocking;
use crate::error::{AppError, ErrorCode};
use crate::state::AppState;

fn refused(refusal: Refusal) -> AppError {
    AppError::new(ErrorCode::Refused, refusal.to_string())
}

fn service(state: &State<'_, AppState>) -> Arc<FileViewService> {
    state.file_view.clone()
}

/// `developer.OPEN_FILE_VIEW`: the view is shown. Reads only the root's entries (or, within the
/// release delay, answers the tree as it was).
#[tauri::command]
pub async fn files_open_view(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<TreeSnapshot, AppError> {
    let root = state.pictures.get(&project_id)?.root;
    let service = service(&state);
    blocking::disk(move || service.open(&project_id, &root).map_err(refused)).await?
}

/// `developer.EXPAND_FOLDER`.
#[tauri::command]
pub async fn files_expand(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<TreeUpdate, AppError> {
    let service = service(&state);
    blocking::disk(move || service.expand(&project_id, &path).map_err(refused)).await?
}

/// `developer.COLLAPSE_FOLDER`.
#[tauri::command]
pub async fn files_collapse(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<(), AppError> {
    let service = service(&state);
    blocking::disk(move || service.collapse(&project_id, &path).map_err(refused)).await?
}

/// The folders of `path` for the tree of `Move to…`, one folder at a time.
#[tauri::command]
pub async fn files_folders(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<Vec<FileEntry>, AppError> {
    let service = service(&state);
    blocking::disk(move || service.folders(&project_id, &path).map_err(refused)).await?
}

/// `developer.LEAVE_FILE_VIEW`, once the UI closed the tabs (saved or discarded as the developer
/// chose): the view is kept for 10 s, then released.
#[tauri::command]
pub async fn files_leave(state: State<'_, AppState>, project_id: String) -> Result<(), AppError> {
    let service = service(&state);
    let Some(wait) = service.leave(&project_id) else {
        return Ok(());
    };
    tauri::async_runtime::spawn(async move {
        let mut wait = wait;
        loop {
            tokio::time::sleep(wait).await;
            // A view shown again and left again has a later timer: this one finds it not due,
            // or not released at all, and ends.
            let step = {
                let service = service.clone();
                let project_id = project_id.clone();
                blocking::disk(move || service.release_step(&project_id)).await
            };
            match step {
                Ok(Some(more)) => wait = more,
                _ => break,
            }
        }
    });
    Ok(())
}

/// `developer.CREATE_ENTRY`: an empty file, or with `folder` a folder, named `name` in `parent`.
#[tauri::command]
pub async fn files_create(
    state: State<'_, AppState>,
    project_id: String,
    parent: String,
    name: String,
    folder: bool,
) -> Result<EntryChange, AppError> {
    let service = service(&state);
    blocking::disk(move || {
        service
            .create(&project_id, &parent, &name, folder)
            .map_err(refused)
    })
    .await?
}

/// `developer.RENAME_ENTRY`.
#[tauri::command]
pub async fn files_rename(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
    name: String,
) -> Result<EntryChange, AppError> {
    let service = service(&state);
    blocking::disk(move || service.rename(&project_id, &path, &name).map_err(refused)).await?
}

/// `developer.MOVE_ENTRY`: `path` into the folder `target` (empty: the project root).
#[tauri::command]
pub async fn files_move(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
    target: String,
) -> Result<EntryChange, AppError> {
    let service = service(&state);
    blocking::disk(move || {
        service
            .move_into(&project_id, &path, &target)
            .map_err(refused)
    })
    .await?
}

/// `developer.DELETE_ENTRY`: to the system's trash.
#[tauri::command]
pub async fn files_delete(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<TreeUpdate, AppError> {
    let service = service(&state);
    blocking::disk(move || service.delete(&project_id, &path).map_err(refused)).await?
}
