//! The Tauri commands of Ask more: opening a conversation, and the conversation and the decisions of
//! the developer in it. A conversation is named by its identity, generated at opening.

use std::sync::Arc;

use tauri::State;

use super::build::AskTarget;
use crate::blocking;
use crate::error::{AppError, ErrorCode};
use crate::general::service::General;
use crate::general::wire::{AskSnapshot, AsksSnapshot};
use crate::rounds::Rounds;
use crate::state::AppState;
use crate::subagents::wire::{Step, SubAgentConversation, SubAgentsEvent};

fn refused(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::Refused, message)
}

/// `developer.ASK_MORE`: reads the chosen thing from the files (or, for a question, the presented
/// round) and opens a new conversation; returns its identity. Refused, with nothing opened, when
/// the thing no longer exists.
#[tauri::command]
pub async fn ask_open(
    state: State<'_, AppState>,
    general: State<'_, Arc<General>>,
    rounds: State<'_, Arc<Rounds>>,
    project_id: String,
    target: AskTarget,
) -> Result<String, AppError> {
    let picture = state.pictures.get(&project_id)?;
    let (general, rounds) = (general.inner().clone(), rounds.inner().clone());
    let name = picture
        .root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned());
    let root = picture.root.clone();
    let opened = blocking::disk_named("ask open", name, move || {
        super::open(&general, &rounds, &project_id, &root, &target)
    })
    .await?;
    opened.ok_or_else(|| refused("The chosen thing no longer exists."))
}

/// The Ask more conversations of an open project, the most recent first.
#[tauri::command]
pub async fn ask_list(
    general: State<'_, Arc<General>>,
    project_id: String,
) -> Result<AsksSnapshot, AppError> {
    Ok(AsksSnapshot {
        asks: general.asks(&project_id),
        project_id,
    })
}

/// One conversation as it is now.
#[tauri::command]
pub async fn ask_snapshot(
    general: State<'_, Arc<General>>,
    ask_id: String,
) -> Result<AskSnapshot, AppError> {
    general.ask_snapshot(&ask_id).map_err(refused)
}

/// `developer.SEND_MESSAGE`: the first message starts omp and carries the instruction.
#[tauri::command]
pub async fn ask_send_message(
    general: State<'_, Arc<General>>,
    ask_id: String,
    text: String,
) -> Result<(), AppError> {
    ensure_ask(&general, &ask_id)?;
    general.send_message(&ask_id, &text).map_err(refused)
}

/// `developer.STOP_AGENT`: without a confirmation, only the turn is cancelled.
#[tauri::command]
pub async fn ask_stop(general: State<'_, Arc<General>>, ask_id: String) -> Result<(), AppError> {
    ensure_ask(&general, &ask_id)?;
    general.stop(&ask_id).map_err(refused)
}

/// `developer.ANSWER_PERMISSION`.
#[tauri::command]
pub async fn ask_answer_permission(
    general: State<'_, Arc<General>>,
    ask_id: String,
    request: u32,
    option: String,
) -> Result<(), AppError> {
    ensure_ask(&general, &ask_id)?;
    general
        .answer_permission(&ask_id, u64::from(request), &option)
        .map_err(refused)
}

/// `developer.CLOSE_ASK`, once confirmed when its agent works: the conversation ends and omp with
/// it.
#[tauri::command]
pub async fn ask_close(general: State<'_, Arc<General>>, ask_id: String) -> Result<(), AppError> {
    let ended = general.close_ask(&ask_id).map_err(refused)?;
    ended.await;
    Ok(())
}

/// The list of the sub-agents of a conversation.
#[tauri::command]
pub async fn ask_sub_agents(
    general: State<'_, Arc<General>>,
    ask_id: String,
) -> Result<SubAgentsEvent, AppError> {
    ensure_ask(&general, &ask_id)?;
    general.sub_agents(&ask_id).map_err(refused)
}

/// The read-only conversation of the sub-agent at `path` of a conversation.
#[tauri::command]
pub async fn ask_sub_agent_conversation(
    general: State<'_, Arc<General>>,
    ask_id: String,
    path: Vec<Step>,
) -> Result<SubAgentConversation, AppError> {
    ensure_ask(&general, &ask_id)?;
    general
        .sub_agent_conversation(&ask_id, &path)
        .map_err(refused)
}

/// What the Jobs view shows of the sub-agents of a conversation: whether their list is on screen,
/// and the conversation of which sub-agent. Their transcripts are read only for what is shown.
#[tauri::command]
pub async fn ask_sub_agents_view(
    general: State<'_, Arc<General>>,
    ask_id: String,
    list_shown: bool,
    path: Option<Vec<Step>>,
) -> Result<(), AppError> {
    ensure_ask(&general, &ask_id)?;
    general
        .sub_agents_interest(&ask_id, list_shown, path)
        .map_err(refused)
}

/// The commands take a conversation of Ask more only: never the General of a project.
fn ensure_ask(general: &General, ask_id: &str) -> Result<(), AppError> {
    if general.is_ask(ask_id) {
        Ok(())
    } else {
        Err(refused("The conversation is not open."))
    }
}
