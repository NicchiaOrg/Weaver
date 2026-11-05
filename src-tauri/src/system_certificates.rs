//! System Certificates in the app: the one Machine `system-certificates`, the export it asks for,
//! the event and the commands of the UI (`requirements/tech#system-certificates`).
//!
//! The Machine, the bundle, the variables and the classification of errors are in
//! `weaver_core::system_certificates` and `weaver_core::omp::failure`. This module is the glue:
//! [`SystemCertificates`] holds the Machine, runs each export on the blocking pool (never on the
//! IPC or UI thread), tells every window ([`events::emit_system_certificates_changed`]) and tells
//! the rest of the backend when an export ended.
//!
//! # For the other parts of the backend
//!
//! - **Variables of an omp process:** [`variables`] reads the setting and the bundle path from the
//!   app state. Apply the result after the inherited environment, at every spawn of an omp process
//!   (`weaver_core::system_certificates::apply` for a `Command`):
//!
//!   ```ignore
//!   let variables = system_certificates::variables(&app.state::<AppState>());
//!   weaver_core::system_certificates::apply(&mut command, &variables);
//!   ```
//!
//! - **Explanations of a failure:** `weaver_core::omp::failure::explain(text, enabled)` where
//!   `enabled` is `settings.current().system_certificates.enabled`.
//! - **Export on request:** [`SystemCertificates::request_export`] with `automatic: true` for the
//!   exports the app starts by itself (start, Zscaler turn-on, daily refresh).
//! - **Export ended** (`system-certificates.EXPORT_ENDED`): [`SystemCertificates::subscribe_ended`],
//!   a broadcast like [`crate::settings::SettingsService::subscribe`]; [`SystemCertificates::wait_idle`]
//!   waits for the end of the export that runs, if any.
//! - **The first check of omp** (`requirements/tech#system-certificates`, "Start order"):
//!   `service.startup_export(weaver_core::system_certificates::EXPORT_LIMIT).await` resolves once the
//!   age of the bundle and the Zscaler turn-on are decided (file metadata only, never an export);
//!   its `running` field tells whether an export started with the app and still runs, and
//!   `ended().await` waits for its end, within the same limit. Nothing else of the start waits.
//!
//! # Time, the turn-on and the banners
//!
//! * `time.CERTIFICATES_REFRESH_DUE`: a timer (the injected [`Scheduler`]) a day after the last
//!   export ended, successful or not, while the setting is on; the Machine starts an automatic
//!   export from `Ready` and `Failed`.
//! * [`turn_on`]: the Zscaler turn-on, a task of the start outside the startup steps.
//! * The state of both banners is in [`Status`] (`turn_on_notice`, `failure_banner`), so a window
//!   that opens later reads it with `system_certificates_status`; the commands
//!   `system_certificates_dismiss_notice` and `system_certificates_dismiss_failure` close them in
//!   every window.

use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, State};
use tokio::sync::{broadcast, watch};
use weaver_core::project::clock::{Clock, SystemClock};
use weaver_core::settings::SettingKey;
use weaver_core::system_certificates::{
    self as certificates, BundleInfo, ExportEnded, Machine, Request, Status, Ticket, Variables,
    REFRESH_EVERY,
};

use crate::error::{AppError, ErrorCode};
use crate::omp::{Scheduler, TauriScheduler};
use crate::state::AppState;
use crate::{blocking, events, log_info, log_warn};

pub mod turn_on;

/// What the Machine asks the system to do. The real one reads the system and writes the bundle;
/// the tests use a fake.
pub trait Ops: Send + Sync + 'static {
    /// Reads the certificates the system trusts and writes the bundle (30 s at most). Blocking.
    fn export(&self) -> Result<BundleInfo, String>;
    /// The bundle on disk: its certificate count read from the file and its modification time.
    fn bundle_info(&self) -> Option<BundleInfo>;
}

/// The wiring of a started [`SystemCertificates`].
pub struct Runtime {
    pub ops: Arc<dyn Ops>,
    pub scheduler: Arc<dyn Scheduler>,
    pub clock: Arc<dyn Clock>,
    /// The `systemCertificates.enabled` setting in force now.
    pub enabled: Arc<dyn Fn() -> bool + Send + Sync>,
    /// Tells every window that the status changed.
    pub emit: Arc<dyn Fn(&Status) + Send + Sync>,
}

/// A request to export, refused because the setting is off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingIsOff;

struct Shared {
    machine: Mutex<Machine>,
    runtime: OnceLock<Runtime>,
    /// `system-certificates.EXPORT_ENDED`.
    ended: broadcast::Sender<ExportEnded>,
    /// Whether an export runs.
    exporting: watch::Sender<bool>,
    /// The timer of `time.CERTIFICATES_REFRESH_DUE`: alive only while the setting is on. Locked
    /// after the Machine, never before.
    refresh: Mutex<Option<Box<dyn Any + Send>>>,
    /// What the start has decided so far (see [`SystemCertificates::startup_export`]).
    startup: watch::Sender<Startup>,
    /// Set while Weaver turns the setting on by itself: the export that follows is automatic
    /// whoever hears of the change first.
    automatic_turn_on: AtomicBool,
}

/// The decisions of the start that come before the first check of omp: the age of the bundle and
/// the Zscaler turn-on. Each is made once.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Startup {
    bundle_decided: bool,
    zscaler_decided: bool,
    /// An automatic export started in one of them.
    export_started: bool,
}

/// The app-wide state of System Certificates. Cheap to clone; managed by Tauri.
#[derive(Clone)]
pub struct SystemCertificates {
    shared: Arc<Shared>,
}

impl Default for SystemCertificates {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCertificates {
    /// `Off`, until [`SystemCertificates::start`] tells what the system holds.
    pub fn new() -> Self {
        let (ended, _) = broadcast::channel(16);
        let (exporting, _) = watch::channel(false);
        let (startup, _) = watch::channel(Startup::default());
        Self {
            shared: Arc::new(Shared {
                machine: Mutex::new(Machine::new(None)),
                runtime: OnceLock::new(),
                ended,
                exporting,
                refresh: Mutex::new(None),
                startup,
                automatic_turn_on: AtomicBool::new(false),
            }),
        }
    }

