//! The Machine `project` in the app (`requirements/tech#project`, `requirements/ui#project`): whether
//! a folder is a project and whether its Model is empty, the files Weaver installs for the agent
//! and the commands of Get Started, `Import from Code` and `Update skills`.
//!
//! The rules are in `trama_core::project_files` (what is written, the one report, at most one check
//! at a time); this module keeps one small record per open project, runs the work in the project's
//! queue and tells the window with one `project-state` event per change:
//!
//! | Event of the Machine | Where |
//! |----------------------|-------|
//! | `developer.INITIALISE_PROJECT` | [`ProjectSetup::initialise`] |
//! | `developer.UPDATE_SKILLS` | [`ProjectSetup::update_skills`] |
//! | window opens in `Has a Model` | [`ProjectHooks::window_opened`]: the check runs in the background |
//! | `workspace.FILES_CHANGED`, `PROJECT_FOLDER_APPEARED`, `PROJECT_FOLDER_REMOVED` | [`ProjectHooks::batch_applied`] |
//! | `proposal.APPROVED` | the approval regenerates the Requirement skills itself (`transitions::approve_with`) |
//!
//! The state is never stored apart from the files: it is worked out from whether `.trama/` exists
//! and from what the project's picture holds, after every batch of changes.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::oneshot;
use trama_core::project::image::ProjectImage;
use trama_core::project_files::folder_tree::{self, TreeEntry};
use trama_core::project_files::schedule::{Runs, Want};
use trama_core::project_files::{
    self, has_trama, InitialiseError, ProjectState, SyncOptions, SyncReport,
};
use ts_rs::TS;

use crate::blocking;
use crate::error::{AppError, ErrorCode};
use crate::events::EventName;
use crate::jobs::starts::Ctx;
use crate::jobs::Jobs;
use crate::pictures::{Rebuild, Revision, SLICE_REQUIREMENTS};
use crate::state::AppState;
use crate::windows::jobs::BoxFuture;
use crate::windows::project::ProjectHooks;
use crate::windows::WindowManager;
use crate::{log_info, log_warn};

/// How long after the last change under `.trama/requirements/` the Requirement skills are generated again.
pub const REGENERATION_DELAY: Duration = Duration::from_secs(1);

/// What the check of the installed files did when the window opened, as the window shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CheckOutcome {
    /// Nothing to say: everything was in place, or the failure is gone.
    None,
    /// The check wrote files: the window says so once.
    Updated { files: u32 },
    /// The check failed: the window shows `Retry` at every opening until it succeeds.
    Failed { reason: String },
}

/// The opening check of a project window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillsCheck {
    /// How many opening checks ended; the window tells an outcome once per number.
    #[ts(type = "number")]
    pub opening: u64,
    pub outcome: CheckOutcome,
}

impl SkillsCheck {
    fn none() -> Self {
        Self {
            opening: 0,
            outcome: CheckOutcome::None,
        }
    }

    /// What the check says after a run of `want` ended with `result`.
    fn after(&self, want: Want, result: &Result<SyncReport, String>) -> SkillsCheck {
        match want {
            Want::OpeningCheck => SkillsCheck {
                opening: self.opening + 1,
                outcome: match result {
                    Ok(report) if report.is_unchanged() => CheckOutcome::None,
                    Ok(report) => CheckOutcome::Updated {
                        files: u32::try_from(report.changes.len()).unwrap_or(u32::MAX),
                    },
                    Err(reason) => CheckOutcome::Failed {
                        reason: reason.clone(),
                    },
                },
            },
            // `Update skills` that succeeds ends a failure of the opening check.
            Want::UpdateSkills if result.is_ok() => SkillsCheck {
                opening: self.opening,
                outcome: CheckOutcome::None,
            },
            Want::UpdateSkills | Want::RequirementSkills => self.clone(),
        }
    }
}

/// What the window knows of the project: the state, the files to install and the opening check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatus {
    pub project_id: String,
    /// Grows with every change of this status; a window drops an older one.
    #[ts(type = "number")]
    pub revision: u64,
    pub state: ProjectState,
    /// The agent files `Initialise` installs, relative to the project folder.
    pub files: Vec<String>,
    /// The folder holds names not starting with a dot (the warning of `Start from Scratch`).
    pub folder_has_files: bool,
    pub check: SkillsCheck,
}

