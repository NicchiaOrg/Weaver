//! omp in the app: the one Machine `omp-setup`, the processes it asks for, the timers it needs and
//! the commands and event of the UI (`requirements/tech#omp`, `requirements/tech#omp-setup`).
//!
//! The Machine itself is `weaver_core::omp::machine` (pure: events in, effects out). This module
//! is the glue: [`OmpSetup`] holds it, runs each [`Effect`] on the blocking pool, turns what comes
//! back into an [`Event`], keeps the 3 s re-check timer alive only while the Machine wants it,
//! tells every window ([`events::emit_omp_changed`]) and tells the backend whether omp is ready.
//!
//! # For the jobs
//!
//! Nothing that starts omp (a job, a General conversation) may start before the Machine is Ready:
//!
//! ```ignore
//! let omp = app.state::<OmpSetup>();
//! let ready = omp.wait_ready().await;              // `Arc<ReadyOmp>`: omp, version, models
//! let path = omp.child_path(&ready.omp);           // the PATH every omp process gets
//! let settings = app.state::<AppState>().settings.current();
//! // ...start `ready.omp` with PATH = path; when it cannot be started at all:
//! omp.report_missing(Some("could not start omp".into()));
//! ```
//!
//! `omp.ready()` answers at once (`None` while not Ready); `omp.subscribe()` is a watch of the same.
//! A job created while omp is not ready waits on `wait_ready`; one that is already working is not
//! touched by a later state change.

use std::any::Any;
use std::ffi::OsString;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

use tauri::{AppHandle, Manager, State};
use tokio::sync::watch;
use weaver_core::omp::failure::Failure;
use weaver_core::omp::lookup::{LookupEnv, Os};
use weaver_core::omp::machine::{
    Effect, Event, Machine, OmpStatus, ReadyOmp, Ticket, RECHECK_EVERY,
};
use weaver_core::omp::path_env;
use weaver_core::omp::probe::{self, ProbeResult};
use weaver_core::omp::providers::{provider, Provider};
use weaver_core::omp::run::{which, SystemExec};
use weaver_core::omp::terminal;
use weaver_core::project::clock::{Clock, SystemClock};
use weaver_core::settings::SettingKey;

use crate::error::{AppError, ErrorCode};
use crate::{blocking, events, log_info, log_warn};

/// The first look for a newer omp, after start, and how often the app asks whether one is due.
/// The Machine lets at most one check through per hour.
const FIRST_UPDATE_CHECK: Duration = Duration::from_secs(60);
const UPDATE_CHECK_TICK: Duration = Duration::from_secs(10 * 60);

/// The terminal could not be opened: why, and the command the developer can copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalFailure {
    pub reason: String,
    pub command: String,
}

/// What the Machine asks the system to do. The real one runs processes; the tests use a fake.
pub trait Ops: Send + Sync + 'static {
    /// The lookup, `omp --version` and `omp models --json` for the `ompPath` setting.
    fn probe(&self, setting: &str) -> ProbeResult;
    /// The same questions to an omp already found.
    fn probe_found(&self, omp: &Path) -> ProbeResult;
    /// `omp update` (5 min).
    fn update(&self, omp: &Path) -> Result<(), Failure>;
    /// `omp update --check` (30 s): the newer version, if any.
    fn update_check(&self, omp: &Path) -> Result<Option<String>, Failure>;
    /// `omp auth-broker logout <provider>` (30 s).
    fn logout(&self, omp: &Path, provider: &Provider) -> Result<(), Failure>;
    /// Opens the system terminal with the official installer.
    fn open_installer(&self) -> Result<(), TerminalFailure>;
    /// Opens the system terminal with `omp login <provider>`.
    fn open_login(&self, omp: &Path, provider: &Provider) -> Result<(), TerminalFailure>;
    /// The `PATH` of a process that runs `omp`.
    fn child_path(&self, omp: &Path) -> OsString;
}

/// Where work runs and time passes.
pub trait Scheduler: Send + Sync + 'static {
    /// Runs `job` off the calling thread (the blocking pool).
    fn spawn(&self, job: Box<dyn FnOnce() + Send>);
    /// Calls `tick` after `first`, then every `period`, until the returned guard is dropped.
    fn repeat(
        &self,
        first: Duration,
        period: Duration,
        tick: Arc<dyn Fn() + Send + Sync>,
    ) -> Box<dyn Any + Send>;
}

/// The wiring of a started [`OmpSetup`].
pub struct Runtime {
    pub ops: Arc<dyn Ops>,
    pub scheduler: Arc<dyn Scheduler>,
    pub clock: Arc<dyn Clock>,
    /// Tells the windows that the status changed.
    pub emit: Arc<dyn Fn(&OmpStatus) + Send + Sync>,
}

