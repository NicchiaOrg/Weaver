//! Weaver's settings (`requirements/data#developer.CHANGE_SETTINGS`): one typed schema with its
//! defaults and validation, read from and written to one `settings.json` in the app's config
//! folder.
//!
//! | Module       | Responsibility |
//! |--------------|----------------|
//! | this one     | [`Settings`], [`SettingKey`], parsing of one value, reading and writing the file text |
//! | [`schema`]   | Titles, descriptions, defaults and choices the Settings view shows |
//! | [`editor`]   | Editor presets, custom command splitting and filling |
//! | [`store`]    | [`SettingsStore`]: the values in force, serialised atomic writes, revisions |
//!
//! The TypeScript types of the UI are generated from these types; the backend is the only reader
//! and writer of the file. Nothing in a project, in the repository or in the environment is read
//! as a setting. Weaver v1 stores no secret (a secret would live in the OS keychain, `keyring`
//! service `weaver`, never here).
//!
//! # Reading a setting from another Task
//!
//! In the app: `app.state::<AppState>().settings.current()` gives the [`Settings`] in force now;
//! take it when starting an omp process and keep it, a running job never sees a later change.
//! `settings.subscribe()` delivers every change with the keys that changed (omp setup reacts to
//! [`SettingKey::OmpPath`]). [`Settings::editor_command`] builds the argument list of the editor.

pub mod editor;
pub mod schema;
pub mod store;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use ts_rs::TS;

pub use editor::{CommandError, EditorPreset};
pub use schema::{field, schema, Choice, FieldKind, SettingField};
pub use store::{Applied, SettingsError, SettingsSnapshot, SettingsStore};

/// Name of the file in the config folder.
pub const FILE_NAME: &str = "settings.json";

/// Value of `"version"` in the file.
pub const FILE_VERSION: u32 = 1;

/// The key of the file that holds the instant of the last check for an update of Weaver. It is
/// not a setting (the developer never sees or changes it) but the file keeps it, and every write
/// of the settings leaves it as it is.
pub const LAST_UPDATE_CHECK_KEY: &str = "lastUpdateCheck";

/// The theme of every window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follows the system.
    Auto,
    Light,
    Dark,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Auto, Theme::Light, Theme::Dark];

    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Auto => "auto",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.as_str() == text)
    }

    pub fn label(self) -> &'static str {
        match self {
            Theme::Auto => "Auto",
            Theme::Light => "Light",
            Theme::Dark => "Dark",
        }
    }
}

/// Whether omp asks before acting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum Permissions {
    /// omp runs everything without asking.
    Yolo,
    Ask,
}

impl Permissions {
    pub const ALL: [Permissions; 2] = [Permissions::Yolo, Permissions::Ask];

    pub fn as_str(self) -> &'static str {
        match self {
            Permissions::Yolo => "yolo",
            Permissions::Ask => "ask",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|permissions| permissions.as_str() == text)
    }

    pub fn label(self) -> &'static str {
        match self {
            Permissions::Yolo => "Run without asking",
            Permissions::Ask => "Ask before acting",
        }
    }
}

/// The model of each role: a name as omp accepts it, with an optional `:<thinking level>`
/// suffix. Empty means the default of omp. Never checked against a list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModelSettings {
    pub analysis: String,
    pub scan: String,
    pub execution: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EditorSettings {
    pub preset: EditorPreset,
    /// Used only with the `custom` preset; must contain `{file}`.
    pub command: String,
}

/// The System Certificates group (`requirements/tech#system-certificates`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SystemCertificatesSettings {
    /// Whether omp trusts the certificates the system trusts (off by default). The first boolean
    /// setting; its text form is `true` or `false`.
    pub enabled: bool,
}

/// The message of a refused boolean.
pub const BOOLEAN_MESSAGE: &str = "Choose on or off.";

/// The settings in force.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Trimmed; empty means "look for omp".
    pub omp_path: String,
    pub models: ModelSettings,
    /// At least 1.
    pub max_parallel_agents: u32,
    /// The language of the agent's texts, free text, trimmed; empty adds no language sentence.
    pub language: String,
    pub theme: Theme,
    pub editor: EditorSettings,
    pub permissions: Permissions,
    pub system_certificates: SystemCertificatesSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            omp_path: String::new(),
            models: ModelSettings::default(),
            max_parallel_agents: 20,
            language: String::new(),
            theme: Theme::Auto,
            editor: EditorSettings {
                preset: EditorPreset::Vscode,
                command: String::new(),
            },
            permissions: Permissions::Yolo,
            system_certificates: SystemCertificatesSettings { enabled: false },
        }
    }
}