/// What `Initialise` answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InitialiseResult {
    /// The agent files that exist with other content, when overwriting them was not confirmed:
    /// nothing was created. Empty when the project was initialised.
    pub conflicts: Vec<String>,
}

/// What the module needs of the app: kept behind a trait so that tests drive it with a folder and no window.
pub trait Environment: Send + Sync + 'static {
    /// The root and the current picture of an open project.
    fn picture(&self, project_id: &str) -> Option<(PathBuf, Arc<ProjectImage>)>;
    /// Runs `work` in the project's queue, under the project lock.
    fn locked(
        &self,
        project_id: &str,
        work: Box<dyn FnOnce() + Send>,
    ) -> BoxFuture<Result<(), String>>;
    /// Reads the project again and tells the windows what changed (after `.trama/` appeared).
    fn rebuilt(&self, project_id: &str) -> BoxFuture<()>;
    fn emit(&self, status: &ProjectStatus);
}

/// Runs `work` in the project's queue and returns what it returned.
async fn locked<T: Send + 'static>(
    env: &dyn Environment,
    project_id: &str,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    let (send, receive) = oneshot::channel();
    env.locked(
        project_id,
        Box::new(move || {
            let _ = send.send(work());
        }),
    )
    .await?;
    receive.await.map_err(|_| "the work was dropped".to_owned())
}

type Waiter = oneshot::Sender<Result<SyncReport, String>>;

struct Entry {
    root: PathBuf,
    revision: u64,
    state: ProjectState,
    folder_has_files: bool,
    check: SkillsCheck,
    runs: Runs,
    /// Requests waiting for the next run, and those the running one serves.
    waiting: Vec<Waiter>,
    serving: Vec<Waiter>,
    /// Grows with every change under `.trama/requirements/`: only the last one regenerates.
    regeneration: u64,
}

impl Entry {
    fn status(&self, project_id: &str) -> ProjectStatus {
        ProjectStatus {
            project_id: project_id.to_owned(),
            revision: self.revision,
            state: self.state,
            files: project_files::initialise::installed_paths(),
            folder_has_files: self.folder_has_files,
            check: self.check.clone(),
        }
    }
}

/// The folder holds a name not starting with a dot.
fn folder_has_files(root: &Path) -> bool {
    std::fs::read_dir(root).is_ok_and(|listing| {
        listing
            .filter_map(Result::ok)
            .any(|item| !item.file_name().to_string_lossy().starts_with('.'))
    })
}

/// The state of the folder and whether it holds files of its own, from the disk and the picture.
fn measure(root: &Path, image: &ProjectImage) -> (ProjectState, bool) {
    (
        ProjectState::of(has_trama(root), image.model()),
        folder_has_files(root),
    )
}

/// What a run of `want` does to the project at `root`, in the project's queue.
fn run_work(root: &Path, want: Want) -> Result<SyncReport, String> {
    if !has_trama(root) {
        return Err("the folder is not a project yet: initialise it first".to_owned());
    }
    let options = match want {
        Want::RequirementSkills => SyncOptions::requirement_skills(),
        Want::OpeningCheck => SyncOptions::everything(),
        Want::UpdateSkills => SyncOptions::update_skills(),
    };
    project_files::sync(root, options).map_err(|error| error.0)
}

/// The record of each open project.
pub struct ProjectSetup {
    env: Arc<dyn Environment>,
    delay: Duration,
    entries: Mutex<std::collections::HashMap<String, Entry>>,
    me: Weak<ProjectSetup>,
}

impl ProjectSetup {
    pub fn new(env: Arc<dyn Environment>) -> Arc<Self> {
        Self::with_delay(env, REGENERATION_DELAY)
    }

    /// [`ProjectSetup::new`] with another delay for the regeneration after a change of the Requirements (tests).
    pub fn with_delay(env: Arc<dyn Environment>, delay: Duration) -> Arc<Self> {
        Arc::new_cyclic(|me| Self {
            env,
            delay,
            entries: Mutex::new(Default::default()),
            me: me.clone(),
        })
    }