struct Core {
    machine: Machine,
    /// The timer of `RECHECK_DUE`; alive only while the Machine wants it.
    recheck: Option<Box<dyn Any + Send>>,
    /// The timer of `UPDATE_CHECK_DUE`.
    update_timer: Option<Box<dyn Any + Send>>,
}

struct Shared {
    core: Mutex<Core>,
    runtime: OnceLock<Runtime>,
    ready: watch::Sender<Option<Arc<ReadyOmp>>>,
}

/// The app-wide readiness of omp. Cheap to clone; managed by Tauri.
#[derive(Clone)]
pub struct OmpSetup {
    shared: Arc<Shared>,
}

impl Default for OmpSetup {
    fn default() -> Self {
        Self::new()
    }
}

impl OmpSetup {
    /// Checking, until [`OmpSetup::start`] tells where omp is.
    pub fn new() -> Self {
        let (ready, _) = watch::channel(None);
        Self {
            shared: Arc::new(Shared {
                core: Mutex::new(Core {
                    machine: Machine::new(),
                    recheck: None,
                    update_timer: None,
                }),
                runtime: OnceLock::new(),
                ready,
            }),
        }
    }

    /// Starts the Machine: the first check for the `ompPath` setting, and the hourly look for a
    /// newer omp. Does nothing when already started.
    pub fn start(&self, runtime: Runtime, omp_path: &str) {
        if self.shared.runtime.set(runtime).is_err() {
            return;
        }
        let weak = Arc::downgrade(&self.shared);
        let timer = {
            let runtime = self.shared.runtime.get().expect("just set");
            runtime.scheduler.repeat(
                FIRST_UPDATE_CHECK,
                UPDATE_CHECK_TICK,
                Arc::new(move || dispatch_weak(&weak, Event::UpdateCheckDue)),
            )
        };
        self.shared
            .core
            .lock()
            .expect("omp state poisoned")
            .update_timer = Some(timer);
        self.shared.dispatch(Event::Start {
            omp_path: omp_path.to_owned(),
        });
    }

    /// The status the UI shows.
    pub fn status(&self) -> OmpStatus {
        self.shared
            .core
            .lock()
            .expect("omp state poisoned")
            .machine
            .status()
            .clone()
    }

    /// omp for a job: `Some` only while the Machine is Ready.
    pub fn ready(&self) -> Option<Arc<ReadyOmp>> {
        self.shared.ready.borrow().clone()
    }

    /// A watch of [`OmpSetup::ready`].
    pub fn subscribe(&self) -> watch::Receiver<Option<Arc<ReadyOmp>>> {
        self.shared.ready.subscribe()
    }

    /// Waits until the Machine is Ready and returns omp.
    pub async fn wait_ready(&self) -> Arc<ReadyOmp> {
        let mut receiver = self.shared.ready.subscribe();
        loop {
            if let Some(ready) = receiver.borrow_and_update().clone() {
                return ready;
            }
            // The sender lives as long as `self`; the error cannot happen while we hold it.
            if receiver.changed().await.is_err() {
                std::future::pending::<()>().await;
            }
        }
    }

    /// The `PATH` of a process that runs `omp`; the base PATH when the Machine has not started.
    pub fn child_path(&self, omp: &Path) -> OsString {
        match self.shared.runtime.get() {
            Some(runtime) => runtime.ops.child_path(omp),
            None => path_env::inherited_path(),
        }
    }

    /// `omp.MISSING` while Ready: a job could not start omp, so it disappeared.
    pub fn report_missing(&self, reason: Option<String>) {
        let omp_path = self.status().omp_path;
        self.shared.dispatch(Event::OmpMissing { omp_path, reason });
    }

    /// `developer.INSTALL_OMP`.
    pub fn install(&self) {
        self.shared.dispatch(Event::InstallOmp);
    }

    /// `developer.LOG_IN_PROVIDER`.
    pub fn log_in(&self, provider: &Provider) {
        self.shared.dispatch(Event::LogInProvider {
            provider: provider.id.to_owned(),
        });
    }

    /// `developer.LOG_OUT_PROVIDER`.
    pub fn log_out(&self, provider: &Provider) {
        self.shared.dispatch(Event::LogOutProvider {
            provider: provider.id.to_owned(),
        });
    }

    /// `developer.CHANGE_SETTINGS`: the `ompPath` setting is now `omp_path`.
    pub fn change_omp_path(&self, omp_path: &str) {
        self.shared.dispatch(Event::ChangeSettings {
            omp_path: omp_path.to_owned(),
        });
    }
}

