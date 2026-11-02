//! The window manager: machine `window` (`app` feature).
//!
//! One process serves every open project, with one window per project (`p-<project hash>`) and,
//! when no project window is open, the `welcome` window. This module holds the rules and their
//! order; everything that touches the system (creating a window, a dialog, exiting) goes through
//! [`Shell`], so the tests drive the whole machine with a fake.
//!
//! | Event | Where |
//! |-------|-------|
//! | `developer.OPEN_PROJECT` | [`WindowManager::open_project`]: opens the project, or brings its window forward |
//! | `desktop.APP_STARTED` | [`WindowManager::restore`]: reopens the projects of the last session |
//! | `notification.CLICKED` | [`WindowManager::focus_project`]: the window comes forward showing [`ShowRequest`] |
//! | `developer.CLOSE_WINDOW` | [`WindowManager::close_window`] |
//! | `developer.QUIT_APP` | [`WindowManager::exit_requested`] and [`WindowManager::quit`] |
//! | `window.OPENED` / `window.CLOSED` | [`WindowJobs::window_opened`] / [`WindowJobs::window_closed`] |
//!
//! Closing a project window or quitting with jobs working asks a native confirmation first
//! ([`Shell::confirm`], `tauri-plugin-dialog`, which works with a hung webview); cancelling changes
//! nothing. After the confirmation the jobs are suspended (the future of
//! [`WindowJobs::window_closed`] completes), then the window closes or the app exits.
//!
//! The picture of a project ([`Pictures`]) is opened with its window and fed by the project's
//! watcher ([`Watch`]), which calls [`Pictures::rebuild`] once per batch of changes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, Weak};

use serde::Serialize;
use trama_core::paths;
use trama_core::project::clock::{now_instant, Clock};
use ts_rs::TS;

use super::jobs::{BoxFuture, CloseReason, NoJobs, OpenReason, WindowJobs, WindowOpened};
use super::launch;
use super::local_state::{LocalState, StateStore};
use super::project::{NoProject, ProjectHooks};
use super::registry::{decide_exit, label_of, ExitDecision, OpenProject, OpenStart, Registry};
use super::watch::{BatchSink, Watch, WatchGuard};
use crate::blocking;
use crate::error::{AppError, ErrorCode};
use crate::events::ProjectChanged;
use crate::pictures::{Pictures, Rebuild};
use crate::{log_info, log_warn};

/// What a window shows when it comes forward because of a notification (`notification.CLICKED`).
/// The UI switches to `view`, and to the conversation of `jobId` when there is one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShowRequest {
    /// The project hash (the window's label is `p-<hash>`).
    pub project_id: String,
    pub view: ShowView,
    /// The job whose conversation to open, in the Jobs view.
    pub job_id: Option<String>,
}

/// A view of the project window, named as the sidebar's `ViewId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum ShowView {
    Board,
    Diagram,
    Requirements,
    Specs,
    History,
    Jobs,
    General,
    Settings,
}

/// The text of a native confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmPrompt {
    pub title: String,
    pub message: String,
    /// The label of the button that goes on; the other one is `Cancel`.
    pub accept: String,
}

/// The count phrase, the pronoun for the jobs as an object, and the pronoun with its verb.
fn jobs_phrase(working: usize) -> (String, &'static str, &'static str) {
    if working == 1 {
        ("1 job is".to_owned(), "it", "it resumes")
    } else {
        (format!("{working} jobs are"), "them", "they resume")
    }
}

/// Closing the window of `project` while `working` jobs work.
pub fn close_prompt(project: &str, working: usize) -> ConfirmPrompt {
    let (count, them, _) = jobs_phrase(working);
    ConfirmPrompt {
        title: format!("Close {project}?"),
        message: format!("{count} working in {project}. Closing the window suspends {them}."),
        accept: "Suspend and Close".to_owned(),
    }
}

/// Quitting Weaver while `working` jobs work, in any project.
pub fn quit_prompt(working: usize) -> ConfirmPrompt {
    let (count, them, they) = jobs_phrase(working);
    ConfirmPrompt {
        title: "Quit Weaver?".to_owned(),
        message: format!(
            "{count} working. Quitting suspends {them}; {they} when Weaver starts again."
        ),
        accept: "Suspend and Quit".to_owned(),
    }
}

/// Everything the manager asks of the system.
pub trait Shell: Send + Sync + 'static {
    /// Creates and shows the window `label` for a project (title: the folder name).
    fn create_project_window(&self, label: &str, title: &str) -> Result<(), String>;
    /// Shows the welcome window: creates it, or brings the open one forward.
    fn show_welcome(&self) -> Result<(), String>;
    fn close_welcome(&self);
    /// Brings a window forward (restored, shown, focused). `false` when it does not exist.
    fn focus(&self, label: &str) -> bool;
    /// Brings any window forward. `false` when no window exists.
    fn focus_any(&self) -> bool;
    /// Closes a window for good, with no further event.
    fn destroy(&self, label: &str);
    /// How many windows exist (project windows and the welcome window).
    fn window_count(&self) -> usize;
    /// A native confirmation; `false` when cancelled or dismissed.
    fn confirm(&self, prompt: ConfirmPrompt) -> BoxFuture<bool>;
    /// Asks the app to exit with `code` (raising `RunEvent::ExitRequested` again).
    fn exit(&self, code: i32);
    fn emit_show(&self, label: &str, request: &ShowRequest);
    fn publish(&self, change: ProjectChanged);
    /// Writes the size and position of the open windows now.
    fn save_window_state(&self);
    /// A batch of file changes of a project, before its picture is rebuilt (the request editor
    /// follows `.trama/requests/`).
    fn files_changed(&self, _project_id: &str, _root: &Path, _batch: &Rebuild) {}
    /// Waits until the request editor of a project window saved what is pending (a window is closing).
    fn flush_before_close(&self, _project_id: &str) -> BoxFuture<()> {
        Box::pin(async {})
    }
}

/// A project that could not be opened without anyone asking at that moment (the restore at
/// start, a second launch): it is reported as `<project> could not open: <reason>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenFailure {
    pub path: PathBuf,
    /// The canonical path's project hash, or the one of the path as given when it cannot be
    /// resolved.
    pub project_id: String,
    pub name: String,
    pub reason: String,
}

pub type FailureSink = Arc<dyn Fn(OpenFailure) + Send + Sync>;

