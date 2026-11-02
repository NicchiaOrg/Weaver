//! `trama model show | list | check`: read-only views of the Model, the Requirements and the
//! Specs, run in the CLI.

use serde_json::{json, Value};
use trama_core::command::CommandReply;
use trama_core::project::browse;
use trama_core::project::diagnostic::{Diagnostic, Severity};
use trama_core::project::image::ProjectImage;

use super::reply;
use crate::spec::*;
use trama_core::project::model_view;

pub fn commands() -> Vec<Command> {
    vec![
        Command::new("model", "show", "Prints the System, or a Model element or Requirement section with the Requirements that apply to it.", show)
            .notes(&[
                "Without `<ref>` it prints the System with its Actors, Features and Requirement Areas. A Feature lists its Machines; a Machine prints its State tree, its Events with their receivers and its Transitions; a State prints its routes. Each element view is followed by the Requirement sections that apply to it (those titled with it or with what contains it).",
                "`requirements/<area>` prints an Area with its fragments; `requirements/<area>#General` and `requirements/<area>#<element>` print one section.",
            ])
            .arg(Arg::Reference)
            .output(vec![
                req("kind", one_of(&["system", "feature", "machine", "actor", "state", "transitions", "event", "area", "section", "specs"]), "What the reference named."),
                opt("reference", line(), "The reference, as given (absent for the System without one)."),
                opt("requirements", list(object(vec![
                    req("area", line(), "The Area."),
                    req("fragment", line(), "The fragment that holds the section."),
                    req("section", line(), "The section title."),
                    req("body", text(), "The section text."),
                ])), "The Requirement sections that apply to the element."),
            ])
            .example("trama model show order/Paid"),
        Command::new("model", "list", "Lists the Model and the Requirements, or searches them for a text.", list_model)
            .notes(&[
                "Without `text` it lists the Actors, the Features with their Machines and the Requirement Areas. With `text` it prints every element, State, Event, Requirement section and Specs entry whose text contains it, compared without regard to case; the Requirements are searched too.",
            ])
            .input(vec![opt("text", nonblank(), "The text to search for.")])
            .output(vec![
                req("text", nullable(line()), "The text searched for, or null."),
                req("hits", list(object(vec![
                    req("reference", line(), "The reference to pass to `trama model show`."),
                    req("kind", line(), "What the hit is."),
                    req("title", line(), "Its title."),
                    req("excerpt", line(), "The first line that holds the text."),
                ])), "What was found (only with `text`)."),
                req("actors", list(object(vec![req("ref", line(), "The id of the Actor."), req("title", line(), "The title.")])), "The Actors (without `text`)."),
                req("features", list(object(vec![
                    req("ref", line(), "The id of the Feature."),
                    req("title", line(), "The title."),
                    req("machines", list(object(vec![req("ref", line(), "The id of the Machine."), req("title", line(), "The title.")])), "Its Machines."),
                ])), "The Features (without `text`)."),
                req("areas", list(object(vec![
                    req("area", line(), "The id of the Area."),
                    req("title", line(), "The title."),
                    req("description", line(), "When an agent needs it."),
                ])), "The Requirement Areas (without `text`)."),
            ])
            .example(r#"trama model list --input '{"text":"payment"}'"#),
        Command::new("model", "check", "Validates the Model, the Requirements and the Specs.", check)
            .notes(&[
                "Exits 1 when there is an error; a warning (a fragment over 12,288 bytes) does not change the exit code.",
            ])
            .output(vec![
                req("ok", boolean(), "True when there is no error."),
                req("errors", int_min(0), "How many errors."),
                req("warnings", int_min(0), "How many warnings."),
                req("diagnostics", list(object(vec![
                    req("path", line(), "The file, relative to `.trama/`."),
                    req("severity", one_of(&["error", "warning"]), "How serious."),
                    req("message", line(), "What is wrong."),
                ])), "Every problem, by path."),
            ])
            .example("trama model check"),
    ]
}

fn show(call: &Call) -> Result<CommandReply, Failure> {
    let (markdown, data) = model_view::show(call.image, call.arg).map_err(Failure::refused)?;
    Ok(reply(markdown, data))
}

fn overview(image: &ProjectImage) -> (String, Value, Value, Value) {
    let model = image.model();
    let mut markdown = String::from("# Model\n");
    let actors: Vec<Value> = model
        .actors
        .values()
        .map(|a| json!({ "ref": a.id, "title": a.title }))
        .collect();
    let features: Vec<Value> = model
        .features
        .iter()
        .map(|(id, f)| {
            let machines: Vec<Value> = model
                .machines
                .values()
                .filter(|m| m.feature == *id)
                .map(|m| json!({ "ref": m.id, "title": m.title }))
                .collect();
            json!({ "ref": id, "title": f.title, "machines": machines })
        })
        .collect();
    let areas: Vec<Value> = image
        .areas()
        .iter()
        .map(|a| json!({ "area": a.id, "title": a.title, "description": a.description }))
        .collect();
    if !actors.is_empty() {
        markdown.push_str("\n## Actors\n\n");
        for actor in model.actors.values() {
            markdown.push_str(&format!("- `{}`: {}\n", actor.id, actor.title));
        }
    }
    if !features.is_empty() {
        markdown.push_str("\n## Features\n\n");
        for (id, feature) in &model.features {
            markdown.push_str(&format!("- `{id}`: {}\n", feature.title));
            for machine in model.machines.values().filter(|m| m.feature == *id) {
                markdown.push_str(&format!(
                    "  - Machine `{}`: {}\n",
                    machine.id, machine.title
                ));
            }
        }
    }
    if model.is_empty() {
        markdown.push_str("\nThe Model has no Feature and no Actor yet.\n");
    }
    if !areas.is_empty() {
        markdown.push_str("\n## Requirements\n\n");
        for area in image.areas() {
            markdown.push_str(&format!(
                "- `requirements/{}`: {}. {}\n",
                area.id, area.title, area.description
            ));
        }
    }
    (markdown, json!(actors), json!(features), json!(areas))
}

fn list_model(call: &Call) -> Result<CommandReply, Failure> {
    let text = call.input.get("text").and_then(Value::as_str);
    match text {
        Some(text) => {
            let hits = browse::search(call.image, text);
            let mut markdown = format!("# Search for `{}`\n\n", text.trim());
            if hits.is_empty() {
                markdown.push_str("Nothing found.\n");
            }
            for hit in &hits {
                markdown.push_str(&format!(
                    "- `{}` ({}): {}: {}\n",
                    hit.reference, hit.kind, hit.title, hit.excerpt
                ));
            }
            Ok(reply(
                markdown,
                json!({ "text": text.trim(), "hits": hits, "actors": [], "features": [], "areas": [] }),
            ))
        }
        None => {
            let (markdown, actors, features, areas) = overview(call.image);
            Ok(reply(
                markdown,
                json!({ "text": null, "hits": [], "actors": actors, "features": features, "areas": areas }),
            ))
        }
    }
}

fn check(call: &Call) -> Result<CommandReply, Failure> {
    let diagnostics: &[Diagnostic] = call.image.diagnostics();
    let errors = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warnings = diagnostics.len() - errors;
    let mut markdown = String::from("# Model check\n\n");
    markdown.push_str(&match (errors, warnings) {
        (0, 0) => "No problem found.\n".to_owned(),
        (errors, warnings) => format!(
            "{errors} error{}, {warnings} warning{}.\n",
            plural(errors),
            plural(warnings)
        ),
    });
    if !diagnostics.is_empty() {
        markdown.push('\n');
    }
    for diagnostic in diagnostics {
        markdown.push_str(&format!(
            "- `{}`: {}: {}\n",
            diagnostic.path,
            severity_word(diagnostic.severity),
            diagnostic.message
        ));
    }
    let mut out = reply(
        markdown,
        json!({ "ok": errors == 0, "errors": errors, "warnings": warnings, "diagnostics": diagnostics }),
    );
    if errors > 0 {
        out.exit = 1;
        out.note = Some(format!("The project has {errors} error{}.", plural(errors)));
    }
    Ok(out)
}

fn severity_word(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}