    fn entries(&self) -> MutexGuard<'_, std::collections::HashMap<String, Entry>> {
        self.entries.lock().expect("project records poisoned")
    }

    fn arc(&self) -> Option<Arc<ProjectSetup>> {
        self.me.upgrade()
    }

    fn not_open(project_id: &str) -> AppError {
        AppError::new(
            ErrorCode::InvalidInput,
            format!("the project `{project_id}` is not open"),
        )
    }

    /// The status of an open project, for a window that just loaded.
    pub async fn status(&self, project_id: &str) -> Result<ProjectStatus, AppError> {
        if let Some(entry) = self.entries().get(project_id) {
            return Ok(entry.status(project_id));
        }
        // The window asked before the record was made: the answer is worked out without keeping it.
        let (root, image) = self
            .env
            .picture(project_id)
            .ok_or_else(|| Self::not_open(project_id))?;
        let (state, folder_has_files) = blocking::disk(move || measure(&root, &image)).await?;
        let entry = Entry {
            root: PathBuf::new(),
            revision: 0,
            state,
            folder_has_files,
            check: SkillsCheck::none(),
            runs: Runs::default(),
            waiting: Vec::new(),
            serving: Vec::new(),
            regeneration: 0,
        };
        Ok(entry.status(project_id))
    }

    /// The state and the folder look again after a batch; the window is told when they differ.
    async fn refresh(&self, project_id: &str) {
        let Some((root, image)) = self.env.picture(project_id) else {
            return;
        };
        let Ok((state, has_files)) = blocking::disk(move || measure(&root, &image)).await else {
            return;
        };
        let status = {
            let mut entries = self.entries();
            let Some(entry) = entries.get_mut(project_id) else {
                return;
            };
            if entry.state == state && entry.folder_has_files == has_files {
                return;
            }
            entry.state = state;
            entry.folder_has_files = has_files;
            entry.revision += 1;
            entry.status(project_id)
        };
        self.env.emit(&status);
    }

    /// Asks for a run. The returned receiver hears the report of the run that serves the request
    /// (`waited` only); `None` when the project has no record.
    fn request(
        &self,
        project_id: &str,
        want: Want,
        waited: bool,
    ) -> Option<oneshot::Receiver<Result<SyncReport, String>>> {
        let (send, receive) = oneshot::channel();
        let start = {
            let mut entries = self.entries();
            let entry = entries.get_mut(project_id)?;
            if waited {
                entry.waiting.push(send);
            }
            let start = entry.runs.request(want);
            if start.is_some() {
                entry.serving = std::mem::take(&mut entry.waiting);
            }
            start
        };
        if let (Some(first), Some(me)) = (start, self.arc()) {
            let id = project_id.to_owned();
            tauri::async_runtime::spawn(async move { me.serve(id, first).await });
        }
        Some(receive)
    }

    /// Runs the check, then the one more run that the requests made meanwhile coalesced into.
    async fn serve(self: Arc<Self>, project_id: String, first: Want) {
        let mut current = Some(first);
        while let Some(want) = current.take() {
            let Some(root) = self
                .entries()
                .get(&project_id)
                .map(|entry| entry.root.clone())
            else {
                return;
            };
            let result = match locked(self.env.as_ref(), &project_id, move || {
                run_work(&root, want)
            })
            .await
            {
                Ok(result) => result,
                Err(reason) => Err(reason),
            };
            match &result {
                Ok(report) if !report.is_unchanged() => {
                    log_info!("{} installed files written", report.changes.len())
                }
                Ok(_) => {}
                Err(reason) => log_warn!("the installed files could not be written: {reason}"),
            }
            let (waiters, status, next) = {
                let mut entries = self.entries();
                // The window closed meanwhile: the waiters are dropped and hear that.
                let Some(entry) = entries.get_mut(&project_id) else {
                    return;
                };
                let waiters = std::mem::take(&mut entry.serving);
                let check = entry.check.after(want, &result);
                let status = (check != entry.check).then(|| {
                    entry.check = check;
                    entry.revision += 1;
                    entry.status(&project_id)
                });
                let next = entry.runs.finished();
                if next.is_some() {
                    entry.serving = std::mem::take(&mut entry.waiting);
                }
                (waiters, status, next)
            };
            for waiter in waiters {
                let _ = waiter.send(result.clone());
            }
            if let Some(status) = status {
                self.env.emit(&status);
            }
            current = next;
        }
    }

    /// `developer.UPDATE_SKILLS`: writes again every installed file that differs, creates the
    /// starting Areas when the Requirements are missing, and reports what it did.
    pub async fn update_skills(&self, project_id: &str) -> Result<SyncReport, AppError> {
        let receive = self
            .request(project_id, Want::UpdateSkills, true)
            .ok_or_else(|| Self::not_open(project_id))?;
        match receive.await {
            Ok(Ok(report)) => Ok(report),
            Ok(Err(reason)) => Err(AppError::new(ErrorCode::Internal, reason)),
            Err(_) => Err(AppError::new(ErrorCode::InvalidInput, "the window closed")),
        }
    }

    /// `developer.INITIALISE_PROJECT`. Conflicts are an answer, not an error: nothing was created.
    pub async fn initialise(
        &self,
        project_id: &str,
        force: bool,
    ) -> Result<InitialiseResult, AppError> {
        let (root, _) = self
            .env
            .picture(project_id)
            .ok_or_else(|| Self::not_open(project_id))?;
        let outcome = locked(self.env.as_ref(), project_id, move || {
            project_files::initialise(&root, force)
        })
        .await
        .map_err(|reason| AppError::new(ErrorCode::ProjectBusy, reason))?;
        match outcome {
            Ok(done) => {
                log_info!(
                    "a project was initialised with {} agent files",
                    done.agent_files.len()
                );
                self.env.rebuilt(project_id).await;
                self.refresh(project_id).await;
                Ok(InitialiseResult {
                    conflicts: Vec::new(),
                })
            }
            Err(InitialiseError::Conflicts(conflicts)) => Ok(InitialiseResult { conflicts }),
            Err(InitialiseError::Exists) => {
                self.env.rebuilt(project_id).await;
                self.refresh(project_id).await;
                Err(AppError::new(
                    ErrorCode::Refused,
                    "the folder is already a project",
                ))
            }
            Err(InitialiseError::Failed(reason)) => Err(AppError::new(ErrorCode::Internal, reason)),
        }
    }

    /// The Requirements changed: the Requirement skills are generated again 1 s after the last change.
    fn regenerate_later(&self, project_id: &str) {
        let generation = {
            let mut entries = self.entries();
            let Some(entry) = entries.get_mut(project_id) else {
                return;
            };
            entry.regeneration += 1;
            entry.regeneration
        };
        let Some(me) = self.arc() else { return };
        let id = project_id.to_owned();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(me.delay).await;
            let current = me.entries().get(&id).is_some_and(|entry| {
                entry.regeneration == generation && entry.state != ProjectState::NotInitialised
            });
            if current {
                me.request(&id, Want::RequirementSkills, false);
            }
        });
    }

    async fn opened(self: Arc<Self>, project_id: String, root: PathBuf) {
        let Some((_, image)) = self.env.picture(&project_id) else {
            return;
        };
        let measuring = root.clone();
        let Ok((state, has_files)) = blocking::disk(move || measure(&measuring, &image)).await
        else {
            return;
        };
        let status = {
            let mut entries = self.entries();
            let entry = Entry {
                root,
                revision: 1,
                state,
                folder_has_files: has_files,
                check: SkillsCheck::none(),
                runs: Runs::default(),
                waiting: Vec::new(),
                serving: Vec::new(),
                regeneration: 0,
            };
            let status = entry.status(&project_id);
            entries.insert(project_id.clone(), entry);
            status
        };
        self.env.emit(&status);
        if state == ProjectState::HasModel {
            self.request(&project_id, Want::OpeningCheck, false);
        }
    }

    async fn batch(self: Arc<Self>, project_id: String, requirements_changed: bool) {
        self.refresh(&project_id).await;
        if requirements_changed {
            self.regenerate_later(&project_id);
        }
    }
}

