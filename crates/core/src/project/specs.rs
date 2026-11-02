//! Specs (`specs.yaml`): the project's technologies and their constraints.
//!
//! Every field is optional and is a list of entries, except `notes`, which is Markdown text.
//! An entry has `name` (required) and optionally `version`, `note` and `chosenByAgent`; names
//! of one field are compared without case and cannot repeat.

use std::collections::BTreeSet;

use serde::Serialize;
use serde_yaml::Value;

/// The fields, in the order they are written. `notes` is the only one that is not a list.
pub const SPECS_FIELDS: [&str; 10] = [
    "languages",
    "frameworks",
    "data",
    "testing",
    "ci",
    "repository",
    "deployment",
    "cloud",
    "infrastructure",
    "notes",
];

/// The name of the field of free Markdown text.
pub const NOTES_FIELD: &str = "notes";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecEntry {
    pub name: String,
    pub version: Option<String>,
    pub note: Option<String>,
    #[serde(rename = "chosenByAgent")]
    pub chosen_by_agent: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Specs {
    /// The list fields present, in [`SPECS_FIELDS`] order.
    pub lists: Vec<(String, Vec<SpecEntry>)>,
    pub notes: Option<String>,
}

impl Specs {
    /// Whether the Specs hold `field` (a list field, even empty, or `notes`).
    pub fn has_field(&self, field: &str) -> bool {
        if field == NOTES_FIELD {
            self.notes.is_some()
        } else {
            self.lists.iter().any(|(name, _)| name == field)
        }
    }

    pub fn entries(&self, field: &str) -> Option<&[SpecEntry]> {
        self.lists
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, e)| e.as_slice())
    }
}

const ENTRY_KEYS: [&str; 4] = ["name", "version", "note", "chosenByAgent"];

