//! Validation of the input object against the fields of a command, whole, before anything
//! else happens. Every problem is listed with its JSON path (`questions[0].options[1].label`).

use serde_json::{Map, Value};

use crate::spec::{Field, Problem, StrRule, Ty};

/// The problems of `input` against `fields`; empty when it is acceptable.
pub fn validate(fields: &[Field], input: &Map<String, Value>) -> Vec<Problem> {
    let mut problems = Vec::new();
    check_object(fields, input, "", &mut problems);
    problems
}

fn join(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_owned()
    } else {
        format!("{path}.{name}")
    }
}

fn check_object(
    fields: &[Field],
    object: &Map<String, Value>,
    path: &str,
    problems: &mut Vec<Problem>,
) {
    for key in object.keys() {
        if !fields.iter().any(|f| f.name == key) {
            let message = if fields.is_empty() {
                "unknown field: this command takes no fields".to_owned()
            } else {
                let names: Vec<String> = fields.iter().map(|f| format!("`{}`", f.name)).collect();
                format!("unknown field; the fields are {}", names.join(", "))
            };
            problems.push(Problem::new(join(path, key), message));
        }
    }
    for field in fields {
        match object.get(field.name) {
            Some(value) => check_value(&field.ty, value, &join(path, field.name), problems),
            None if field.required => problems.push(Problem::new(
                join(path, field.name),
                "missing required field",
            )),
            None => {}
        }
    }
}

/// What a JSON value is, for messages.
fn describe(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "an integer",
        Value::Number(_) => "a number with decimals",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    }
}

fn expected(path: &str, what: &str, value: &Value, problems: &mut Vec<Problem>) {
    problems.push(Problem::new(
        path,
        format!("expected {what}, found {}", describe(value)),
    ));
}