    /// `desktop.APP_STARTED`: reads the bundle on disk and the setting. With the setting on, a
    /// bundle written less than a day ago is `Ready` (the daily refresh is due a day after it was
    /// written); a stale or missing one starts an automatic export. Reads the disk: call it from
    /// the blocking pool. Does nothing when already started.
    pub fn start(&self, runtime: Runtime) {
        let bundle = runtime.ops.bundle_info();
        let enabled = (runtime.enabled)();
        let now = runtime.clock.now_ms();
        {
            // The Machine is replaced under its lock, before anything can use the runtime.
            let mut machine = self
                .shared
                .machine
                .lock()
                .expect("system certificates state poisoned");
            if self.shared.runtime.set(runtime).is_err() {
                return;
            }
            *machine = Machine::new(bundle);
        }
        let started = self
            .shared
            .update(|machine| {
                let ticket = machine.app_started(enabled, now);
                (ticket.is_some(), ticket)
            })
            .unwrap_or(false);
        if enabled && !started {
            // Ready without an export: a day after the bundle was written.
            let written = bundle.map_or(now, |bundle| bundle.written_ms);
            let due = written.saturating_add(duration_ms(REFRESH_EVERY)) - now;
            self.shared
                .arm_refresh(Duration::from_millis(u64::try_from(due).unwrap_or(0)));
        }
        self.shared.startup.send_modify(|startup| {
            startup.bundle_decided = true;
            startup.export_started |= started;
        });
    }

    /// The status the UI shows.
    pub fn status(&self) -> Status {
        self.shared
            .machine
            .lock()
            .expect("system certificates state poisoned")
            .status()
            .clone()
    }

    /// `developer.CHANGE_SETTINGS`: the `systemCertificates.enabled` setting is now `enabled`.
    /// Turned on, an export starts (not automatic); turned off, the state goes `Off` and an
    /// export that is running is ignored when it ends. Telling it again the value in force
    /// changes nothing. While Weaver turns the setting on by itself (see
    /// [`SystemCertificates::automatic_turn_on`]) the change to on is held back: the export
    /// starts with [`SystemCertificates::request_export`], after the notice is shown, or when the
    /// guard ends.
    pub fn setting_changed(&self, enabled: bool) {
        if enabled && self.shared.automatic_turn_on.load(Ordering::SeqCst) {
            return;
        }
        self.shared
            .update(|machine| ((), machine.setting_changed(enabled)));
    }

    /// While the guard lives, the setting turned on is turned on by Weaver (the Zscaler turn-on):
    /// the settings follower does not start the export, so that nothing exports before the
    /// banners are shown (`requirements/tech#system-certificates`, "Order"). The export is asked
    /// with `request_export(true)`, which hears of the setting in force by itself. When the guard
    /// ends the Machine is brought up to date with the setting, so that a turn-on that stopped
    /// half way is never lost.
    pub fn automatic_turn_on(&self) -> AutomaticTurnOn {
        self.shared.automatic_turn_on.store(true, Ordering::SeqCst);
        AutomaticTurnOn(Arc::clone(&self.shared))
    }

    /// Weaver turned the setting on by itself (`system-certificates.AUTO_ENABLED`): the notice
    /// shows in every window, and in the windows that open later, until it is closed.
    pub fn show_turn_on_notice(&self) {
        self.shared.update(|machine| {
            machine.show_turn_on_notice();
            ((), None)
        });
    }

    /// The developer closed the turn-on notice, or went to Settings from it: it closes in every
    /// window.
    pub fn dismiss_turn_on_notice(&self) {
        self.shared.update(|machine| {
            machine.dismiss_turn_on_notice();
            ((), None)
        });
    }

    /// The developer closed the banner of the failed automatic export: it closes in every window.
    pub fn dismiss_failure_banner(&self) {
        self.shared.update(|machine| {
            machine.dismiss_failure_banner();
            ((), None)
        });
    }

    /// `developer.REFRESH_SYSTEM_CERTIFICATES` (`automatic: false`) and the exports the app starts
    /// by itself (`automatic: true`: start, Zscaler turn-on, daily refresh). The one entry point
    /// of every export: when one runs the request joins it and starts nothing, and a request that
    /// is automatic makes the running export automatic. With the setting off the request is
    /// refused. Reads the setting in force, so call it from the blocking pool.
    pub fn request_export(&self, automatic: bool) -> Result<(), SettingIsOff> {
        let Some(runtime) = self.shared.runtime.get() else {
            return Err(SettingIsOff);
        };
        // The setting in force, so that a request that comes before the change event of the
        // setting is not refused.
        let enabled = (runtime.enabled)();
        let request = self.shared.update(|machine| {
            let started = machine.setting_changed(enabled);
            match machine.request_export(automatic) {
                Request::Started(ticket) => (Request::Started(ticket), Some(ticket)),
                other => (other, started),
            }
        });
        match request {
            Some(Request::Refused) | None => Err(SettingIsOff),
            Some(Request::Started(_) | Request::Joined) => Ok(()),
        }
    }

    /// `system-certificates.EXPORT_ENDED`: every export that ends while the setting is on, whether
    /// it succeeded or not. An export that ends after the setting was turned off is not sent.
    /// A receiver that falls behind skips to the latest.
    pub fn subscribe_ended(&self) -> broadcast::Receiver<ExportEnded> {
        self.shared.ended.subscribe()
    }

    /// Whether an export runs now.
    pub fn exporting(&self) -> bool {
        *self.shared.exporting.borrow()
    }

    /// Waits until no export runs (at once when none does). An export ends within 30 s.
    pub async fn wait_idle(&self) {
        let mut receiver = self.shared.exporting.subscribe();
        // The sender lives as long as `self`.
        let _ = receiver.wait_for(|exporting| !*exporting).await;
    }

    /// The start has decided the age of the bundle (called by [`SystemCertificates::start`]; call
    /// it too when the step that starts the Machine failed, so that nobody waits for it).
    pub fn bundle_decided(&self) {
        self.shared
            .startup
            .send_modify(|startup| startup.bundle_decided = true);
    }

    /// The start has decided the Zscaler turn-on; `export_started` tells whether the turn-on
    /// started the export. Call it once, also when looking for Zscaler failed.
    pub fn zscaler_decided(&self, export_started: bool) {
        self.shared.startup.send_modify(|startup| {
            startup.zscaler_decided = true;
            startup.export_started |= export_started;
        });
    }