/// What [`WindowManager::open_project`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenOutcome {
    Opened,
    /// The project had its window (or was being opened): it came to the front.
    Focused,
}

/// What [`WindowManager::close_window`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseOutcome {
    Closed,
    /// The developer cancelled the confirmation: nothing changed.
    Cancelled,
    /// There is no such window, or it is already closing.
    Ignored,
}

pub struct WindowManager {
    shell: Arc<dyn Shell>,
    pictures: Arc<Pictures>,
    store: Arc<StateStore>,
    clock: Arc<dyn Clock>,
    watch: Arc<dyn Watch>,
    failures: FailureSink,
    jobs: RwLock<Arc<dyn WindowJobs>>,
    project: RwLock<Arc<dyn ProjectHooks>>,
    registry: Mutex<Registry>,
    watchers: Mutex<HashMap<String, WatchGuard>>,
    me: Weak<WindowManager>,
}

/// The name of a folder: its last component, or the whole path for a root (`/`).
pub fn folder_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.to_string_lossy().into_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The canonical folder of a project. The reason is the sentence the developer reads.
pub fn canonical_folder(root: &Path) -> Result<PathBuf, AppError> {
    let canonical = paths::canonical_root(root).map_err(|error| {
        let why = if error.kind() == std::io::ErrorKind::NotFound {
            "the folder does not exist".to_owned()
        } else {
            error.to_string()
        };
        AppError::new(ErrorCode::InvalidInput, why)
    })?;
    if !canonical.is_dir() {
        return Err(AppError::new(ErrorCode::InvalidInput, "it is not a folder"));
    }
    Ok(canonical)
}

impl WindowManager {
    pub fn new(
        shell: Arc<dyn Shell>,
        pictures: Arc<Pictures>,
        store: Arc<StateStore>,
        clock: Arc<dyn Clock>,
        watch: Arc<dyn Watch>,
        failures: FailureSink,
    ) -> Arc<Self> {
        Arc::new_cyclic(|me| Self {
            shell,
            pictures,
            store,
            clock,
            watch,
            failures,
            jobs: RwLock::new(Arc::new(NoJobs)),
            project: RwLock::new(Arc::new(NoProject)),
            registry: Mutex::new(Registry::new()),
            watchers: Mutex::new(HashMap::new()),
            me: me.clone(),
        })
    }

    /// Installs the jobs of the agent (see [`super::jobs`]).
    pub fn set_jobs(&self, jobs: Arc<dyn WindowJobs>) {
        *self.jobs.write().expect("jobs slot poisoned") = jobs;
    }

    /// Installs the project hooks (see [`super::project`]).
    pub fn set_project(&self, project: Arc<dyn ProjectHooks>) {
        *self.project.write().expect("project slot poisoned") = project;
    }

    fn project(&self) -> Arc<dyn ProjectHooks> {
        self.project.read().expect("project slot poisoned").clone()
    }

    fn jobs(&self) -> Arc<dyn WindowJobs> {
        self.jobs.read().expect("jobs slot poisoned").clone()
    }

