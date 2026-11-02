//! The settings in the app: the store behind `<config>/settings.json`, the commands the Settings
//! view uses and the event that tells every window about a change.
//!
//! The values, their validation and the file are in `trama_core::settings`; this module adds the
//! lazy load (the first use reads the file, so it must happen on the blocking pool or in the
//! startup step), the broadcast of changes to the rest of the backend and the Tauri commands.
//!
//! How another part of the backend uses a setting:
//!
//! ```ignore
//! let settings = app.state::<AppState>().settings.current(); // `Settings` in force now
//! let omp = settings.omp_path;                               // copy what the process needs
//! ```
//!
//! A change takes effect for the processes started after it: read the settings when starting the
//! process and keep the copy, never re-read them for a job that is already working.
//! [`SettingsService::subscribe`] delivers each change with the keys that changed.

use std::sync::{Arc, OnceLock};

use tauri::{AppHandle, Manager};
use tokio::sync::broadcast;
use trama_core::paths::AppDirs;
use trama_core::project::clock::SystemClock;
use trama_core::settings::{
    SettingChange, SettingKey, Settings, SettingsError, SettingsSnapshot, SettingsStore, FILE_NAME,
};

use crate::blocking;
use crate::error::{AppError, ErrorCode};
use crate::state::AppState;
use crate::{events, log_info, log_warn};

/// A setting changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsChanged {
    pub revision: u64,
    /// The settings whose value differs from before.
    pub changed: Vec<SettingKey>,
    /// The settings in force after the change.
    pub settings: Settings,
}

/// The settings store of the app, loaded on first use.
pub struct SettingsService {
    path: std::path::PathBuf,
    store: OnceLock<SettingsStore>,
    changes: broadcast::Sender<SettingsChanged>,
}

impl SettingsService {
    pub fn new(dirs: &AppDirs) -> Self {
        Self::at(dirs.config_dir().join(FILE_NAME))
    }

    /// A service over another file (tests use a temporary folder, never the real settings).
    pub fn at(path: std::path::PathBuf) -> Self {
        let (changes, _) = broadcast::channel(16);
        Self {
            path,
            store: OnceLock::new(),
            changes,
        }
    }

    /// The store; the first call reads the file (a few hundred bytes), so call it from the
    /// blocking pool or the startup step.
    pub fn store(&self) -> &SettingsStore {
        self.store
            .get_or_init(|| SettingsStore::load(self.path.clone(), Arc::new(SystemClock)))
    }

    /// The settings in force now.
    pub fn current(&self) -> Settings {
        self.store().current()
    }

    pub fn snapshot(&self) -> SettingsSnapshot {
        self.store().snapshot()
    }

    /// Every change from now on, with the keys that changed (omp setup reacts to
    /// [`SettingKey::OmpPath`]). A receiver that falls behind skips to the latest changes.
    pub fn subscribe(&self) -> broadcast::Receiver<SettingsChanged> {
        self.changes.subscribe()
    }

    /// Validates and writes `change`. Returns the new snapshot and whether the file was written;
    /// subscribers hear about it first.
    pub fn apply(&self, change: &SettingChange) -> Result<(SettingsSnapshot, bool), SettingsError> {
        let applied = self.store().apply(change)?;
        if applied.written {
            // No receiver is not an error: nobody listens yet.
            let _ = self.changes.send(SettingsChanged {
                revision: applied.snapshot.revision,
                changed: applied.changed.clone(),
                settings: applied.snapshot.settings.clone(),
            });
        }
        Ok((applied.snapshot, applied.written))
    }

    /// The instant of the last check for an update of Weaver, when the file holds one.
    pub fn last_update_check(&self) -> Option<String> {
        self.store().last_update_check()
    }

    /// Stores the instant of a check for an update next to the settings (no change event).
    pub fn record_update_check(&self, instant: &str) -> Result<(), String> {
        self.store().record_update_check(instant)
    }