/// One setting. The serialised form is the dotted key of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
pub enum SettingKey {
    #[serde(rename = "ompPath")]
    OmpPath,
    #[serde(rename = "models.analysis")]
    ModelsAnalysis,
    #[serde(rename = "models.scan")]
    ModelsScan,
    #[serde(rename = "models.execution")]
    ModelsExecution,
    #[serde(rename = "maxParallelAgents")]
    MaxParallelAgents,
    #[serde(rename = "language")]
    Language,
    #[serde(rename = "theme")]
    Theme,
    #[serde(rename = "editor.preset")]
    EditorPreset,
    #[serde(rename = "editor.command")]
    EditorCommand,
    #[serde(rename = "permissions")]
    Permissions,
    #[serde(rename = "systemCertificates.enabled")]
    SystemCertificatesEnabled,
}

/// The message of a refused whole number.
pub const WHOLE_NUMBER_MESSAGE: &str = "Enter a whole number of at least 1.";

impl SettingKey {
    pub const ALL: [SettingKey; 11] = [
        SettingKey::OmpPath,
        SettingKey::ModelsAnalysis,
        SettingKey::ModelsScan,
        SettingKey::ModelsExecution,
        SettingKey::MaxParallelAgents,
        SettingKey::Language,
        SettingKey::Theme,
        SettingKey::EditorPreset,
        SettingKey::EditorCommand,
        SettingKey::Permissions,
        SettingKey::SystemCertificatesEnabled,
    ];

    /// The key in the file: `models.analysis` is `analysis` inside `models`.
    pub fn path(self) -> &'static str {
        match self {
            SettingKey::OmpPath => "ompPath",
            SettingKey::ModelsAnalysis => "models.analysis",
            SettingKey::ModelsScan => "models.scan",
            SettingKey::ModelsExecution => "models.execution",
            SettingKey::MaxParallelAgents => "maxParallelAgents",
            SettingKey::Language => "language",
            SettingKey::Theme => "theme",
            SettingKey::EditorPreset => "editor.preset",
            SettingKey::EditorCommand => "editor.command",
            SettingKey::Permissions => "permissions",
            SettingKey::SystemCertificatesEnabled => "systemCertificates.enabled",
        }
    }
}

/// A change the developer asks for: the key and the new value as text (a whole number as its
/// digits). The backend validates it; nothing is stored when it is refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SettingChange {
    pub key: SettingKey,
    pub value: String,
}

/// A key of the file that was ignored, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct SettingIssue {
    pub key: SettingKey,
    pub reason: String,
}

impl Settings {
    /// The value of `key` as text (a whole number as its digits).
    pub fn text(&self, key: SettingKey) -> String {
        match key {
            SettingKey::OmpPath => self.omp_path.clone(),
            SettingKey::ModelsAnalysis => self.models.analysis.clone(),
            SettingKey::ModelsScan => self.models.scan.clone(),
            SettingKey::ModelsExecution => self.models.execution.clone(),
            SettingKey::MaxParallelAgents => self.max_parallel_agents.to_string(),
            SettingKey::Language => self.language.clone(),
            SettingKey::Theme => self.theme.as_str().to_owned(),
            SettingKey::EditorPreset => self.editor.preset.as_str().to_owned(),
            SettingKey::EditorCommand => self.editor.command.clone(),
            SettingKey::Permissions => self.permissions.as_str().to_owned(),
            SettingKey::SystemCertificatesEnabled => self.system_certificates.enabled.to_string(),
        }
    }

    /// Whether `key` holds its default.
    pub fn is_default(&self, key: SettingKey) -> bool {
        self.text(key) == Settings::default().text(key)
    }