fn check_value(ty: &Ty, value: &Value, path: &str, problems: &mut Vec<Problem>) {
    match ty {
        Ty::Str(rule) => match value {
            Value::String(text) => check_string(rule, text, path, problems),
            other => expected(path, "a string", other, problems),
        },
        Ty::Int { min, max } => match value.as_i64() {
            Some(n) => {
                if min.is_some_and(|min| n < min) {
                    problems.push(Problem::new(
                        path,
                        format!("must be at least {} (found {n})", min.expect("checked")),
                    ));
                }
                if max.is_some_and(|max| n > max) {
                    problems.push(Problem::new(
                        path,
                        format!("must be at most {} (found {n})", max.expect("checked")),
                    ));
                }
            }
            None if value.is_u64() => problems.push(Problem::new(path, "is too large")),
            None => expected(path, "an integer", value, problems),
        },
        Ty::Bool => {
            if !value.is_boolean() {
                expected(path, "a boolean", value, problems);
            }
        }
        Ty::Enum(values) => match value.as_str() {
            Some(text) if values.contains(&text) => {}
            Some(text) => {
                let names: Vec<String> = values.iter().map(|v| format!("`{v}`")).collect();
                problems.push(Problem::new(
                    path,
                    format!("`{text}` is not one of {}", names.join(", ")),
                ));
            }
            None => expected(path, "a string", value, problems),
        },
        Ty::List { item, min, max } => match value.as_array() {
            Some(items) => {
                if items.len() < *min {
                    problems.push(Problem::new(
                        path,
                        format!(
                            "needs at least {min} item{} (has {})",
                            plural(*min),
                            items.len()
                        ),
                    ));
                }
                if let Some(max) = max.filter(|max| items.len() > *max) {
                    problems.push(Problem::new(
                        path,
                        format!(
                            "takes at most {max} item{} (has {})",
                            plural(max),
                            items.len()
                        ),
                    ));
                }
                for (index, element) in items.iter().enumerate() {
                    check_value(item, element, &format!("{path}[{index}]"), problems);
                }
            }
            None => expected(path, "a list", value, problems),
        },
        Ty::Object(fields) => match value.as_object() {
            Some(object) => check_object(fields, object, path, problems),
            None => expected(path, "an object", value, problems),
        },
        Ty::Nullable(inner) => {
            if !value.is_null() {
                check_value(inner, value, path, problems);
            }
        }
        Ty::Any => {}
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

fn check_string(rule: &StrRule, text: &str, path: &str, problems: &mut Vec<Problem>) {
    if rule.nonblank && text.trim().is_empty() {
        problems.push(Problem::new(path, "must not be blank"));
    }
    if rule.line && text.contains(['\n', '\r']) {
        problems.push(Problem::new(path, "must be a single line"));
    }
    if let Some(max) = rule.max {
        let length = text.chars().count();
        if length > max {
            problems.push(Problem::new(
                path,
                format!("must have at most {max} characters (has {length})"),
            ));
        }
    }
}

/// The text of the error of invalid input: every problem on a line of its own.
pub fn describe_problems(problems: &[Problem]) -> String {
    let mut text = String::from("Invalid input:");
    for problem in problems {
        text.push_str(&format!("\n  - {}: {}", problem.path, problem.message));
    }
    text
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::spec::*;

    fn question_fields() -> Vec<Field> {
        vec![req(
            "questions",
            list_min(
                object(vec![
                    req("id", line(), "id"),
                    req("question", nonblank(), "text"),
                    opt("context", nonblank(), "context"),
                    req(
                        "options",
                        list_min(
                            object(vec![
                                req("label", line(), "l"),
                                req("description", nonblank(), "d"),
                            ]),
                            2,
                        ),
                        "options",
                    ),
                    req("recommended", int_min(0), "r"),
                    req("multi", boolean(), "m"),
                ]),
                1,
            ),
            "questions",
        )]
    }

    fn problems(fields: &[Field], input: Value) -> Vec<(String, String)> {
        validate(fields, input.as_object().unwrap())
            .into_iter()
            .map(|p| (p.path, p.message))
            .collect()
    }

    #[test]
    fn every_problem_is_listed_with_its_json_path() {
        let found = problems(
            &question_fields(),
            json!({
                "questions": [
                    {"id": "a", "question": "Q?", "options": [{"label": "x", "description": "d"}, {"label": "", "description": 5}],
                     "recommended": -1, "multi": "no", "extra": 1}
                ],
                "other": true
            }),
        );
        let paths: Vec<&str> = found.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(
            paths,
            [
                "other",
                "questions[0].extra",
                "questions[0].options[1].label",
                "questions[0].options[1].description",
                "questions[0].recommended",
                "questions[0].multi",
            ]
        );
    }

    #[test]
    fn missing_required_fields_and_short_lists_are_named() {
        let found = problems(
            &question_fields(),
            json!({ "questions": [{"id": "a", "options": [{"label": "x", "description": "d"}]}] }),
        );
        let paths: Vec<&str> = found.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(
            paths,
            [
                "questions[0].question",
                "questions[0].options",
                "questions[0].recommended",
                "questions[0].multi"
            ]
        );
        assert_eq!(found[1].1, "needs at least 2 items (has 1)");
        assert_eq!(problems(&question_fields(), json!({}))[0].0, "questions");
        assert_eq!(
            problems(&question_fields(), json!({ "questions": [] }))[0].1,
            "needs at least 1 item (has 0)"
        );
    }

    #[test]
    fn strings_integers_and_enums_are_checked() {
        let fields = vec![
            opt("title", line_max(5), ""),
            opt("n", int_min(1), ""),
            opt("kind", one_of(&["a", "b"]), ""),
            opt("flag", boolean(), ""),
        ];
        assert!(problems(
            &fields,
            json!({ "title": "abc", "n": 1, "kind": "a", "flag": true })
        )
        .is_empty());
        let found = problems(
            &fields,
            json!({ "title": "abcdef", "n": 0, "kind": "c", "flag": 1 }),
        );
        assert_eq!(found.len(), 4);
        assert_eq!(found[0].1, "must have at most 5 characters (has 6)");
        assert_eq!(found[1].1, "must be at least 1 (found 0)");
        assert_eq!(found[2].1, "`c` is not one of `a`, `b`");
        assert_eq!(found[3].1, "expected a boolean, found an integer");
        assert_eq!(
            problems(&fields, json!({ "title": "a\nb" }))[0].1,
            "must be a single line"
        );
        assert_eq!(
            problems(&fields, json!({ "title": "  " }))[0].1,
            "must not be blank"
        );
        assert_eq!(
            problems(&fields, json!({ "n": 1.5 }))[0].1,
            "expected an integer, found a number with decimals"
        );
        assert_eq!(
            problems(&fields, json!({ "title": 7 }))[0].1,
            "expected a string, found an integer"
        );
    }

    #[test]
    fn a_command_without_fields_accepts_only_nothing() {
        assert!(problems(&[], json!({})).is_empty());
        assert_eq!(
            problems(&[], json!({ "x": 1 }))[0].1,
            "unknown field: this command takes no fields"
        );
    }

    #[test]
    fn the_error_text_lists_each_problem_on_its_own_line() {
        let text = describe_problems(&[Problem::new("a", "bad"), Problem::new("b[0].c", "worse")]);
        assert_eq!(text, "Invalid input:\n  - a: bad\n  - b[0].c: worse");
    }
}