impl ProjectHooks for ProjectSetup {
    fn window_opened(&self, project_id: &str, root: &Path) {
        if let Some(me) = self.arc() {
            let (id, root) = (project_id.to_owned(), root.to_path_buf());
            tauri::async_runtime::spawn(async move { me.opened(id, root).await });
        }
    }

    fn batch_applied(&self, project_id: &str, revision: Option<&Revision>) {
        if let Some(me) = self.arc() {
            let requirements =
                revision.is_some_and(|r| r.slices.iter().any(|slice| slice == SLICE_REQUIREMENTS));
            let id = project_id.to_owned();
            tauri::async_runtime::spawn(async move { me.batch(id, requirements).await });
        }
    }

    fn window_closed(&self, project_id: &str) {
        self.entries().remove(project_id);
    }
}

// --- the real environment -----------------------------------------------------------------------

/// The app: its pictures, its queues and its windows.
pub struct TauriEnvironment {
    app: AppHandle,
}

impl TauriEnvironment {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl Environment for TauriEnvironment {
    fn picture(&self, project_id: &str) -> Option<(PathBuf, Arc<ProjectImage>)> {
        self.app
            .state::<AppState>()
            .pictures
            .get(project_id)
            .ok()
            .map(|picture| (picture.root, picture.image))
    }

