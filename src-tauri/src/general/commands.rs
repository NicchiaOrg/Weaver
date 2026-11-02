//! The Tauri commands of General: the conversation and the decisions of the developer in it.

use std::sync::Arc;

use tauri::State;

use super::service::General;
use super::wire::GeneralSnapshot;
use crate::error::{AppError, ErrorCode};

fn refused(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::Refused, message)
}

/// The conversation and the state of the General of an open project.
#[tauri::command]
pub async fn general_snapshot(
    general: State<'_, Arc<General>>,
    project_id: String,
) -> Result<GeneralSnapshot, AppError> {
    general.snapshot(&project_id).map_err(refused)
}

/// `developer.SEND_MESSAGE`: omp starts when it is not running.
#[tauri::command]
pub async fn general_send_message(
    general: State<'_, Arc<General>>,
    project_id: String,
    text: String,
) -> Result<(), AppError> {
    general.send_message(&project_id, &text).map_err(refused)
}

/// `developer.STOP_AGENT`.
#[tauri::command]
pub async fn general_stop(
    general: State<'_, Arc<General>>,
    project_id: String,
) -> Result<(), AppError> {
    general.stop(&project_id).map_err(refused)
}

/// `developer.ANSWER_PERMISSION`.
#[tauri::command]
pub async fn general_answer_permission(
    general: State<'_, Arc<General>>,
    project_id: String,
    request: u32,
    option: String,
) -> Result<(), AppError> {
    general
        .answer_permission(&project_id, u64::from(request), &option)
        .map_err(refused)
}

/// `developer.CLEAR_GENERAL`, once confirmed.
#[tauri::command]
pub async fn general_clear(
    general: State<'_, Arc<General>>,
    project_id: String,
) -> Result<(), AppError> {
    general.clear(&project_id).map_err(refused)
}
