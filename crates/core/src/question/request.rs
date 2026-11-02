//! The request of a round: what the agent (or Weaver, for the prepared question of a Drift) asks,
//! and the rules it must obey (`requirements/rules#question-round`).
//!
//! A round is validated whole, before it is queued: every problem is reported with the path of the
//! field (`questions[1].options[0].label`), and a round with any problem is refused entirely.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One option of a question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionOption {
    /// The answer; plain text.
    pub label: String,
    /// What choosing it means, in Markdown.
    pub description: String,
    /// More detail, in Markdown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// One question of a round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    /// The question, in Markdown.
    pub question: String,
    /// What the developer needs to answer, in Markdown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    pub options: Vec<QuestionOption>,
    /// The index of the recommended option.
    pub recommended: usize,
    pub multi: bool,
}

/// A validated round: at least one question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundRequest {
    pub questions: Vec<Question>,
}

/// A problem of a request, with the path of the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub path: String,
    pub message: String,
}

impl Problem {
    fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

/// The problems as one text, one per line.
pub fn describe(problems: &[Problem]) -> String {
    problems
        .iter()
        .map(|p| format!("{}: {}", p.path, p.message))
        .collect::<Vec<_>>()
        .join("\n")
}

const QUESTION_KEYS: &[&str] = &[
    "id",
    "question",
    "context",
    "options",
    "recommended",
    "multi",
];
const OPTION_KEYS: &[&str] = &["label", "description", "details"];

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    }
}

fn unknown_keys(
    object: &Map<String, Value>,
    known: &[&str],
    path: &str,
    problems: &mut Vec<Problem>,
) {
    for key in object.keys().filter(|key| !known.contains(&key.as_str())) {
        let at = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        problems.push(Problem::new(
            at,
            format!("unknown field (known: {})", known.join(", ")),
        ));
    }
}

/// A string field: required or optional, non-blank, and on one line when `line`.
fn text_field(
    object: &Map<String, Value>,
    key: &str,
    path: &str,
    required: bool,
    line: bool,
    problems: &mut Vec<Problem>,
) {
    let at = format!("{path}.{key}");
    match object.get(key) {
        None if required => problems.push(Problem::new(at, "missing required field")),
        None => {}
        Some(Value::String(text)) => {
            if text.trim().is_empty() {
                problems.push(Problem::new(at, "must not be blank"));
            } else if line && text.contains(['\n', '\r']) {
                problems.push(Problem::new(at, "must be on one line"));
            }
        }
        Some(other) => problems.push(Problem::new(
            at,
            format!("expected a string, found {}", kind(other)),
        )),
    }
}

fn check_option(option: &Value, path: &str, problems: &mut Vec<Problem>) {
    let Some(object) = option.as_object() else {
        problems.push(Problem::new(
            path,
            format!("expected an object, found {}", kind(option)),
        ));
        return;
    };
    unknown_keys(object, OPTION_KEYS, path, problems);
    text_field(object, "label", path, true, true, problems);
    text_field(object, "description", path, true, false, problems);
    text_field(object, "details", path, false, false, problems);
}

fn check_question(
    question: &Value,
    path: &str,
    seen: &mut HashSet<String>,
    problems: &mut Vec<Problem>,
) {
    let Some(object) = question.as_object() else {
        problems.push(Problem::new(
            path,
            format!("expected an object, found {}", kind(question)),
        ));
        return;
    };
    unknown_keys(object, QUESTION_KEYS, path, problems);
    text_field(object, "id", path, true, true, problems);
    if let Some(id) = object
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
    {
        if !seen.insert(id.to_owned()) {
            problems.push(Problem::new(
                format!("{path}.id"),
                format!("`{id}` is used by an earlier question: ids are unique"),
            ));
        }
    }
    text_field(object, "question", path, true, false, problems);
    text_field(object, "context", path, false, false, problems);

    let mut options = 0;
    match object.get("options") {
        None => problems.push(Problem::new(
            format!("{path}.options"),
            "missing required field",
        )),
        Some(Value::Array(list)) => {
            options = list.len();
            if list.len() < 2 {
                problems.push(Problem::new(
                    format!("{path}.options"),
                    format!("needs at least 2 options (found {})", list.len()),
                ));
            }
            let mut labels = HashSet::new();
            for (index, option) in list.iter().enumerate() {
                let at = format!("{path}.options[{index}]");
                check_option(option, &at, problems);
                if let Some(label) = option
                    .get("label")
                    .and_then(Value::as_str)
                    .filter(|l| !l.trim().is_empty())
                {
                    if !labels.insert(label.to_owned()) {
                        problems.push(Problem::new(format!("{at}.label"), format!("`{label}` is used by an earlier option: labels are unique within a question")));
                    }
                }
            }
        }
        Some(other) => problems.push(Problem::new(
            format!("{path}.options"),
            format!("expected a list, found {}", kind(other)),
        )),
    }

    match object.get("recommended") {
        None => problems.push(Problem::new(
            format!("{path}.recommended"),
            "missing required field",
        )),
        Some(value) => match value.as_i64() {
            Some(n) if n < 0 => problems.push(Problem::new(
                format!("{path}.recommended"),
                format!("must be at least 0 (found {n})"),
            )),
            Some(n) if options > 0 && n as usize >= options => problems.push(Problem::new(
                format!("{path}.recommended"),
                format!(
                    "must be the index of an option, 0 to {} (found {n})",
                    options - 1
                ),
            )),
            Some(_) => {}
            None => problems.push(Problem::new(
                format!("{path}.recommended"),
                format!("expected an integer, found {}", kind(value)),
            )),
        },
    }
    match object.get("multi") {
        None => problems.push(Problem::new(
            format!("{path}.multi"),
            "missing required field",
        )),
        Some(Value::Bool(_)) => {}
        Some(other) => problems.push(Problem::new(
            format!("{path}.multi"),
            format!("expected a boolean, found {}", kind(other)),
        )),
    }
}

