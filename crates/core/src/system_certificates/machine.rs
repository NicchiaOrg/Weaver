//! The Machine `system-certificates`: whether omp trusts the certificate authorities the system
//! trusts, one for the whole app.
//!
//! Pure: calls in, the new [`Status`] and the work to start out. The app runs the work
//! (`crate::system_certificates`), reports its end with [`Machine::export_ended`] and sends every
//! change of the status to every window.
//!
//! | State | Phase | Meaning |
//! |-------|-------|---------|
//! | `Off` | [`Phase::Off`] | The setting is off; a bundle left on disk is unused |
//! | `On`/`Exporting` | [`Phase::Exporting`] | An export runs; the earlier bundle, if any, stays in use |
//! | `On`/`Ready` | [`Phase::Ready`] | The bundle exists and was written by the last export |
//! | `On`/`Failed` | [`Phase::Failed`] | The last export failed; the earlier bundle, if any, stays in use |
//!
//! `Phase::Off` holds exactly while the setting is off. At most one export runs at a time: a
//! request while one runs joins it. An export that ends after the setting was turned off (even if
//! it was turned on again since) changes nothing.

use serde::Serialize;
use ts_rs::TS;

use super::{is_stale, BundleInfo};
use crate::project::clock::format_instant;

/// The state of the Machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "SystemCertificatesPhase")]
pub enum Phase {
    Off,
    Exporting,
    Ready,
    Failed,
}

/// Everything the UI shows of the Machine. Sent whole at each change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "SystemCertificatesStatus")]
pub struct Status {
    /// Increases with every change; a window keeps the highest it saw.
    #[ts(type = "number")]
    pub revision: u64,
    pub phase: Phase,
    /// Whether the export that runs, or ended last, was automatic (at start, after Weaver turned
    /// the setting on by itself, daily) and not asked with **Refresh** or by turning the setting
    /// on in Settings. A request that joins an automatic export keeps it automatic.
    pub automatic: bool,
    /// How many certificates the bundle in use holds; `None` without a bundle. Kept while the
    /// setting is off, and while an export runs or failed.
    pub count: Option<u32>,
    /// When the bundle in use was written, as a UTC ISO 8601 instant; `None` without a bundle.
    pub written_at: Option<String>,
    /// Failed: why the last export failed.
    pub error: Option<String>,
    /// Whether the notice that Weaver turned the setting on by itself (because Zscaler is
    /// installed) shows in every window. In memory only: it never comes back after Weaver quits.
    pub turn_on_notice: bool,
    /// Whether the banner of a failed automatic export shows in every window: set when an
    /// automatic export fails, replaced by a newer failure (also one that was asked), cleared
    /// when the developer closes it, when an export succeeds or when the setting is turned off.
    /// It stays while another export runs.
    pub failure_banner: bool,
    /// Why the export the banner speaks of failed; `Some` exactly while `failure_banner`. The
    /// banner says what the failed status line says, with the bundle in use now.
    pub failure_reason: Option<String>,
}

/// One export in progress. Only the export of the current ticket changes the Machine when it ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ticket(u64);

/// What a request to export came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    /// Nothing was running: run the export of this ticket now.
    Started(Ticket),
    /// An export was already running; this request starts nothing.
    Joined,
    /// The setting is off: nothing is exported.
    Refused,
}

/// An export that ended while the setting was on: the app tells the windows and omp
/// (`system-certificates.EXPORT_ENDED`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportEnded {
    /// Whether the export was automatic.
    pub automatic: bool,
    /// Why it failed; `None` when it succeeded.
    pub error: Option<String>,
}

pub struct Machine {
    enabled: bool,
    /// The bundle in use: the one on disk at start, then the one of the last successful export.
    bundle: Option<BundleInfo>,
    running: Option<Ticket>,
    next_ticket: u64,
    turn_on_notice: bool,
    /// The reason of the failure the banner shows; `None` when no banner shows.
    failure_banner: Option<String>,
    status: Status,
}