    /// **The signal for the first check of omp** (`requirements/tech#system-certificates`, "Start
    /// order"): resolves when the start has decided the age of the bundle and the Zscaler
    /// turn-on, which look at file metadata only and never wait for an export. `running` tells
    /// whether an export started with the app and is still running then; if it is, `ended`
    /// waits for its end. `limit` bounds the whole wait, decisions and export together (omp
    /// passes [`certificates::EXPORT_LIMIT`], 30 s): after it the answer is whatever is known, and
    /// `ended` does not wait any longer.
    pub async fn startup_export(&self, limit: Duration) -> StartupExport {
        let deadline = tokio::time::Instant::now() + limit;
        let mut receiver = self.shared.startup.subscribe();
        // The sender lives as long as `self`.
        let decided = tokio::time::timeout_at(
            deadline,
            receiver.wait_for(|startup| startup.bundle_decided && startup.zscaler_decided),
        )
        .await
        .ok()
        .and_then(Result::ok)
        .map(|startup| *startup);
        let started = decided.is_some_and(|startup| startup.export_started);
        StartupExport {
            service: self.clone(),
            running: started && self.exporting(),
            deadline,
        }
    }
}

/// What [`SystemCertificates::startup_export`] found out.
pub struct StartupExport {
    service: SystemCertificates,
    /// An export started with the app (after the age of the bundle or the Zscaler turn-on) and
    /// was still running when the decisions were made.
    pub running: bool,
    deadline: tokio::time::Instant,
}

impl StartupExport {
    /// Waits for the end of the export of the start, or for the limit. Returns at once when no
    /// export is running.
    pub async fn ended(&self) {
        if self.running {
            // The limit is not an error: omp checks anyway, with the bundle that is there.
            let _ = tokio::time::timeout_at(self.deadline, self.service.wait_idle()).await;
        }
    }
}

/// See [`SystemCertificates::automatic_turn_on`].
pub struct AutomaticTurnOn(Arc<Shared>);

impl Drop for AutomaticTurnOn {
    fn drop(&mut self) {
        self.0.automatic_turn_on.store(false, Ordering::SeqCst);
        // The changes of the setting held back meanwhile: the Machine learns the setting in
        // force. After `request_export` it already has it and nothing happens.
        if let Some(runtime) = self.0.runtime.get() {
            let enabled = (runtime.enabled)();
            self.0
                .update(|machine| ((), machine.setting_changed(enabled)));
        }
    }
}

fn duration_ms(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

impl Shared {
    /// Applies `change` to the Machine; tells the windows when the status changed; starts the
    /// export when `change` asks for one. `None` before the start.
    fn update<R>(
        self: &Arc<Self>,
        change: impl FnOnce(&mut Machine) -> (R, Option<Ticket>),
    ) -> Option<R> {
        let runtime = self.runtime.get()?;
        let (result, ticket, status, changed) = {
            let mut machine = self
                .machine
                .lock()
                .expect("system certificates state poisoned");
            let before = machine.status().revision;
            let (result, ticket) = change(&mut machine);
            // In the lock, so that the watch never ends on an older value.
            self.exporting.send_replace(machine.exporting());
            if !machine.enabled() {
                // The setting is off: no daily refresh until an export ends with it on.
                self.refresh
                    .lock()
                    .expect("system certificates timer poisoned")
                    .take();
            }
            let status = machine.status().clone();
            let changed = status.revision != before;
            (result, ticket, status, changed)
        };
        if changed {
            (runtime.emit)(&status);
        }
        if let Some(ticket) = ticket {
            let shared = Arc::clone(self);
            let automatic = status.automatic;
            runtime
                .scheduler
                .spawn(Box::new(move || shared.run_export(ticket, automatic)));
        }
        Some(result)
    }

    /// Runs one export (on the blocking pool) and reports how it ended. A panic in the work is a
    /// failure of the export, never a stuck Machine.
    fn run_export(self: &Arc<Self>, ticket: Ticket, automatic: bool) {
        let runtime = self.runtime.get().expect("exports start after start");
        let started = Instant::now();
        let result = guard(|| runtime.ops.export())
            .unwrap_or_else(|| Err("the export failed unexpectedly".to_owned()));
        let took = started.elapsed().as_millis();
        let kind = if automatic { "automatic" } else { "asked" };
        match &result {
            Ok(bundle) => log_info!(
                "system certificates export ({kind}) succeeded: {} certificates in {took} ms",
                bundle.count
            ),
            Err(reason) => {
                log_info!("system certificates export ({kind}) failed in {took} ms: {reason}")
            }
        }
        let ended = self
            .update(|machine| (machine.export_ended(ticket, result), None))
            .flatten();
        if ended.is_some() {
            // A day after this export ended, successful or not.
            self.arm_refresh(REFRESH_EVERY);
        }
        match ended {
            // No receiver is not an error: nobody listens yet.
            Some(ended) => drop(self.ended.send(ended)),
            None => {
                log_info!("system certificates export ({kind}) ignored: the setting was turned off")
            }
        }
    }

    /// Starts the timer of `time.CERTIFICATES_REFRESH_DUE`, `first` from now and then every day,
    /// replacing the earlier one. Nothing while the setting is off.
    fn arm_refresh(self: &Arc<Self>, first: Duration) {
        let Some(runtime) = self.runtime.get() else {
            return;
        };
        let machine = self
            .machine
            .lock()
            .expect("system certificates state poisoned");
        if !machine.enabled() {
            return;
        }
        let weak: Weak<Shared> = Arc::downgrade(self);
        let timer = runtime.scheduler.repeat(
            first,
            REFRESH_EVERY,
            Arc::new(move || {
                if let Some(shared) = weak.upgrade() {
                    shared.refresh_due();
                }
            }),
        );
        *self
            .refresh
            .lock()
            .expect("system certificates timer poisoned") = Some(timer);
    }

    /// `time.CERTIFICATES_REFRESH_DUE`: from `Ready` and `Failed` an automatic export starts.
    fn refresh_due(self: &Arc<Self>) {
        log_info!("system certificates: the daily refresh is due");
        self.update(|machine| ((), machine.refresh_due()));
    }
}

/// Runs `work`; a panic is logged by the panic hook and comes back as `None`.
fn guard<T>(work: impl FnOnce() -> T) -> Option<T> {
    catch_unwind(AssertUnwindSafe(work)).ok()
}

// --- the real system ----------------------------------------------------------------------------

/// The real export: the certificates the system trusts, Mozilla's public roots, the bundle file.
pub struct SystemOps {
    bundle: PathBuf,
}

impl SystemOps {
    pub fn new(bundle: PathBuf) -> Self {
        Self { bundle }
    }
}

impl Ops for SystemOps {
    fn export(&self) -> Result<BundleInfo, String> {
        certificates::export(&self.bundle)
    }

    fn bundle_info(&self) -> Option<BundleInfo> {
        certificates::read_bundle_info(&self.bundle)
    }
}

/// The variables of an omp process, read from the setting in force and the bundle path: the one
/// function every spawn of an omp process uses (the commands of `requirements/tech#omp`, jobs,
/// General and their resumes, the terminals of the installer and of the logins). Empty with the
/// setting off or without a bundle. Never register the result with a job.
pub fn variables(state: &AppState) -> Variables {
    certificates::variables(
        state.settings.current().system_certificates.enabled,
        &state.dirs.system_certificates_file(),
    )
}

/// The startup step `system certificates`: reads the bundle on disk and the setting, starts the
/// Machine, follows later changes of the setting, and starts the Zscaler turn-on as a task of its
/// own (outside the steps, so that what goes wrong in it is only logged). Runs on the blocking
/// pool.
pub fn start(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    // Subscribe before reading, so a change between the two is not lost.
    let mut changes = state.settings.subscribe();
    let service = app.state::<SystemCertificates>().inner().clone();
    // Whatever goes wrong below, the first check of omp does not wait for decisions that will
    // never be made.
    let decisions = StartupDecisions {
        service: service.clone(),
        zscaler_export: false,
    };
    let enabled_handle = app.clone();
    let emit_handle = app.clone();
    service.start(Runtime {
        ops: Arc::new(SystemOps::new(state.dirs.system_certificates_file())),
        scheduler: Arc::new(TauriScheduler),
        clock: Arc::new(SystemClock),
        enabled: Arc::new(move || {
            enabled_handle
                .state::<AppState>()
                .settings
                .current()
                .system_certificates
                .enabled
        }),
        emit: Arc::new(move |status| {
            if let Err(error) = events::emit_system_certificates_changed(&emit_handle, status) {
                crate::log_error!("cannot tell the windows about System Certificates: {error}");
            }
        }),
    });

    let settings_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        use tokio::sync::broadcast::error::RecvError;
        loop {
            match changes.recv().await {
                Ok(change)
                    if change
                        .changed
                        .contains(&SettingKey::SystemCertificatesEnabled) =>
                {
                    service.setting_changed(change.settings.system_certificates.enabled);
                }
                Ok(_) => {}
                // Fell behind: look at the setting as it is now.
                Err(RecvError::Lagged(_)) => {
                    let handle = settings_handle.clone();
                    match blocking::disk(move || handle.state::<AppState>().settings.current())
                        .await
                    {
                        Ok(current) => {
                            service.setting_changed(current.system_certificates.enabled);
                        }
                        Err(error) => log_warn!("cannot read the settings again: {error}"),
                    }
                }
                Err(RecvError::Closed) => break,
            }
        }
    });

