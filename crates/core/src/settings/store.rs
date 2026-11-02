//! The settings in force and the file behind them.
//!
//! [`SettingsStore::load`] never fails: a missing file is every default, an unreadable one is
//! every default with the error kept for the Settings view and the file left alone. A change
//! ([`SettingsStore::apply`]) is validated, then written atomically; writes are serialised by one
//! lock. A refused or failed change leaves the earlier file and the earlier values in force.
//! The first successful change after an unreadable file sets that file aside as
//! `settings.invalid-<UTC timestamp>.json` next to the new one.
//!
//! Synchronous `std` I/O: the app calls it from the blocking pool.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::Serialize;
use ts_rs::TS;

use super::schema::{schema, SettingField};
use super::{SettingChange, SettingIssue, SettingKey, Settings, FILE_NAME};
use crate::atomic;
use crate::project::clock::{format_instant, Clock};
use crate::project::text;

/// What the UI needs: the values in force, what was ignored in the file and why, the error of an
/// unreadable file, and the description of every setting. `revision` grows with every change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    #[ts(type = "number")]
    pub revision: u64,
    pub settings: Settings,
    /// Keys of the file that were ignored (wrong type or invalid value), each with the reason.
    pub issues: Vec<SettingIssue>,
    /// Why the whole file was not used, when it was unreadable: every default is then in force.
    pub file_error: Option<String>,
    pub schema: Vec<SettingField>,
}

/// Why a change did not happen.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    /// The value is not valid; the message is what the developer reads under the field.
    #[error("{0}")]
    Invalid(String),
    /// The file could not be written; the earlier file and values are still in force.
    #[error("{0}")]
    Write(String),
}

/// The result of a change that was accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    pub snapshot: SettingsSnapshot,
    /// The settings whose value differs from before (empty when nothing changed).
    pub changed: Vec<SettingKey>,
    /// Whether the file was written (false when the value was already in force).
    pub written: bool,
}

struct State {
    settings: Settings,
    issues: Vec<SettingIssue>,
    file_error: Option<String>,
    last_update_check: Option<String>,
    revision: u64,
}

pub struct SettingsStore {
    path: PathBuf,
    clock: Arc<dyn Clock>,
    state: Mutex<State>,
}

