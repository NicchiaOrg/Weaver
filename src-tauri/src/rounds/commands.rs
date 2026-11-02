//! The Tauri commands of the question rounds: the round on screen and what the developer does in it.

use std::sync::Arc;

use tauri::State;

use super::service::Rounds;
use super::view::{ActionInput, QuestionRoundView};
use crate::error::{AppError, ErrorCode};

/// The round presented in the window of a project, when there is one: fetched when the window
/// loads; after that `question-round-changed` brings the changes.
#[tauri::command]
pub async fn question_round_get(
    rounds: State<'_, Arc<Rounds>>,
    project_id: String,
) -> Result<Option<QuestionRoundView>, AppError> {
    Ok(rounds.view(&project_id))
}

/// What the developer does in the presented round; the round on screen afterwards.
#[tauri::command]
pub async fn question_round_act(
    rounds: State<'_, Arc<Rounds>>,
    project_id: String,
    round_id: u64,
    action: ActionInput,
) -> Result<Option<QuestionRoundView>, AppError> {
    rounds
        .act(&project_id, round_id, action.into())
        .map_err(|error| AppError::new(ErrorCode::Refused, error.to_string()))
}
