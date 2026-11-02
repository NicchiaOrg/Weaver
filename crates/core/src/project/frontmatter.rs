//! YAML front matter between two `---` lines.
//!
//! Keys are written in the order the Requirements list them (the caller passes it), absent
//! optional keys are simply not in the mapping and so not written, and unknown keys are kept
//! (after the known ones, in their original order) when a file is rewritten.

use serde_yaml::{Mapping, Value};

/// A file split into its front matter and the body after the closing `---` line.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub front: Mapping,
    pub body: String,
}

/// Splits normalized text into front matter and body.
///
/// Text that does not start with a `---` line has no front matter: the mapping is empty and
/// the body is the whole text. A front matter that is not closed, is not YAML or is not a
/// mapping is an error with its reason.
pub fn parse(text: &str) -> Result<Document, String> {
    let Some(rest) = text.strip_prefix("---\n") else {
        if text == "---" {
            return Err("the front matter is not closed (no closing `---` line)".to_owned());
        }
        return Ok(Document {
            front: Mapping::new(),
            body: text.to_owned(),
        });
    };
    let mut offset = 0;
    let mut closing = None;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches('\n') == "---" {
            closing = Some((offset, offset + line.len()));
            break;
        }
        offset += line.len();
    }
    let (yaml_end, body_start) = closing
        .ok_or_else(|| "the front matter is not closed (no closing `---` line)".to_owned())?;
    let value: Value = serde_yaml::from_str(&rest[..yaml_end])
        .map_err(|e| format!("invalid front matter: {e}"))?;
    let front = match value {
        Value::Null => Mapping::new(),
        Value::Mapping(map) => map,
        _ => return Err("the front matter must be a YAML mapping".to_owned()),
    };
    Ok(Document {
        front,
        body: rest[body_start..].to_owned(),
    })
}

/// The mapping with the keys of `key_order` first (those present, in that order), then the
/// unknown keys in their original order.
pub fn ordered(front: &Mapping, key_order: &[&str]) -> Mapping {
    let mut out = Mapping::new();
    for key in key_order {
        let key = Value::String((*key).to_owned());
        if let Some(value) = front.get(&key) {
            out.insert(key, value.clone());
        }
    }
    for (key, value) in front {
        if !out.contains_key(key) {
            out.insert(key.clone(), value.clone());
        }
    }
    out
}

/// Renders front matter (keys in `key_order`, unknown keys kept) followed by `body`. An empty
/// mapping renders no front matter at all.
pub fn render(front: &Mapping, key_order: &[&str], body: &str) -> String {
    if front.is_empty() {
        return body.to_owned();
    }
    let yaml = serde_yaml::to_string(&Value::Mapping(ordered(front, key_order)))
        .expect("a YAML mapping always serialises");
    format!("---\n{yaml}---\n{body}")
}

/// A string value of `key`.
pub fn get_str<'a>(front: &'a Mapping, key: &str) -> Option<&'a str> {
    front.get(key).and_then(Value::as_str)
}

/// A boolean value of `key`.
pub fn get_bool(front: &Mapping, key: &str) -> Option<bool> {
    front.get(key).and_then(Value::as_bool)
}

/// A list of strings under `key`; `None` when the key is absent, not a list, or holds anything
/// but strings.
pub fn get_str_list(front: &Mapping, key: &str) -> Option<Vec<String>> {
    front
        .get(key)?
        .as_sequence()?
        .iter()
        .map(|v| v.as_str().map(str::to_owned))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, Value)]) -> Mapping {
        let mut m = Mapping::new();
        for (k, v) in pairs {
            m.insert(Value::String((*k).to_owned()), v.clone());
        }
        m
    }

    #[test]
    fn splits_front_matter_and_body() {
        let doc = parse("---\ncolumn: todo\ntechnical: true\n---\n# Title\n\nBody\n").unwrap();
        assert_eq!(get_str(&doc.front, "column"), Some("todo"));
        assert_eq!(get_bool(&doc.front, "technical"), Some(true));
        assert_eq!(doc.body, "# Title\n\nBody\n");
    }

    #[test]
    fn text_without_front_matter_is_all_body() {
        let doc = parse("# Title\n").unwrap();
        assert!(doc.front.is_empty());
        assert_eq!(doc.body, "# Title\n");
        // `---` later in the text is a thematic break, not front matter.
        let doc = parse("# T\n\n---\n\nrest\n").unwrap();
        assert!(doc.front.is_empty());
    }

    #[test]
    fn a_dash_line_inside_the_body_does_not_end_the_front_matter_twice() {
        let doc = parse("---\na: 1\n---\nbody\n---\nmore\n").unwrap();
        assert_eq!(doc.body, "body\n---\nmore\n");
    }

    #[test]
    fn broken_front_matter_is_an_error_with_a_reason() {
        assert!(parse("---\na: 1\n# no closing\n")
            .unwrap_err()
            .contains("not closed"));
        assert!(parse("---\na: [1\n---\nx\n")
            .unwrap_err()
            .contains("invalid front matter"));
        assert!(parse("---\n- a\n- b\n---\nx\n")
            .unwrap_err()
            .contains("mapping"));
        assert!(parse("---").unwrap_err().contains("not closed"));
    }

    #[test]
    fn an_empty_front_matter_is_an_empty_mapping() {
        let doc = parse("---\n---\nbody\n").unwrap();
        assert!(doc.front.is_empty());
        assert_eq!(doc.body, "body\n");
    }

    #[test]
    fn keys_are_written_in_the_given_order_unknown_ones_kept_after() {
        let front = map(&[
            ("zzz", Value::String("kept".into())),
            ("created", Value::String("2026-11-02T07:07:19.522Z".into())),
            ("column", Value::String("todo".into())),
            ("aaa", Value::Bool(true)),
        ]);
        let text = render(&front, &["column", "technical", "created"], "# T\n");
        assert_eq!(
            text,
            "---\ncolumn: todo\ncreated: 2026-11-02T07:07:19.522Z\nzzz: kept\naaa: true\n---\n# T\n"
        );
        let again = parse(&text).unwrap();
        assert_eq!(get_str(&again.front, "zzz"), Some("kept"));
        assert_eq!(again.body, "# T\n");
    }

    #[test]
    fn absent_optional_keys_are_not_written() {
        let front = map(&[("column", Value::String("todo".into()))]);
        let text = render(&front, &["column", "proposal", "drift"], "x\n");
        assert!(!text.contains("proposal"));
        assert!(!text.contains("drift"));
    }

    #[test]
    fn strings_that_look_like_other_types_stay_strings_after_a_round_trip() {
        let front = map(&[
            ("note", Value::String("true".into())),
            ("n", Value::String("12".into())),
        ]);
        let text = render(&front, &["note", "n"], "");
        let doc = parse(&text).unwrap();
        assert_eq!(get_str(&doc.front, "note"), Some("true"));
        assert_eq!(get_str(&doc.front, "n"), Some("12"));
    }

    #[test]
    fn list_helper_rejects_non_string_items() {
        let doc = parse("---\nrefs:\n  - a\n  - b\nbad:\n  - 1\n---\n").unwrap();
        assert_eq!(
            get_str_list(&doc.front, "refs"),
            Some(vec!["a".to_owned(), "b".to_owned()])
        );
        assert_eq!(get_str_list(&doc.front, "bad"), None);
        assert_eq!(get_str_list(&doc.front, "missing"), None);
    }
}