impl SettingsStore {
    /// Reads `path` (`<config>/settings.json`). `clock` dates the file set aside when it was
    /// unreadable.
    pub fn load(path: PathBuf, clock: Arc<dyn Clock>) -> Self {
        let read = match fs::read(&path) {
            Ok(bytes) => text::decode(&bytes)
                .map(Some)
                .map_err(|_| "it is not valid UTF-8 text".to_owned()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("it cannot be read ({e})")),
        };
        let parsed = match read {
            Ok(Some(text)) => Settings::parse_file(&text).map(|(settings, issues)| {
                (settings, issues, Settings::read_last_update_check(&text))
            }),
            Ok(None) => Ok((Settings::default(), Vec::new(), None)),
            Err(reason) => Err(reason),
        };
        let (settings, issues, last_update_check, file_error) = match parsed {
            Ok((settings, issues, last)) => (settings, issues, last, None),
            Err(reason) => (Settings::default(), Vec::new(), None, Some(reason)),
        };
        Self {
            path,
            clock,
            state: Mutex::new(State {
                settings,
                issues,
                file_error,
                last_update_check,
                revision: 1,
            }),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().expect("settings state poisoned")
    }

    /// The settings in force now.
    pub fn current(&self) -> Settings {
        self.state().settings.clone()
    }

    pub fn snapshot(&self) -> SettingsSnapshot {
        Self::snapshot_of(&self.state())
    }

    fn snapshot_of(state: &State) -> SettingsSnapshot {
        SettingsSnapshot {
            revision: state.revision,
            settings: state.settings.clone(),
            issues: state.issues.clone(),
            file_error: state.file_error.clone(),
            schema: schema(),
        }
    }

    /// The instant of the last check for an update of Weaver, when the file holds one.
    pub fn last_update_check(&self) -> Option<String> {
        self.state().last_update_check.clone()
    }

    /// Stores the instant of a check for an update (UTC ISO 8601) next to the settings, which it
    /// leaves as they are; it is not a change of a setting (no revision, no event). Refused
    /// while the file is unreadable: it is only set aside by a change of a setting.
    pub fn record_update_check(&self, instant: &str) -> Result<(), String> {
        let mut state = self.state();
        if !crate::project::clock::is_instant(instant) {
            return Err("the instant of the check is not a UTC ISO 8601 instant".to_owned());
        }
        if state.file_error.is_some() {
            return Err(format!(
                "{FILE_NAME} is not readable; it was left unchanged"
            ));
        }
        atomic::write(
            &self.path,
            state.settings.file_text_with(Some(instant)).as_bytes(),
            None,
        )
        .map_err(|e| format!("cannot write {FILE_NAME} ({e})"))?;
        state.last_update_check = Some(instant.to_owned());
        Ok(())
    }

    /// Validates `change` and writes it. A value already in force writes nothing, unless the
    /// file holds something that was ignored or was unreadable: that is a change of the file.
    pub fn apply(&self, change: &SettingChange) -> Result<Applied, SettingsError> {
        let mut state = self.state();
        let mut next = state.settings.clone();
        next.set_text(change.key, &change.value)
            .map_err(SettingsError::Invalid)?;

        let file_needs_rewrite =
            state.file_error.is_some() || state.issues.iter().any(|issue| issue.key == change.key);
        if next == state.settings && !file_needs_rewrite {
            return Ok(Applied {
                snapshot: Self::snapshot_of(&state),
                changed: Vec::new(),
                written: false,
            });
        }

        if state.file_error.is_some() {
            self.set_unreadable_file_aside().map_err(|e| {
                SettingsError::Write(format!("cannot set the unreadable {FILE_NAME} aside ({e})"))
            })?;
        }
        atomic::write(
            &self.path,
            next.file_text_with(state.last_update_check.as_deref())
                .as_bytes(),
            None,
        )
        .map_err(|e| SettingsError::Write(format!("cannot write {FILE_NAME} ({e})")))?;

        let changed: Vec<SettingKey> = SettingKey::ALL
            .into_iter()
            .filter(|key| next.text(*key) != state.settings.text(*key))
            .collect();
        state.settings = next;
        state.issues.clear();
        state.file_error = None;
        state.revision += 1;
        Ok(Applied {
            snapshot: Self::snapshot_of(&state),
            changed,
            written: true,
        })
    }

    /// Renames the unreadable file to `settings.invalid-<UTC timestamp>.json` (colons of the time
    /// become dashes so the name is valid everywhere). A file that is already gone needs nothing.
    fn set_unreadable_file_aside(&self) -> io::Result<()> {
        if !self.path.exists() {
            return Ok(());
        }
        let stamp = format_instant(self.clock.now_ms()).replace(':', "-");
        let folder = self.path.parent().unwrap_or_else(|| Path::new("."));
        let stem = FILE_NAME.trim_end_matches(".json");
        let mut target = folder.join(format!("{stem}.invalid-{stamp}.json"));
        let mut n = 1;
        while target.exists() {
            n += 1;
            target = folder.join(format!("{stem}.invalid-{stamp}-{n}.json"));
        }
        fs::rename(&self.path, target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::clock::ManualClock;
    use crate::settings::{EditorPreset, Theme};

    fn clock() -> Arc<dyn Clock> {
        Arc::new(ManualClock::at("2026-11-02T07:07:19.522Z"))
    }

    fn change(key: SettingKey, value: &str) -> SettingChange {
        SettingChange {
            key,
            value: value.to_owned(),
        }
    }

    fn store_in(dir: &Path) -> SettingsStore {
        SettingsStore::load(dir.join(FILE_NAME), clock())
    }

    fn file_json(dir: &Path) -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(dir.join(FILE_NAME)).unwrap()).unwrap()
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_missing_file_is_every_default_and_nothing_is_created() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(&dir.path().join("config"));
        let snapshot = store.snapshot();
        assert_eq!(snapshot.settings, Settings::default());
        assert!(snapshot.issues.is_empty());
        assert_eq!(snapshot.file_error, None);
        assert_eq!(snapshot.schema.len(), SettingKey::ALL.len());
        assert!(!dir.path().join("config").exists());
    }

    #[test]
    fn a_change_creates_the_folder_and_the_file_with_the_version() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("config");
        let store = store_in(&folder);
        let applied = store.apply(&change(SettingKey::Theme, "dark")).unwrap();
        assert!(applied.written);
        assert_eq!(applied.changed, [SettingKey::Theme]);
        assert_eq!(applied.snapshot.settings.theme, Theme::Dark);
        assert_eq!(applied.snapshot.revision, 2);
        assert_eq!(
            file_json(&folder),
            serde_json::json!({ "version": 1, "theme": "dark" })
        );
        assert_eq!(store_in(&folder).current().theme, Theme::Dark);
    }