/// Parses the text of a `specs.yaml`; an empty file is empty Specs. Reports every error with
/// its field and entry.
pub fn parse_specs(text: &str) -> Result<Specs, Vec<String>> {
    let value: Value =
        serde_yaml::from_str(text).map_err(|e| vec![format!("invalid YAML: {e}")])?;
    let map = match value {
        Value::Null => return Ok(Specs::default()),
        Value::Mapping(map) => map,
        _ => return Err(vec!["the Specs must be a YAML mapping of fields".to_owned()]),
    };
    let mut errors = Vec::new();
    let mut lists: Vec<(String, Vec<SpecEntry>)> = Vec::new();
    let mut notes = None;
    for (key, value) in &map {
        let Some(field) = key.as_str() else {
            errors.push("a field name is not text".to_owned());
            continue;
        };
        if !SPECS_FIELDS.contains(&field) {
            errors.push(format!(
                "unknown field `{field}` (fields: {})",
                SPECS_FIELDS.join(", ")
            ));
            continue;
        }
        if field == NOTES_FIELD {
            match value {
                Value::String(s) => notes = Some(s.clone()),
                Value::Null => notes = Some(String::new()),
                _ => errors.push("`notes` must be Markdown text".to_owned()),
            }
            continue;
        }
        if let Some(entries) = parse_list(field, value, &mut errors) {
            lists.push((field.to_owned(), entries));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    lists.sort_by_key(|(name, _)| SPECS_FIELDS.iter().position(|f| f == name));
    Ok(Specs { lists, notes })
}

fn parse_list(field: &str, value: &Value, errors: &mut Vec<String>) -> Option<Vec<SpecEntry>> {
    let items = match value {
        Value::Null => return Some(Vec::new()),
        Value::Sequence(items) => items,
        _ => {
            errors.push(format!("`{field}` must be a list of entries"));
            return None;
        }
    };
    let mut entries = Vec::new();
    let mut names = BTreeSet::new();
    for (index, item) in items.iter().enumerate() {
        let place = format!("`{field}` entry {}", index + 1);
        let Value::Mapping(entry) = item else {
            errors.push(format!("{place} must be a mapping with a `name`"));
            continue;
        };
        for key in entry.keys() {
            if !key.as_str().is_some_and(|k| ENTRY_KEYS.contains(&k)) {
                errors.push(format!(
                    "{place}: unknown key `{}` (an entry has name, version, note and chosenByAgent)",
                    key.as_str().unwrap_or("?")
                ));
            }
        }
        let name = match entry.get("name") {
            Some(Value::String(n)) if !n.trim().is_empty() => n.trim().to_owned(),
            Some(_) => {
                errors.push(format!("{place}: `name` must be non-empty text"));
                continue;
            }
            None => {
                errors.push(format!("{place}: `name` is required"));
                continue;
            }
        };
        let place = format!("`{field}` entry `{name}`");
        if !names.insert(name.to_lowercase()) {
            errors.push(format!(
                "{place}: the name is used twice in `{field}` (names are compared without case)"
            ));
        }
        let mut text_of = |key: &str| -> Option<String> {
            match entry.get(key) {
                None | Some(Value::Null) => None,
                Some(Value::String(s)) => Some(s.clone()),
                Some(Value::Number(n)) => Some(n.to_string()),
                Some(Value::Bool(b)) => Some(b.to_string()),
                Some(_) => {
                    errors.push(format!("{place}: `{key}` must be text"));
                    None
                }
            }
        };
        entries.push(SpecEntry {
            name,
            version: text_of("version"),
            note: text_of("note"),
            chosen_by_agent: text_of("chosenByAgent"),
        });
    }
    Some(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = "languages:
  - name: TypeScript
    version: \"5.6\"
frameworks:
  - name: React
    chosenByAgent: fits the existing screens and the team's experience
data:
  - name: PostgreSQL
    version: \"16\"
    note: holds orders and payments
notes: |
  Free Markdown: constraints that do not fit an entry.
";

    fn errors(text: &str) -> String {
        parse_specs(text).expect_err("must fail").join(" | ")
    }

    #[test]
    fn the_format_example_parses() {
        let specs = parse_specs(EXAMPLE).unwrap();
        let fields: Vec<_> = specs.lists.iter().map(|(f, _)| f.as_str()).collect();
        assert_eq!(fields, ["languages", "frameworks", "data"]);
        let ts = &specs.entries("languages").unwrap()[0];
        assert_eq!(
            (ts.name.as_str(), ts.version.as_deref()),
            ("TypeScript", Some("5.6"))
        );
        assert_eq!(
            specs.entries("frameworks").unwrap()[0]
                .chosen_by_agent
                .as_deref(),
            Some("fits the existing screens and the team's experience")
        );
        assert_eq!(
            specs.entries("data").unwrap()[0].note.as_deref(),
            Some("holds orders and payments")
        );
        assert_eq!(
            specs.notes.as_deref(),
            Some("Free Markdown: constraints that do not fit an entry.\n")
        );
        assert!(specs.has_field("languages") && specs.has_field("notes") && !specs.has_field("ci"));
    }

    #[test]
    fn fields_are_listed_in_the_documented_order_whatever_the_file_order() {
        let specs = parse_specs("data:\n  - name: A\nlanguages:\n  - name: B\n").unwrap();
        let fields: Vec<_> = specs.lists.iter().map(|(f, _)| f.as_str()).collect();
        assert_eq!(fields, ["languages", "data"]);
    }

    #[test]
    fn an_empty_file_is_empty_specs() {
        assert_eq!(parse_specs("").unwrap(), Specs::default());
        assert_eq!(parse_specs("\n").unwrap(), Specs::default());
    }

    #[test]
    fn unknown_fields_are_errors() {
        let e = errors("languagez:\n  - name: A\n");
        assert!(e.contains("unknown field `languagez`"), "{e}");
    }

    #[test]
    fn list_fields_are_lists_and_notes_is_text() {
        assert!(errors("languages: Rust\n").contains("`languages` must be a list"));
        assert!(errors("notes:\n  - a\n").contains("`notes` must be Markdown text"));
        assert!(errors("- a\n").contains("YAML mapping"));
        assert!(errors("languages: [\n").contains("invalid YAML"));
    }

    #[test]
    fn an_entry_needs_a_name_and_knows_only_four_keys() {
        assert!(errors("languages:\n  - version: '1'\n")
            .contains("`languages` entry 1: `name` is required"));
        assert!(errors("languages:\n  - name: ''\n").contains("non-empty"));
        assert!(errors("languages:\n  - A\n").contains("must be a mapping"));
        let e = errors("languages:\n  - name: A\n    colour: red\n");
        assert!(
            e.contains("`languages` entry 1: unknown key `colour`"),
            "{e}"
        );
        assert!(errors("languages:\n  - name: A\n    note: [1]\n").contains("`note` must be text"));
    }

    #[test]
    fn names_are_unique_within_a_field_without_case() {
        let e = errors("languages:\n  - name: Rust\n  - name: rust\n");
        assert!(
            e.contains("`languages` entry `rust`: the name is used twice"),
            "{e}"
        );
        // The same name in two fields is fine.
        assert!(parse_specs("languages:\n  - name: Rust\ntesting:\n  - name: Rust\n").is_ok());
    }

    #[test]
    fn unquoted_numeric_versions_are_read_as_text() {
        let specs = parse_specs("languages:\n  - name: Go\n    version: 1.22\n").unwrap();
        assert_eq!(
            specs.entries("languages").unwrap()[0].version.as_deref(),
            Some("1.22")
        );
    }

    #[test]
    fn all_errors_are_reported_together() {
        let e = parse_specs("languages:\n  - name: A\n  - name: a\nnope: 1\n").unwrap_err();
        assert_eq!(e.len(), 2, "{e:?}");
    }
}
