//! Backend → UI events. The UI never polls and holds no domain state: it fetches a snapshot
//! when a window loads (or when it sees a gap in revisions) and applies these updates.

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

use crate::error::AppError;
use crate::startup::StartupFailure;
use crate::updates::UpdateStatus;
use trama_core::omp::machine::OmpStatus;
use trama_core::settings::SettingsSnapshot;

/// Names of the events, as the UI listens to them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum EventName {
    ProjectChanged,
    StartupFailed,
    UpdateChanged,
    SettingsChanged,
    OmpChanged,
    /// A window is asked to show a view (a notification click, a second launch); see [`crate::windows`].
    ShowRequested,
    Notice,
    /// The Jobs list of a project changed; carries the whole list (see [`crate::jobs`]).
    JobsChanged,
    /// The conversation of a job changed; carries the entries that changed.
    JobEntries,
    /// The state of a project's General chat changed; carries its whole state (see [`crate::general`]).
    GeneralChanged,
    /// The conversation of a project's General changed; carries the entries that changed.
    GeneralEntries,
    /// The open design requests of a project changed on disk or by a decision; see [`crate::requests`].
    RequestsChanged,
    /// The window of a project is closing: the request editor saves what is pending and answers.
    RequestsFlush,
    /// The round presented in a project's window changed (opened, ended, or one queued behind it);
    /// carries the round on screen; see [`crate::rounds`].
    QuestionRoundChanged,
    /// The state of a project (Not initialised, Empty Model, Has a Model), the files it installs and the opening check
    /// changed; carries the whole status (see [`crate::project_setup`]).
    ProjectState,
}

impl EventName {
    pub fn as_str(self) -> &'static str {
        match self {
            EventName::ProjectChanged => "project-changed",
            EventName::StartupFailed => "startup-failed",
            EventName::UpdateChanged => "update-changed",
            EventName::SettingsChanged => "settings-changed",
            EventName::OmpChanged => "omp-changed",
            EventName::ShowRequested => "show-requested",
            EventName::Notice => "notice",
            EventName::JobsChanged => "jobs-changed",
            EventName::JobEntries => "job-entries",
            EventName::GeneralChanged => "general-changed",
            EventName::GeneralEntries => "general-entries",
            EventName::RequestsChanged => "requests-changed",
            EventName::RequestsFlush => "requests-flush",
            EventName::QuestionRoundChanged => "question-round-changed",
            EventName::ProjectState => "project-state",
        }
    }
}

/// A project's picture was rebuilt. Each rebuild increments `revision`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChanged {
    /// The project hash (`p-<hash>` is the label of its window).
    pub project_id: String,
    #[ts(type = "number")]
    pub revision: u64,
    /// Names of the slices of the picture that changed.
    pub slices: Vec<String>,
}

pub fn emit_project_changed(app: &AppHandle, payload: &ProjectChanged) -> Result<(), AppError> {
    app.emit(EventName::ProjectChanged.as_str(), payload)
        .map_err(AppError::internal)
}

/// Something the developer must read at the top of the Board: a decision refused when it was
/// applied, or a Proposal sent back to Draft while he was deciding it. The UI shows it until
/// he closes it or the window closes; it keeps no history of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    /// The project hash.
    pub project_id: String,
    /// Distinguishes one notice from another; a notice sent twice with the same id shows once.
    pub id: String,
    pub text: String,
}

pub fn emit_notice(app: &AppHandle, payload: &Notice) -> Result<(), AppError> {
    app.emit(EventName::Notice.as_str(), payload)
        .map_err(AppError::internal)
}

/// A startup step failed after the window may already have loaded (see [`crate::startup`]).
pub fn emit_startup_failed(app: &AppHandle, payload: &StartupFailure) -> Result<(), AppError> {
    app.emit(EventName::StartupFailed.as_str(), payload)
        .map_err(AppError::internal)
}

/// What the update on offer is changed (found, hidden by **Later**, downloading); see [`crate::updates`].
pub fn emit_update_changed(app: &AppHandle, payload: &UpdateStatus) -> Result<(), AppError> {
    app.emit(EventName::UpdateChanged.as_str(), payload)
        .map_err(AppError::internal)
}

/// A setting changed; every window applies the snapshot (see [`crate::settings`]).
pub fn emit_settings_changed(app: &AppHandle, payload: &SettingsSnapshot) -> Result<(), AppError> {
    app.emit(EventName::SettingsChanged.as_str(), payload)
        .map_err(AppError::internal)
}

/// omp's readiness changed (the Machine `omp-setup`); every window applies the whole status.
pub fn emit_omp_changed(app: &AppHandle, payload: &OmpStatus) -> Result<(), AppError> {
    app.emit(EventName::OmpChanged.as_str(), payload)
        .map_err(AppError::internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_event_is_called_project_changed_and_carries_id_revision_and_slices() {
        assert_eq!(EventName::ProjectChanged.as_str(), "project-changed");
        assert_eq!(
            serde_json::to_value(EventName::ProjectChanged).unwrap(),
            serde_json::json!("project-changed")
        );
        let json = serde_json::to_value(ProjectChanged {
            project_id: "0123456789abcdef".into(),
            revision: 3,
            slices: vec!["board".into()],
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "projectId": "0123456789abcdef", "revision": 3, "slices": ["board"] })
        );
    }

    #[test]
    fn the_startup_failure_event_is_called_startup_failed() {
        assert_eq!(EventName::StartupFailed.as_str(), "startup-failed");
        assert_eq!(
            serde_json::to_value(EventName::StartupFailed).unwrap(),
            serde_json::json!("startup-failed")
        );
    }
}