/// Every problem of `input` (the object `{questions: [...]}`), in the order of the fields.
pub fn problems(input: &Value) -> Vec<Problem> {
    let mut problems = Vec::new();
    let Some(object) = input.as_object() else {
        problems.push(Problem::new(
            "",
            format!("the input must be an object, found {}", kind(input)),
        ));
        return problems;
    };
    unknown_keys(object, &["questions"], "", &mut problems);
    match object.get("questions") {
        None => problems.push(Problem::new("questions", "missing required field")),
        Some(Value::Array(list)) => {
            if list.is_empty() {
                problems.push(Problem::new("questions", "needs at least 1 question"));
            }
            let mut seen = HashSet::new();
            for (index, question) in list.iter().enumerate() {
                check_question(
                    question,
                    &format!("questions[{index}]"),
                    &mut seen,
                    &mut problems,
                );
            }
        }
        Some(other) => problems.push(Problem::new(
            "questions",
            format!("expected a list, found {}", kind(other)),
        )),
    }
    problems
}

/// Validates `input` whole and reads it.
pub fn parse(input: &Value) -> Result<RoundRequest, Vec<Problem>> {
    let found = problems(input);
    if !found.is_empty() {
        return Err(found);
    }
    serde_json::from_value(input.clone()).map_err(|e| vec![Problem::new("", e.to_string())])
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn option(label: &str) -> Value {
        json!({ "label": label, "description": "what it means" })
    }

    fn question(id: &str) -> Value {
        json!({ "id": id, "question": "Which one?", "options": [option("a"), option("b")], "recommended": 0, "multi": false })
    }

    fn paths(input: Value) -> Vec<String> {
        problems(&input).into_iter().map(|p| p.path).collect()
    }

    #[test]
    fn a_complete_round_is_read() {
        let round = parse(&json!({ "questions": [question("x"), {
            "id": "y", "question": "Q", "context": "More.", "multi": true, "recommended": 1,
            "options": [option("a"), { "label": "b", "description": "d", "details": "Long." }],
        }] }))
        .unwrap();
        assert_eq!(round.questions.len(), 2);
        assert_eq!(round.questions[1].context.as_deref(), Some("More."));
        assert_eq!(
            round.questions[1].options[1].details.as_deref(),
            Some("Long.")
        );
    }

    #[test]
    fn every_problem_is_named_by_its_path_and_the_round_is_refused_whole() {
        let mut second = question("x");
        second["options"] = json!([option("a"), { "label": " ", "description": "" }, option("a")]);
        second["recommended"] = json!(3);
        second["context"] = json!("  ");
        let input = json!({ "questions": [question("x"), second] });
        assert_eq!(
            paths(input.clone()),
            [
                "questions[1].id",
                "questions[1].context",
                "questions[1].options[1].label",
                "questions[1].options[1].description",
                "questions[1].options[2].label",
                "questions[1].recommended",
            ]
        );
        assert!(parse(&input).is_err());
    }

    #[test]
    fn at_least_one_question_and_two_options() {
        assert_eq!(paths(json!({ "questions": [] })), ["questions"]);
        assert_eq!(paths(json!({})), ["questions"]);
        let mut one = question("x");
        one["options"] = json!([option("a")]);
        assert_eq!(
            paths(json!({ "questions": [one] })),
            ["questions[0].options"]
        );
    }

    #[test]
    fn ids_are_unique_and_non_blank_and_labels_are_unique_within_a_question_only() {
        assert_eq!(
            paths(json!({ "questions": [question("x"), question("x")] })),
            ["questions[1].id"]
        );
        assert_eq!(
            paths(json!({ "questions": [question(" ")] })),
            ["questions[0].id"]
        );
        // The same label in two questions is fine.
        assert!(parse(&json!({ "questions": [question("x"), question("y")] })).is_ok());
    }

    #[test]
    fn recommended_is_an_integer_inside_the_options_and_multi_a_boolean() {
        for (recommended, ok) in [
            (json!(0), true),
            (json!(1), true),
            (json!(2), false),
            (json!(-1), false),
            (json!(0.5), false),
            (json!("0"), false),
        ] {
            let mut q = question("x");
            q["recommended"] = recommended.clone();
            assert_eq!(
                problems(&json!({ "questions": [q] })).is_empty(),
                ok,
                "{recommended}"
            );
        }
        let mut q = question("x");
        q["multi"] = json!("yes");
        assert_eq!(paths(json!({ "questions": [q] })), ["questions[0].multi"]);
        let mut q = question("x");
        q.as_object_mut().unwrap().remove("multi");
        assert_eq!(paths(json!({ "questions": [q] })), ["questions[0].multi"]);
    }

    #[test]
    fn blank_details_and_unknown_fields_are_refused() {
        let mut q = question("x");
        q["options"][1]["details"] = json!("");
        q["other"] = json!(1);
        assert_eq!(
            paths(json!({ "questions": [q], "extra": true })),
            [
                "extra",
                "questions[0].other",
                "questions[0].options[1].details"
            ]
        );
    }
}