impl Machine {
    /// `Off`, with the bundle found on disk at start (`None` when there is none).
    pub fn new(bundle: Option<BundleInfo>) -> Self {
        Self {
            enabled: false,
            bundle,
            running: None,
            next_ticket: 0,
            turn_on_notice: false,
            failure_banner: None,
            status: Status {
                revision: 1,
                phase: Phase::Off,
                automatic: false,
                count: bundle.map(|bundle| bundle.count),
                written_at: bundle.map(|bundle| format_instant(bundle.written_ms)),
                error: None,
                turn_on_notice: false,
                failure_banner: false,
                failure_reason: None,
            },
        }
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Whether the Machine is `On` (the setting is on).
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Whether an export runs.
    pub fn exporting(&self) -> bool {
        self.running.is_some()
    }

    /// `desktop.APP_STARTED` with the setting `enabled` and the clock at `now_ms`: from `Off`, with
    /// the setting on, `Ready` when the bundle was written less than a day ago, else an automatic
    /// export. With the setting off, or when already started, nothing happens.
    pub fn app_started(&mut self, enabled: bool, now_ms: i64) -> Option<Ticket> {
        if !enabled || self.enabled {
            return None;
        }
        self.enabled = true;
        if is_stale(self.bundle.map(|bundle| bundle.written_ms), now_ms) {
            return Some(self.start(true));
        }
        self.commit(Phase::Ready, false, None);
        None
    }

    /// `developer.CHANGE_SETTINGS`, with the setting now `enabled`. Turned on: from `Off`, an
    /// export that was not asked automatically starts. Turned off: back to `Off`; a running export
    /// is ignored when it ends. A setting that is already as the Machine has it changes nothing,
    /// so this may be called again with the value in force.
    pub fn setting_changed(&mut self, enabled: bool) -> Option<Ticket> {
        if enabled == self.enabled {
            return None;
        }
        if enabled {
            self.enabled = true;
            return Some(self.start(false));
        }
        self.enabled = false;
        self.running = None;
        self.failure_banner = None;
        self.commit(Phase::Off, false, None);
        None
    }

    /// `developer.REFRESH_SYSTEM_CERTIFICATES` (`automatic: false`) and the automatic exports
    /// (`automatic: true`: the start, the Zscaler turn-on, the daily refresh). From `Ready` and
    /// `Failed` an export starts; while one runs the request joins it and starts nothing (a
    /// request that is automatic makes the running export automatic). With the setting off the
    /// request is refused.
    pub fn request_export(&mut self, automatic: bool) -> Request {
        if !self.enabled {
            return Request::Refused;
        }
        if self.running.is_some() {
            if automatic && !self.status.automatic {
                self.commit(Phase::Exporting, true, None);
            }
            return Request::Joined;
        }
        Request::Started(self.start(automatic))
    }

    /// `time.CERTIFICATES_REFRESH_DUE`: a day passed since the last export ended. From `Ready` and
    /// `Failed` an automatic export starts; while one runs (an export asked since, still going)
    /// and with the setting off, nothing happens.
    pub fn refresh_due(&mut self) -> Option<Ticket> {
        if !self.enabled || self.running.is_some() {
            return None;
        }
        Some(self.start(true))
    }

    /// Weaver turned the setting on by itself: the notice shows in every window.
    pub fn show_turn_on_notice(&mut self) {
        self.turn_on_notice = true;
        self.recommit();
    }

    /// The developer closed the notice, or went to Settings from it: it closes in every window.
    pub fn dismiss_turn_on_notice(&mut self) {
        self.turn_on_notice = false;
        self.recommit();
    }

    /// The developer closed the banner of the failed export: it closes in every window. A newer
    /// failure shows it again.
    pub fn dismiss_failure_banner(&mut self) {
        self.failure_banner = None;
        self.recommit();
    }

    /// The export of `ticket` ended. Ignored (`None`) when the setting was turned off since, or
    /// when it is not the export in progress; else `Ready` with the new bundle, or `Failed` with
    /// the reason and the earlier bundle still in use.
    pub fn export_ended(
        &mut self,
        ticket: Ticket,
        result: Result<BundleInfo, String>,
    ) -> Option<ExportEnded> {
        if !self.enabled || self.running != Some(ticket) {
            return None;
        }
        self.running = None;
        let automatic = self.status.automatic;
        let error = match result {
            Ok(bundle) => {
                self.bundle = Some(bundle);
                self.failure_banner = None;
                self.commit(Phase::Ready, automatic, None);
                None
            }
            Err(reason) => {
                // An automatic failure shows the banner. A failure that was asked only replaces
                // the reason of a banner that is showing: the line in Settings tells the rest.
                if automatic || self.failure_banner.is_some() {
                    self.failure_banner = Some(reason.clone());
                }
                self.commit(Phase::Failed, automatic, Some(reason.clone()));
                Some(reason)
            }
        };
        Some(ExportEnded { automatic, error })
    }

    fn start(&mut self, automatic: bool) -> Ticket {
        self.next_ticket += 1;
        let ticket = Ticket(self.next_ticket);
        self.running = Some(ticket);
        self.commit(Phase::Exporting, automatic, None);
        ticket
    }

    fn build(&self, phase: Phase, automatic: bool, error: Option<String>, revision: u64) -> Status {
        Status {
            revision,
            phase,
            automatic,
            count: self.bundle.map(|bundle| bundle.count),
            written_at: self.bundle.map(|bundle| format_instant(bundle.written_ms)),
            error,
            turn_on_notice: self.turn_on_notice,
            failure_banner: self.failure_banner.is_some(),
            failure_reason: self.failure_banner.clone(),
        }
    }

    /// Commits the status as it is, with the banner flags as they are now.
    fn recommit(&mut self) {
        let Status {
            phase,
            automatic,
            error,
            ..
        } = self.status.clone();
        self.commit(phase, automatic, error);
    }

    /// Makes `next` the status, with the next revision, when it differs from the current one.
    fn commit(&mut self, phase: Phase, automatic: bool, error: Option<String>) {
        let next = self.build(phase, automatic, error, self.status.revision);
        if next != self.status {
            self.status = Status {
                revision: self.status.revision + 1,
                ..next
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY_MS: i64 = 24 * 60 * 60 * 1000;
    const NOW: i64 = 1_800_000_000_000;

    fn bundle(count: u32, written_ms: i64) -> BundleInfo {
        BundleInfo { count, written_ms }
    }

    fn started(request: Request) -> Ticket {
        match request {
            Request::Started(ticket) => ticket,
            other => panic!("expected a started export, got {other:?}"),
        }
    }

    /// A Machine with the setting on, `Ready` with a bundle of `count` certificates.
    fn ready(count: u32) -> Machine {
        let mut machine = Machine::new(Some(bundle(count, NOW)));
        assert_eq!(machine.app_started(true, NOW), None);
        machine
    }

    #[test]
    fn it_starts_off_and_keeps_the_bundle_found_on_disk() {
        let machine = Machine::new(Some(bundle(150, NOW)));
        let status = machine.status();
        assert_eq!(status.phase, Phase::Off);
        assert_eq!(status.count, Some(150));
        assert_eq!(status.written_at.as_deref(), Some(&*format_instant(NOW)));
        assert!(!status.automatic && status.error.is_none());
        assert!(!machine.enabled() && !machine.exporting());
        let none = Machine::new(None);
        assert_eq!(
            (none.status().count, none.status().written_at.clone()),
            (None, None)
        );
    }

    #[test]
    fn the_start_with_the_setting_off_does_nothing() {
        let mut machine = Machine::new(None);
        let revision = machine.status().revision;
        assert_eq!(machine.app_started(false, NOW), None);
        assert_eq!(machine.status().phase, Phase::Off);
        assert_eq!(machine.status().revision, revision);
    }

    #[test]
    fn the_start_with_the_setting_on_and_a_fresh_bundle_is_ready_without_exporting() {
        let mut machine = Machine::new(Some(bundle(150, NOW - DAY_MS)));
        assert_eq!(machine.app_started(true, NOW), None);
        assert_eq!(machine.status().phase, Phase::Ready);
        assert_eq!(machine.status().count, Some(150));
        assert!(machine.enabled() && !machine.exporting());
    }

    #[test]
    fn the_start_with_the_setting_on_and_a_stale_or_missing_bundle_exports_automatically() {
        for found in [None, Some(bundle(150, NOW - DAY_MS - 1))] {
            let mut machine = Machine::new(found);
            assert!(machine.app_started(true, NOW).is_some());
            assert_eq!(machine.status().phase, Phase::Exporting);
            assert!(machine.status().automatic);
            // The earlier bundle stays in use meanwhile.
            assert_eq!(machine.status().count, found.map(|b| b.count));
            assert!(machine.exporting());
        }
    }

    #[test]
    fn turning_the_setting_on_exports_and_it_is_not_automatic() {
        let mut machine = Machine::new(Some(bundle(150, NOW)));
        let ticket = machine.setting_changed(true).expect("an export starts");
        assert_eq!(machine.status().phase, Phase::Exporting);
        assert!(!machine.status().automatic);
        let ended = machine
            .export_ended(ticket, Ok(bundle(200, NOW + 5)))
            .unwrap();
        assert_eq!(
            ended,
            ExportEnded {
                automatic: false,
                error: None
            }
        );
        let status = machine.status();
        assert_eq!(status.phase, Phase::Ready);
        assert_eq!(status.count, Some(200));
        assert_eq!(
            status.written_at.as_deref(),
            Some(&*format_instant(NOW + 5))
        );
        assert_eq!(status.error, None);
    }

    #[test]
    fn a_failed_export_is_failed_with_the_reason_and_the_earlier_bundle_in_use() {
        let mut machine = ready(150);
        let ticket = started(machine.request_export(false));
        let ended = machine
            .export_ended(ticket, Err("no certificate authority found".into()))
            .unwrap();
        assert_eq!(
            ended.error.as_deref(),
            Some("no certificate authority found")
        );
        let status = machine.status();
        assert_eq!(status.phase, Phase::Failed);
        assert_eq!(
            status.error.as_deref(),
            Some("no certificate authority found")
        );
        assert_eq!(status.count, Some(150));
        assert_eq!(status.written_at.as_deref(), Some(&*format_instant(NOW)));
    }

    #[test]
    fn a_failed_export_without_an_earlier_bundle_has_none_in_use() {
        let mut machine = Machine::new(None);
        let ticket = machine.setting_changed(true).unwrap();
        machine.export_ended(ticket, Err("the export did not finish within 30 s".into()));
        let status = machine.status();
        assert_eq!(status.phase, Phase::Failed);
        assert_eq!((status.count, status.written_at.clone()), (None, None));
    }

    #[test]
    fn refresh_from_ready_or_failed_exports_again_and_clears_the_error() {
        let mut machine = ready(150);
        let first = started(machine.request_export(false));
        machine.export_ended(first, Err("boom".into()));
        assert_eq!(machine.status().phase, Phase::Failed);
        let second = started(machine.request_export(false));
        assert_ne!(first, second);
        assert_eq!(machine.status().phase, Phase::Exporting);
        assert_eq!(machine.status().error, None);
        machine.export_ended(second, Ok(bundle(151, NOW + 9)));
        assert_eq!(machine.status().phase, Phase::Ready);
        let third = started(machine.request_export(true));
        assert!(machine.status().automatic);
        machine.export_ended(third, Ok(bundle(151, NOW + 10)));
        assert!(
            machine.status().automatic,
            "the flag says how the last export started"
        );
    }

    #[test]
    fn a_request_while_one_runs_joins_it_and_starts_nothing() {
        let mut machine = ready(150);
        let ticket = started(machine.request_export(false));
        let revision = machine.status().revision;
        assert_eq!(machine.request_export(false), Request::Joined);
        assert_eq!(machine.request_export(false), Request::Joined);
        assert_eq!(machine.status().revision, revision);
        // The joined export is the one that ends the state.
        assert!(machine.export_ended(ticket, Ok(bundle(1, NOW))).is_some());
        assert_eq!(machine.status().phase, Phase::Ready);
        // And a second end of the same export changes nothing.
        let revision = machine.status().revision;
        assert_eq!(machine.export_ended(ticket, Ok(bundle(1, NOW))), None);
        assert_eq!(machine.status().revision, revision);
    }

    #[test]
    fn an_automatic_request_that_joins_makes_the_running_export_automatic() {
        let mut machine = Machine::new(None);
        let ticket = machine.setting_changed(true).unwrap();
        assert!(!machine.status().automatic);
        let revision = machine.status().revision;
        assert_eq!(machine.request_export(true), Request::Joined);
        assert!(machine.status().automatic);
        assert_eq!(machine.status().revision, revision + 1);
        // A request that is not automatic never makes an automatic export asked.
        assert_eq!(machine.request_export(false), Request::Joined);
        assert!(machine.status().automatic);
        let ended = machine.export_ended(ticket, Err("x".into())).unwrap();
        assert!(ended.automatic);
    }

    #[test]
    fn with_the_setting_off_a_request_is_refused() {
        let mut machine = Machine::new(Some(bundle(150, NOW)));
        assert_eq!(machine.request_export(false), Request::Refused);
        assert_eq!(machine.request_export(true), Request::Refused);
        assert_eq!(machine.status().phase, Phase::Off);
        let mut machine = ready(150);
        machine.setting_changed(false);
        assert_eq!(machine.request_export(false), Request::Refused);
    }

    #[test]
    fn turning_the_setting_off_goes_off_and_keeps_the_bundle_info() {
        let mut machine = ready(150);
        assert_eq!(machine.setting_changed(false), None);
        assert_eq!(machine.status().phase, Phase::Off);
        assert_eq!(machine.status().count, Some(150));
        assert!(!machine.enabled());
    }

    #[test]
    fn an_export_that_ends_after_the_setting_was_turned_off_changes_nothing() {
        let mut machine = ready(150);
        let ticket = started(machine.request_export(false));
        machine.setting_changed(false);
        let after_off = machine.status().clone();
        assert_eq!(machine.export_ended(ticket, Ok(bundle(999, NOW + 1))), None);
        assert_eq!(machine.export_ended(ticket, Err("late".into())), None);
        assert_eq!(*machine.status(), after_off);
        assert_eq!(after_off.phase, Phase::Off);
    }

    #[test]
    fn an_export_of_before_the_setting_was_turned_off_and_on_again_is_ignored_too() {
        let mut machine = ready(150);
        let old = started(machine.request_export(false));
        machine.setting_changed(false);
        let new = machine.setting_changed(true).expect("a new export starts");
        assert_ne!(old, new);
        assert_eq!(machine.export_ended(old, Ok(bundle(999, NOW + 1))), None);
        assert_eq!(machine.status().phase, Phase::Exporting);
        assert_eq!(machine.status().count, Some(150));
        assert!(machine
            .export_ended(new, Ok(bundle(160, NOW + 2)))
            .is_some());
        assert_eq!(machine.status().count, Some(160));
    }

    #[test]
    fn the_setting_in_force_changes_nothing_when_told_again() {
        let mut machine = Machine::new(None);
        assert_eq!(machine.setting_changed(false), None);
        let ticket = machine.setting_changed(true).unwrap();
        let revision = machine.status().revision;
        assert_eq!(machine.setting_changed(true), None);
        assert_eq!(machine.status().revision, revision);
        machine.export_ended(ticket, Ok(bundle(1, NOW)));
        assert_eq!(machine.setting_changed(true), None);
        assert_eq!(machine.app_started(true, NOW), None);
        assert_eq!(machine.status().phase, Phase::Ready);
    }

    #[test]
    fn the_revision_grows_with_every_change_and_only_then() {
        let mut machine = Machine::new(None);
        let mut last = machine.status().revision;
        let mut grew = |machine: &Machine| {
            let now = machine.status().revision;
            let grew = now > last;
            last = now;
            grew
        };
        let ticket = machine.setting_changed(true).unwrap();
        assert!(grew(&machine));
        machine.export_ended(ticket, Ok(bundle(1, NOW)));
        assert!(grew(&machine));
        machine.setting_changed(false);
        assert!(grew(&machine));
        machine.setting_changed(false);
        assert!(!grew(&machine));
    }

    #[test]
    fn the_status_reaches_the_ui_in_camel_case() {
        let mut machine = Machine::new(Some(bundle(3, NOW)));
        machine.setting_changed(true);
        let json = serde_json::to_value(machine.status()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "revision": machine.status().revision,
                "phase": "exporting",
                "automatic": false,
                "count": 3,
                "writtenAt": format_instant(NOW),
                "error": null,
                "turnOnNotice": false,
                "failureBanner": false,
                "failureReason": null,
            })
        );
    }

    #[test]
    fn the_daily_refresh_starts_an_automatic_export_from_ready_and_from_failed_only() {
        let mut machine = ready(10);
        let ticket = machine.refresh_due().expect("from Ready");
        assert_eq!(machine.status().phase, Phase::Exporting);
        assert!(machine.status().automatic);
        // While it runs, a second tick starts nothing.
        assert_eq!(machine.refresh_due(), None);
        machine.export_ended(ticket, Err("no certificate authority found".into()));
        assert_eq!(machine.status().phase, Phase::Failed);
        let again = machine.refresh_due().expect("from Failed");
        machine.export_ended(again, Ok(bundle(11, NOW)));
        assert_eq!(machine.status().phase, Phase::Ready);
        machine.setting_changed(false);
        assert_eq!(machine.refresh_due(), None, "off: nothing");
    }

    #[test]
    fn the_turn_on_notice_shows_until_it_is_closed_and_changes_the_revision_once() {
        let mut machine = Machine::new(None);
        let first = machine.status().revision;
        machine.show_turn_on_notice();
        assert!(machine.status().turn_on_notice);
        assert_eq!(machine.status().revision, first + 1);
        machine.show_turn_on_notice();
        assert_eq!(machine.status().revision, first + 1);
        // Turning the setting on and off does not take it away.
        machine.setting_changed(true);
        machine.setting_changed(false);
        assert!(machine.status().turn_on_notice);
        machine.dismiss_turn_on_notice();
        assert!(!machine.status().turn_on_notice);
        let closed = machine.status().revision;
        machine.dismiss_turn_on_notice();
        assert_eq!(machine.status().revision, closed);
    }

    #[test]
    fn only_a_failed_automatic_export_shows_the_banner_and_a_success_or_off_removes_it() {
        // Asked: no banner.
        let mut machine = ready(5);
        let asked = started(machine.request_export(false));
        machine.export_ended(asked, Err("could not read".into()));
        assert_eq!(machine.status().phase, Phase::Failed);
        assert!(!machine.status().failure_banner && machine.status().failure_reason.is_none());

        // Automatic: the banner, with its reason; a newer failure shows it again after Close.
        let ticket = machine.refresh_due().unwrap();
        assert!(!machine.status().failure_banner, "nothing to show yet");
        machine.export_ended(ticket, Err("no certificate authority found".into()));
        assert!(machine.status().failure_banner);
        assert_eq!(
            machine.status().failure_reason.as_deref(),
            Some("no certificate authority found")
        );
        machine.dismiss_failure_banner();
        assert!(!machine.status().failure_banner && machine.status().failure_reason.is_none());
        let newer = machine.refresh_due().unwrap();
        machine.export_ended(newer, Err("could not read".into()));
        assert_eq!(
            machine.status().failure_reason.as_deref(),
            Some("could not read")
        );

        // A successful export removes it.
        let ok = machine.refresh_due().unwrap();
        machine.export_ended(ok, Ok(bundle(6, NOW)));
        assert!(!machine.status().failure_banner);

        // Turning the setting off removes it too.
        let failing = machine.refresh_due().unwrap();
        machine.export_ended(failing, Err("boom".into()));
        assert!(machine.status().failure_banner);
        machine.setting_changed(false);
        assert!(!machine.status().failure_banner);
        machine.setting_changed(true);
        assert!(!machine.status().failure_banner, "it does not come back");
    }

    #[test]
    fn the_banner_stays_while_another_export_runs() {
        let mut machine = ready(5);
        let ticket = machine.refresh_due().unwrap();
        machine.export_ended(ticket, Err("no certificate authority found".into()));
        // Refresh, and a daily export after the next failure: the banner is not hidden by the
        // start of an export, only by the four events of the Task.
        let refresh = started(machine.request_export(false));
        assert_eq!(machine.status().phase, Phase::Exporting);
        assert!(machine.status().failure_banner);
        assert_eq!(
            machine.status().failure_reason.as_deref(),
            Some("no certificate authority found")
        );
        machine.export_ended(refresh, Ok(bundle(7, NOW)));
        assert!(
            !machine.status().failure_banner,
            "a successful export removes it"
        );
    }

    #[test]
    fn a_failed_refresh_after_an_automatic_failure_replaces_the_text_of_the_banner_that_shows() {
        let mut machine = ready(5);
        let ticket = machine.refresh_due().unwrap();
        machine.export_ended(ticket, Err("no certificate authority found".into()));
        let refresh = started(machine.request_export(false));
        machine.export_ended(refresh, Err("could not read".into()));
        // A newer failure replaces the banner; it was not shown by the refresh itself.
        assert!(machine.status().failure_banner);
        assert_eq!(
            machine.status().failure_reason.as_deref(),
            Some("could not read")
        );
        assert_eq!(machine.status().error.as_deref(), Some("could not read"));

        // After Close, a failed refresh shows nothing: the line in Settings only.
        machine.dismiss_failure_banner();
        let again = started(machine.request_export(false));
        machine.export_ended(again, Err("boom".into()));
        assert!(!machine.status().failure_banner);
    }

    #[test]
    fn a_request_that_joins_an_automatic_export_makes_its_failure_show_the_banner() {
        let mut machine = Machine::new(None);
        let ticket = machine.setting_changed(true).unwrap();
        assert_eq!(machine.request_export(true), Request::Joined);
        machine.export_ended(ticket, Err("no certificate authority found".into()));
        assert!(machine.status().failure_banner);
    }
}