    #[test]
    fn an_unknown_key_is_ignored_and_dropped_at_the_next_write() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{ "version": 1, "colour": "red", "language": "Italian" }"#,
        )
        .unwrap();
        let store = store_in(dir.path());
        assert_eq!(store.current().language, "Italian");
        assert!(store.snapshot().issues.is_empty());
        store.apply(&change(SettingKey::Theme, "light")).unwrap();
        assert_eq!(
            file_json(dir.path()),
            serde_json::json!({ "version": 1, "language": "Italian", "theme": "light" })
        );
    }

    #[test]
    fn a_wrong_value_is_the_default_and_the_issue_names_the_key_until_the_next_write() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{ "maxParallelAgents": 0, "theme": "dark" }"#,
        )
        .unwrap();
        let store = store_in(dir.path());
        let snapshot = store.snapshot();
        assert_eq!(snapshot.settings.max_parallel_agents, 20);
        assert_eq!(snapshot.settings.theme, Theme::Dark);
        assert_eq!(snapshot.issues.len(), 1);
        assert_eq!(snapshot.issues[0].key, SettingKey::MaxParallelAgents);
        assert!(!snapshot.issues[0].reason.is_empty());

        store
            .apply(&change(SettingKey::Language, "Italian"))
            .unwrap();
        assert!(store.snapshot().issues.is_empty());
        assert_eq!(
            file_json(dir.path()),
            serde_json::json!({ "version": 1, "language": "Italian", "theme": "dark" })
        );
    }

    #[test]
    fn choosing_the_default_again_clears_the_issue_in_the_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{ "maxParallelAgents": "many" }"#,
        )
        .unwrap();
        let store = store_in(dir.path());
        let applied = store
            .apply(&change(SettingKey::MaxParallelAgents, "20"))
            .unwrap();
        assert!(applied.written);
        assert!(applied.changed.is_empty());
        assert!(applied.snapshot.issues.is_empty());
        assert_eq!(file_json(dir.path()), serde_json::json!({ "version": 1 }));
    }

    #[test]
    fn an_unreadable_file_is_every_default_with_the_error_and_is_not_rewritten() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(FILE_NAME), "{ not json").unwrap();
        let store = store_in(dir.path());
        let snapshot = store.snapshot();
        assert_eq!(snapshot.settings, Settings::default());
        assert!(snapshot
            .file_error
            .as_deref()
            .unwrap()
            .contains("not valid JSON"));
        assert_eq!(
            fs::read_to_string(dir.path().join(FILE_NAME)).unwrap(),
            "{ not json"
        );
        assert_eq!(names(dir.path()), [FILE_NAME]);
    }

    #[test]
    fn an_unreadable_file_is_renamed_at_the_first_change_next_to_the_new_one() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(FILE_NAME), "{ not json").unwrap();
        let store = store_in(dir.path());
        store.apply(&change(SettingKey::Theme, "dark")).unwrap();
        assert_eq!(
            names(dir.path()),
            ["settings.invalid-2026-11-02T07-07-19.522Z.json", FILE_NAME]
        );
        assert_eq!(
            fs::read_to_string(
                dir.path()
                    .join("settings.invalid-2026-11-02T07-07-19.522Z.json")
            )
            .unwrap(),
            "{ not json"
        );
        assert_eq!(
            file_json(dir.path()),
            serde_json::json!({ "version": 1, "theme": "dark" })
        );
        assert_eq!(store.snapshot().file_error, None);

        // The new file is fine: the next change renames nothing.
        store.apply(&change(SettingKey::Theme, "light")).unwrap();
        assert_eq!(names(dir.path()).len(), 2);
    }

    #[test]
    fn a_file_that_is_not_a_json_object_or_not_utf8_is_unreadable_too() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(FILE_NAME), "[1]").unwrap();
        assert!(store_in(dir.path()).snapshot().file_error.is_some());
        fs::write(dir.path().join(FILE_NAME), [0xff, 0xfe, 0x00]).unwrap();
        assert!(store_in(dir.path())
            .snapshot()
            .file_error
            .unwrap()
            .contains("UTF-8"));
    }

    #[test]
    fn a_bom_and_crlf_are_accepted() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(FILE_NAME),
            "\u{feff}{\r\n  \"theme\": \"light\"\r\n}\r\n",
        )
        .unwrap();
        let store = store_in(dir.path());
        assert_eq!(store.current().theme, Theme::Light);
        assert_eq!(store.snapshot().file_error, None);
    }

    #[test]
    fn two_unreadable_files_set_aside_in_the_same_millisecond_keep_both() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(FILE_NAME), "{ one").unwrap();
        store_in(dir.path())
            .apply(&change(SettingKey::Theme, "dark"))
            .unwrap();
        fs::write(dir.path().join(FILE_NAME), "{ two").unwrap();
        store_in(dir.path())
            .apply(&change(SettingKey::Theme, "dark"))
            .unwrap();
        assert_eq!(names(dir.path()).len(), 3);
    }

    #[test]
    fn going_back_to_the_default_removes_the_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        store
            .apply(&change(SettingKey::ModelsScan, "fast"))
            .unwrap();
        store
            .apply(&change(SettingKey::MaxParallelAgents, "3"))
            .unwrap();
        assert_eq!(
            file_json(dir.path()),
            serde_json::json!({ "version": 1, "models": { "scan": "fast" }, "maxParallelAgents": 3 })
        );
        store.apply(&change(SettingKey::ModelsScan, "")).unwrap();
        assert_eq!(
            file_json(dir.path()),
            serde_json::json!({ "version": 1, "maxParallelAgents": 3 })
        );
        store
            .apply(&change(SettingKey::MaxParallelAgents, "20"))
            .unwrap();
        assert_eq!(file_json(dir.path()), serde_json::json!({ "version": 1 }));
    }

    #[test]
    fn a_value_already_in_force_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        let applied = store.apply(&change(SettingKey::Theme, "auto")).unwrap();
        assert!(!applied.written);
        assert!(applied.changed.is_empty());
        assert_eq!(applied.snapshot.revision, 1);
        assert!(!dir.path().join(FILE_NAME).exists());
    }

    #[test]
    fn a_refused_value_writes_nothing_and_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        store
            .apply(&change(SettingKey::MaxParallelAgents, "5"))
            .unwrap();
        let before = fs::read_to_string(dir.path().join(FILE_NAME)).unwrap();
        let refused = store
            .apply(&change(SettingKey::MaxParallelAgents, "zero"))
            .unwrap_err();
        assert_eq!(
            refused,
            SettingsError::Invalid("Enter a whole number of at least 1.".to_owned())
        );
        assert_eq!(store.current().max_parallel_agents, 5);
        assert_eq!(
            fs::read_to_string(dir.path().join(FILE_NAME)).unwrap(),
            before
        );
        assert_eq!(store.snapshot().revision, 2);
    }

    #[test]
    fn the_editor_preset_and_command_are_stored_together() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        store
            .apply(&change(SettingKey::EditorPreset, "custom"))
            .unwrap();
        assert_eq!(
            store
                .apply(&change(SettingKey::EditorCommand, "ed"))
                .unwrap_err(),
            SettingsError::Invalid("The command must contain {file}.".to_owned())
        );
        store
            .apply(&change(SettingKey::EditorCommand, "ed {file}"))
            .unwrap();
        assert_eq!(
            file_json(dir.path()),
            serde_json::json!({ "version": 1, "editor": { "preset": "custom", "command": "ed {file}" } })
        );
        assert_eq!(store.current().editor.preset, EditorPreset::Custom);
    }

    #[test]
    fn a_failed_write_leaves_the_earlier_file_and_values_in_force() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        store.apply(&change(SettingKey::Theme, "dark")).unwrap();
        // The target becomes a non-empty folder, so the rename of the new file onto it fails.
        let target = dir.path().join(FILE_NAME);
        fs::remove_file(&target).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(target.join("inner"), "x").unwrap();

        let error = store
            .apply(&change(SettingKey::Theme, "light"))
            .unwrap_err();
        assert!(
            matches!(&error, SettingsError::Write(message) if message.contains("cannot write settings.json")),
            "{error:?}"
        );
        assert_eq!(store.current().theme, Theme::Dark);
        assert_eq!(store.snapshot().revision, 2);
        assert_eq!(names(dir.path()), [FILE_NAME]);
    }

    #[test]
    fn the_instant_of_the_last_update_check_survives_every_write_of_the_settings() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{ "version": 1, "lastUpdateCheck": "2026-11-01T08:00:00.000Z", "colour": "red" }"#,
        )
        .unwrap();
        let store = store_in(dir.path());
        assert_eq!(
            store.last_update_check().as_deref(),
            Some("2026-11-01T08:00:00.000Z")
        );
        store.apply(&change(SettingKey::Theme, "dark")).unwrap();
        assert_eq!(
            file_json(dir.path()),
            serde_json::json!({ "version": 1, "lastUpdateCheck": "2026-11-01T08:00:00.000Z", "theme": "dark" })
        );
        assert!(store.snapshot().issues.is_empty());
    }

    #[test]
    fn recording_a_check_keeps_the_settings_and_is_no_change_of_a_setting() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        store
            .apply(&change(SettingKey::Language, "Italian"))
            .unwrap();
        store
            .record_update_check("2026-11-02T07:07:19.522Z")
            .unwrap();
        assert_eq!(
            file_json(dir.path()),
            serde_json::json!({ "version": 1, "language": "Italian", "lastUpdateCheck": "2026-11-02T07:07:19.522Z" })
        );
        assert_eq!(store.snapshot().revision, 2);
        assert_eq!(
            store_in(dir.path()).last_update_check().as_deref(),
            Some("2026-11-02T07:07:19.522Z")
        );
        assert!(store.record_update_check("yesterday").is_err());
    }

    #[test]
    fn a_check_is_not_recorded_over_an_unreadable_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(FILE_NAME), "{ not json").unwrap();
        let store = store_in(dir.path());
        assert!(store
            .record_update_check("2026-11-02T07:07:19.522Z")
            .unwrap_err()
            .contains("left unchanged"));
        assert_eq!(
            fs::read_to_string(dir.path().join(FILE_NAME)).unwrap(),
            "{ not json"
        );
    }

    #[test]
    fn concurrent_changes_are_serialised_and_all_land() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(store_in(dir.path()));
        let handles: Vec<_> = (1..=8)
            .map(|n| {
                let store = store.clone();
                std::thread::spawn(move || {
                    store
                        .apply(&change(SettingKey::MaxParallelAgents, &n.to_string()))
                        .unwrap();
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let final_value = store.current().max_parallel_agents;
        assert_eq!(file_json(dir.path())["maxParallelAgents"], final_value);
        assert_eq!(store.snapshot().revision, 9);
    }
}