    /// Reads the file now, so that the first command does not, and logs what was ignored (keys
    /// only, never values).
    pub fn load_now(&self) {
        let snapshot = self.snapshot();
        if let Some(reason) = &snapshot.file_error {
            log_warn!("{FILE_NAME} is not used, every default is in force: {reason}");
        }
        for issue in &snapshot.issues {
            log_warn!(
                "{FILE_NAME}: the value of `{}` is ignored: {}",
                issue.key.path(),
                issue.reason
            );
        }
    }
}

/// The settings in force, the ignored keys and the description of every setting.
#[tauri::command]
pub async fn settings_get(app: AppHandle) -> Result<SettingsSnapshot, AppError> {
    blocking::disk_named("settings_get", None, move || {
        app.state::<AppState>().settings.snapshot()
    })
    .await
}

/// Validates and stores one change, tells every window, and returns the new snapshot. A refused
/// value is `invalid-input` with the message to show under the field; a failed write is
/// `internal` with its reason, and the earlier file and values stay in force.
#[tauri::command]
pub async fn settings_set(
    app: AppHandle,
    change: SettingChange,
) -> Result<SettingsSnapshot, AppError> {
    let key = change.key;
    let outcome = {
        let app = app.clone();
        blocking::disk_named("settings_set", None, move || {
            app.state::<AppState>().settings.apply(&change)
        })
        .await?
    };
    match outcome {
        Ok((snapshot, written)) => {
            if written {
                log_info!("setting `{}` changed", key.path());
                if let Err(error) = events::emit_settings_changed(&app, &snapshot) {
                    crate::log_error!("cannot tell the windows about the settings change: {error}");
                }
            }
            Ok(snapshot)
        }
        Err(SettingsError::Invalid(message)) => {
            Err(AppError::new(ErrorCode::InvalidInput, message))
        }
        Err(SettingsError::Write(message)) => {
            crate::log_error!("setting `{}` was not saved: {message}", key.path());
            Err(AppError::internal(message))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(key: SettingKey, value: &str) -> SettingChange {
        SettingChange {
            key,
            value: value.to_owned(),
        }
    }

    #[test]
    fn a_change_reaches_the_subscribers_with_the_keys_that_changed() {
        let dir = tempfile::tempdir().unwrap();
        let service = SettingsService::at(dir.path().join("settings.json"));
        let mut changes = service.subscribe();
        service
            .apply(&change(SettingKey::OmpPath, "/opt/omp"))
            .unwrap();
        let heard = changes.try_recv().unwrap();
        assert_eq!(heard.changed, [SettingKey::OmpPath]);
        assert_eq!(heard.settings.omp_path, "/opt/omp");
        assert_eq!(heard.revision, 2);
    }

    #[test]
    fn a_value_already_in_force_and_a_refused_value_tell_nobody() {
        let dir = tempfile::tempdir().unwrap();
        let service = SettingsService::at(dir.path().join("settings.json"));
        let mut changes = service.subscribe();
        let (_, written) = service.apply(&change(SettingKey::Theme, "auto")).unwrap();
        assert!(!written);
        assert!(service
            .apply(&change(SettingKey::MaxParallelAgents, "0"))
            .is_err());
        assert!(changes.try_recv().is_err());
    }

    #[test]
    fn the_file_is_read_on_first_use_and_the_service_works_without_subscribers() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.json"),
            r#"{ "maxParallelAgents": 4 }"#,
        )
        .unwrap();
        let service = SettingsService::at(dir.path().join("settings.json"));
        assert_eq!(service.current().max_parallel_agents, 4);
        service.apply(&change(SettingKey::Theme, "dark")).unwrap();
        assert_eq!(service.current().theme, trama_core::settings::Theme::Dark);
    }

    #[test]
    fn the_settings_file_is_in_the_config_folder() {
        let dirs = AppDirs::from_bases(
            std::path::Path::new("/cfg"),
            std::path::Path::new("/data"),
            None,
        );
        assert_eq!(
            SettingsService::new(&dirs).path,
            std::path::Path::new("/cfg/dev.nicchia.weaver/settings.json")
        );
    }
}
