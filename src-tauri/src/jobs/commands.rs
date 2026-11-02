//! The Tauri commands of the jobs: the Jobs view and the decisions that start work (`Run`,
//! `Run all`, `Scan System`). A decision that starts a job runs in the project's queue, with the
//! facts of that moment, and is refused with a sentence the Board shows as a notice.

use std::sync::Arc;

use tauri::State;
use weaver_core::project::image::ProjectImage;

use super::registry::Jobs;
use super::starts::Ctx;
use super::wire::{ConversationSnapshot, JobsSnapshot};
use crate::error::{AppError, ErrorCode};

fn refused(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::Refused, message)
}

/// The Jobs list of an open project.
#[tauri::command]
pub async fn jobs_snapshot(
    jobs: State<'_, Arc<Jobs>>,
    project_id: String,
) -> Result<JobsSnapshot, AppError> {
    Ok(jobs.snapshot(&project_id))
}

/// The conversation of a job.
#[tauri::command]
pub async fn job_conversation(
    jobs: State<'_, Arc<Jobs>>,
    project_id: String,
    job_id: String,
) -> Result<ConversationSnapshot, AppError> {
    jobs.conversation(&project_id, &job_id).map_err(refused)
}

/// `developer.SEND_MESSAGE`.
#[tauri::command]
pub async fn job_send_message(
    jobs: State<'_, Arc<Jobs>>,
    project_id: String,
    job_id: String,
    text: String,
) -> Result<(), AppError> {
    jobs.send_message(&project_id, &job_id, &text)
        .map_err(refused)
}

/// `developer.STOP_AGENT`.
#[tauri::command]
pub async fn job_stop(
    jobs: State<'_, Arc<Jobs>>,
    project_id: String,
    job_id: String,
) -> Result<(), AppError> {
    jobs.stop(&project_id, &job_id).map_err(refused)
}

/// `developer.ANSWER_PERMISSION`.
#[tauri::command]
pub async fn job_answer_permission(
    jobs: State<'_, Arc<Jobs>>,
    project_id: String,
    job_id: String,
    request: u32,
    option: String,
) -> Result<(), AppError> {
    let request = u64::from(request);
    jobs.answer_permission(&project_id, &job_id, request, &option)
        .map_err(refused)?;
    // The notification that announced the permission is no longer true.
    jobs.tell(&job_id, false, |observer, job| {
        observer.permission_answered(job, request)
    });
    Ok(())
}

/// The `X` of a finished job's row.
#[tauri::command]
pub async fn job_remove(
    jobs: State<'_, Arc<Jobs>>,
    project_id: String,
    job_id: String,
) -> Result<(), AppError> {
    jobs.remove(&project_id, &job_id).map_err(refused)
}

/// Starts jobs in the project's queue with a fresh picture of the files.
async fn start_in_queue(
    jobs: &Arc<Jobs>,
    project_id: &str,
    what: impl FnOnce(&Jobs, &Ctx) -> Result<(), String> + Send + 'static,
) -> Result<(), AppError> {
    let project = project_id.to_owned();
    let worker = jobs.clone();
    jobs.host()
        .mutate(
            project_id,
            Box::new(move |root| {
                let image = ProjectImage::load(root);
                what(
                    &worker,
                    &Ctx {
                        project_id: &project,
                        root,
                        image: &image,
                    },
                )?;
                Ok(None)
            }),
        )
        .await
        .map_err(refused)
}

/// `developer.RUN_TASK`.
#[tauri::command]
pub async fn run_task(
    jobs: State<'_, Arc<Jobs>>,
    project_id: String,
    task_id: String,
) -> Result<(), AppError> {
    let jobs = jobs.inner().clone();
    start_in_queue(&jobs, &project_id, move |jobs, ctx| {
        let label = weaver_core::jobs::names::card_label(
            &weaver_core::project::shortnum::peek(ctx.root, ctx.image.cards()),
            weaver_core::project::cards::CardKind::Task,
            &task_id,
        );
        jobs.start_run_task(ctx, &task_id)
            .map(|_| ())
            .map_err(|refusal| format!("Could not run {label}: {refusal}."))
    })
    .await
}

/// `developer.RUN_ALL`.
#[tauri::command]
pub async fn run_all(jobs: State<'_, Arc<Jobs>>, project_id: String) -> Result<(), AppError> {
    let jobs = jobs.inner().clone();
    start_in_queue(&jobs, &project_id, |jobs, ctx| {
        jobs.start_run_all(ctx)
            .map(|_| ())
            .map_err(|refusal| format!("Could not run all the Tasks: {refusal}."))
    })
    .await
}

/// `developer.SCAN_SYSTEM`, once the developer confirmed.
#[tauri::command]
pub async fn scan_system(jobs: State<'_, Arc<Jobs>>, project_id: String) -> Result<(), AppError> {
    let jobs = jobs.inner().clone();
    start_in_queue(&jobs, &project_id, |jobs, ctx| {
        jobs.start_scan(ctx)
            .map(|_| ())
            .map_err(|refusal| format!("Could not scan the System: {refusal}."))
    })
    .await
}
