//! The Zscaler turn-on (`requirements/tech#system-certificates`, "Zscaler turn-on";
//! `requirements/rules#system-certificates`): once per user, when Zscaler is installed and the
//! setting is off, Weaver turns System Certificates on by itself.
//!
//! The order is fixed and the first step that fails stops the rest:
//!
//! 1. the setting is written through the settings service, which tells every window;
//! 2. `systemCertificates.autoEnabledAt` is written in `state.json` (a failure is only logged: the
//!    setting stays on);
//! 3. `system-certificates.AUTO_ENABLED`: the notice shows in every window and the system
//!    notification is shown ([`crate::notifications::Notifications::system_certificates_turned_on`]);
//! 4. the export, as an automatic export.
//!
//! [`turn_on`] is the rule over a [`Host`]; [`run_at_start`] is the task the app starts at launch,
//! outside the startup steps: whatever goes wrong in it is only logged.

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};
use weaver_core::omp::lookup::Os;
use weaver_core::project::clock::{format_instant, Clock, SystemClock};
use weaver_core::settings::{SettingChange, SettingKey};
use weaver_core::system_certificates::zscaler;

use super::{AutomaticTurnOn, SystemCertificates};
use crate::notifications::Notifications;
use crate::state::AppState;
use crate::{events, log_info, log_warn};

/// What the rule reads and does. The real one is [`AppHost`]; the tests use a fake.
pub trait Host {
    /// Whether the `systemCertificates.enabled` setting is on.
    fn setting_is_on(&self) -> bool;
    /// Whether `state.json` holds `systemCertificates.autoEnabledAt`. An unreadable file says no.
    fn remembered(&self) -> bool;
    /// Whether Zscaler is installed (file metadata only).
    fn zscaler_installed(&self) -> bool;
    /// Writes the setting through the settings service, which tells every window.
    fn turn_on_setting(&self) -> Result<(), String>;
    /// Writes `systemCertificates.autoEnabledAt` with the instant now.
    fn remember(&self) -> Result<(), String>;
    /// `system-certificates.AUTO_ENABLED`: the notice in every window and the notification.
    fn announce(&self);
    /// Asks for the export, automatic. `false` when it was refused.
    fn export(&self) -> bool;
}

/// Turns the setting on when the conditions hold: the setting is off, `state.json` has no
/// `autoEnabledAt` and Zscaler is installed. `Ok(true)` when it did and the export started;
/// `Ok(false)` when there was nothing to do (nothing is written or remembered) or the export was
/// refused; `Err` with why the setting could not be written (nothing was remembered or shown, so
/// the next start tries again).
pub fn turn_on(host: &dyn Host) -> Result<bool, String> {
    if host.setting_is_on() || host.remembered() || !host.zscaler_installed() {
        return Ok(false);
    }
    host.turn_on_setting()?;
    log_info!("System Certificates turned on by Weaver: Zscaler is installed");
    if let Err(reason) = host.remember() {
        log_warn!("cannot remember that System Certificates was turned on, the setting stays on: {reason}");
    }
    host.announce();
    Ok(host.export())
}

/// The task of the start: runs [`turn_on`] over the app, logs a failure, and tells whether the
/// export started.
pub fn run_at_start(app: &AppHandle) -> bool {
    match turn_on(&AppHost::new(app.clone())) {
        Ok(started) => started,
        Err(reason) => {
            log_warn!("System Certificates was not turned on: {reason}");
            false
        }
    }
}

/// The app behind [`Host`].
pub struct AppHost {
    app: AppHandle,
    /// While the setting is turned on, until the export is asked: the export is automatic
    /// whoever hears of the change first.
    automatic: Mutex<Option<AutomaticTurnOn>>,
}

impl AppHost {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            automatic: Mutex::new(None),
        }
    }

    fn service(&self) -> SystemCertificates {
        self.app.state::<SystemCertificates>().inner().clone()
    }

    fn release_automatic(&self) {
        self.automatic
            .lock()
            .expect("turn-on guard poisoned")
            .take();
    }
}

impl Host for AppHost {
    fn setting_is_on(&self) -> bool {
        self.app
            .state::<AppState>()
            .settings
            .current()
            .system_certificates
            .enabled
    }

    fn remembered(&self) -> bool {
        self.app
            .state::<AppState>()
            .local_state
            .snapshot()
            .system_certificates
            .auto_enabled_at
            .is_some()
    }

    fn zscaler_installed(&self) -> bool {
        zscaler::installed(
            Os::current(),
            &zscaler::SystemFiles,
            std::env::var_os("ProgramFiles").as_deref(),
        )
    }

    fn turn_on_setting(&self) -> Result<(), String> {
        *self.automatic.lock().expect("turn-on guard poisoned") =
            Some(self.service().automatic_turn_on());
        let change = SettingChange {
            key: SettingKey::SystemCertificatesEnabled,
            value: "true".to_owned(),
        };
        match self.app.state::<AppState>().settings.apply(&change) {
            Ok((snapshot, written)) => {
                if written {
                    if let Err(error) = events::emit_settings_changed(&self.app, &snapshot) {
                        crate::log_error!(
                            "cannot tell the windows about the settings change: {error}"
                        );
                    }
                }
                Ok(())
            }
            Err(error) => {
                self.release_automatic();
                Err(error.to_string())
            }
        }
    }