    fn registry(&self) -> MutexGuard<'_, Registry> {
        self.registry.lock().expect("window registry poisoned")
    }

    fn arc(&self) -> Arc<WindowManager> {
        self.me
            .upgrade()
            .expect("the manager is alive while it is used")
    }

    /// The projects whose window is open, in opening order.
    pub fn open_projects(&self) -> Vec<OpenProject> {
        self.registry().projects().to_vec()
    }

    /// How many jobs work, in every project (what a quit confirms).
    pub fn working_jobs(&self) -> usize {
        let jobs = self.jobs();
        self.open_projects()
            .iter()
            .map(|project| jobs.working_jobs(&project.id))
            .sum()
    }

    /// A window got the focus (the most recent one comes forward for a second launch).
    pub fn on_focused(&self, label: &str) {
        self.registry().focused(label);
    }

    fn report(&self, failure: OpenFailure) {
        log_warn!("{} could not open: {}", failure.name, failure.reason);
        (self.failures)(failure);
    }

    // --- developer.OPEN_PROJECT ---------------------------------------------------------------

    /// `developer.OPEN_PROJECT`: opens the project in its window, or brings the window it has
    /// forward; a folder never gets a second window. The welcome window closes once the project
    /// window is open.
    pub async fn open_project(
        &self,
        root: PathBuf,
        reason: OpenReason,
    ) -> Result<OpenOutcome, AppError> {
        let given = root.clone();
        let canonical = blocking::disk(move || canonical_folder(&root)).await??;
        let id = paths::project_hash_of_canonical(&canonical);
        let start = self.registry().begin_open(&id);
        match start {
            OpenStart::AlreadyOpen => {
                self.focus_project(&id, None);
                return Ok(OpenOutcome::Focused);
            }
            OpenStart::InFlight => return Ok(OpenOutcome::Focused),
            OpenStart::Reserved => {}
        }
        let outcome = self.open_reserved(&id, &canonical, reason).await;
        if let Err(error) = &outcome {
            self.registry().abort_open(&id);
            log_warn!("{} could not open: {}", folder_name(&given), error.message);
            // While the last session is restored the welcome window waits for the end of it.
            if !self.registry().is_restoring() {
                self.ensure_a_window();
            }
        }
        outcome.map(|()| OpenOutcome::Opened)
    }

    async fn open_reserved(
        &self,
        id: &str,
        canonical: &Path,
        reason: OpenReason,
    ) -> Result<(), AppError> {
        let name = folder_name(canonical);
        self.pictures.open(canonical.to_path_buf()).await?;

        let guard = match self.watch.start(canonical, self.sink_for(id)) {
            Ok(guard) => guard,
            Err(why) => {
                self.pictures.close(id);
                return Err(AppError::internal(why));
            }
        };
        if let Err(why) = self.shell.create_project_window(&label_of(id), &name) {
            drop(guard);
            self.pictures.close(id);
            return Err(AppError::internal(format!(
                "the window could not be opened: {why}"
            )));
        }
        self.watchers
            .lock()
            .expect("watchers poisoned")
            .insert(id.to_owned(), guard);
        self.registry().finish_open(OpenProject {
            id: id.to_owned(),
            root: canonical.to_path_buf(),
            name: name.clone(),
        });

        let path_text = canonical.to_string_lossy().into_owned();
        match now_instant(self.clock.as_ref()) {
            Ok(opened) => {
                let store = self.store.clone();
                let stored = blocking::disk(move || {
                    store.update(|state| state.record_open(&path_text, &name, &opened))
                })
                .await;
                match stored {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => log_warn!("the recent projects were not stored: {error}"),
                    Err(error) => log_warn!("the recent projects were not stored: {error}"),
                }
            }
            Err(error) => log_warn!("the recent projects were not stored: {error}"),
        }
        log_info!("opened {}", folder_name(canonical));
        self.jobs().window_opened(&WindowOpened {
            project_id: id.to_owned(),
            root: canonical.to_path_buf(),
            reason,
        });
        self.project().window_opened(id, canonical);
        self.shell.close_welcome();
        Ok(())
    }

    /// One batch of changes of a project: the picture is rebuilt, and the UI told once.
    fn sink_for(&self, id: &str) -> BatchSink {
        let manager = self.me.clone();
        let id = id.to_owned();
        Arc::new(move |batch: Rebuild| {
            let Some(manager) = manager.upgrade() else {
                return;
            };
            let id = id.clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(picture) = manager.pictures.get(&id) {
                    manager.shell.files_changed(&id, &picture.root, &batch);
                }
                let published = manager.pictures.rebuild(&id, batch).await;
                match &published {
                    Ok(Some(revision)) => manager.shell.publish(revision.event()),
                    Ok(None) => {}
                    // The window closed while the batch waited: nothing to update.
                    Err(error) if error.code == ErrorCode::InvalidInput => {}
                    Err(error) => log_warn!("the picture of a project was not rebuilt: {error}"),
                }
                if let Ok(revision) = &published {
                    manager.project().batch_applied(&id, revision.as_ref());
                }
            });
        })
    }

    /// The welcome window is the window of Weaver when no project window is open.
    fn ensure_a_window(&self) {
        if self.registry().projects().is_empty() {
            if let Err(why) = self.shell.show_welcome() {
                log_warn!("the welcome window could not be shown: {why}");
            }
        }
    }

    // --- notification.CLICKED, a second launch, the Dock ---------------------------------------

    /// The window of the project comes to the front, showing `show` when given (what the
    /// notification was about). `false` when the project has no window.
    pub fn focus_project(&self, project_id: &str, show: Option<ShowRequest>) -> bool {
        let label = label_of(project_id);
        if !self.shell.focus(&label) {
            return false;
        }
        if let Some(request) = show {
            self.shell.emit_show(&label, &request);
        }
        true
    }

    /// The most recently focused window comes to the front; the welcome window shows when there
    /// is none.
    pub fn focus_recent(&self) {
        let order: Vec<String> = self.registry().focus_order().map(str::to_owned).collect();
        if order.iter().any(|label| self.shell.focus(label)) || self.shell.focus_any() {
            return;
        }
        if let Err(why) = self.shell.show_welcome() {
            log_warn!("the welcome window could not be shown: {why}");
        }
    }

    /// A second launch handed its arguments over: a folder is `developer.OPEN_PROJECT`, anything
    /// else brings the most recently used window forward.
    pub fn second_launch(&self, args: Vec<String>, cwd: PathBuf) {
        match launch::folder_argument(&args, &cwd) {
            Some(folder) => self.open_in_background(folder, OpenReason::Developer),
            None => self.focus_recent(),
        }
    }

    /// Opens a project without a caller waiting for the result: a failure is reported.
    pub fn open_in_background(&self, folder: PathBuf, reason: OpenReason) {
        let manager = self.arc();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = manager.open_project(folder.clone(), reason).await {
                manager.report(OpenFailure {
                    project_id: paths::project_hash_of_canonical(&folder),
                    name: folder_name(&folder),
                    path: folder,
                    reason: error.message,
                });
            }
        });
    }

    /// A click on the Dock icon with no window open (macOS): the welcome window shows.
    pub fn reopen(&self, has_visible_windows: bool) {
        if !has_visible_windows {
            self.focus_recent();
        }
    }

    // --- desktop.APP_STARTED --------------------------------------------------------------------

    /// `desktop.APP_STARTED`: opens the projects whose window was open when Weaver last quit, in
    /// their opening order (their jobs that were working resume), then `folder` when the launch
    /// named one. Without any project window the welcome window shows, with the recent projects.
    /// A project that cannot open is reported, and the others go on.
    pub async fn restore(&self, folder: Option<PathBuf>) {
        self.registry().set_restoring(true);
        let store = self.store.clone();
        let session = match blocking::disk(move || store.snapshot()).await {
            Ok(state) => state.last_session,
            Err(error) => {
                log_warn!("the last session could not be read: {error}");
                Vec::new()
            }
        };
        let wanted = session
            .into_iter()
            .map(|path| (PathBuf::from(path), OpenReason::AppStart))
            .chain(folder.map(|f| (f, OpenReason::Developer)));
        for (path, reason) in wanted {
            if let Err(error) = self.open_project(path.clone(), reason).await {
                self.report(OpenFailure {
                    project_id: paths::project_hash_of_canonical(&path),
                    name: folder_name(&path),
                    path,
                    reason: error.message,
                });
            }
        }
        self.registry().set_restoring(false);
        self.ensure_a_window();
    }

    // --- developer.CLOSE_WINDOW ------------------------------------------------------------------

    /// `developer.CLOSE_WINDOW`: with jobs working in the project a native confirmation comes
    /// first; cancelling changes nothing. Then the jobs are suspended and the window closes; the
    /// project does not reopen at the next start. Closing the last project window opens the
    /// welcome window.
    pub async fn close_window(&self, project_id: &str) -> CloseOutcome {
        let Some(project) = self.registry().begin_close(project_id) else {
            return CloseOutcome::Ignored;
        };
        let working = self.jobs().working_jobs(project_id);
        if working > 0
            && !self
                .shell
                .confirm(close_prompt(&project.name, working))
                .await
        {
            self.registry().cancel_close(project_id);
            return CloseOutcome::Cancelled;
        }
        // The welcome window opens before the last project window goes: the app never has no window.
        if self.registry().is_last_project(project_id) {
            if let Err(why) = self.shell.show_welcome() {
                log_warn!("the welcome window could not be shown: {why}");
            }
        }
        // The request being written is saved before the jobs end and the window goes: closing never sends it.
        self.shell.flush_before_close(project_id).await;
        self.jobs()
            .window_closed(project_id, CloseReason::WindowClosed)
            .await;
        self.shell.save_window_state();
        self.forget_project(project_id);
        self.remember_closed(&project).await;
        self.shell.destroy(&label_of(project_id));
        log_info!("closed {}", project.name);
        CloseOutcome::Closed
    }

    /// `WindowEvent::CloseRequested` of a project window (already prevented by the caller): the
    /// close goes on in the background, since the confirmation and the suspension of the jobs
    /// take time.
    pub fn close_requested(&self, project_id: &str) {
        let manager = self.arc();
        let id = project_id.to_owned();
        tauri::async_runtime::spawn(async move {
            manager.close_window(&id).await;
        });
    }

    /// Stops what the open window held: the watcher, the picture and its place in the registry.
    fn forget_project(&self, project_id: &str) {
        self.project().window_closed(project_id);
        self.watchers
            .lock()
            .expect("watchers poisoned")
            .remove(project_id);
        self.pictures.close(project_id);
        self.registry().remove(project_id);
    }

    /// The project leaves the session: it will not reopen at the next start.
    async fn remember_closed(&self, project: &OpenProject) {
        let store = self.store.clone();
        let path = project.root.to_string_lossy().into_owned();
        match blocking::disk(move || store.update(|state| state.record_close(&path))).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => log_warn!("the session was not stored: {error}"),
            Err(error) => log_warn!("the session was not stored: {error}"),
        }
    }

    /// A project window was destroyed without going through [`Self::close_window`] (the system
    /// closed it): it is closed like the developer closing it, minus the confirmation.
    pub fn on_destroyed(&self, label: &str) {
        self.registry().forget_label(label);
        let Some(id) = crate::pictures::project_of_label(label) else {
            return;
        };
        let Some(project) = self.registry().project(id).cloned() else {
            return;
        };
        let manager = self.arc();
        tauri::async_runtime::spawn(async move {
            let quitting = manager.registry().quit_state() != super::registry::QuitState::Idle;
            let reason = if quitting {
                CloseReason::AppQuit
            } else {
                CloseReason::WindowClosed
            };
            manager.jobs().window_closed(&project.id, reason).await;
            manager.forget_project(&project.id);
            if !quitting {
                manager.remember_closed(&project).await;
                manager.ensure_a_window();
            }
        });
    }

    // --- developer.QUIT_APP -----------------------------------------------------------------------

    /// `RunEvent::ExitRequested`: decides, and for a quit starts the quit path, which exits
    /// again when it is done. `windows_left` is how many windows still exist.
    pub fn exit_requested(&self, code: Option<i32>, windows_left: usize) -> ExitDecision {
        let state = self.registry().quit_state();
        let decision = decide_exit(state, code, windows_left, cfg!(target_os = "macos"));
        if let ExitDecision::Quit { code } = decision {
            let manager = self.arc();
            tauri::async_runtime::spawn(async move { manager.quit(code, true).await });
        }
        decision
    }

    /// `developer.QUIT_APP`: with jobs working in any project a native confirmation comes first
    /// (`confirm`); cancelling changes nothing. Then the session is written (the projects open
    /// now reopen at the next start), the jobs of every project are suspended (`window.CLOSED`)
    /// and the app exits with `code`. A shutdown that allows no confirmation passes
    /// `confirm = false`: the jobs are suspended without asking.
    pub async fn quit(&self, code: i32, confirm: bool) {
        if !self.registry().begin_quit() {
            return;
        }
        let working = self.working_jobs();
        if confirm && working > 0 && !self.shell.confirm(quit_prompt(working)).await {
            self.registry().cancel_quit();
            return;
        }
        let (projects, restoring) = {
            let registry = self.registry();
            (registry.projects().to_vec(), registry.is_restoring())
        };
        // Written first: a crash while the jobs are suspended still keeps the session.
        if !restoring {
            let store = self.store.clone();
            let open_now: Vec<String> = projects
                .iter()
                .map(|p| p.root.to_string_lossy().into_owned())
                .collect();
            match blocking::disk(move || store.update(|state| state.set_last_session(open_now)))
                .await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => log_warn!("the session was not stored: {error}"),
                Err(error) => log_warn!("the session was not stored: {error}"),
            }
        }
        let jobs = self.jobs();
        for project in &projects {
            self.shell.flush_before_close(&project.id).await;
        }
        let ending: Vec<_> = projects
            .iter()
            .map(|project| {
                tauri::async_runtime::spawn(jobs.window_closed(&project.id, CloseReason::AppQuit))
            })
            .collect();
        for handle in ending {
            if let Err(error) = handle.await {
                log_warn!("the jobs of a project were not suspended: {error}");
            }
        }
        self.watchers.lock().expect("watchers poisoned").clear();
        self.shell.save_window_state();
        self.registry().quit_ready();
        log_info!("quitting");
        self.shell.exit(code);
    }

    // --- the welcome window's data ----------------------------------------------------------------

    /// The recent projects, most recent first (blocking: call from the blocking pool).
    pub fn recent(&self) -> LocalState {
        self.store.snapshot()
    }

    /// Removes a project from the recent list; returns the list that remains.
    pub fn remove_recent(&self, path: &str) -> Result<LocalState, AppError> {
        self.store
            .update(|state| state.remove_recent(path))
            .map_err(AppError::internal)?;
        Ok(self.store.snapshot())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use trama_core::project::clock::ManualClock;

    use super::*;
    use crate::windows::watch::{Watch, WatchGuard};

    /// Everything the manager did to the system, in order.
    #[derive(Default)]
    struct FakeShell {
        log: Mutex<Vec<String>>,
        windows: Mutex<Vec<String>>,
        answers: Mutex<Vec<bool>>,
        prompts: Mutex<Vec<ConfirmPrompt>>,
        published: Mutex<Vec<ProjectChanged>>,
        fail_windows: Mutex<bool>,
    }

    impl FakeShell {
        fn note(&self, line: impl Into<String>) {
            self.log.lock().unwrap().push(line.into());
        }
        fn log(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }
        fn windows(&self) -> Vec<String> {
            self.windows.lock().unwrap().clone()
        }
        fn answer(&self, yes: bool) {
            self.answers.lock().unwrap().push(yes);
        }
    }

    impl Shell for FakeShell {
        fn create_project_window(&self, label: &str, title: &str) -> Result<(), String> {
            if *self.fail_windows.lock().unwrap() {
                return Err("no display".into());
            }
            self.note(format!("create {label} {title}"));
            self.windows.lock().unwrap().push(label.to_owned());
            Ok(())
        }
        fn show_welcome(&self) -> Result<(), String> {
            let mut windows = self.windows.lock().unwrap();
            if !windows.iter().any(|w| w == "welcome") {
                windows.push("welcome".to_owned());
                drop(windows);
                self.note("create welcome");
            } else {
                drop(windows);
                self.note("focus welcome");
            }
            Ok(())
        }
        fn close_welcome(&self) {
            let mut windows = self.windows.lock().unwrap();
            if let Some(at) = windows.iter().position(|w| w == "welcome") {
                windows.remove(at);
                drop(windows);
                self.note("close welcome");
            }
        }
        fn focus(&self, label: &str) -> bool {
            let exists = self.windows.lock().unwrap().iter().any(|w| w == label);
            if exists {
                self.note(format!("focus {label}"));
            }
            exists
        }
        fn focus_any(&self) -> bool {
            let first = self.windows.lock().unwrap().first().cloned();
            match first {
                Some(label) => {
                    self.note(format!("focus {label}"));
                    true
                }
                None => false,
            }
        }
        fn destroy(&self, label: &str) {
            self.windows.lock().unwrap().retain(|w| w != label);
            self.note(format!("destroy {label}"));
        }
        fn window_count(&self) -> usize {
            self.windows.lock().unwrap().len()
        }
        fn confirm(&self, prompt: ConfirmPrompt) -> BoxFuture<bool> {
            self.note(format!("confirm {}", prompt.title));
            self.prompts.lock().unwrap().push(prompt);
            let answer = self.answers.lock().unwrap().remove(0);
            Box::pin(async move { answer })
        }
        fn exit(&self, code: i32) {
            self.note(format!("exit {code}"));
        }
        fn emit_show(&self, label: &str, request: &ShowRequest) {
            self.note(format!(
                "show {label} {:?} {:?}",
                request.view, request.job_id
            ));
        }
        fn publish(&self, change: ProjectChanged) {
            self.published.lock().unwrap().push(change);
        }
        fn save_window_state(&self) {
            self.note("save window state");
        }
    }

    /// Keeps the sink of each watcher so a test can feed a batch.
    #[derive(Default)]
    struct FakeWatch {
        sinks: Mutex<HashMap<PathBuf, BatchSink>>,
        stopped: Arc<AtomicUsize>,
        fail: Mutex<bool>,
    }

    struct CountOnDrop(Arc<AtomicUsize>);
    impl Drop for CountOnDrop {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl Watch for FakeWatch {
        fn start(&self, root: &Path, sink: BatchSink) -> Result<WatchGuard, String> {
            if *self.fail.lock().unwrap() {
                return Err("cannot watch".into());
            }
            self.sinks.lock().unwrap().insert(root.to_path_buf(), sink);
            Ok(WatchGuard::new(CountOnDrop(self.stopped.clone())))
        }
    }

    /// Records what the jobs were told, and reports a number of working jobs per project.
    #[derive(Default)]
    struct FakeJobs {
        working: Mutex<HashMap<String, usize>>,
        events: Mutex<Vec<String>>,
    }

    impl FakeJobs {
        fn set_working(&self, id: &str, n: usize) {
            self.working.lock().unwrap().insert(id.to_owned(), n);
        }
        fn events(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }
    }

    impl WindowJobs for FakeJobs {
        fn working_jobs(&self, project_id: &str) -> usize {
            self.working
                .lock()
                .unwrap()
                .get(project_id)
                .copied()
                .unwrap_or(0)
        }
        fn window_opened(&self, event: &WindowOpened) {
            self.events
                .lock()
                .unwrap()
                .push(format!("opened {} {:?}", event.project_id, event.reason));
        }
        fn window_closed(&self, project_id: &str, reason: CloseReason) -> BoxFuture<()> {
            self.events
                .lock()
                .unwrap()
                .push(format!("closed {project_id} {reason:?}"));
            Box::pin(async {})
        }
    }

    struct Fixture {
        manager: Arc<WindowManager>,
        shell: Arc<FakeShell>,
        watch: Arc<FakeWatch>,
        jobs: Arc<FakeJobs>,
        store: Arc<StateStore>,
        failures: Arc<Mutex<Vec<OpenFailure>>>,
        data: tempfile::TempDir,
        _clock: Arc<ManualClock>,
    }

    fn fixture() -> Fixture {
        let data = tempfile::tempdir().unwrap();
        let shell = Arc::new(FakeShell::default());
        let watch = Arc::new(FakeWatch::default());
        let jobs = Arc::new(FakeJobs::default());
        let store = Arc::new(StateStore::in_dir(data.path()));
        let clock = Arc::new(ManualClock::at("2026-11-02T07:00:00.000Z"));
        let failures: Arc<Mutex<Vec<OpenFailure>>> = Arc::default();
        let sink = failures.clone();
        let manager = WindowManager::new(
            shell.clone(),
            Arc::new(Pictures::new()),
            store.clone(),
            clock.clone(),
            watch.clone(),
            Arc::new(move |failure| sink.lock().unwrap().push(failure)),
        );
        manager.set_jobs(jobs.clone());
        Fixture {
            manager,
            shell,
            watch,
            jobs,
            store,
            failures,
            data,
            _clock: clock,
        }
    }

    /// A project folder in a temporary folder; returns its canonical root and its hash.
    fn project(name: &str) -> (tempfile::TempDir, PathBuf, String) {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join(name);
        std::fs::create_dir_all(root.join(".trama")).unwrap();
        let canonical = paths::canonical_root(&root).unwrap();
        let id = paths::project_hash_of_canonical(&canonical);
        (parent, canonical, id)
    }

    #[tokio::test]
    async fn opening_a_folder_opens_its_window_once_and_a_second_open_brings_it_forward() {
        let f = fixture();
        let (_keep, root, id) = project("alpha");
        assert_eq!(
            f.manager
                .open_project(root.clone(), OpenReason::Developer)
                .await
                .unwrap(),
            OpenOutcome::Opened
        );
        assert_eq!(
            f.manager
                .open_project(root.clone(), OpenReason::Developer)
                .await
                .unwrap(),
            OpenOutcome::Focused
        );
        // A path that reaches the same folder another way is the same project.
        let roundabout = root.join(".trama").join("..");
        assert_eq!(
            f.manager
                .open_project(roundabout, OpenReason::Developer)
                .await
                .unwrap(),
            OpenOutcome::Focused
        );

        let label = label_of(&id);
        let created = f
            .shell
            .log()
            .iter()
            .filter(|l| l.starts_with("create p-"))
            .count();
        assert_eq!(
            created,
            1,
            "never two windows for a project: {:?}",
            f.shell.log()
        );
        assert_eq!(f.shell.windows(), std::slice::from_ref(&label));
        assert!(
            f.shell.log().contains(&format!("create {label} alpha")),
            "the title is the folder name"
        );
        assert_eq!(
            f.shell
                .log()
                .iter()
                .filter(|l| **l == format!("focus {label}"))
                .count(),
            2
        );
        assert_eq!(f.jobs.events(), [format!("opened {id} Developer")]);
        assert_eq!(f.watch.sinks.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn opening_a_project_closes_the_welcome_window_after_the_project_window_exists() {
        let f = fixture();
        f.shell.show_welcome().unwrap();
        let (_keep, root, id) = project("alpha");
        f.manager
            .open_project(root, OpenReason::Developer)
            .await
            .unwrap();
        let log = f.shell.log();
        let created = log.iter().position(|l| l.starts_with("create p-")).unwrap();
        let closed = log.iter().position(|l| l == "close welcome").unwrap();
        assert!(created < closed, "{log:?}");
        assert_eq!(f.shell.windows(), [label_of(&id)]);
    }

    #[tokio::test]
    async fn opening_records_the_recent_project_and_the_session() {
        let f = fixture();
        let (_keep, root, _id) = project("alpha");
        f.manager
            .open_project(root.clone(), OpenReason::Developer)
            .await
            .unwrap();
        let state = f.store.snapshot();
        let path = root.to_string_lossy().into_owned();
        assert_eq!(state.recent[0].path, path);
        assert_eq!(state.recent[0].name, "alpha");
        assert_eq!(state.recent[0].opened, "2026-11-02T07:00:00.000Z");
        assert_eq!(state.last_session, [path]);
        assert!(f.data.path().join("state.json").exists());
    }

    #[tokio::test]
    async fn a_folder_that_does_not_exist_or_is_a_file_is_refused_and_leaves_the_welcome_window() {
        let f = fixture();
        f.manager.restore(None).await;
        let dir = tempfile::tempdir().unwrap();
        let error = f
            .manager
            .open_project(dir.path().join("missing"), OpenReason::Developer)
            .await
            .unwrap_err();
        assert_eq!(error.message, "the folder does not exist");
        let file = dir.path().join("a.txt");
        std::fs::write(&file, "x").unwrap();
        let error = f
            .manager
            .open_project(file, OpenReason::Developer)
            .await
            .unwrap_err();
        assert_eq!(error.message, "it is not a folder");
        assert_eq!(
            f.shell.windows(),
            ["welcome"],
            "with no project window the welcome window shows"
        );
        assert!(f.manager.open_projects().is_empty());
    }

    #[tokio::test]
    async fn a_window_that_cannot_be_created_leaves_no_picture_nor_watcher_behind() {
        let f = fixture();
        *f.shell.fail_windows.lock().unwrap() = true;
        let (_keep, root, id) = project("alpha");
        let error = f
            .manager
            .open_project(root.clone(), OpenReason::Developer)
            .await
            .unwrap_err();
        assert!(error.message.contains("no display"), "{}", error.message);
        assert!(f.manager.pictures.get(&id).is_err());
        assert_eq!(f.watch.stopped.load(Ordering::SeqCst), 1);
        // The project can be opened again once the window can be created.
        *f.shell.fail_windows.lock().unwrap() = false;
        assert_eq!(
            f.manager
                .open_project(root, OpenReason::Developer)
                .await
                .unwrap(),
            OpenOutcome::Opened
        );
    }

    #[tokio::test]
    async fn a_watcher_that_cannot_start_fails_the_open_with_its_reason() {
        let f = fixture();
        *f.watch.fail.lock().unwrap() = true;
        let (_keep, root, id) = project("alpha");
        let error = f
            .manager
            .open_project(root, OpenReason::Developer)
            .await
            .unwrap_err();
        assert_eq!(error.message, "cannot watch");
        assert!(f.manager.pictures.get(&id).is_err());
        assert!(!f.shell.log().iter().any(|l| l.starts_with("create p-")));
    }

    #[tokio::test]
    async fn a_batch_from_the_watcher_rebuilds_the_picture_and_tells_the_ui_once() {
        let f = fixture();
        let (_keep, root, id) = project("alpha");
        f.manager
            .open_project(root.clone(), OpenReason::Developer)
            .await
            .unwrap();
        let before = f.manager.pictures.get(&id).unwrap().revision;

        let file = root.join(".trama").join("model").join("system.md");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "---\nname: Alpha\n---\n\n# Alpha\n").unwrap();
        let sink = f.watch.sinks.lock().unwrap().get(&root).unwrap().clone();
        sink(Rebuild::Paths(vec![file]));

        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            let published = f.shell.published.lock().unwrap().clone();
            if let Some(change) = published.first() {
                assert_eq!(published.len(), 1, "one batch, one event");
                assert_eq!(change.project_id, id);
                assert!(change.revision > before);
                assert!(
                    change.slices.iter().any(|s| s == "model"),
                    "{:?}",
                    change.slices
                );
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "no project-changed event"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    #[tokio::test]
    async fn a_notification_click_brings_the_window_forward_showing_what_it_was_about() {
        let f = fixture();
        let (_keep, root, id) = project("alpha");
        f.manager
            .open_project(root, OpenReason::Developer)
            .await
            .unwrap();
        let request = ShowRequest {
            project_id: id.clone(),
            view: ShowView::Jobs,
            job_id: Some("abcd1234".into()),
        };
        assert!(f.manager.focus_project(&id, Some(request)));
        let log = f.shell.log();
        assert_eq!(
            &log[log.len() - 2..],
            [
                format!("focus {}", label_of(&id)),
                format!("show {} Jobs Some(\"abcd1234\")", label_of(&id))
            ]
        );
        assert!(
            !f.manager.focus_project("0000000000000000", None),
            "a project without a window cannot come forward"
        );
    }

    #[tokio::test]
    async fn closing_a_window_without_jobs_working_closes_it_and_removes_it_from_the_session() {
        let f = fixture();
        let (_a, root_a, id_a) = project("alpha");
        let (_b, root_b, _id_b) = project("beta");
        f.manager
            .open_project(root_a.clone(), OpenReason::Developer)
            .await
            .unwrap();
        f.manager
            .open_project(root_b.clone(), OpenReason::Developer)
            .await
            .unwrap();

        assert_eq!(f.manager.close_window(&id_a).await, CloseOutcome::Closed);
        assert!(!f.shell.log().iter().any(|l| l.starts_with("confirm")));
        assert_eq!(
            f.store.snapshot().last_session,
            [root_b.to_string_lossy().into_owned()]
        );
        assert_eq!(
            f.store.snapshot().recent.len(),
            2,
            "closing a window keeps it in the recent projects"
        );
        assert!(f
            .jobs
            .events()
            .contains(&format!("closed {id_a} WindowClosed")));
        assert_eq!(
            f.shell.windows().len(),
            1,
            "no welcome window while another project window is open"
        );
        assert!(f.manager.pictures.get(&id_a).is_err());
        assert_eq!(f.watch.stopped.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn closing_the_last_project_window_opens_the_welcome_window_first() {
        let f = fixture();
        let (_a, root, id) = project("alpha");
        f.manager
            .open_project(root, OpenReason::Developer)
            .await
            .unwrap();
        assert_eq!(f.manager.close_window(&id).await, CloseOutcome::Closed);
        let log = f.shell.log();
        let welcome = log.iter().position(|l| l == "create welcome").unwrap();
        let destroyed = log
            .iter()
            .position(|l| l.starts_with("destroy p-"))
            .unwrap();
        assert!(welcome < destroyed, "{log:?}");
        assert_eq!(f.shell.windows(), ["welcome"]);
        assert!(f.store.snapshot().last_session.is_empty());
    }

    #[tokio::test]
    async fn closing_with_jobs_working_asks_the_number_and_cancel_changes_nothing() {
        let f = fixture();
        let (_a, root, id) = project("alpha");
        f.manager
            .open_project(root.clone(), OpenReason::Developer)
            .await
            .unwrap();
        f.jobs.set_working(&id, 2);
        f.shell.answer(false);
        assert_eq!(f.manager.close_window(&id).await, CloseOutcome::Cancelled);

        let prompt = f.shell.prompts.lock().unwrap()[0].clone();
        assert_eq!(prompt.title, "Close alpha?");
        assert!(
            prompt.message.starts_with("2 jobs are working in alpha."),
            "{}",
            prompt.message
        );
        assert_eq!(
            f.shell.windows(),
            [label_of(&id)],
            "the window is still there"
        );
        assert_eq!(
            f.jobs.events(),
            [format!("opened {id} Developer")],
            "no job was suspended"
        );
        assert_eq!(
            f.store.snapshot().last_session,
            [root.to_string_lossy().into_owned()]
        );
        assert!(f.manager.pictures.get(&id).is_ok());
        // A new request asks again, and confirming suspends the jobs, then closes.
        f.shell.answer(true);
        assert_eq!(f.manager.close_window(&id).await, CloseOutcome::Closed);
        let events = f.jobs.events();
        assert_eq!(events.last().unwrap(), &format!("closed {id} WindowClosed"));
        assert!(f.shell.windows().iter().all(|w| w != &label_of(&id)));
    }

    #[tokio::test]
    async fn one_job_is_worded_in_the_singular() {
        let close = close_prompt("alpha", 1);
        assert_eq!(
            close.message,
            "1 job is working in alpha. Closing the window suspends it."
        );
        let quit = quit_prompt(1);
        assert_eq!(
            quit.message,
            "1 job is working. Quitting suspends it; it resumes when Weaver starts again."
        );
        let quit = quit_prompt(3);
        assert_eq!(
            quit.message,
            "3 jobs are working. Quitting suspends them; they resume when Weaver starts again."
        );
        assert_eq!(quit.accept, "Suspend and Quit");
    }

    #[tokio::test]
    async fn quitting_writes_the_session_suspends_every_project_and_exits() {
        let f = fixture();
        let (_a, root_a, id_a) = project("alpha");
        let (_b, root_b, id_b) = project("beta");
        f.manager
            .open_project(root_a.clone(), OpenReason::Developer)
            .await
            .unwrap();
        f.manager
            .open_project(root_b.clone(), OpenReason::Developer)
            .await
            .unwrap();
        f.jobs.set_working(&id_a, 1);
        f.jobs.set_working(&id_b, 2);
        f.shell.answer(true);

        f.manager.quit(0, true).await;
        assert_eq!(
            f.shell.prompts.lock().unwrap()[0].message,
            "3 jobs are working. Quitting suspends them; they resume when Weaver starts again."
        );
        let events = f.jobs.events();
        assert!(
            events.contains(&format!("closed {id_a} AppQuit"))
                && events.contains(&format!("closed {id_b} AppQuit")),
            "{events:?}"
        );
        assert_eq!(
            f.store.snapshot().last_session,
            [
                root_a.to_string_lossy().into_owned(),
                root_b.to_string_lossy().into_owned()
            ],
            "quitting keeps the open windows in the session"
        );
        let log = f.shell.log();
        assert_eq!(log.last().unwrap(), "exit 0");
        assert!(
            f.shell.windows().len() == 2,
            "the windows are not destroyed one by one: the process exits"
        );
        // The exit that follows goes through.
        assert_eq!(f.manager.exit_requested(Some(0), 2), ExitDecision::Allow);
    }

    #[tokio::test]
    async fn cancelling_the_quit_changes_nothing_and_a_new_quit_asks_again() {
        let f = fixture();
        let (_a, root, id) = project("alpha");
        f.manager
            .open_project(root, OpenReason::Developer)
            .await
            .unwrap();
        f.jobs.set_working(&id, 1);
        f.shell.answer(false);
        f.manager.quit(0, true).await;
        assert!(!f.shell.log().iter().any(|l| l.starts_with("exit")));
        assert_eq!(f.jobs.events(), [format!("opened {id} Developer")]);
        f.shell.answer(true);
        f.manager.quit(0, true).await;
        assert_eq!(f.shell.log().last().unwrap(), "exit 0");
    }

    #[tokio::test]
    async fn a_shutdown_that_allows_no_confirmation_suspends_the_jobs_without_asking() {
        let f = fixture();
        let (_a, root, id) = project("alpha");
        f.manager
            .open_project(root, OpenReason::Developer)
            .await
            .unwrap();
        f.jobs.set_working(&id, 4);
        f.manager.quit(0, false).await;
        assert!(f.shell.prompts.lock().unwrap().is_empty());
        assert!(f.jobs.events().contains(&format!("closed {id} AppQuit")));
        assert_eq!(f.shell.log().last().unwrap(), "exit 0");
    }

    #[tokio::test]
    async fn a_quit_asked_with_a_code_exits_with_that_code() {
        let f = fixture();
        f.manager.quit(7, true).await;
        assert_eq!(f.shell.log().last().unwrap(), "exit 7");
    }

    #[tokio::test]
    async fn a_quit_while_the_session_is_being_restored_does_not_overwrite_it() {
        let f = fixture();
        let (_a, root_a, _) = project("alpha");
        let (_b, root_b, _) = project("beta");
        let (a, b) = (
            root_a.to_string_lossy().into_owned(),
            root_b.to_string_lossy().into_owned(),
        );
        f.store
            .update(|s| s.set_last_session(vec![a.clone(), b.clone()]))
            .unwrap();
        f.manager.registry().set_restoring(true);
        f.manager.quit(0, false).await;
        assert_eq!(f.store.snapshot().last_session, [a, b]);
    }

    #[tokio::test]
    async fn start_reopens_the_projects_of_the_last_session_in_order_and_resumes_their_jobs() {
        let f = fixture();
        let (_a, root_a, id_a) = project("alpha");
        let (_b, root_b, id_b) = project("beta");
        f.store
            .update(|s| {
                s.set_last_session(vec![
                    root_b.to_string_lossy().into_owned(),
                    root_a.to_string_lossy().into_owned(),
                ])
            })
            .unwrap();
        f.manager.restore(None).await;

        assert_eq!(f.shell.windows(), [label_of(&id_b), label_of(&id_a)]);
        assert_eq!(
            f.jobs.events(),
            [
                format!("opened {id_b} AppStart"),
                format!("opened {id_a} AppStart")
            ]
        );
        assert!(!f.shell.windows().iter().any(|w| w == "welcome"));
        assert!(f.failures.lock().unwrap().is_empty());
        assert!(!f.manager.registry().is_restoring());
    }

    #[tokio::test]
    async fn start_without_a_last_session_shows_the_welcome_window() {
        let f = fixture();
        f.manager.restore(None).await;
        assert_eq!(f.shell.windows(), ["welcome"]);
        assert!(f.jobs.events().is_empty());
    }

    #[tokio::test]
    async fn a_project_of_the_session_that_cannot_open_is_reported_and_the_others_open() {
        let f = fixture();
        let (_a, root_a, id_a) = project("alpha");
        let gone = root_a.parent().unwrap().join("gone");
        f.store
            .update(|s| {
                s.set_last_session(vec![
                    gone.to_string_lossy().into_owned(),
                    root_a.to_string_lossy().into_owned(),
                ])
            })
            .unwrap();
        f.manager.restore(None).await;
        assert_eq!(f.shell.windows(), [label_of(&id_a)]);
        let failures = f.failures.lock().unwrap().clone();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].name, "gone");
        assert_eq!(failures[0].reason, "the folder does not exist");
    }

    #[tokio::test]
    async fn when_no_project_of_the_session_opens_the_welcome_window_shows() {
        let f = fixture();
        let dir = tempfile::tempdir().unwrap();
        let gone = dir.path().join("gone").to_string_lossy().into_owned();
        f.store.update(|s| s.set_last_session(vec![gone])).unwrap();
        f.manager.restore(None).await;
        assert_eq!(f.shell.windows(), ["welcome"]);
        assert_eq!(f.failures.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_second_launch_with_a_folder_opens_or_focuses_and_without_one_focuses_the_recent_window(
    ) {
        let f = fixture();
        let (_a, root_a, id_a) = project("alpha");
        let (_b, root_b, id_b) = project("beta");
        let program = "weaver".to_owned();
        let launch = |root: &Path| vec![program.clone(), root.to_string_lossy().into_owned()];

        f.manager.second_launch(launch(&root_a), PathBuf::from("/"));
        f.manager.second_launch(launch(&root_b), PathBuf::from("/"));
        // The opens run in the background: wait for both windows.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while f.shell.windows().len() < 2 {
            assert!(
                std::time::Instant::now() < deadline,
                "the windows did not open"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        f.manager.second_launch(launch(&root_a), PathBuf::from("/"));
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !f
            .shell
            .log()
            .iter()
            .any(|l| *l == format!("focus {}", label_of(&id_a)))
        {
            assert!(
                std::time::Instant::now() < deadline,
                "the window of alpha did not come forward"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            f.shell.windows().len(),
            2,
            "never two windows for a project"
        );

        // Without a folder the most recently focused window comes forward.
        f.manager.on_focused(&label_of(&id_b));
        f.manager.second_launch(vec![program], PathBuf::from("/"));
        assert_eq!(
            f.shell.log().last().unwrap(),
            &format!("focus {}", label_of(&id_b))
        );
    }

    #[tokio::test]
    async fn the_dock_icon_with_no_window_shows_the_welcome_window() {
        let f = fixture();
        f.manager.reopen(false);
        assert_eq!(f.shell.windows(), ["welcome"]);
        f.manager.reopen(true);
        assert_eq!(f.shell.windows(), ["welcome"]);
    }

    #[tokio::test]
    async fn a_window_destroyed_by_the_system_is_closed_like_the_developer_closing_it() {
        let f = fixture();
        let (_a, root, id) = project("alpha");
        f.manager
            .open_project(root.clone(), OpenReason::Developer)
            .await
            .unwrap();
        f.shell.destroy(&label_of(&id));
        f.manager.on_destroyed(&label_of(&id));
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !f.manager.open_projects().is_empty()
            || !f.shell.windows().iter().any(|w| w == "welcome")
        {
            assert!(
                std::time::Instant::now() < deadline,
                "the project was not closed"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(f
            .jobs
            .events()
            .contains(&format!("closed {id} WindowClosed")));
        assert!(f.store.snapshot().last_session.is_empty());
    }

    #[test]
    fn project_folders_are_named_by_their_last_component() {
        assert_eq!(folder_name(Path::new("/work/my project")), "my project");
        assert_eq!(folder_name(Path::new("/")), "/");
    }
}