    /// Sets `key` from text, validating it. Text values are trimmed. On refusal nothing changes
    /// and the reason is the message the developer reads.
    pub fn set_text(&mut self, key: SettingKey, text: &str) -> Result<(), String> {
        let trimmed = text.trim();
        match key {
            SettingKey::OmpPath => self.omp_path = trimmed.to_owned(),
            SettingKey::ModelsAnalysis => self.models.analysis = trimmed.to_owned(),
            SettingKey::ModelsScan => self.models.scan = trimmed.to_owned(),
            SettingKey::ModelsExecution => self.models.execution = trimmed.to_owned(),
            SettingKey::Language => self.language = trimmed.to_owned(),
            SettingKey::MaxParallelAgents => {
                let digits_only =
                    !trimmed.is_empty() && trimmed.bytes().all(|b| b.is_ascii_digit());
                match trimmed.parse::<u32>() {
                    Ok(n) if digits_only && n >= 1 => self.max_parallel_agents = n,
                    _ => return Err(WHOLE_NUMBER_MESSAGE.to_owned()),
                }
            }
            SettingKey::Theme => {
                self.theme = Theme::parse(trimmed)
                    .ok_or_else(|| "Choose Auto, Light or Dark.".to_owned())?;
            }
            SettingKey::EditorPreset => {
                self.editor.preset = EditorPreset::parse(trimmed)
                    .ok_or_else(|| "Choose one of the editors in the list.".to_owned())?;
            }
            SettingKey::EditorCommand => {
                // Empty is the default (no custom command); it is refused only while the custom
                // preset is in force, or by `editor_command` when the preset is used.
                if !trimmed.is_empty() || self.editor.preset == EditorPreset::Custom {
                    editor::parse_custom(trimmed).map_err(|e| e.to_string())?;
                }
                self.editor.command = trimmed.to_owned();
            }
            SettingKey::Permissions => {
                self.permissions = Permissions::parse(trimmed)
                    .ok_or_else(|| "Choose Run without asking or Ask before acting.".to_owned())?;
            }
            SettingKey::SystemCertificatesEnabled => {
                self.system_certificates.enabled = match trimmed {
                    "true" => true,
                    "false" => false,
                    _ => return Err(BOOLEAN_MESSAGE.to_owned()),
                };
            }
        }
        Ok(())
    }

    /// Sets `key` from a JSON value of the file. A value of the wrong type is a refusal like any
    /// other invalid value.
    fn set_json(&mut self, key: SettingKey, value: &Value) -> Result<(), String> {
        let text = match (key, value) {
            (SettingKey::SystemCertificatesEnabled, Value::Bool(on)) => on.to_string(),
            (SettingKey::SystemCertificatesEnabled, _) => {
                return Err("Expected true or false.".to_owned())
            }
            (SettingKey::MaxParallelAgents, Value::Number(n)) => n
                .as_u64()
                .map(|n| n.to_string())
                .unwrap_or_else(|| n.to_string()),
            (SettingKey::MaxParallelAgents, _) => return Err("Expected a whole number.".to_owned()),
            (_, Value::String(s)) => s.clone(),
            _ => return Err("Expected text.".to_owned()),
        };
        self.set_text(key, &text)
    }

    /// The program and arguments that open `file` (absolute) at `line` (1 when unknown) in the
    /// editor of the settings. The first element is the program, to be found on the PATH with the
    /// rules of omp; nothing is run through a shell.
    pub fn editor_command(
        &self,
        file: &str,
        line: Option<u32>,
    ) -> Result<Vec<String>, CommandError> {
        let line = editor::effective_line(line);
        let template: Vec<String> = match self.editor.preset.template() {
            Some(template) => template.iter().map(|word| (*word).to_owned()).collect(),
            None => editor::parse_custom(&self.editor.command)?,
        };
        Ok(template
            .iter()
            .map(|word| editor::fill(word, file, line))
            .collect())
    }

