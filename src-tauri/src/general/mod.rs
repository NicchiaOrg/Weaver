//! General chat (task "Chat General"): the free conversation with omp of a project, always in its
//! window.
//!
//! General is not a job: it holds no Task, announces no card, asks no question round, notifies
//! nothing and shows nothing on the Board. It has no instruction, skill or written request; the
//! developer writes what he wants, and on his explicit request the agent starts a job of Weaver
//! with `trama job new`, which is a normal job with a conversation of its own.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`service`] | [`General`]: the Generals of the open projects, the Machine `general-chat` of each, its record |
//! | [`actor`] | The task that owns a General's omp process and ACP session |
//! | [`commands`] | The Tauri commands |
//! | [`wire`] | The state and the conversation as the UI gets them, and where the events go |
//!
//! The pure part (the Machine, the command line of omp, the instruction, the record) is
//! `trama_core::jobs::general`. The process, the JSON-RPC, the ACP calls and the conversation are
//! the jobs' own ([`crate::jobs`]).
//!
//! # Where it plugs in
//!
//! * [`install`] (called by [`crate::jobs::install`]): the state `Arc<General>`, and
//!   [`crate::jobs::Jobs::set_general`], which lets `trama job new` work from the General that is
//!   working and from no other caller.
//! * [`with_windows`] (called by [`crate::jobs::install_windows`]): the window of a project
//!   opening reads its record; the window closing cancels the turn and ends omp.

pub mod actor;
pub mod commands;
pub mod service;
pub mod wire;

#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::events::EventName;
use crate::jobs::Jobs;
use crate::log_info;
use crate::windows::jobs::{BoxFuture, CloseReason, WindowJobs, WindowOpened};

pub use service::General;
use wire::{GeneralEntries, GeneralEvents, GeneralState};

/// The events of General, to every window (the UI keeps what concerns its project).
struct TauriEvents {
    app: AppHandle,
}

impl GeneralEvents for TauriEvents {
    fn state(&self, state: &GeneralState) {
        let _ = self.app.emit(EventName::GeneralChanged.as_str(), state);
    }

    fn entries(&self, entries: &GeneralEntries) {
        let _ = self.app.emit(EventName::GeneralEntries.as_str(), entries);
    }
}

/// Creates the General of the projects and plugs it into the jobs. Call after the jobs exist.
pub fn install(app: &AppHandle, jobs: &Arc<Jobs>) -> Arc<General> {
    let general = General::new(
        jobs.host().clone(),
        Arc::new(TauriEvents { app: app.clone() }),
    );
    app.manage(general.clone());
    jobs.set_general(general.clone());
    log_info!("General is ready");
    general
}

/// The windows' hook of the jobs, with General's: the same calls, and General follows the window.
struct GeneralWindows {
    jobs: Arc<dyn WindowJobs>,
    general: Arc<General>,
}

impl WindowJobs for GeneralWindows {
    fn working_jobs(&self, project_id: &str) -> usize {
        // General is not a job: a turn in it never asks for a confirmation.
        self.jobs.working_jobs(project_id)
    }

    fn window_opened(&self, event: &WindowOpened) {
        self.jobs.window_opened(event);
        let general = self.general.clone();
        let (project_id, root) = (event.project_id.clone(), event.root.clone());
        tauri::async_runtime::spawn(async move {
            general.open(&project_id, Path::new(&root)).await;
        });
    }

    fn window_closed(&self, project_id: &str, reason: CloseReason) -> BoxFuture<()> {
        // Both start ending their omp now; the window closes when both are ended.
        let jobs = self.jobs.window_closed(project_id, reason);
        let general = self.general.window_closed(project_id);
        Box::pin(async move {
            jobs.await;
            general.await;
        })
    }
}

/// The hook of the windows with General added to `jobs`.
pub fn with_windows(jobs: Arc<dyn WindowJobs>, general: Arc<General>) -> Arc<dyn WindowJobs> {
    Arc::new(GeneralWindows { jobs, general })
}