    fn locked(
        &self,
        project_id: &str,
        work: Box<dyn FnOnce() + Send>,
    ) -> BoxFuture<Result<(), String>> {
        let app = self.app.clone();
        let id = project_id.to_owned();
        Box::pin(async move {
            app.state::<AppState>()
                .mutations
                .run(&id, move || {
                    work();
                    Ok(())
                })
                .await
                .map_err(|error| error.message)
        })
    }

    fn rebuilt(&self, project_id: &str) -> BoxFuture<()> {
        let app = self.app.clone();
        let id = project_id.to_owned();
        Box::pin(async move {
            match app
                .state::<AppState>()
                .pictures
                .rebuild(&id, Rebuild::All)
                .await
            {
                Ok(Some(revision)) => {
                    let _ = crate::events::emit_project_changed(&app, &revision.event());
                }
                Ok(None) => {}
                Err(error) => log_warn!("the picture of a project was not rebuilt: {error}"),
            }
        })
    }

    fn emit(&self, status: &ProjectStatus) {
        if let Err(error) = self.app.emit(EventName::ProjectState.as_str(), status) {
            log_warn!("the windows were not told about a project: {error}");
        }
    }
}

/// Creates the record keeper and manages it, for the commands.
pub fn install(app: &AppHandle) -> Arc<ProjectSetup> {
    let setup = ProjectSetup::new(Arc::new(TauriEnvironment::new(app.clone())));
    app.manage(setup.clone());
    setup
}

/// Creates the record keeper and hooks it to the windows: it hears of every window that opens or
/// closes and of every batch of changes, from the first window. Call right after
/// [`crate::jobs::install_windows`], before the projects are restored.
pub fn install_windows(app: &AppHandle, manager: &Arc<WindowManager>) {
    manager.set_project(install(app));
}

// --- commands ----------------------------------------------------------------------------------------

/// The state of the project and what its window needs for Get Started and the opening check;
/// later changes arrive as `project-state`.
#[tauri::command]
pub async fn project_status(
    setup: State<'_, Arc<ProjectSetup>>,
    project_id: String,
) -> Result<ProjectStatus, AppError> {
    setup.status(&project_id).await
}

/// `developer.INITIALISE_PROJECT`; `force` confirms the overwrite of agent files that differ.
#[tauri::command]
pub async fn initialise_project(
    setup: State<'_, Arc<ProjectSetup>>,
    project_id: String,
    force: bool,
) -> Result<InitialiseResult, AppError> {
    let setup = setup.inner().clone();
    setup.initialise(&project_id, force).await
}

/// `developer.UPDATE_SKILLS`: the same structured report the command line prints.
#[tauri::command]
pub async fn update_skills(
    setup: State<'_, Arc<ProjectSetup>>,
    project_id: String,
) -> Result<SyncReport, AppError> {
    let setup = setup.inner().clone();
    setup.update_skills(&project_id).await
}

/// The entries of one folder of the project for the tree of `Import from Code`: `path` is relative
/// to the project folder, empty for the project folder itself.
#[tauri::command]
pub async fn project_folder(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<Vec<TreeEntry>, AppError> {
    let root = state.pictures.get(&project_id)?.root;
    blocking::disk(move || {
        folder_tree::list(&root, &path)
            .map_err(|reason| AppError::new(ErrorCode::InvalidInput, reason))
    })
    .await?
}

/// `developer.IMPORT_CODE`: starts the import job, leaving out `excluded` (paths relative to the
/// project folder, with `/`). Refused unless the Model is empty.
#[tauri::command]
pub async fn import_code(
    jobs: State<'_, Arc<Jobs>>,
    project_id: String,
    excluded: Vec<String>,
) -> Result<(), AppError> {
    let jobs = jobs.inner().clone();
    let worker = jobs.clone();
    let project = project_id.clone();
    jobs.host()
        .mutate(
            &project_id,
            Box::new(move |root| {
                let image = ProjectImage::load(root);
                if !image.model().is_empty() {
                    return Err("Could not start the import: the Model already holds a Feature or an Actor.".to_owned());
                }
                worker
                    .start_import(&Ctx { project_id: &project, root, image: &image }, excluded)
                    .map(|_| None)
                    .map_err(|refusal| format!("Could not start the import: {refusal}."))
            }),
        )
        .await
        .map_err(|message| AppError::new(ErrorCode::Refused, message))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;

    use tokio::sync::Notify;
    use trama_core::project_files::initialise;

    use super::*;

    const ID: &str = "0123456789abcdef";

    /// A folder with its picture read from disk at each question, and the statuses it announced.
    struct FakeEnvironment {
        root: PathBuf,
        emitted: Mutex<Vec<ProjectStatus>>,
        changed: Arc<Notify>,
    }

    impl FakeEnvironment {
        fn new(root: &Path) -> Arc<Self> {
            Arc::new(Self {
                root: root.to_path_buf(),
                emitted: Mutex::new(Vec::new()),
                changed: Arc::new(Notify::new()),
            })
        }

        fn last(&self) -> Option<ProjectStatus> {
            self.emitted.lock().unwrap().last().cloned()
        }
    }

    impl Environment for FakeEnvironment {
        fn picture(&self, _project_id: &str) -> Option<(PathBuf, Arc<ProjectImage>)> {
            Some((self.root.clone(), Arc::new(ProjectImage::load(&self.root))))
        }

        fn locked(
            &self,
            _project_id: &str,
            work: Box<dyn FnOnce() + Send>,
        ) -> BoxFuture<Result<(), String>> {
            let changed = self.changed.clone();
            Box::pin(async move {
                let done = blocking::disk(work).await.map_err(|error| error.message);
                // Whoever waits on the files looks again.
                changed.notify_waiters();
                done
            })
        }

        fn rebuilt(&self, _project_id: &str) -> BoxFuture<()> {
            Box::pin(async {})
        }

        fn emit(&self, status: &ProjectStatus) {
            self.emitted.lock().unwrap().push(status.clone());
            self.changed.notify_waiters();
        }
    }

    /// Waits, without a timer of its own, until `condition` holds for what was announced.
    async fn until(env: &FakeEnvironment, condition: impl Fn(&[ProjectStatus]) -> bool) {
        let waiting = async {
            loop {
                let notified = env.changed.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                if condition(&env.emitted.lock().unwrap()) {
                    return;
                }
                notified.await;
            }
        };
        tokio::time::timeout(Duration::from_secs(20), waiting)
            .await
            .expect("the status never came");
    }

    fn feature(root: &Path) {
        let path = root.join(".trama/model/shop/feature.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "# Shop\n\nSells things.\n").unwrap();
    }

    fn project_with_a_model(root: &Path) {
        initialise(root, false).unwrap();
        feature(root);
    }

    fn setup(env: &Arc<FakeEnvironment>) -> Arc<ProjectSetup> {
        ProjectSetup::with_delay(env.clone(), Duration::from_millis(20))
    }

    fn open(setup: &ProjectSetup, root: &Path) {
        setup.window_opened(ID, root);
    }

    #[tokio::test]
    async fn a_folder_without_trama_offers_the_files_and_initialising_it_makes_an_empty_model() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.c"), "int main;").unwrap();
        let env = FakeEnvironment::new(dir.path());
        let setup = setup(&env);
        open(&setup, dir.path());
        until(&env, |all| !all.is_empty()).await;
        let first = env.last().unwrap();
        assert_eq!(
            (first.state, first.folder_has_files),
            (ProjectState::NotInitialised, true)
        );
        assert_eq!(first.files, initialise::installed_paths());
        assert!(first.files.contains(&".omp/WATCHDOG.yml".to_owned()));

        let done = setup.initialise(ID, false).await.unwrap();
        assert!(done.conflicts.is_empty());
        until(&env, |all| {
            all.last()
                .is_some_and(|s| s.state == ProjectState::EmptyModel)
        })
        .await;
        assert!(dir.path().join(".trama/model/system.md").is_file());
        assert!(dir
            .path()
            .join(".agents/skills/trama-design/SKILL.md")
            .is_file());
        // An Empty Model is not checked when the window opens, and nothing is reported.
        assert_eq!(env.last().unwrap().check, SkillsCheck::none());
    }

    #[tokio::test]
    async fn differing_agent_files_are_listed_and_nothing_is_created_until_the_overwrite_is_confirmed(
    ) {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".omp")).unwrap();
        fs::write(dir.path().join(".omp/WATCHDOG.yml"), "mine").unwrap();
        let env = FakeEnvironment::new(dir.path());
        let setup = setup(&env);
        open(&setup, dir.path());
        until(&env, |all| !all.is_empty()).await;

        let answer = setup.initialise(ID, false).await.unwrap();
        assert_eq!(answer.conflicts, [".omp/WATCHDOG.yml"]);
        assert!(!dir.path().join(".trama").exists());
        assert_eq!(
            fs::read_to_string(dir.path().join(".omp/WATCHDOG.yml")).unwrap(),
            "mine"
        );
        assert_eq!(env.last().unwrap().state, ProjectState::NotInitialised);

        assert!(setup
            .initialise(ID, true)
            .await
            .unwrap()
            .conflicts
            .is_empty());
        until(&env, |all| {
            all.last()
                .is_some_and(|s| s.state == ProjectState::EmptyModel)
        })
        .await;
        assert_ne!(
            fs::read_to_string(dir.path().join(".omp/WATCHDOG.yml")).unwrap(),
            "mine"
        );
    }