    /// The settings of a file's text. A missing key is the default; an unknown key is ignored; a
    /// key of the wrong type or with an invalid value counts as its default and is listed in the
    /// issues. A text that is not a JSON object is an error (every default then).
    pub fn parse_file(text: &str) -> Result<(Settings, Vec<SettingIssue>), String> {
        let value: Value =
            serde_json::from_str(text).map_err(|e| format!("it is not valid JSON ({e})"))?;
        let Value::Object(root) = value else {
            return Err("it does not hold a JSON object".to_owned());
        };
        let mut settings = Settings::default();
        let mut issues = Vec::new();
        for key in SettingKey::ALL {
            let mut parts = key.path().split('.');
            let first = parts.next().expect("a key has a path");
            let found = match parts.next() {
                None => root.get(first),
                Some(second) => match root.get(first) {
                    Some(Value::Object(inner)) => inner.get(second),
                    Some(_) => {
                        issues.push(SettingIssue {
                            key,
                            reason: "Expected an object around it.".to_owned(),
                        });
                        continue;
                    }
                    None => None,
                },
            };
            if let Some(value) = found {
                // `set_json` leaves the field untouched when it refuses.
                if let Err(reason) = settings.set_json(key, value) {
                    issues.push(SettingIssue { key, reason });
                }
            }
        }
        Ok((settings, issues))
    }

    /// The instant of the last check for an update held by a file's text: a UTC ISO 8601 instant
    /// under [`LAST_UPDATE_CHECK_KEY`]; anything else is no instant.
    pub fn read_last_update_check(text: &str) -> Option<String> {
        let value: Value = serde_json::from_str(text).ok()?;
        let instant = value.get(LAST_UPDATE_CHECK_KEY)?.as_str()?;
        crate::project::clock::is_instant(instant).then(|| instant.to_owned())
    }

    /// The text of the file: `"version"` and only the values that differ from the default, so
    /// setting a value back to its default removes its key.
    pub fn file_text(&self) -> String {
        self.file_text_with(None)
    }

