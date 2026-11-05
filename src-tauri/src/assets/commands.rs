//! The Tauri commands of the Assets view. They take asset names, never paths: the backend builds
//! `.weaver/assets/<name>` after checking the name. The only absolute paths are the sources of a
//! drop from outside, as Tauri reports them.
//!
//! Each command that reads or changes files runs on the blocking pool and answers the list of the
//! assets as it is after the action, so the UI applies the result and the list together. A refused
//! action is an [`ErrorCode::Refused`] error whose message is the reason the UI shows in a toast;
//! nothing changes when it is refused.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{State, WebviewWindow};
use weaver_core::file_view::Refusal;

use super::{AssetRenamed, AssetsCopied, AssetsDeleted, AssetsService, AssetsSnapshot};
use crate::blocking;
use crate::error::{AppError, ErrorCode};
use crate::state::AppState;

fn refused(refusal: Refusal) -> AppError {
    AppError::new(ErrorCode::Refused, refusal.to_string())
}

fn service(state: &State<'_, AppState>) -> Arc<AssetsService> {
    state.assets.clone()
}

/// The assets of an open project as the backend holds them, for a window that starts; later
/// changes arrive as `assets-changed`.
#[tauri::command]
pub async fn assets_list(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<AssetsSnapshot, AppError> {
    let root = state.pictures.get(&project_id)?.root;
    let service = service(&state);
    blocking::disk(move || service.snapshot(&project_id, &root)).await
}

/// `developer.DROP_ASSETS`: copies `paths` (dropped from outside, absolute) into
/// `.weaver/assets/`. Runs to its end even when the view is left or the window closes.
#[tauri::command]
pub async fn assets_copy_in(
    state: State<'_, AppState>,
    project_id: String,
    paths: Vec<String>,
) -> Result<AssetsCopied, AppError> {
    let root = state.pictures.get(&project_id)?.root;
    let service = service(&state);
    let sources: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    blocking::disk_named("assets_copy_in", None, move || {
        service
            .copy_in(&project_id, &root, &sources)
            .map_err(refused)
    })
    .await?
}

/// `developer.RENAME_ASSET`.
#[tauri::command]
pub async fn assets_rename(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
    new_name: String,
) -> Result<AssetRenamed, AppError> {
    let root = state.pictures.get(&project_id)?.root;
    let service = service(&state);
    blocking::disk(move || {
        service
            .rename(&project_id, &root, &name, &new_name)
            .map_err(refused)
    })
    .await?
}

/// `developer.DELETE_ASSETS`: each asset to the system's trash, on its own; the ones the trash
/// refuses stay and come back with their reasons.
#[tauri::command]
pub async fn assets_delete(
    state: State<'_, AppState>,
    project_id: String,
    names: Vec<String>,
) -> Result<AssetsDeleted, AppError> {
    let root = state.pictures.get(&project_id)?.root;
    let service = service(&state);
    blocking::disk(move || service.delete(&project_id, &root, &names)).await
}

/// `developer.DRAG_ASSETS_OUT`: starts a native drag of the assets `names` out of this window,
/// as copies. Answers once the system took the drag over; the assets are never changed, whatever
/// the app that receives them asks.
#[tauri::command]
pub async fn assets_drag_out(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project_id: String,
    names: Vec<String>,
) -> Result<(), AppError> {
    // Only the window of the project may drag its assets.
    if window.label() != crate::windows::registry::label_of(&project_id) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "The assets belong to another window.",
        ));
    }
    let root = state.pictures.get(&project_id)?.root;
    let paths = blocking::disk(move || weaver_core::assets::ops::drag_paths(&root, &names))
        .await?
        .map_err(refused)?;
    super::drag::start(&window, paths)
        .await
        .map_err(|reason| AppError::internal(format!("The drag could not start: {reason}")))
}