    #[tokio::test]
    async fn a_file_edited_by_hand_is_written_again_when_the_window_opens_and_the_window_says_so_once(
    ) {
        let dir = tempfile::tempdir().unwrap();
        project_with_a_model(dir.path());
        fs::write(dir.path().join(".omp/WATCHDOG.yml"), "edited").unwrap();
        let env = FakeEnvironment::new(dir.path());
        let setup = setup(&env);
        open(&setup, dir.path());
        until(&env, |all| all.last().is_some_and(|s| s.check.opening == 1)).await;
        let status = env.last().unwrap();
        assert_eq!(
            (status.state, status.check.outcome.clone()),
            (ProjectState::HasModel, CheckOutcome::Updated { files: 1 })
        );
        assert_ne!(
            fs::read_to_string(dir.path().join(".omp/WATCHDOG.yml")).unwrap(),
            "edited"
        );

        // Opened again with everything in place: the check ran and has nothing to say.
        setup.window_closed(ID);
        let env = FakeEnvironment::new(dir.path());
        let setup = self::setup(&env);
        open(&setup, dir.path());
        until(&env, |all| all.last().is_some_and(|s| s.check.opening == 1)).await;
        assert_eq!(env.last().unwrap().check.outcome, CheckOutcome::None);
    }

    #[tokio::test]
    async fn a_failed_check_is_shown_until_update_skills_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        project_with_a_model(dir.path());
        fs::remove_dir_all(dir.path().join(".omp")).unwrap();
        // `.omp` is a file: nothing can be written below it.
        fs::write(dir.path().join(".omp"), "in the way").unwrap();
        let env = FakeEnvironment::new(dir.path());
        let setup = setup(&env);
        open(&setup, dir.path());
        until(&env, |all| all.last().is_some_and(|s| s.check.opening == 1)).await;
        let CheckOutcome::Failed { reason } = env.last().unwrap().check.outcome else {
            panic!("{:?}", env.last())
        };
        assert!(reason.contains(".omp"), "{reason}");

