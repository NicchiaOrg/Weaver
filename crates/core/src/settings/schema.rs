//! What the Settings view shows of each setting: title, description, default and choices.
//!
//! The view takes all of it from here, so titles and defaults are written once and never drift
//! from the validation in [`super::Settings`]. Keys are never shown to the developer.

use serde::Serialize;
use ts_rs::TS;

use super::{EditorPreset, Permissions, SettingKey, Settings, Theme};

/// How the view edits a setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    /// Free text, saved on blur or Enter.
    Text,
    /// A model name: a combobox with free typing.
    Model,
    /// A whole number, saved on blur or Enter.
    Integer,
    /// One of the `choices`, saved on choice.
    Choice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct Choice {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SettingField {
    pub key: SettingKey,
    pub title: String,
    /// What the setting does, at most [`MAX_DESCRIPTION`] characters.
    pub description: String,
    /// A short sentence shown in bold before the description (the advice of a model role).
    pub advice: Option<String>,
    pub kind: FieldKind,
    /// The default, as text.
    pub default_value: String,
    /// The values the developer may choose, for `Choice` fields.
    pub choices: Vec<Choice>,
}

/// Longest description.
pub const MAX_DESCRIPTION: usize = 160;

fn choices<T: Copy>(
    all: &[T],
    value: fn(T) -> &'static str,
    label: fn(T) -> &'static str,
) -> Vec<Choice> {
    all.iter()
        .map(|item| Choice {
            value: value(*item).to_owned(),
            label: label(*item).to_owned(),
        })
        .collect()
}

/// The description of one setting.
pub fn field(key: SettingKey) -> SettingField {
    let (title, description, advice, kind): (&str, &str, Option<&str>, FieldKind) = match key {
        SettingKey::OmpPath => (
            "omp path",
            "Where the omp program is. Empty means the places the official installer uses.",
            None,
            FieldKind::Text,
        ),
        SettingKey::ModelsAnalysis => (
            "Analysis",
            "Designs Proposals and refines them. Empty means the default of omp.",
            Some("Your most capable model."),
            FieldKind::Model,
        ),
        SettingKey::ModelsScan => (
            "Scan",
            "Compares the Model with the code. Empty means the default of omp.",
            Some("A fast, light model."),
            FieldKind::Model,
        ),
        SettingKey::ModelsExecution => (
            "Execution",
            "Carries out Tasks. Empty means the default of omp.",
            Some("A mid-range model."),
            FieldKind::Model,
        ),
        SettingKey::MaxParallelAgents => (
            "Max parallel sub-agents",
            "How many sub-agents may work at the same time. A whole number of at least 1.",
            None,
            FieldKind::Integer,
        ),
        SettingKey::Language => (
            "Language of the agent's texts",
            "The language the agent writes its texts in. The interface stays in English. Empty means English.",
            None,
            FieldKind::Text,
        ),
        SettingKey::Theme => (
            "Theme",
            "Auto follows the system. A change applies to every window at once.",
            None,
            FieldKind::Choice,
        ),
        SettingKey::EditorPreset => (
            "Editor",
            "The editor that opens a file at a line.",
            None,
            FieldKind::Choice,
        ),
        SettingKey::EditorCommand => (
            "Custom command",
            "Program and arguments. {file} is required, {line} is optional. It runs without a shell.",
            None,
            FieldKind::Text,
        ),
        SettingKey::Permissions => (
            "Permissions",
            "Whether omp runs commands and edits files without asking the developer first.",
            None,
            FieldKind::Choice,
        ),
    };
    let choices = match key {
        SettingKey::Theme => choices(&Theme::ALL, Theme::as_str, Theme::label),
        SettingKey::EditorPreset => choices(
            &EditorPreset::ALL,
            EditorPreset::as_str,
            EditorPreset::label,
        ),
        SettingKey::Permissions => {
            choices(&Permissions::ALL, Permissions::as_str, Permissions::label)
        }
        _ => Vec::new(),
    };
    SettingField {
        key,
        title: title.to_owned(),
        description: description.to_owned(),
        advice: advice.map(str::to_owned),
        kind,
        default_value: Settings::default().text(key),
        choices,
    }
}

/// The description of every setting, in the order of [`SettingKey::ALL`].
pub fn schema() -> Vec<SettingField> {
    SettingKey::ALL.into_iter().map(field).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_setting_has_a_title_and_a_short_description() {
        let fields = schema();
        assert_eq!(fields.len(), SettingKey::ALL.len());
        for field in &fields {
            assert!(!field.title.is_empty());
            assert!(!field.description.is_empty());
            assert!(
                field.description.chars().count() <= MAX_DESCRIPTION,
                "{}",
                field.title
            );
        }
    }

    #[test]
    fn titles_never_show_a_key() {
        for field in schema() {
            assert!(!field.title.contains('.'), "{}", field.title);
            assert_ne!(field.title, field.key.path());
        }
    }

    #[test]
    fn defaults_come_from_the_settings_and_choices_from_the_enumerations() {
        let by_key = |key| field(key);
        assert_eq!(by_key(SettingKey::MaxParallelAgents).default_value, "20");
        assert_eq!(by_key(SettingKey::Theme).default_value, "auto");
        assert_eq!(by_key(SettingKey::EditorPreset).default_value, "vscode");
        assert_eq!(by_key(SettingKey::Permissions).default_value, "yolo");
        assert_eq!(by_key(SettingKey::OmpPath).default_value, "");
        let labels = |key| {
            by_key(key)
                .choices
                .into_iter()
                .map(|c| c.label)
                .collect::<Vec<_>>()
        };
        assert_eq!(labels(SettingKey::Theme), ["Auto", "Light", "Dark"]);
        assert_eq!(
            labels(SettingKey::Permissions),
            ["Run without asking", "Ask before acting"]
        );
        assert_eq!(
            labels(SettingKey::EditorPreset),
            [
                "VS Code",
                "Cursor",
                "Zed",
                "JetBrains",
                "Sublime Text",
                "Custom command"
            ]
        );
        assert!(by_key(SettingKey::Language).choices.is_empty());
    }

    #[test]
    fn the_model_roles_carry_their_advice() {
        assert_eq!(
            field(SettingKey::ModelsAnalysis).advice.as_deref(),
            Some("Your most capable model.")
        );
        assert_eq!(
            field(SettingKey::ModelsScan).advice.as_deref(),
            Some("A fast, light model.")
        );
        assert_eq!(
            field(SettingKey::ModelsExecution).advice.as_deref(),
            Some("A mid-range model.")
        );
        assert_eq!(field(SettingKey::Theme).advice, None);
    }

    #[test]
    fn texts_use_no_first_person() {
        for field in schema() {
            let text = format!(
                "{} {} {}",
                field.title,
                field.description,
                field.advice.clone().unwrap_or_default()
            );
            for word in text.split(|c: char| !c.is_alphabetic()) {
                assert!(
                    !matches!(word.to_lowercase().as_str(), "i" | "we" | "my" | "our"),
                    "{text}"
                );
            }
        }
    }
}