    fn remember(&self) -> Result<(), String> {
        let instant = format_instant(SystemClock.now_ms());
        self.app
            .state::<AppState>()
            .local_state
            .update(|state| state.system_certificates.auto_enabled_at = Some(instant))
            .map_err(|error| error.to_string())
    }

    fn announce(&self) {
        self.service().show_turn_on_notice();
        if let Some(notifications) = self.app.try_state::<Arc<Notifications>>() {
            notifications.system_certificates_turned_on();
        }
    }

    fn export(&self) -> bool {
        let asked = self.service().request_export(true).is_ok();
        self.release_automatic();
        asked
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    /// A system that records what the rule does, in order.
    struct Fake {
        on: Cell<bool>,
        remembered: Cell<bool>,
        installed: bool,
        setting_fails: bool,
        state_fails: bool,
        export_refused: bool,
        calls: RefCell<Vec<&'static str>>,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                on: Cell::new(false),
                remembered: Cell::new(false),
                installed: true,
                setting_fails: false,
                state_fails: false,
                export_refused: false,
                calls: RefCell::default(),
            }
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls.borrow().clone()
        }
    }

    impl Host for Fake {
        fn setting_is_on(&self) -> bool {
            self.on.get()
        }

        fn remembered(&self) -> bool {
            self.remembered.get()
        }

        fn zscaler_installed(&self) -> bool {
            self.installed
        }

        fn turn_on_setting(&self) -> Result<(), String> {
            if self.setting_fails {
                return Err("the settings file cannot be written".to_owned());
            }
            self.calls.borrow_mut().push("setting");
            self.on.set(true);
            Ok(())
        }

        fn remember(&self) -> Result<(), String> {
            self.calls.borrow_mut().push("remember");
            if self.state_fails {
                return Err("state.json cannot be written".to_owned());
            }
            self.remembered.set(true);
            Ok(())
        }

        fn announce(&self) {
            self.calls.borrow_mut().push("announce");
        }

        fn export(&self) -> bool {
            self.calls.borrow_mut().push("export");
            !self.export_refused
        }
    }

    #[test]
    fn it_turns_the_setting_on_remembers_it_says_it_and_exports_in_that_order() {
        let host = Fake::new();
        assert_eq!(turn_on(&host), Ok(true));
        assert_eq!(host.calls(), ["setting", "remember", "announce", "export"]);
    }

    #[test]
    fn it_does_it_once_even_when_the_developer_turns_the_setting_off_and_zscaler_stays() {
        let host = Fake::new();
        assert_eq!(turn_on(&host), Ok(true));
        host.on.set(false);
        assert_eq!(turn_on(&host), Ok(false));
        assert_eq!(turn_on(&host), Ok(false));
        assert_eq!(
            host.calls()
                .iter()
                .filter(|call| **call == "announce")
                .count(),
            1,
            "the event and the banners go out once"
        );
        assert_eq!(host.calls().iter().filter(|c| **c == "setting").count(), 1);
    }

    #[test]
    fn a_flag_that_is_there_means_never_again() {
        let host = Fake::new();
        host.remembered.set(true);
        assert_eq!(turn_on(&host), Ok(false));
        assert!(host.calls().is_empty());
    }

    #[test]
    fn a_setting_that_is_already_on_writes_and_remembers_nothing() {
        let host = Fake::new();
        host.on.set(true);
        assert_eq!(turn_on(&host), Ok(false));
        assert!(host.calls().is_empty());
        assert!(!host.remembered.get());
    }

    #[test]
    fn without_zscaler_nothing_happens() {
        let host = Fake {
            installed: false,
            ..Fake::new()
        };
        assert_eq!(turn_on(&host), Ok(false));
        assert!(host.calls().is_empty());
    }

    #[test]
    fn a_failed_setting_write_remembers_and_shows_nothing_and_the_next_start_tries_again() {
        let host = Fake {
            setting_fails: true,
            ..Fake::new()
        };
        assert_eq!(
            turn_on(&host),
            Err("the settings file cannot be written".to_owned())
        );
        assert!(host.calls().is_empty());
        assert!(!host.remembered.get() && !host.on.get());
        // The next start, with the write working, goes through.
        let next = Fake::new();
        assert_eq!(turn_on(&next), Ok(true));
    }

    #[test]
    fn a_failed_state_write_is_only_logged_and_the_setting_stays_on() {
        let host = Fake {
            state_fails: true,
            ..Fake::new()
        };
        assert_eq!(turn_on(&host), Ok(true));
        assert_eq!(host.calls(), ["setting", "remember", "announce", "export"]);
        assert!(host.on.get());
    }

    #[test]
    fn a_refused_export_is_not_an_error() {
        let host = Fake {
            export_refused: true,
            ..Fake::new()
        };
        assert_eq!(turn_on(&host), Ok(false));
        assert_eq!(host.calls(), ["setting", "remember", "announce", "export"]);
    }
}
