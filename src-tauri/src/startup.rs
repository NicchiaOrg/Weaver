//! Startup as a list of isolated steps (`requirements/quality#Reliability`).
//!
//! Each step (logging, settings, local socket, recent projects, each project window, skills sync
//! per project) is a closure that returns `Ok(())` or a reason. A step that fails, or panics, is
//! logged, recorded as a [`StartupFailure`] and announced to the UI; it never stops the other
//! steps. The UI shows `Weaver could not start: <reason>` (no project) or
//! `<project> could not open: <reason>` (a project window) with an **Open log** button.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;

use serde::Serialize;
use tauri::AppHandle;
use ts_rs::TS;

use crate::events;
use crate::{log_at, log_error};

/// A step that did not complete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StartupFailure {
    /// The project hash (`p-<hash>` is its window's label); `None` for the app as a whole.
    pub project_id: Option<String>,
    /// The project's folder name; `None` for the app as a whole.
    pub project_name: Option<String>,
    /// The step's name, for the log and for telling failures apart.
    pub step: String,
    /// What went wrong, in plain words, without any content of the project.
    pub reason: String,
}

/// The project a step belongs to.
#[derive(Debug, Clone)]
pub struct ProjectRef {
    pub id: String,
    pub name: String,
}

type Run = Box<dyn FnOnce() -> Result<(), String> + Send + 'static>;

pub struct Step {
    name: &'static str,
    project: Option<ProjectRef>,
    run: Run,
}

impl Step {
    /// A step of the app as a whole.
    pub fn app(
        name: &'static str,
        run: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) -> Self {
        Self {
            name,
            project: None,
            run: Box::new(run),
        }
    }

    /// A step of one project's window.
    pub fn project(
        project: ProjectRef,
        name: &'static str,
        run: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) -> Self {
        Self {
            name,
            project: Some(project),
            run: Box::new(run),
        }
    }
}

/// Runs one step here and now. A panic is a failure like any other.
pub fn run_step(step: Step) -> Option<StartupFailure> {
    let Step { name, project, run } = step;
    let outcome = match catch_unwind(AssertUnwindSafe(run)) {
        Ok(result) => result,
        Err(_) => Err(format!("the step `{name}` panicked")),
    };
    let reason = outcome.err()?;
    let failure = StartupFailure {
        project_id: project.as_ref().map(|p| p.id.clone()),
        project_name: project.as_ref().map(|p| p.name.clone()),
        step: name.to_owned(),
        reason,
    };
    log_at!(
        crate::logging::Level::Error,
        ctx: crate::logging::Context { project: failure.project_name.as_deref(), job: None },
        "startup step `{}` failed: {}",
        failure.step,
        failure.reason
    );
    Some(failure)
}

/// Runs the steps one after the other; each failure is reported, none stops the rest.
pub fn run_all(steps: Vec<Step>) -> Vec<StartupFailure> {
    steps.into_iter().filter_map(run_step).collect()
}

/// The failures so far, for windows that load after they happened.
#[derive(Default)]
pub struct StartupReport {
    failures: Mutex<Vec<StartupFailure>>,
}

impl StartupReport {
    pub fn record(&self, failure: StartupFailure) {
        self.failures
            .lock()
            .expect("startup report poisoned")
            .push(failure);
    }

    pub fn failures(&self) -> Vec<StartupFailure> {
        self.failures
            .lock()
            .expect("startup report poisoned")
            .clone()
    }
}

/// Runs `step` here, records its failure and tells the UI. For the steps the others need first.
pub fn run_now(app: &AppHandle, report: &StartupReport, step: Step) {
    if let Some(failure) = run_step(step) {
        announce(app, report, failure);
    }
}

/// Runs `step` on the blocking pool (never on the main or IPC thread), records its failure and
/// tells the UI.
pub fn spawn(app: &AppHandle, step: Step) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(failure) = run_step(step) {
            use tauri::Manager;
            announce(&app, &app.state::<StartupReport>(), failure);
        }
    });
}

fn announce(app: &AppHandle, report: &StartupReport, failure: StartupFailure) {
    report.record(failure.clone());
    if let Err(error) = events::emit_startup_failed(app, &failure) {
        log_error!(
            "cannot tell the UI about the failed step `{}`: {error}",
            failure.step
        );
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;

    fn counting(ran: &Arc<AtomicUsize>) -> Step {
        let ran = ran.clone();
        Step::app("counting", move || {
            ran.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }

    #[test]
    fn a_failed_step_does_not_stop_the_others() {
        let ran = Arc::new(AtomicUsize::new(0));
        let failures = run_all(vec![
            counting(&ran),
            Step::app("socket", || Err("address already in use".to_owned())),
            counting(&ran),
            Step::project(
                ProjectRef {
                    id: "0123456789abcdef".into(),
                    name: "Acme".into(),
                },
                "skills",
                || Err("cannot write .omp/skills".to_owned()),
            ),
            counting(&ran),
        ]);
        assert_eq!(ran.load(Ordering::SeqCst), 3);
        assert_eq!(
            failures,
            vec![
                StartupFailure {
                    project_id: None,
                    project_name: None,
                    step: "socket".into(),
                    reason: "address already in use".into()
                },
                StartupFailure {
                    project_id: Some("0123456789abcdef".into()),
                    project_name: Some("Acme".into()),
                    step: "skills".into(),
                    reason: "cannot write .omp/skills".into()
                },
            ]
        );
    }

    #[test]
    fn a_panicking_step_is_a_failure_and_the_next_step_still_runs() {
        let ran = Arc::new(AtomicUsize::new(0));
        let failures = run_all(vec![
            Step::app("settings", || panic!("boom")),
            counting(&ran),
        ]);
        assert_eq!(ran.load(Ordering::SeqCst), 1);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].step, "settings");
        assert_eq!(failures[0].reason, "the step `settings` panicked");
    }

    #[test]
    fn steps_that_succeed_report_nothing() {
        assert!(run_all(vec![Step::app("a", || Ok(())), Step::app("b", || Ok(()))]).is_empty());
    }

    #[test]
    fn the_report_keeps_failures_in_order() {
        let report = StartupReport::default();
        let make = |step: &str| StartupFailure {
            project_id: None,
            project_name: None,
            step: step.into(),
            reason: "r".into(),
        };
        report.record(make("a"));
        report.record(make("b"));
        assert_eq!(
            report
                .failures()
                .iter()
                .map(|f| f.step.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    #[test]
    fn a_failure_serialises_for_the_ui_in_camel_case() {
        let json = serde_json::to_value(StartupFailure {
            project_id: Some("p".into()),
            project_name: Some("Acme".into()),
            step: "skills".into(),
            reason: "r".into(),
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "projectId": "p", "projectName": "Acme", "step": "skills", "reason": "r" })
        );
    }
}