fn dispatch_weak(shared: &Weak<Shared>, event: Event) {
    if let Some(shared) = shared.upgrade() {
        shared.dispatch(event);
    }
}

impl Shared {
    /// Applies `event`, keeps the timer in step, tells the windows and the jobs, and runs the effects.
    fn dispatch(self: &Arc<Self>, event: Event) {
        let Some(runtime) = self.runtime.get() else {
            return;
        };
        let (effects, status, changed, ready) = {
            let mut core = self.core.lock().expect("omp state poisoned");
            let before = core.machine.status().clone();
            let effects = core.machine.handle(event, runtime.clock.now_ms());
            let wants = core.machine.wants_recheck();
            if wants && core.recheck.is_none() {
                let weak = Arc::downgrade(self);
                core.recheck = Some(runtime.scheduler.repeat(
                    RECHECK_EVERY,
                    RECHECK_EVERY,
                    Arc::new(move || dispatch_weak(&weak, Event::RecheckDue)),
                ));
            } else if !wants {
                // Dropping the guard stops the timer: the re-checks end with the state.
                core.recheck = None;
            }
            let status = core.machine.status().clone();
            let changed = status.revision != before.revision;
            if changed && status.state != before.state {
                log_info!("omp setup: {:?} -> {:?}", before.state, status.state);
            }
            (effects, status, changed, core.machine.ready().map(Arc::new))
        };
        self.ready.send_if_modified(|current| {
            if current.as_deref() == ready.as_deref() {
                false
            } else {
                *current = ready;
                true
            }
        });
        if changed {
            (runtime.emit)(&status);
        }
        for effect in effects {
            let shared = Arc::clone(self);
            runtime
                .scheduler
                .spawn(Box::new(move || shared.run(effect)));
        }
    }

    /// Runs one effect (on the blocking pool) and dispatches what came of it. A panic in the
    /// work is a failure of that operation, never a stuck Machine.
    fn run(self: &Arc<Self>, effect: Effect) {
        let runtime = self.runtime.get().expect("effects run after start");
        let ops = &runtime.ops;
        match effect {
            Effect::Probe(ticket) => {
                let Ticket { omp_path, .. } = &ticket;
                let result =
                    guard(|| ops.probe(omp_path)).unwrap_or_else(|| ProbeResult::Missing {
                        reason: Some("the check of omp failed unexpectedly".to_owned()),
                    });
                self.dispatch(Event::Probed { ticket, result });
            }
            Effect::RunUpdate { omp_path, omp } => {
                match guard(|| ops.update(&omp)).unwrap_or(Err(Failure::NotStarted)) {
                    Ok(()) => self.dispatch(Event::UpdateEnded { omp_path }),
                    Err(failure) => {
                        log_warn!("omp update failed");
                        let after = guard(|| ops.probe_found(&omp))
                            .unwrap_or(ProbeResult::Missing { reason: None });
                        self.dispatch(Event::UpdateFailed {
                            omp_path,
                            reason: failure.reason_with_hint(),
                            after,
                        });
                    }
                }
            }
            Effect::CheckForUpdate { omp_path, omp } => match guard(|| ops.update_check(&omp)) {
                Some(Ok(available)) => self.dispatch(Event::UpdateCheckDone {
                    omp_path,
                    available,
                }),
                // A check that fails is logged; the UI never hears of it and no job waits for it.
                Some(Err(_)) | None => log_warn!("the check for a newer omp failed"),
            },
            Effect::OpenInstaller => {
                if let Some(Err(failure)) = guard(|| ops.open_installer()) {
                    self.dispatch(Event::TerminalFailed {
                        reason: failure.reason,
                        command: failure.command,
                    });
                }
            }
            Effect::OpenLogin { omp, provider: id } => {
                let Some(provider) = provider(&id) else {
                    return;
                };
                if let Some(Err(failure)) = guard(|| ops.open_login(&omp, provider)) {
                    self.dispatch(Event::TerminalFailed {
                        reason: failure.reason,
                        command: failure.command,
                    });
                }
            }
            Effect::Logout {
                omp_path,
                omp,
                provider: id,
            } => {
                let Some(provider) = provider(&id) else {
                    return;
                };
                let outcome = match guard(|| ops.logout(&omp, provider)) {
                    Some(Ok(())) => Ok(()),
                    Some(Err(failure)) => Err(failure.reason_with_hint()),
                    None => Err("the log out failed unexpectedly".to_owned()),
                };
                self.dispatch(Event::LoggedOut {
                    omp_path,
                    provider: id,
                    outcome,
                });
            }
        }
    }
}