    // The Zscaler turn-on: after the Machine started (it asks it to export) and the follower
    // of the setting is listening. Looking for Zscaler never holds up anything.
    let turn_on_handle = app.clone();
    drop(tauri::async_runtime::spawn_blocking(move || {
        let mut decisions = decisions;
        decisions.zscaler_export = turn_on::run_at_start(&turn_on_handle);
    }));
    Ok(())
}

/// Tells [`SystemCertificates`] that the decisions of the start are made, when it is dropped,
/// also while a panic unwinds.
struct StartupDecisions {
    service: SystemCertificates,
    /// The Zscaler turn-on started the export.
    zscaler_export: bool,
}

impl Drop for StartupDecisions {
    fn drop(&mut self) {
        self.service.bundle_decided();
        self.service.zscaler_decided(self.zscaler_export);
    }
}

// --- commands -------------------------------------------------------------------------------------

/// The status of System Certificates, for a window that just loaded; later changes arrive as
/// `system-certificates-changed`.
#[tauri::command]
pub async fn system_certificates_status(
    service: State<'_, SystemCertificates>,
) -> Result<Status, AppError> {
    Ok(service.status())
}

/// **Refresh**: exports the certificates again. Returns at once with the status; the progress and
/// the outcome arrive as `system-certificates-changed`. Refused (`invalid-input`) while the
/// setting is off; while an export runs it joins that one.
#[tauri::command]
pub async fn system_certificates_refresh(
    service: State<'_, SystemCertificates>,
) -> Result<Status, AppError> {
    let service = service.inner().clone();
    blocking::disk_named("system_certificates_refresh", None, move || {
        service
            .request_export(false)
            .map(|()| service.status())
            .map_err(|SettingIsOff| {
                AppError::new(
                    ErrorCode::InvalidInput,
                    "System Certificates is off: turn it on in Settings first.",
                )
            })
    })
    .await?
}

/// **Close** on the turn-on notice, or **Open Settings** on it: the notice closes in every window.
#[tauri::command]
pub async fn system_certificates_dismiss_notice(
    service: State<'_, SystemCertificates>,
) -> Result<Status, AppError> {
    service.dismiss_turn_on_notice();
    Ok(service.status())
}