    /// Like [`Settings::file_text`], with the instant of the last check for an update of Weaver
    /// (`lastUpdateCheck`), which the file keeps next to the settings.
    pub fn file_text_with(&self, last_update_check: Option<&str>) -> String {
        let mut root = Map::new();
        root.insert("version".to_owned(), Value::from(FILE_VERSION));
        if let Some(instant) = last_update_check {
            root.insert(LAST_UPDATE_CHECK_KEY.to_owned(), Value::from(instant));
        }
        for key in SettingKey::ALL
            .into_iter()
            .filter(|key| !self.is_default(*key))
        {
            let value = match key {
                SettingKey::MaxParallelAgents => Value::from(self.max_parallel_agents),
                SettingKey::SystemCertificatesEnabled => {
                    Value::from(self.system_certificates.enabled)
                }
                _ => Value::from(self.text(key)),
            };
            match key.path().split_once('.') {
                None => {
                    root.insert(key.path().to_owned(), value);
                }
                Some((outer, inner)) => {
                    let Value::Object(group) = root
                        .entry(outer.to_owned())
                        .or_insert_with(|| Value::Object(Map::new()))
                    else {
                        unreachable!("only objects are inserted under a group key");
                    };
                    group.insert(inner.to_owned(), value);
                }
            }
        }
        let mut text =
            serde_json::to_string_pretty(&Value::Object(root)).expect("a JSON value serialises");
        text.push('\n');
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_documented_ones() {
        let defaults = Settings::default();
        assert_eq!(defaults.omp_path, "");
        assert_eq!(defaults.models, ModelSettings::default());
        assert_eq!(defaults.max_parallel_agents, 20);
        assert_eq!(defaults.language, "");
        assert_eq!(defaults.theme, Theme::Auto);
        assert_eq!(defaults.editor.preset, EditorPreset::Vscode);
        assert_eq!(defaults.editor.command, "");
        assert_eq!(defaults.permissions, Permissions::Yolo);
        assert!(!defaults.system_certificates.enabled);
    }

    #[test]
    fn the_boolean_setting_reads_writes_and_refuses_like_the_others() {
        let key = SettingKey::SystemCertificatesEnabled;
        let mut settings = Settings::default();
        settings.set_text(key, " true ").unwrap();
        assert!(settings.system_certificates.enabled);
        assert_eq!(settings.text(key), "true");
        assert!(!settings.is_default(key));
        for bad in ["", "yes", "1", "True", "on"] {
            assert_eq!(
                settings.set_text(key, bad),
                Err(BOOLEAN_MESSAGE.to_owned()),
                "{bad}"
            );
            assert!(settings.system_certificates.enabled);
        }
        // The file holds a JSON boolean, and only when it differs from the default.
        let file: Value = serde_json::from_str(&settings.file_text()).unwrap();
        assert_eq!(
            file,
            serde_json::json!({ "version": 1, "systemCertificates": { "enabled": true } })
        );
        settings.set_text(key, "false").unwrap();
        let file: Value = serde_json::from_str(&settings.file_text()).unwrap();
        assert_eq!(file, serde_json::json!({ "version": 1 }));
        // Reading: a JSON boolean is accepted.
        let (read, issues) =
            Settings::parse_file(r#"{"systemCertificates":{"enabled":true}}"#).unwrap();
        assert!(read.system_certificates.enabled && issues.is_empty());
    }

    #[test]
    fn a_wrong_type_for_the_boolean_counts_as_the_default_and_is_reported() {
        for text in [
            r#"{"systemCertificates":{"enabled":"true"}}"#,
            r#"{"systemCertificates":{"enabled":1}}"#,
            r#"{"systemCertificates":{"enabled":null}}"#,
        ] {
            let (read, issues) = Settings::parse_file(text).unwrap();
            assert!(!read.system_certificates.enabled, "{text}");
            assert_eq!(issues.len(), 1, "{text}");
            assert_eq!(issues[0].key, SettingKey::SystemCertificatesEnabled);
            assert!(!issues[0].reason.is_empty());
        }
        let (read, issues) = Settings::parse_file(r#"{"systemCertificates":true}"#).unwrap();
        assert!(!read.system_certificates.enabled);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn keys_serialise_as_their_file_path_and_every_key_has_one() {
        for key in SettingKey::ALL {
            assert_eq!(
                serde_json::to_value(key).unwrap(),
                serde_json::json!(key.path())
            );
            assert_eq!(
                serde_json::from_value::<SettingKey>(serde_json::json!(key.path())).unwrap(),
                key
            );
        }
    }

    #[test]
    fn enum_names_match_their_serialised_form() {
        for theme in Theme::ALL {
            assert_eq!(
                serde_json::to_value(theme).unwrap(),
                serde_json::json!(theme.as_str())
            );
            assert_eq!(Theme::parse(theme.as_str()), Some(theme));
        }
        for permissions in Permissions::ALL {
            assert_eq!(
                serde_json::to_value(permissions).unwrap(),
                serde_json::json!(permissions.as_str())
            );
            assert_eq!(Permissions::parse(permissions.as_str()), Some(permissions));
        }
    }

    #[test]
    fn the_ui_gets_camel_case_nested_values() {
        let json = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "ompPath": "",
                "models": { "analysis": "", "scan": "", "execution": "" },
                "maxParallelAgents": 20,
                "language": "",
                "theme": "auto",
                "editor": { "preset": "vscode", "command": "" },
                "permissions": "yolo",
                "systemCertificates": { "enabled": false },
            })
        );
    }

    #[test]
    fn text_values_are_trimmed() {
        let mut settings = Settings::default();
        settings
            .set_text(SettingKey::OmpPath, "  /usr/bin/omp \n")
            .unwrap();
        settings
            .set_text(SettingKey::Language, " Italian ")
            .unwrap();
        settings
            .set_text(SettingKey::ModelsScan, " claude-haiku:low ")
            .unwrap();
        assert_eq!(settings.omp_path, "/usr/bin/omp");
        assert_eq!(settings.language, "Italian");
        assert_eq!(settings.models.scan, "claude-haiku:low");
    }

    #[test]
    fn model_names_are_never_checked() {
        let mut settings = Settings::default();
        settings
            .set_text(SettingKey::ModelsAnalysis, "no/such model:whatever")
            .unwrap();
        assert_eq!(settings.models.analysis, "no/such model:whatever");
    }

    #[test]
    fn the_parallel_agents_must_be_a_whole_number_of_at_least_one() {
        let mut settings = Settings::default();
        for ok in ["1", "20", " 7 ", "4294967295"] {
            settings
                .set_text(SettingKey::MaxParallelAgents, ok)
                .unwrap();
        }
        assert_eq!(settings.max_parallel_agents, u32::MAX);
        for bad in ["", "0", "-1", "1.5", "abc", "1e3", "+2", "4294967296", "١٢"] {
            let before = settings.clone();
            assert_eq!(
                settings.set_text(SettingKey::MaxParallelAgents, bad),
                Err("Enter a whole number of at least 1.".to_owned()),
                "{bad:?}"
            );
            assert_eq!(settings, before);
        }
    }

    #[test]
    fn enumerations_refuse_unknown_values() {
        let mut settings = Settings::default();
        assert!(settings.set_text(SettingKey::Theme, "sepia").is_err());
        assert!(settings
            .set_text(SettingKey::EditorPreset, "emacs")
            .is_err());
        assert!(settings.set_text(SettingKey::Permissions, "maybe").is_err());
        assert_eq!(settings, Settings::default());
        settings.set_text(SettingKey::Theme, "dark").unwrap();
        settings.set_text(SettingKey::EditorPreset, "zed").unwrap();
        settings.set_text(SettingKey::Permissions, "ask").unwrap();
        assert_eq!(
            (settings.theme, settings.editor.preset, settings.permissions),
            (Theme::Dark, EditorPreset::Zed, Permissions::Ask)
        );
    }

    #[test]
    fn the_custom_command_must_contain_file_and_split() {
        let mut settings = Settings::default();
        settings
            .set_text(SettingKey::EditorPreset, "custom")
            .unwrap();
        assert_eq!(
            settings.set_text(SettingKey::EditorCommand, "myeditor"),
            Err("The command must contain {file}.".to_owned())
        );
        assert_eq!(
            settings.set_text(SettingKey::EditorCommand, ""),
            Err("The command must contain {file}.".to_owned())
        );
        assert!(settings
            .set_text(SettingKey::EditorCommand, "myeditor 'x {file}")
            .is_err());
        settings
            .set_text(SettingKey::EditorCommand, "myeditor +{line} {file}")
            .unwrap();
        assert_eq!(settings.editor.command, "myeditor +{line} {file}");
    }

    #[test]
    fn an_empty_command_is_the_default_while_another_preset_is_in_force() {
        let mut settings = Settings::default();
        settings.set_text(SettingKey::EditorCommand, "").unwrap();
        assert!(settings
            .set_text(SettingKey::EditorCommand, "no placeholder")
            .is_err());
    }

    #[test]
    fn the_editor_command_of_each_preset_is_filled() {
        let mut settings = Settings::default();
        let file = "/work/my project/src/a.rs";
        let expected = [
            (
                "vscode",
                vec!["code", "--goto", "/work/my project/src/a.rs:12"],
            ),
            (
                "cursor",
                vec!["cursor", "--goto", "/work/my project/src/a.rs:12"],
            ),
            ("zed", vec!["zed", "/work/my project/src/a.rs:12"]),
            (
                "jetbrains",
                vec!["idea", "--line", "12", "/work/my project/src/a.rs"],
            ),
            ("sublime", vec!["subl", "/work/my project/src/a.rs:12"]),
        ];
        for (preset, argv) in expected {
            settings.set_text(SettingKey::EditorPreset, preset).unwrap();
            assert_eq!(
                settings.editor_command(file, Some(12)).unwrap(),
                argv,
                "{preset}"
            );
        }
        settings
            .set_text(SettingKey::EditorPreset, "vscode")
            .unwrap();
        assert_eq!(
            settings.editor_command(file, None).unwrap()[2],
            "/work/my project/src/a.rs:1"
        );
    }

    #[test]
    fn the_custom_editor_command_is_split_then_filled_without_a_shell() {
        let mut settings = Settings::default();
        settings
            .set_text(SettingKey::EditorPreset, "custom")
            .unwrap();
        settings
            .set_text(
                SettingKey::EditorCommand,
                r#""my editor" --at={file}#{line} '{file}'"#,
            )
            .unwrap();
        assert_eq!(
            settings.editor_command("/a b/c.rs", Some(3)).unwrap(),
            ["my editor", "--at=/a b/c.rs#3", "/a b/c.rs"]
        );
    }

    #[test]
    fn the_custom_preset_without_a_command_cannot_open_anything() {
        let mut settings = Settings::default();
        settings
            .set_text(SettingKey::EditorPreset, "custom")
            .unwrap();
        assert_eq!(
            settings.editor_command("/a.rs", None),
            Err(CommandError::MissingFile)
        );
    }

    #[test]
    fn a_file_holds_only_the_version_and_the_values_that_differ_from_the_default() {
        assert_eq!(Settings::default().file_text(), "{\n  \"version\": 1\n}\n");
        let mut settings = Settings::default();
        settings.set_text(SettingKey::ModelsScan, "fast").unwrap();
        settings
            .set_text(SettingKey::MaxParallelAgents, "3")
            .unwrap();
        settings.set_text(SettingKey::Theme, "dark").unwrap();
        let value: Value = serde_json::from_str(&settings.file_text()).unwrap();
        assert_eq!(
            value,
            serde_json::json!({ "version": 1, "models": { "scan": "fast" }, "maxParallelAgents": 3, "theme": "dark" })
        );
    }

    #[test]
    fn what_is_written_is_read_back() {
        let mut settings = Settings::default();
        settings.set_text(SettingKey::OmpPath, "/opt/omp").unwrap();
        settings
            .set_text(SettingKey::ModelsExecution, "mid:high")
            .unwrap();
        settings
            .set_text(SettingKey::EditorPreset, "custom")
            .unwrap();
        settings
            .set_text(SettingKey::EditorCommand, "ed {file}")
            .unwrap();
        settings.set_text(SettingKey::Permissions, "ask").unwrap();
        let (read, issues) = Settings::parse_file(&settings.file_text()).unwrap();
        assert_eq!(read, settings);
        assert!(issues.is_empty());
    }

    #[test]
    fn unknown_keys_are_ignored_without_an_issue() {
        let (settings, issues) = Settings::parse_file(
            r#"{ "version": 1, "colour": "red", "models": { "other": "x", "scan": "s" } }"#,
        )
        .unwrap();
        assert_eq!(settings.models.scan, "s");
        assert!(issues.is_empty());
    }

    #[test]
    fn a_key_of_the_wrong_type_or_value_is_its_default_and_an_issue() {
        let (settings, issues) = Settings::parse_file(
            r#"{ "maxParallelAgents": "many", "theme": "sepia", "ompPath": 7, "language": "Italian", "models": { "scan": ["x"] }, "editor": { "preset": "zed", "command": "nope" } }"#,
        )
        .unwrap();
        assert_eq!(settings.max_parallel_agents, 20);
        assert_eq!(settings.theme, Theme::Auto);
        assert_eq!(settings.omp_path, "");
        assert_eq!(settings.models.scan, "");
        assert_eq!(settings.editor.command, "");
        assert_eq!(settings.language, "Italian");
        assert_eq!(settings.editor.preset, EditorPreset::Zed);
        let keys: Vec<_> = issues.iter().map(|i| i.key).collect();
        assert_eq!(
            keys,
            [
                SettingKey::OmpPath,
                SettingKey::ModelsScan,
                SettingKey::MaxParallelAgents,
                SettingKey::Theme,
                SettingKey::EditorCommand
            ]
        );
        assert!(issues.iter().all(|i| !i.reason.is_empty()));
    }

    #[test]
    fn a_group_that_is_not_an_object_ignores_its_keys() {
        let (settings, issues) = Settings::parse_file(r#"{ "models": "fast" }"#).unwrap();
        assert_eq!(settings, Settings::default());
        assert_eq!(issues.len(), 3);
    }

    #[test]
    fn a_whole_number_of_the_file_must_be_an_integer_of_at_least_one() {
        for bad in ["0", "-3", "2.5", "1e2", "99999999999"] {
            let (settings, issues) =
                Settings::parse_file(&format!(r#"{{ "maxParallelAgents": {bad} }}"#)).unwrap();
            assert_eq!(settings.max_parallel_agents, 20, "{bad}");
            assert_eq!(issues.len(), 1, "{bad}");
        }
        let (settings, _) = Settings::parse_file(r#"{ "maxParallelAgents": 5 }"#).unwrap();
        assert_eq!(settings.max_parallel_agents, 5);
    }

    #[test]
    fn text_that_is_not_a_json_object_does_not_parse() {
        assert!(Settings::parse_file("{ nope").is_err());
        assert!(Settings::parse_file("[1, 2]").is_err());
        assert!(Settings::parse_file("").is_err());
    }
}