/// Runs `work`; a panic is logged by the panic hook and comes back as `None`.
fn guard<T>(work: impl FnOnce() -> T) -> Option<T> {
    catch_unwind(AssertUnwindSafe(work)).ok()
}

// --- the real system ----------------------------------------------------------------------------

/// The real processes: omp itself, the lookup and the system terminal.
pub struct SystemOps {
    os: Os,
    local_bin: PathBuf,
    /// macOS and Linux: the login PATH read at start (or the inherited one).
    base_path: OsString,
}

impl SystemOps {
    pub fn new(local_bin: PathBuf, base_path: OsString) -> Self {
        Self {
            os: Os::current(),
            local_bin,
            base_path,
        }
    }

    /// The PATH to search now: Windows reads the registry at every lookup.
    fn search_path(&self) -> OsString {
        if self.os == Os::Windows {
            let root = std::env::var_os("SystemRoot")
                .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
            let exec = SystemExec::new(self.local_bin.clone(), None, path_env::inherited_path());
            path_env::read_windows_path(&exec, &root)
        } else {
            self.base_path.clone()
        }
    }

    fn environment(&self) -> (LookupEnv, SystemExec) {
        let lookup = LookupEnv::from_process(self.search_path());
        let exec = SystemExec::new(
            self.local_bin.clone(),
            lookup.bun_bin(),
            lookup.path.clone(),
        );
        (lookup, exec)
    }

    fn open(&self, command: String, search_path: &OsString) -> Result<(), TerminalFailure> {
        let fail = |reason: String| TerminalFailure {
            reason,
            command: command.clone(),
        };
        let launch = terminal::launch_plan(self.os, &command, &|name| which(name, search_path))
            .map_err(|error| fail(error.to_string()))?;
        terminal::open(self.os, &launch).map_err(|error| fail(error.to_string()))
    }
}

impl Ops for SystemOps {
    fn probe(&self, setting: &str) -> ProbeResult {
        let (lookup, exec) = self.environment();
        probe::probe(setting, &lookup, &exec)
    }

    fn probe_found(&self, omp: &Path) -> ProbeResult {
        probe::probe_found(omp, &self.environment().1)
    }

    fn update(&self, omp: &Path) -> Result<(), Failure> {
        probe::run_update(omp, &self.environment().1)
    }

    fn update_check(&self, omp: &Path) -> Result<Option<String>, Failure> {
        probe::check_for_update(omp, &self.environment().1)
    }

    fn logout(&self, omp: &Path, provider: &Provider) -> Result<(), Failure> {
        probe::logout(omp, provider, &self.environment().1)
    }

    fn open_installer(&self) -> Result<(), TerminalFailure> {
        self.open(
            terminal::installer_command(self.os).to_owned(),
            &self.search_path(),
        )
    }

    fn open_login(&self, omp: &Path, provider: &Provider) -> Result<(), TerminalFailure> {
        let command =
            terminal::login_command(self.os, omp, provider).map_err(|error| TerminalFailure {
                reason: error.to_string(),
                command: format!("omp login {}", provider.id),
            })?;
        self.open(command, &self.search_path())
    }

    fn child_path(&self, omp: &Path) -> OsString {
        self.environment().1.child_path(omp)
    }
}

/// The scheduler of the app: Tauri's blocking pool and Tokio's timers.
pub struct TauriScheduler;