/// **Close** on the banner of a failed automatic export: it closes in every window.
#[tauri::command]
pub async fn system_certificates_dismiss_failure(
    service: State<'_, SystemCertificates>,
) -> Result<Status, AppError> {
    service.dismiss_failure_banner();
    Ok(service.status())
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    use weaver_core::paths::AppDirs;
    use weaver_core::project::clock::ManualClock;
    use weaver_core::settings::{SettingChange, SettingKey};
    use weaver_core::system_certificates::Phase;

    use super::*;

    const NOW: i64 = 1_800_000_000_000;
    const DAY_MS: i64 = 24 * 60 * 60 * 1000;

    fn bundle(count: u32, written_ms: i64) -> BundleInfo {
        BundleInfo { count, written_ms }
    }

    /// An export that never runs by itself: the test runs it with [`Rig::run_export`].
    #[derive(Default)]
    struct FakeOps {
        results: Mutex<VecDeque<Result<BundleInfo, String>>>,
        exports: AtomicUsize,
        on_disk: Option<BundleInfo>,
        panics: AtomicBool,
    }

    impl Ops for FakeOps {
        fn export(&self) -> Result<BundleInfo, String> {
            self.exports.fetch_add(1, Ordering::SeqCst);
            if self.panics.load(Ordering::SeqCst) {
                panic!("the reader exploded");
            }
            self.results
                .lock()
                .unwrap()
                .pop_front()
                .expect("a result was prepared")
        }

        fn bundle_info(&self) -> Option<BundleInfo> {
            self.on_disk
        }
    }

    /// A timer the test fires by hand; it is `cancelled` when the service drops it.
    struct FakeTimer {
        first: Duration,
        period: Duration,
        tick: Arc<dyn Fn() + Send + Sync>,
        cancelled: Arc<AtomicBool>,
    }

    struct TimerHandle(Arc<AtomicBool>);

    impl Drop for TimerHandle {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[derive(Default)]
    struct ManualScheduler {
        jobs: Mutex<VecDeque<Box<dyn FnOnce() + Send>>>,
        timers: Mutex<Vec<FakeTimer>>,
    }

    impl Scheduler for ManualScheduler {
        fn spawn(&self, job: Box<dyn FnOnce() + Send>) {
            self.jobs.lock().unwrap().push_back(job);
        }

        fn repeat(
            &self,
            first: Duration,
            period: Duration,
            tick: Arc<dyn Fn() + Send + Sync>,
        ) -> Box<dyn Any + Send> {
            let cancelled = Arc::new(AtomicBool::new(false));
            self.timers.lock().unwrap().push(FakeTimer {
                first,
                period,
                tick,
                cancelled: cancelled.clone(),
            });
            Box::new(TimerHandle(cancelled))
        }
    }

    impl ManualScheduler {
        /// The timers that were not dropped, as `(first, period)`.
        fn live(&self) -> Vec<(Duration, Duration)> {
            self.timers
                .lock()
                .unwrap()
                .iter()
                .filter(|timer| !timer.cancelled.load(Ordering::SeqCst))
                .map(|timer| (timer.first, timer.period))
                .collect()
        }

        /// Fires the timer that is alive, as the clock would a day later.
        fn fire(&self) {
            let tick = self
                .timers
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|timer| !timer.cancelled.load(Ordering::SeqCst))
                .map(|timer| timer.tick.clone())
                .expect("a timer is alive");
            tick();
        }
    }

    struct Rig {
        service: SystemCertificates,
        ops: Arc<FakeOps>,
        scheduler: Arc<ManualScheduler>,
        enabled: Arc<AtomicBool>,
        emitted: Arc<Mutex<Vec<Status>>>,
        ended: broadcast::Receiver<ExportEnded>,
    }

    impl Rig {
        fn new(enabled: bool, on_disk: Option<BundleInfo>) -> Self {
            let ops = Arc::new(FakeOps {
                on_disk,
                ..FakeOps::default()
            });
            let scheduler = Arc::new(ManualScheduler::default());
            let enabled = Arc::new(AtomicBool::new(enabled));
            let emitted = Arc::new(Mutex::new(Vec::new()));
            let service = SystemCertificates::new();
            let ended = service.subscribe_ended();
            let setting = Arc::clone(&enabled);
            let sink = Arc::clone(&emitted);
            service.start(Runtime {
                ops: ops.clone(),
                scheduler: scheduler.clone(),
                clock: Arc::new(ManualClock::new(NOW)),
                enabled: Arc::new(move || setting.load(Ordering::SeqCst)),
                emit: Arc::new(move |status| sink.lock().unwrap().push(status.clone())),
            });
            Self {
                service,
                ops,
                scheduler,
                enabled,
                emitted,
                ended,
            }
        }

        fn will_export(&self, result: Result<BundleInfo, String>) {
            self.ops.results.lock().unwrap().push_back(result);
        }

        fn queued(&self) -> usize {
            self.scheduler.jobs.lock().unwrap().len()
        }

        /// Runs the export that was started, as the blocking pool would.
        fn run_export(&self) {
            let job = self
                .scheduler
                .jobs
                .lock()
                .unwrap()
                .pop_front()
                .expect("an export was started");
            job();
        }

        fn turn_setting(&self, enabled: bool) {
            self.enabled.store(enabled, Ordering::SeqCst);
            self.service.setting_changed(enabled);
        }

        fn phases(&self) -> Vec<Phase> {
            self.emitted
                .lock()
                .unwrap()
                .iter()
                .map(|s| s.phase)
                .collect()
        }
    }

    #[test]
    fn nothing_happens_before_the_start() {
        let service = SystemCertificates::new();
        service.setting_changed(true);
        assert_eq!(service.status().phase, Phase::Off);
        assert_eq!(service.request_export(false), Err(SettingIsOff));
        assert!(!service.exporting());
    }

    #[test]
    fn the_start_with_the_setting_off_exports_nothing_and_tells_nobody() {
        let rig = Rig::new(false, Some(bundle(150, NOW - 2 * DAY_MS)));
        assert_eq!(rig.service.status().phase, Phase::Off);
        assert_eq!(rig.queued(), 0);
        assert!(rig.emitted.lock().unwrap().is_empty());
    }

    #[test]
    fn the_start_with_a_fresh_bundle_is_ready_without_an_export() {
        let rig = Rig::new(true, Some(bundle(150, NOW - DAY_MS)));
        assert_eq!(rig.service.status().phase, Phase::Ready);
        assert_eq!(rig.service.status().count, Some(150));
        assert_eq!(rig.queued(), 0);
        assert_eq!(rig.phases(), [Phase::Ready]);
    }

    #[test]
    fn the_start_with_a_stale_bundle_exports_automatically_and_tells_omp_at_the_end() {
        let mut rig = Rig::new(true, Some(bundle(150, NOW - DAY_MS - 1)));
        assert_eq!(rig.service.status().phase, Phase::Exporting);
        assert!(rig.service.status().automatic);
        assert!(rig.service.exporting());
        rig.will_export(Ok(bundle(200, NOW)));
        rig.run_export();
        assert_eq!(rig.service.status().phase, Phase::Ready);
        assert_eq!(rig.service.status().count, Some(200));
        assert!(!rig.service.exporting());
        assert_eq!(
            rig.ended.try_recv().unwrap(),
            ExportEnded {
                automatic: true,
                error: None
            }
        );
        assert!(rig.ended.try_recv().is_err());
    }

    #[test]
    fn turning_the_setting_on_exports_once_and_tells_every_window_each_change() {
        let mut rig = Rig::new(false, None);
        rig.will_export(Ok(bundle(120, NOW)));
        rig.turn_setting(true);
        assert_eq!(rig.queued(), 1);
        assert!(!rig.service.status().automatic);
        rig.run_export();
        assert_eq!(rig.phases(), [Phase::Exporting, Phase::Ready]);
        let revisions: Vec<u64> = rig
            .emitted
            .lock()
            .unwrap()
            .iter()
            .map(|s| s.revision)
            .collect();
        assert!(
            revisions.windows(2).all(|pair| pair[0] < pair[1]),
            "{revisions:?}"
        );
        assert_eq!(
            rig.ended.try_recv().unwrap(),
            ExportEnded {
                automatic: false,
                error: None
            }
        );
        assert_eq!(rig.ops.exports.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_failed_export_is_failed_with_the_reason_and_is_still_an_ended_export() {
        let mut rig = Rig::new(false, Some(bundle(150, NOW)));
        rig.will_export(Err("no certificate authority found".into()));
        rig.turn_setting(true);
        rig.run_export();
        let status = rig.service.status();
        assert_eq!(status.phase, Phase::Failed);
        assert_eq!(
            status.error.as_deref(),
            Some("no certificate authority found")
        );
        assert_eq!(status.count, Some(150));
        assert_eq!(
            rig.ended.try_recv().unwrap().error.as_deref(),
            Some("no certificate authority found")
        );
    }

    #[test]
    fn a_request_while_an_export_runs_joins_it_and_starts_nothing() {
        let rig = Rig::new(false, None);
        rig.will_export(Ok(bundle(1, NOW)));
        rig.turn_setting(true);
        for automatic in [false, true, false] {
            assert_eq!(rig.service.request_export(automatic), Ok(()));
        }
        assert_eq!(rig.queued(), 1, "one export only");
        // The request that was automatic made the running export automatic.
        assert!(rig.service.status().automatic);
        rig.run_export();
        assert_eq!(rig.ops.exports.load(Ordering::SeqCst), 1);
        assert_eq!(rig.service.status().phase, Phase::Ready);
    }

    #[test]
    fn refresh_from_ready_starts_one_export_and_is_refused_with_the_setting_off() {
        let mut rig = Rig::new(true, Some(bundle(150, NOW)));
        assert_eq!(rig.service.request_export(false), Ok(()));
        assert_eq!(rig.queued(), 1);
        assert_eq!(rig.service.status().phase, Phase::Exporting);
        assert!(!rig.service.status().automatic);
        rig.will_export(Ok(bundle(151, NOW)));
        rig.run_export();
        assert!(rig.ended.try_recv().is_ok());

        rig.turn_setting(false);
        assert_eq!(rig.service.request_export(false), Err(SettingIsOff));
        assert_eq!(rig.queued(), 0);
        assert_eq!(rig.service.status().phase, Phase::Off);
    }

    #[test]
    fn a_request_before_the_change_event_of_the_setting_is_not_refused() {
        let rig = Rig::new(false, None);
        rig.will_export(Ok(bundle(1, NOW)));
        // The setting is on, but the Machine has not heard of it yet.
        rig.enabled.store(true, Ordering::SeqCst);
        assert_eq!(rig.service.request_export(true), Ok(()));
        assert_eq!(rig.queued(), 1);
        assert!(rig.service.status().automatic);
        // The change event that comes after starts nothing more.
        rig.service.setting_changed(true);
        assert_eq!(rig.queued(), 1);
    }

    #[test]
    fn an_export_that_ends_after_the_setting_was_turned_off_changes_and_emits_nothing() {
        let mut rig = Rig::new(false, None);
        rig.will_export(Ok(bundle(1, NOW)));
        rig.turn_setting(true);
        rig.turn_setting(false);
        let emitted = rig.emitted.lock().unwrap().len();
        rig.run_export();
        assert_eq!(rig.service.status().phase, Phase::Off);
        assert_eq!(rig.emitted.lock().unwrap().len(), emitted);
        assert!(rig.ended.try_recv().is_err());
        assert!(!rig.service.exporting());
        // The Machine is not stuck: turning the setting on again exports again.
        rig.will_export(Ok(bundle(2, NOW)));
        rig.turn_setting(true);
        rig.run_export();
        assert_eq!(rig.service.status().count, Some(2));
        assert!(rig.ended.try_recv().is_ok());
    }

    #[test]
    fn a_panic_in_the_export_is_a_failure_not_a_stuck_state() {
        let mut rig = Rig::new(false, None);
        rig.ops.panics.store(true, Ordering::SeqCst);
        rig.turn_setting(true);
        rig.run_export();
        let status = rig.service.status();
        assert_eq!(status.phase, Phase::Failed);
        assert_eq!(
            status.error.as_deref(),
            Some("the export failed unexpectedly")
        );
        assert!(rig.ended.try_recv().is_ok());
        assert!(!rig.service.exporting());
    }

    #[test]
    fn a_second_start_changes_nothing() {
        let rig = Rig::new(true, Some(bundle(150, NOW)));
        let before = rig.service.status();
        rig.service.start(Runtime {
            ops: Arc::new(FakeOps::default()),
            scheduler: rig.scheduler.clone(),
            clock: Arc::new(ManualClock::new(NOW + 10 * DAY_MS)),
            enabled: Arc::new(|| true),
            emit: Arc::new(|_| {}),
        });
        assert_eq!(rig.service.status(), before);
        assert_eq!(rig.queued(), 0);
    }

    #[tokio::test]
    async fn wait_idle_returns_when_the_export_ends() {
        let rig = Rig::new(false, None);
        rig.service.wait_idle().await;
        rig.will_export(Ok(bundle(1, NOW)));
        rig.turn_setting(true);
        let waiting = tokio::spawn({
            let service = rig.service.clone();
            async move { service.wait_idle().await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!waiting.is_finished());
        rig.run_export();
        tokio::time::timeout(Duration::from_secs(5), waiting)
            .await
            .expect("woken")
            .unwrap();
    }

    #[test]
    fn a_ready_start_arms_the_daily_refresh_for_a_day_after_the_bundle_was_written() {
        let rig = Rig::new(true, Some(bundle(150, NOW - 10 * 60 * 60 * 1000)));
        assert_eq!(rig.service.status().phase, Phase::Ready);
        assert_eq!(
            rig.scheduler.live(),
            [(
                Duration::from_secs(14 * 60 * 60),
                Duration::from_secs(24 * 60 * 60)
            )]
        );
        // Nothing exports until the timer fires.
        assert_eq!(rig.queued(), 0);
    }

    #[test]
    fn a_start_with_the_setting_off_or_a_stale_bundle_arms_no_timer_until_an_export_ends() {
        let off = Rig::new(false, Some(bundle(150, NOW)));
        assert!(off.scheduler.live().is_empty());

        let stale = Rig::new(true, Some(bundle(150, NOW - 2 * DAY_MS)));
        assert!(stale.scheduler.live().is_empty(), "an export runs");
        stale.will_export(Ok(bundle(160, NOW)));
        stale.run_export();
        assert_eq!(
            stale.scheduler.live(),
            [(
                Duration::from_secs(24 * 60 * 60),
                Duration::from_secs(24 * 60 * 60)
            )],
            "a day after the export ended"
        );
    }

    #[test]
    fn the_timer_starts_an_automatic_export_from_ready_and_again_from_failed() {
        let mut rig = Rig::new(true, Some(bundle(150, NOW)));
        rig.scheduler.fire();
        assert_eq!(rig.queued(), 1);
        assert_eq!(rig.service.status().phase, Phase::Exporting);
        assert!(rig.service.status().automatic);
        rig.will_export(Err("no certificate authority found".into()));
        rig.run_export();
        assert_eq!(rig.service.status().phase, Phase::Failed);
        assert!(rig.service.status().failure_banner);
        assert!(rig.ended.try_recv().unwrap().automatic);
        // A failed export arms the timer too: a day after it ended.
        assert_eq!(rig.scheduler.live().len(), 1);
        rig.scheduler.fire();
        assert_eq!(rig.queued(), 1, "from Failed it exports again");
        rig.will_export(Ok(bundle(151, NOW)));
        rig.run_export();
        assert_eq!(rig.service.status().phase, Phase::Ready);
        assert!(!rig.service.status().failure_banner);
        assert_eq!(rig.ops.exports.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn the_earlier_timer_is_replaced_when_an_export_ends() {
        let rig = Rig::new(true, Some(bundle(150, NOW)));
        assert_eq!(rig.scheduler.timers.lock().unwrap().len(), 1);
        assert_eq!(rig.service.request_export(false), Ok(()));
        rig.will_export(Ok(bundle(151, NOW)));
        rig.run_export();
        assert_eq!(rig.scheduler.timers.lock().unwrap().len(), 2);
        assert_eq!(rig.scheduler.live().len(), 1, "one timer at a time");
    }

    #[test]
    fn turning_the_setting_off_stops_the_timer_and_turning_it_on_restarts_it_at_the_next_end() {
        let rig = Rig::new(true, Some(bundle(150, NOW)));
        assert_eq!(rig.scheduler.live().len(), 1);
        rig.turn_setting(false);
        assert!(rig.scheduler.live().is_empty());
        rig.will_export(Ok(bundle(151, NOW)));
        rig.turn_setting(true);
        assert!(
            rig.scheduler.live().is_empty(),
            "not while the export runs: a day after it ends"
        );
        rig.run_export();
        assert_eq!(rig.scheduler.live().len(), 1);
    }

    #[test]
    fn an_export_that_ends_after_the_setting_was_turned_off_arms_no_timer() {
        let rig = Rig::new(false, None);
        rig.will_export(Ok(bundle(1, NOW)));
        rig.turn_setting(true);
        rig.turn_setting(false);
        rig.run_export();
        assert!(rig.scheduler.live().is_empty());
    }

    #[test]
    fn a_tick_while_an_asked_export_runs_starts_nothing_and_a_tick_with_the_setting_off_neither() {
        let rig = Rig::new(true, Some(bundle(150, NOW)));
        assert_eq!(rig.service.request_export(false), Ok(()));
        rig.scheduler.fire();
        assert_eq!(rig.queued(), 1, "the export that runs, not a second one");
        assert!(
            !rig.service.status().automatic,
            "the asked export does not become automatic"
        );
        rig.will_export(Ok(bundle(151, NOW)));
        rig.run_export();
        rig.turn_setting(false);
        // A tick that was already on its way when the setting went off.
        let queued = rig.queued();
        rig.service.shared.refresh_due();
        assert_eq!(rig.queued(), queued);
        assert_eq!(rig.service.status().phase, Phase::Off);
    }

    #[test]
    fn nothing_exports_before_the_notice_when_weaver_turns_the_setting_on_and_the_export_is_automatic(
    ) {
        let mut rig = Rig::new(false, None);
        let guard = rig.service.automatic_turn_on();
        rig.will_export(Ok(bundle(1, NOW)));
        // The setting is written and the settings follower hears of it first.
        rig.turn_setting(true);
        assert_eq!(rig.queued(), 0, "no export before the notice");
        assert_eq!(rig.service.status().phase, Phase::Off);
        // The notice goes out, then the export is asked.
        rig.service.show_turn_on_notice();
        assert_eq!(rig.queued(), 0);
        assert_eq!(rig.service.request_export(true), Ok(()));
        assert_eq!(rig.queued(), 1, "one export");
        assert!(rig.service.status().automatic);
        // The follower hearing again, late, starts nothing more.
        rig.service.setting_changed(true);
        drop(guard);
        assert_eq!(rig.queued(), 1);
        rig.run_export();
        assert!(rig.ended.try_recv().unwrap().automatic);
    }

    #[test]
    fn when_the_guard_ends_without_an_export_asked_the_machine_learns_the_setting() {
        let rig = Rig::new(false, None);
        let guard = rig.service.automatic_turn_on();
        rig.turn_setting(true);
        assert_eq!(rig.queued(), 0);
        // A turn-on that stopped half way: the setting is on and nothing was asked.
        drop(guard);
        assert_eq!(rig.queued(), 1);
        assert_eq!(rig.service.status().phase, Phase::Exporting);
    }

    #[test]
    fn the_developer_turning_the_setting_on_after_that_is_not_automatic() {
        let rig = Rig::new(false, None);
        drop(rig.service.automatic_turn_on());
        rig.will_export(Ok(bundle(1, NOW)));
        rig.turn_setting(true);
        assert!(!rig.service.status().automatic);
    }

    #[test]
    fn the_turn_on_notice_is_in_the_status_of_every_window_until_it_is_closed() {
        let rig = Rig::new(false, None);
        let before = rig.emitted.lock().unwrap().len();
        rig.service.show_turn_on_notice();
        assert!(rig.service.status().turn_on_notice);
        assert_eq!(
            rig.emitted.lock().unwrap().len(),
            before + 1,
            "every window is told, once"
        );
        // A window that opens later reads it with the status command.
        assert!(rig.service.status().turn_on_notice);
        rig.service.dismiss_turn_on_notice();
        assert!(!rig.service.status().turn_on_notice);
        assert!(!rig.emitted.lock().unwrap().last().unwrap().turn_on_notice);
    }

    #[test]
    fn the_failure_banner_is_in_the_status_and_closing_it_tells_every_window() {
        let rig = Rig::new(true, Some(bundle(150, NOW - 2 * DAY_MS)));
        rig.will_export(Err("no certificate authority found".into()));
        rig.run_export();
        let status = rig.service.status();
        assert!(status.failure_banner && status.phase == Phase::Failed);
        rig.service.dismiss_failure_banner();
        assert!(!rig.service.status().failure_banner);
        assert!(!rig.emitted.lock().unwrap().last().unwrap().failure_banner);
        // The line in Settings stays: only the banner went.
        assert_eq!(rig.service.status().phase, Phase::Failed);
        // Asked from Settings: the line only, no banner.
        rig.will_export(Err("could not read".into()));
        assert_eq!(rig.service.request_export(false), Ok(()));
        rig.run_export();
        assert!(!rig.service.status().failure_banner);
    }

    #[tokio::test]
    async fn the_startup_signal_waits_for_both_decisions_then_reports_the_export_that_runs() {
        let rig = Rig::new(true, Some(bundle(150, NOW - 2 * DAY_MS)));
        assert!(rig.service.exporting());
        let waiting = tokio::spawn({
            let service = rig.service.clone();
            async move { service.startup_export(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!waiting.is_finished(), "Zscaler is not decided yet");
        rig.service.zscaler_decided(false);
        let startup = tokio::time::timeout(Duration::from_secs(5), waiting)
            .await
            .expect("decided")
            .unwrap();
        assert!(startup.running);
        // It resolves at the end of the export, not before.
        let ended = tokio::spawn(async move { startup.ended().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!ended.is_finished());
        rig.will_export(Ok(bundle(160, NOW)));
        rig.run_export();
        tokio::time::timeout(Duration::from_secs(5), ended)
            .await
            .expect("woken by the end of the export")
            .unwrap();
    }

    #[tokio::test]
    async fn the_startup_signal_says_no_export_when_the_bundle_is_fresh_and_does_not_wait() {
        let rig = Rig::new(true, Some(bundle(150, NOW)));
        rig.service.zscaler_decided(false);
        let startup = rig.service.startup_export(Duration::from_secs(5)).await;
        assert!(!startup.running);
        tokio::time::timeout(Duration::from_secs(1), startup.ended())
            .await
            .expect("returns at once");
        let off = Rig::new(false, None);
        off.service.zscaler_decided(false);
        assert!(
            !off.service
                .startup_export(Duration::from_secs(5))
                .await
                .running
        );
    }

    #[tokio::test]
    async fn the_startup_signal_sees_the_export_the_zscaler_turn_on_started() {
        let rig = Rig::new(false, None);
        let guard = rig.service.automatic_turn_on();
        rig.turn_setting(true);
        assert_eq!(rig.service.request_export(true), Ok(()));
        drop(guard);
        rig.service.zscaler_decided(true);
        let startup = rig.service.startup_export(Duration::from_secs(5)).await;
        assert!(startup.running);
        rig.will_export(Ok(bundle(1, NOW)));
        rig.run_export();
        tokio::time::timeout(Duration::from_secs(1), startup.ended())
            .await
            .expect("the export ended");
    }

    #[tokio::test]
    async fn the_startup_signal_gives_up_after_the_limit_and_the_wait_for_the_export_too() {
        // The decision of Zscaler never comes: after the limit the answer is "no export".
        let rig = Rig::new(true, Some(bundle(150, NOW - 2 * DAY_MS)));
        let started = std::time::Instant::now();
        let startup = rig.service.startup_export(Duration::from_millis(60)).await;
        assert!(started.elapsed() >= Duration::from_millis(50));
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(!startup.running);

        // The export never ends: the wait ends with the limit.
        rig.service.zscaler_decided(false);
        let startup = rig.service.startup_export(Duration::from_millis(60)).await;
        assert!(startup.running);
        let waited = std::time::Instant::now();
        tokio::time::timeout(Duration::from_secs(5), startup.ended())
            .await
            .expect("the limit ends the wait");
        assert!(waited.elapsed() < Duration::from_secs(5));
        assert!(rig.service.exporting(), "the export itself is not stopped");
    }

    #[test]
    fn the_variables_come_from_the_setting_and_the_bundle_of_the_app_state() {
        let dir = tempfile::tempdir().unwrap();
        let dirs = AppDirs::from_bases(&dir.path().join("config"), &dir.path().join("data"), None);
        let state = AppState::new(dirs.clone());
        // Setting off, no bundle.
        assert!(variables(&state).is_empty());
        state
            .settings
            .apply(&SettingChange {
                key: SettingKey::SystemCertificatesEnabled,
                value: "true".into(),
            })
            .unwrap();
        // Setting on, no bundle yet.
        assert!(variables(&state).is_empty());
        let file = dirs.system_certificates_file();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "pem").unwrap();
        let on = variables(&state);
        assert_eq!(on.len(), 10);
        assert!(on
            .iter()
            .any(|(name, value)| *name == "SSL_CERT_FILE" && value == file.as_os_str()));
        // Setting off again: the file stays on disk, unused.
        state
            .settings
            .apply(&SettingChange {
                key: SettingKey::SystemCertificatesEnabled,
                value: "false".into(),
            })
            .unwrap();
        assert!(variables(&state).is_empty());
        assert!(file.exists());
    }

    #[test]
    fn the_settings_service_tells_the_change_of_the_boolean_setting() {
        let dir = tempfile::tempdir().unwrap();
        let settings = crate::settings::SettingsService::at(dir.path().join("settings.json"));
        let mut changes = settings.subscribe();
        settings
            .apply(&SettingChange {
                key: SettingKey::SystemCertificatesEnabled,
                value: "true".into(),
            })
            .unwrap();
        let heard = changes.try_recv().unwrap();
        assert_eq!(heard.changed, [SettingKey::SystemCertificatesEnabled]);
        assert!(heard.settings.system_certificates.enabled);
    }
}