        let error = setup.update_skills(ID).await.unwrap_err();
        assert!(error.message.contains(".omp"), "{}", error.message);
        assert!(matches!(
            env.last().unwrap().check.outcome,
            CheckOutcome::Failed { .. }
        ));

        fs::remove_file(dir.path().join(".omp")).unwrap();
        let report = setup.update_skills(ID).await.unwrap();
        assert!(report.changes.iter().any(|c| c.path == ".omp/WATCHDOG.yml"));
        until(&env, |all| {
            all.last()
                .is_some_and(|s| s.check.outcome == CheckOutcome::None)
        })
        .await;
    }

    #[tokio::test]
    async fn deleting_and_creating_trama_goes_to_get_started_and_back() {
        let dir = tempfile::tempdir().unwrap();
        project_with_a_model(dir.path());
        let env = FakeEnvironment::new(dir.path());
        let setup = setup(&env);
        open(&setup, dir.path());
        until(&env, |all| all.last().is_some_and(|s| s.check.opening == 1)).await;
        assert_eq!(env.last().unwrap().state, ProjectState::HasModel);

        fs::remove_dir_all(dir.path().join(".trama")).unwrap();
        setup.batch_applied(ID, None);
        until(&env, |all| {
            all.last()
                .is_some_and(|s| s.state == ProjectState::NotInitialised)
        })
        .await;

        fs::create_dir_all(dir.path().join(".trama/model")).unwrap();
        fs::write(
            dir.path().join(".trama/model/system.md"),
            "# Shop\n\nA shop.\n",
        )
        .unwrap();
        setup.batch_applied(ID, None);
        until(&env, |all| {
            all.last()
                .is_some_and(|s| s.state == ProjectState::EmptyModel)
        })
        .await;

        feature(dir.path());
        setup.batch_applied(ID, None);
        until(&env, |all| {
            all.last()
                .is_some_and(|s| s.state == ProjectState::HasModel)
        })
        .await;
    }

    #[tokio::test]
    async fn the_requirement_skills_follow_a_change_of_the_requirements_once_after_the_last_change()
    {
        let dir = tempfile::tempdir().unwrap();
        project_with_a_model(dir.path());
        let env = FakeEnvironment::new(dir.path());
        let setup = setup(&env);
        open(&setup, dir.path());
        until(&env, |all| all.last().is_some_and(|s| s.check.opening == 1)).await;
        let skill = dir.path().join(".agents/skills/trama-req-ui-main/SKILL.md");
        assert!(!skill.exists());

        fs::write(
            dir.path().join(".trama/requirements/ui/main.md"),
            "---\ndescription: Use when building screens.\n---\n# Main\n\n## General\n- A line.\n",
        )
        .unwrap();
        let revision = Revision {
            project_id: ID.to_owned(),
            revision: 2,
            slices: vec![SLICE_REQUIREMENTS.to_owned()],
        };
        setup.batch_applied(ID, Some(&revision));
        setup.batch_applied(ID, Some(&revision));
        // The skill appears; the status does not change (nothing to say), so wait on the files.
        until(&env, |_| skill.exists()).await;
        assert!(fs::read_to_string(&skill).unwrap().contains("A line."));
        assert!(env.last().unwrap().check.outcome == CheckOutcome::None);
    }

    #[tokio::test]
    async fn update_skills_in_a_folder_that_is_not_a_project_is_refused_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let env = FakeEnvironment::new(dir.path());
        let setup = setup(&env);
        open(&setup, dir.path());
        until(&env, |all| !all.is_empty()).await;
        let error = setup.update_skills(ID).await.unwrap_err();
        assert!(error.message.contains("initialise"), "{}", error.message);
        assert!(fs::read_dir(dir.path()).unwrap().next().is_none());
    }

    #[tokio::test]
    async fn the_status_of_a_window_that_asks_early_is_worked_out_and_a_closed_window_has_none() {
        let dir = tempfile::tempdir().unwrap();
        let env = FakeEnvironment::new(dir.path());
        let setup = setup(&env);
        let status = setup.status(ID).await.unwrap();
        assert_eq!(
            (status.state, status.revision),
            (ProjectState::NotInitialised, 0)
        );
        assert!(env.last().is_none());
    }

    #[test]
    fn the_check_reports_what_the_opening_run_did_and_update_skills_ends_a_failure() {
        use trama_core::project_files::{FileAction, FileChange};
        let changed = SyncReport {
            changes: vec![FileChange {
                path: "a".into(),
                action: FileAction::Rewritten,
            }],
            areas: vec![],
            generated: project_files::sync::Generated {
                mode: project_files::SkillMode::Fragment,
                budget: 0,
                limit: 4000,
                switched: None,
            },
        };
        let none = SkillsCheck::none();
        let opened = none.after(Want::OpeningCheck, &Ok(changed.clone()));
        assert_eq!(
            opened,
            SkillsCheck {
                opening: 1,
                outcome: CheckOutcome::Updated { files: 1 }
            }
        );
        let failed = opened.after(Want::OpeningCheck, &Err("no".into()));
        assert_eq!(
            failed,
            SkillsCheck {
                opening: 2,
                outcome: CheckOutcome::Failed {
                    reason: "no".into()
                }
            }
        );
        // A failed `Update skills` and an automatic regeneration leave it alone; a good one ends it.
        assert_eq!(
            failed.after(Want::UpdateSkills, &Err("still".into())),
            failed
        );
        assert_eq!(
            failed.after(Want::RequirementSkills, &Ok(changed.clone())),
            failed
        );
        assert_eq!(
            failed.after(Want::UpdateSkills, &Ok(changed)),
            SkillsCheck {
                opening: 2,
                outcome: CheckOutcome::None
            }
        );
    }
}