struct AbortOnDrop(tauri::async_runtime::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl Scheduler for TauriScheduler {
    fn spawn(&self, job: Box<dyn FnOnce() + Send>) {
        drop(tauri::async_runtime::spawn_blocking(job));
    }

    fn repeat(
        &self,
        first: Duration,
        period: Duration,
        tick: Arc<dyn Fn() + Send + Sync>,
    ) -> Box<dyn Any + Send> {
        Box::new(AbortOnDrop(tauri::async_runtime::spawn(async move {
            tokio::time::sleep(first).await;
            loop {
                // The tick takes a lock and may emit an event: nothing here waits for the disk.
                tick();
                tokio::time::sleep(period).await;
            }
        })))
    }
}

/// The startup step `omp`: reads the login PATH (macOS and Linux, 5 s) and the `ompPath` setting,
/// starts the Machine and follows later changes of the setting. Runs on the blocking pool.
pub fn start(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<crate::state::AppState>();
    // Subscribe before reading, so a change between the two is not lost.
    let mut changes = state.settings.subscribe();
    let omp_path = state.settings.current().omp_path;

    let base_path = if cfg!(windows) {
        path_env::inherited_path()
    } else {
        login_path()
    };
    let ops = SystemOps::new(state.dirs.local_data_dir().join("bin"), base_path);
    let emit_handle = app.clone();
    let setup = app.state::<OmpSetup>().inner().clone();
    setup.start(
        Runtime {
            ops: Arc::new(ops),
            scheduler: Arc::new(TauriScheduler),
            clock: Arc::new(SystemClock),
            emit: Arc::new(move |status| {
                if let Err(error) = events::emit_omp_changed(&emit_handle, status) {
                    crate::log_error!("cannot tell the windows about omp: {error}");
                }
            }),
        },
        &omp_path,
    );

    let settings_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        use tokio::sync::broadcast::error::RecvError;
        loop {
            match changes.recv().await {
                Ok(change) if change.changed.contains(&SettingKey::OmpPath) => {
                    setup.change_omp_path(&change.settings.omp_path);
                }
                Ok(_) => {}
                // Fell behind: look at the setting as it is now.
                Err(RecvError::Lagged(_)) => {
                    let handle = settings_handle.clone();
                    if let Ok(current) = blocking::disk(move || {
                        handle.state::<crate::state::AppState>().settings.current()
                    })
                    .await
                    {
                        setup.change_omp_path(&current.omp_path);
                    }
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
    Ok(())
}

/// The login PATH of the user's shell, read once; the inherited PATH when that fails.
fn login_path() -> OsString {
    let inherited = path_env::inherited_path();
    let Some(shell) = std::env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|s| s.is_absolute())
    else {
        return inherited;
    };
    let exec = SystemExec::new(PathBuf::new(), None, inherited.clone());
    match path_env::read_login_path(&exec, &shell) {
        Some(path) => path,
        None => {
            log_warn!("the login PATH could not be read; the inherited PATH is used");
            inherited
        }
    }
}

// --- commands -------------------------------------------------------------------------------------

/// The status of omp, for a window that just loaded; later changes arrive as `omp-changed`.
#[tauri::command]
pub async fn omp_status(omp: State<'_, OmpSetup>) -> Result<OmpStatus, AppError> {
    Ok(omp.status())
}

/// **Install omp** and **Open installer again**: opens the system terminal with the installer.
#[tauri::command]
pub async fn omp_install(omp: State<'_, OmpSetup>) -> Result<(), AppError> {
    omp.install();
    Ok(())
}

fn known_provider(id: &str) -> Result<&'static Provider, AppError> {
    provider(id).ok_or_else(|| {
        AppError::new(
            ErrorCode::InvalidInput,
            "that is not a provider omp can log in to",
        )
    })
}

/// **Log in** and **Log in again**: opens the system terminal with `omp login <provider>`.
#[tauri::command]
pub async fn omp_log_in(omp: State<'_, OmpSetup>, provider: String) -> Result<(), AppError> {
    omp.log_in(known_provider(&provider)?);
    Ok(())
}

/// **Log out**: omp forgets the credentials it stored for the provider. Returns at once; the
/// progress and the outcome arrive in the status.
#[tauri::command]
pub async fn omp_log_out(omp: State<'_, OmpSetup>, provider: String) -> Result<(), AppError> {
    omp.log_out(known_provider(&provider)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

    use weaver_core::omp::machine::Phase;
    use weaver_core::omp::probe::OmpModel;

    use super::*;

    fn omp() -> PathBuf {
        PathBuf::from("/opt/omp")
    }

    fn model(provider: &str) -> OmpModel {
        OmpModel {
            provider: provider.into(),
            selector: format!("{provider}/m"),
            name: None,
            thinking_levels: vec![],
        }
    }

    fn ready() -> ProbeResult {
        ProbeResult::Ready {
            omp: omp(),
            version: "18.15.0".into(),
            models: vec![model("anthropic")],
        }
    }

    /// The answers of the fake system, taken in order.
    #[derive(Default)]
    struct FakeOps {
        probes: Mutex<VecDeque<ProbeResult>>,
        updates: Mutex<VecDeque<Result<(), Failure>>>,
        update_checks: Mutex<VecDeque<Result<Option<String>, Failure>>>,
        terminal_fails: AtomicBool,
        probed_paths: Mutex<Vec<String>>,
        installers: Mutex<u32>,
        logins: Mutex<Vec<String>>,
        logouts: Mutex<Vec<String>>,
    }

    impl Ops for FakeOps {
        fn probe(&self, setting: &str) -> ProbeResult {
            self.probed_paths.lock().unwrap().push(setting.to_owned());
            self.probes
                .lock()
                .unwrap()
                .pop_front()
                .expect("a scripted probe")
        }
        fn probe_found(&self, _omp: &Path) -> ProbeResult {
            self.probes
                .lock()
                .unwrap()
                .pop_front()
                .expect("a scripted probe")
        }
        fn update(&self, _omp: &Path) -> Result<(), Failure> {
            self.updates
                .lock()
                .unwrap()
                .pop_front()
                .expect("a scripted update")
        }
        fn update_check(&self, _omp: &Path) -> Result<Option<String>, Failure> {
            self.update_checks
                .lock()
                .unwrap()
                .pop_front()
                .expect("a scripted check")
        }
        fn logout(&self, _omp: &Path, provider: &Provider) -> Result<(), Failure> {
            self.logouts.lock().unwrap().push(provider.id.to_owned());
            Ok(())
        }
        fn open_installer(&self) -> Result<(), TerminalFailure> {
            *self.installers.lock().unwrap() += 1;
            if self.terminal_fails.load(Ordering::SeqCst) {
                Err(TerminalFailure {
                    reason: "no terminal emulator was found".into(),
                    command: "curl x | sh".into(),
                })
            } else {
                Ok(())
            }
        }
        fn open_login(&self, _omp: &Path, provider: &Provider) -> Result<(), TerminalFailure> {
            self.logins.lock().unwrap().push(provider.id.to_owned());
            Ok(())
        }
        fn child_path(&self, _omp: &Path) -> OsString {
            OsString::from("/child")
        }
    }

    type Tick = Arc<dyn Fn() + Send + Sync>;

    /// Jobs wait in a queue until the test runs them; timers never fire by themselves.
    #[derive(Default)]
    struct ManualScheduler {
        jobs: Mutex<VecDeque<Box<dyn FnOnce() + Send>>>,
        timers: Mutex<Vec<(Duration, Tick, Arc<AtomicBool>)>>,
    }

    struct Guard(Arc<AtomicBool>);

    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }

    impl Scheduler for ManualScheduler {
        fn spawn(&self, job: Box<dyn FnOnce() + Send>) {
            self.jobs.lock().unwrap().push_back(job);
        }
        fn repeat(&self, _first: Duration, period: Duration, tick: Tick) -> Box<dyn Any + Send> {
            let alive = Arc::new(AtomicBool::new(true));
            self.timers
                .lock()
                .unwrap()
                .push((period, tick, alive.clone()));
            Box::new(Guard(alive))
        }
    }

    impl ManualScheduler {
        /// Runs every queued job, including the ones they queue.
        fn drain(&self) {
            loop {
                let job = self.jobs.lock().unwrap().pop_front();
                match job {
                    Some(job) => job(),
                    None => break,
                }
            }
        }

        /// The tick of the timer with this period, when it is alive.
        fn live_timer(&self, period: Duration) -> Option<Tick> {
            self.timers
                .lock()
                .unwrap()
                .iter()
                .filter(|(p, _, alive)| *p == period && alive.load(Ordering::SeqCst))
                .map(|(_, tick, _)| tick.clone())
                .next_back()
        }
    }

    struct Rig {
        setup: OmpSetup,
        ops: Arc<FakeOps>,
        scheduler: Arc<ManualScheduler>,
        clock: Arc<ClockAt>,
        emitted: Arc<Mutex<Vec<OmpStatus>>>,
        /// The wiring, taken by `start`.
        runtime: Mutex<Option<Runtime>>,
    }

    struct ClockAt(AtomicI64);

    impl Clock for ClockAt {
        fn now_ms(&self) -> i64 {
            self.0.load(Ordering::SeqCst)
        }
    }

    fn rig() -> Rig {
        let ops = Arc::new(FakeOps::default());
        let scheduler = Arc::new(ManualScheduler::default());
        let clock = Arc::new(ClockAt(AtomicI64::new(1_000_000)));
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let sink = emitted.clone();
        let runtime = Runtime {
            ops: ops.clone(),
            scheduler: scheduler.clone(),
            clock: clock.clone(),
            emit: Arc::new(move |status| sink.lock().unwrap().push(status.clone())),
        };
        Rig {
            setup: OmpSetup::new(),
            ops,
            scheduler,
            clock,
            emitted,
            runtime: Mutex::new(Some(runtime)),
        }
    }

    impl Rig {
        fn start(&self, path: &str) {
            let runtime = self.runtime.lock().unwrap().take().expect("started once");
            self.setup.start(runtime, path);
        }
        fn script_probe(&self, result: ProbeResult) {
            self.ops.probes.lock().unwrap().push_back(result);
        }
        fn phase(&self) -> Phase {
            self.setup.status().state
        }
    }

    #[test]
    fn the_machine_starts_checking_and_becomes_ready_when_omp_answers() {
        let rig = rig();
        assert_eq!(rig.phase(), Phase::Checking);
        assert!(rig.setup.ready().is_none());
        rig.script_probe(ready());
        rig.start("/custom/omp");
        rig.scheduler.drain();
        assert_eq!(rig.phase(), Phase::Ready);
        assert_eq!(
            rig.ops.probed_paths.lock().unwrap().as_slice(),
            ["/custom/omp"]
        );
        let ready = rig.setup.ready().expect("ready");
        assert_eq!(
            (ready.omp.clone(), ready.version.as_str()),
            (omp(), "18.15.0")
        );
        assert_eq!(rig.setup.child_path(&ready.omp), OsString::from("/child"));
        assert_eq!(
            rig.emitted.lock().unwrap().last().unwrap().state,
            Phase::Ready
        );
    }

    #[test]
    fn a_job_waits_until_the_machine_is_ready() {
        let rig = rig();
        rig.script_probe(ProbeResult::Missing { reason: None });
        rig.start("");
        rig.scheduler.drain();
        assert_eq!(rig.phase(), Phase::Missing);
        let mut watcher = rig.setup.subscribe();
        assert!(watcher.borrow_and_update().is_none());
        rig.setup.install();
        rig.script_probe(ready());
        // Installing: the timer ticks and finds omp.
        let tick = rig
            .scheduler
            .live_timer(RECHECK_EVERY)
            .expect("the re-check timer runs while Installing");
        tick();
        rig.scheduler.drain();
        assert_eq!(rig.phase(), Phase::Ready);
        assert!(watcher.has_changed().unwrap());
        assert!(watcher.borrow_and_update().is_some());
    }

    #[test]
    fn the_recheck_timer_lives_only_while_installing_or_logging_in() {
        let rig = rig();
        rig.script_probe(ProbeResult::Missing { reason: None });
        rig.start("");
        rig.scheduler.drain();
        assert!(rig.scheduler.live_timer(RECHECK_EVERY).is_none());
        rig.setup.install();
        rig.scheduler.drain();
        assert_eq!(*rig.ops.installers.lock().unwrap(), 1);
        let tick = rig.scheduler.live_timer(RECHECK_EVERY).expect("running");
        // omp still missing: the timer stays; then it is found and the timer stops.
        rig.script_probe(ProbeResult::Missing { reason: None });
        tick();
        rig.scheduler.drain();
        assert!(rig.scheduler.live_timer(RECHECK_EVERY).is_some());
        rig.script_probe(ready());
        tick();
        rig.scheduler.drain();
        assert_eq!(rig.phase(), Phase::Ready);
        assert!(rig.scheduler.live_timer(RECHECK_EVERY).is_none());
        // A tick that was already on its way does nothing.
        tick();
        rig.scheduler.drain();
        assert_eq!(rig.phase(), Phase::Ready);
    }

    #[test]
    fn two_ticks_in_a_row_start_one_check() {
        let rig = rig();
        rig.script_probe(ProbeResult::Missing { reason: None });
        rig.start("");
        rig.scheduler.drain();
        rig.setup.install();
        rig.scheduler.drain();
        let tick = rig.scheduler.live_timer(RECHECK_EVERY).unwrap();
        rig.script_probe(ProbeResult::Missing { reason: None });
        tick();
        tick();
        assert_eq!(rig.scheduler.jobs.lock().unwrap().len(), 1);
        rig.scheduler.drain();
    }

    #[test]
    fn a_terminal_that_cannot_open_is_reported_with_the_command_to_copy() {
        let rig = rig();
        rig.ops.terminal_fails.store(true, Ordering::SeqCst);
        rig.script_probe(ProbeResult::Missing { reason: None });
        rig.start("");
        rig.scheduler.drain();
        rig.setup.install();
        rig.scheduler.drain();
        let problem = rig.setup.status().terminal_problem.expect("a problem");
        assert_eq!(
            (problem.reason.as_str(), problem.command.as_str()),
            ("no terminal emulator was found", "curl x | sh")
        );
    }

    #[test]
    fn an_outdated_omp_is_updated_and_checked_again() {
        let rig = rig();
        rig.script_probe(ProbeResult::Outdated {
            omp: omp(),
            version: Some("18.0.0".into()),
        });
        rig.ops.updates.lock().unwrap().push_back(Ok(()));
        rig.script_probe(ready());
        rig.start("");
        rig.scheduler.drain();
        assert_eq!(rig.phase(), Phase::Ready);
    }

    #[test]
    fn a_failed_update_asks_omp_again_and_keeps_working_when_it_answers() {
        let rig = rig();
        rig.script_probe(ProbeResult::Outdated {
            omp: omp(),
            version: Some("18.0.0".into()),
        });
        rig.ops
            .updates
            .lock()
            .unwrap()
            .push_back(Err(Failure::Exit {
                code: Some(1),
                stderr: "disk full\n".into(),
            }));
        rig.script_probe(ready());
        rig.start("");
        rig.scheduler.drain();
        assert_eq!(rig.phase(), Phase::Ready);
        assert_eq!(
            rig.setup.status().update_failure.as_deref(),
            Some("disk full")
        );
    }

    #[test]
    fn the_update_check_runs_in_the_background_without_taking_ready_away() {
        let rig = rig();
        rig.script_probe(ready());
        rig.start("");
        rig.scheduler.drain();
        let tick = rig
            .scheduler
            .live_timer(UPDATE_CHECK_TICK)
            .expect("the update timer runs from start");
        rig.ops
            .update_checks
            .lock()
            .unwrap()
            .push_back(Ok(Some("18.16.0".into())));
        tick();
        // The check is queued: a job still finds omp ready meanwhile.
        assert!(rig.setup.ready().is_some());
        rig.ops.updates.lock().unwrap().push_back(Ok(()));
        rig.script_probe(ready());
        rig.scheduler.drain();
        assert_eq!(rig.phase(), Phase::Ready);
        // A second tick inside the hour asks nothing.
        tick();
        assert!(rig.scheduler.jobs.lock().unwrap().is_empty());
        rig.clock.0.fetch_add(
            weaver_core::omp::machine::UPDATE_CHECK_INTERVAL_MS,
            Ordering::SeqCst,
        );
        rig.ops.update_checks.lock().unwrap().push_back(Ok(None));
        tick();
        assert_eq!(rig.scheduler.jobs.lock().unwrap().len(), 1);
        rig.scheduler.drain();
    }

    #[test]
    fn logging_out_runs_omp_and_then_checks_the_providers() {
        let rig = rig();
        rig.script_probe(ready());
        rig.start("");
        rig.scheduler.drain();
        rig.script_probe(ProbeResult::NoProvider {
            omp: omp(),
            version: "18.15.0".into(),
        });
        rig.setup.log_out(provider("anthropic").unwrap());
        rig.scheduler.drain();
        assert_eq!(rig.ops.logouts.lock().unwrap().as_slice(), ["anthropic"]);
        assert_eq!(rig.phase(), Phase::NotConnected);
    }

    #[test]
    fn a_login_opens_the_terminal_for_the_provider_and_waits() {
        let rig = rig();
        rig.script_probe(ProbeResult::NoProvider {
            omp: omp(),
            version: "18.15.0".into(),
        });
        rig.start("");
        rig.scheduler.drain();
        rig.setup.log_in(provider("cursor").unwrap());
        rig.scheduler.drain();
        assert_eq!(rig.ops.logins.lock().unwrap().as_slice(), ["cursor"]);
        assert_eq!(rig.phase(), Phase::LoggingIn);
        assert!(rig.scheduler.live_timer(RECHECK_EVERY).is_some());
    }

    #[test]
    fn a_change_of_the_path_checks_again_and_a_late_result_for_the_old_path_is_dropped() {
        let rig = rig();
        rig.script_probe(ready());
        rig.start("/old");
        // The first check is queued but has not run when the setting changes.
        rig.setup.change_omp_path("/new");
        rig.script_probe(ProbeResult::Missing {
            reason: Some("omp not found at /new".into()),
        });
        rig.scheduler.drain();
        // The probe of "/old" took the first scripted answer (Ready) but its result was dropped.
        assert_eq!(rig.phase(), Phase::Missing);
        assert_eq!(
            rig.setup.status().reason.as_deref(),
            Some("omp not found at /new")
        );
        assert!(rig.setup.ready().is_none());
    }

    #[test]
    fn an_omp_that_cannot_be_started_by_a_job_is_missing() {
        let rig = rig();
        rig.script_probe(ready());
        rig.start("");
        rig.scheduler.drain();
        rig.setup.report_missing(Some("could not start omp".into()));
        assert_eq!(rig.phase(), Phase::Missing);
        assert!(rig.setup.ready().is_none());
    }

    #[tokio::test]
    async fn wait_ready_returns_when_the_machine_becomes_ready() {
        let rig = rig();
        rig.script_probe(ready());
        let setup = rig.setup.clone();
        let waiting = tokio::spawn(async move { setup.wait_ready().await });
        tokio::task::yield_now().await;
        assert!(!waiting.is_finished());
        rig.start("");
        rig.scheduler.drain();
        let omp = waiting.await.unwrap();
        assert_eq!(omp.version, "18.15.0");
    }

    #[test]
    fn a_provider_outside_the_list_is_refused_by_the_commands() {
        assert_eq!(known_provider("anthropic").unwrap().id, "anthropic");
        assert_eq!(
            known_provider("anthropic && calc").unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }
}
