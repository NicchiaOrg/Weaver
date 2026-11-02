//! `trama history new | edit | show | list`. The rules (checks, copies of the Tasks recorded,
//! dates) live in `trama_core::project::history`; this file only reads the input and prints.

use std::convert::Infallible;

use serde_json::{json, Value};
use trama_core::command::CommandReply;
use trama_core::project::cards::CardKind;
use trama_core::project::changes::{self, MutationError};
use trama_core::project::clock::SystemClock;
use trama_core::project::history::{self, Entry, Planned, Problem, TaskCopy};
use trama_core::project::random::OsRandom;
use trama_core::project::shortnum::{self, ShortNumbers};

use super::{card_with, local, reply, written};
use crate::spec::*;

fn entry_fields() -> Vec<Field> {
    card_with(vec![
        req("file", line(), "The file, relative to `.trama/`."),
        req("recorded", list(line()), "The Tasks now recorded in History: their copy is in the entry and their file is gone."),
        opt("kept", list(line()), "The cited Tasks that stay because their copy cannot be made (the reason is on stderr)."),
    ])
}

fn task_fields() -> Vec<Field> {
    card_with(vec![
        req("finished", line(), "When the Task was finished."),
        opt(
            "label",
            line(),
            "The label of the Proposal the Task belonged to.",
        ),
        req("refs", list(line()), "What the Task referred to."),
        opt("body", text(), "What the Task said, in Markdown."),
    ])
}

pub fn commands() -> Vec<Command> {
    vec![
        Command::new("history", "new", "Records a History entry for finished Tasks.", new)
            .notes(&[
                "`tasks` needs at least one Task, each Done and not closed without changes. Every problem is listed and nothing is written.",
                "A Task cited for the first time is recorded in the entry at once: its copy is written into every entry that cites it and its file is deleted.",
            ])
            .input(vec![
                req("title", line(), "The title, on one line."),
                req("summary", line_max(280), "One line of 1 to 280 characters."),
                opt("body", text(), "More detail, in Markdown."),
                req("tasks", list_min(line(), 1), "The Done Tasks the entry records."),
            ])
            .output(entry_fields())
            .place(written)
            .example("trama history new --input -"),
        Command::new("history", "edit", "Changes a History entry: replaces fields and adds Tasks.", edit)
            .notes(&["Replaces the fields given and adds the Tasks given; it never removes a Task. The date of the entry becomes the moment of the edit."])
            .arg(Arg::Card(CardKind::History))
            .input(vec![
                opt("title", line(), "The title, on one line."),
                opt("summary", line_max(280), "One line of 1 to 280 characters."),
                opt("body", text(), "More detail, in Markdown."),
                opt("tasks", list(line()), "Tasks to add."),
            ])
            .output(entry_fields())
            .place(written)
            .example("trama history edit <id> --input -"),
        Command::new("history", "show", "Prints a History entry with the Tasks it cites.", show)
            .arg(Arg::Card(CardKind::History))
            .output(card_with(vec![
                req("summary", line(), "The summary."),
                req("body", text(), "The body."),
                req("created", line(), "When the entry was first written."),
                req("updated", line(), "When the entry was last written or updated."),
                req("tasks", list(object(task_fields())), "The Tasks it cites, in the order cited."),
            ]))
            .example("trama history show <id>"),
        Command::new("history", "list", "Lists the History entries, newest first.", list_entries)
            .output(vec![req(
                "entries",
                list(object(card_with(vec![
                    req("summary", line(), "The summary."),
                    req("updated", line(), "When the entry was last written or updated."),
                    req("tasks", int_min(0), "How many Tasks it cites."),
                ]))),
                "The entries, newest first.",
            )])
            .place(local)
            .example("trama history list"),
    ]
}

fn strings(call: &Call, key: &str) -> Vec<String> {
    call.input
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn text_field<'a>(call: &'a Call, key: &str) -> Option<&'a str> {
    call.input.get(key).and_then(Value::as_str)
}

/// Every problem on a line of its own, for a refusal.
fn refusal(problems: &[Problem]) -> Failure {
    let mut message = String::from("Nothing was written:");
    for problem in problems {
        message.push_str(&format!("\n  - {}: {}", problem.path, problem.message));
    }
    Failure::refused(message)
}

/// Writes what the command planned and prints it.
fn write_planned(call: &Call, planned: Planned) -> Result<CommandReply, Failure> {
    if !planned.changes.is_empty() {
        changes::apply(call.root, &planned.changes, |_| Ok::<(), Infallible>(())).map_err(
            |error| match error {
                MutationError::Refused(never) => match never {},
                other => Failure::failed(other.to_string()),
            },
        )?;
        history::remove_task_folders(call.root, &planned.recorded);
    }
    let mut markdown = format!(
        "# History entry `{}`\n\n{}\n\nFile: `{}`\n",
        planned.id, planned.title, planned.file
    );
    if planned.recorded.is_empty() {
        markdown.push_str("\nNo Task was recorded.\n");
    } else {
        markdown.push_str("\nRecorded in History:\n\n");
        for id in &planned.recorded {
            markdown.push_str(&format!("- `{id}`\n"));
        }
    }
    let mut note = None;
    if !planned.kept.is_empty() {
        markdown.push_str("\nStill Tasks, with the reason:\n\n");
        let mut lines = Vec::new();
        for kept in &planned.kept {
            markdown.push_str(&format!("- `{}`: {}\n", kept.task, kept.reason));
            lines.push(format!("{} stays a Task: {}.", kept.task, kept.reason));
        }
        note = Some(lines.join("\n"));
    }
    let kept: Vec<&str> = planned.kept.iter().map(|k| k.task.as_str()).collect();
    let mut result = reply(
        markdown,
        json!({ "id": planned.id, "number": null, "title": planned.title, "file": planned.file, "recorded": planned.recorded, "kept": kept }),
    );
    result.note = note;
    Ok(result)
}

fn new(call: &Call) -> Result<CommandReply, Failure> {
    let title = text_field(call, "title").unwrap_or_default();
    let summary = text_field(call, "summary").unwrap_or_default();
    let tasks = strings(call, "tasks");
    let planned = history::plan_new(
        call.image,
        title,
        summary,
        text_field(call, "body"),
        &tasks,
        &SystemClock,
        &mut OsRandom,
    )
    .map_err(|problems| refusal(&problems))?;
    write_planned(call, planned)
}

fn edit(call: &Call) -> Result<CommandReply, Failure> {
    let id = call.arg.expect("a History entry is named");
    let tasks = strings(call, "tasks");
    let planned = history::plan_edit(
        call.image,
        id,
        text_field(call, "title"),
        text_field(call, "summary"),
        text_field(call, "body"),
        &tasks,
        &SystemClock,
    )
    .map_err(|problems| refusal(&problems))?;
    write_planned(call, planned)
}

/// The Task of an entry as shown: from the copy the entry keeps, else from the Task file that is
/// still there, else only its identity.
fn task_json(call: &Call, numbers: &ShortNumbers, entry: &Entry, id: &str) -> (Value, String) {
    let copy: Option<TaskCopy> = entry.cleared.get(id).cloned().or_else(|| {
        let task = call.image.card(CardKind::Task, id)?;
        history::copy_of(call.image, task).ok()
    });
    let number = numbers.number_of(CardKind::Task, id);
    match copy {
        Some(copy) => {
            let line = format!(
                "- `{id}` {} (finished {}{})",
                copy.title,
                copy.finished,
                copy.label
                    .as_deref()
                    .map(|l| format!(", {l}"))
                    .unwrap_or_default()
            );
            let mut value = json!({ "id": id, "number": number, "title": copy.title, "finished": copy.finished, "refs": copy.refs, "body": copy.body });
            if let Some(label) = &copy.label {
                value["label"] = json!(label);
            }
            (value, line)
        }
        None => (
            json!({ "id": id, "number": number, "title": "Task not available", "finished": "", "refs": [] }),
            format!("- `{id}` (no copy of the Task is available)"),
        ),
    }
}

fn show(call: &Call) -> Result<CommandReply, Failure> {
    let id = call.arg.expect("a History entry is named");
    let entry = call
        .image
        .card(CardKind::History, id)
        .and_then(history::entry_of)
        .ok_or_else(|| Failure::refused(format!("the History entry {id} cannot be read")))?;
    let numbers = shortnum::peek(call.root, call.image.cards());
    let mut tasks = Vec::new();
    let mut lines = Vec::new();
    for task in &entry.tasks {
        let (value, line) = task_json(call, &numbers, &entry, task);
        tasks.push(value);
        lines.push(line);
    }
    let mut markdown = format!(
        "# {}\n\n`{}` · updated {} · created {}\n\n{}\n",
        entry.title, entry.id, entry.updated, entry.created, entry.summary
    );
    if !entry.text.is_empty() {
        markdown.push_str(&format!("\n{}\n", entry.text));
    }
    markdown.push_str("\n## Tasks\n\n");
    if lines.is_empty() {
        markdown.push_str("No Task.\n");
    }
    for line in lines {
        markdown.push_str(&line);
        markdown.push('\n');
    }
    Ok(reply(
        markdown,
        json!({
            "id": entry.id, "number": null, "title": entry.title, "summary": entry.summary, "body": entry.text,
            "created": entry.created, "updated": entry.updated, "tasks": tasks,
        }),
    ))
}

fn list_entries(call: &Call) -> Result<CommandReply, Failure> {
    let entries = history::entries(call.image);
    let mut markdown = String::from("# History\n\n");
    if entries.is_empty() {
        markdown.push_str("No History entry yet.\n");
    }
    for entry in &entries {
        markdown.push_str(&format!(
            "- `{}` {} ({}, {} Tasks): {}\n",
            entry.id,
            entry.title,
            entry.updated,
            entry.tasks.len(),
            entry.summary
        ));
    }
    let items: Vec<Value> = entries
        .iter()
        .map(|e| json!({ "id": e.id, "number": null, "title": e.title, "summary": e.summary, "updated": e.updated, "tasks": e.tasks.len() }))
        .collect();
    Ok(reply(markdown, json!({ "entries": items })))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use serde_json::Map;
    use trama_core::command::code;
    use trama_core::project::image::ProjectImage;

    use super::*;
    use crate::spec::Caller;

    fn run(
        verb: &str,
        root: &Path,
        arg: Option<&str>,
        input: Value,
    ) -> Result<CommandReply, Failure> {
        let all = commands();
        let command = all.iter().find(|c| c.verb == verb).unwrap();
        let image = ProjectImage::load(root);
        let input: Map<String, Value> = input.as_object().unwrap().clone();
        (command.run)(&Call {
            command,
            root,
            image: &image,
            arg,
            state: None,
            input: &input,
            caller: &Caller::default(),
        })
    }

    fn put(root: &Path, rel: &str, text: &str) {
        let path = root.join(".trama").join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn task(root: &Path, id: &str, column: &str, extra: &str) {
        put(
            root,
            &format!("tasks/{id}-job.md"),
            &format!("---\ncolumn: {column}\ntechnical: true\nrefs: []\ncreated: 2026-11-02T07:00:01.000Z\n{extra}---\n# Task {id}\n\nDo it.\n"),
        );
    }

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        task(
            dir.path(),
            "aaaaaaaa",
            "done",
            "finished: 2026-11-02T08:00:00.000Z\n",
        );
        task(dir.path(), "bbbbbbbb", "todo", "");
        task(dir.path(), "cccccccc", "doing", "holder: jjjjjjjj\n");
        task(
            dir.path(),
            "dddddddd",
            "done",
            "finished: 2026-11-02T08:10:00.000Z\nunchanged: The code already did it.\n",
        );
        dir
    }

    #[test]
    fn new_refuses_tasks_that_are_not_done_or_were_closed_without_changes_listing_every_problem() {
        let dir = project();
        let before = fs::read_dir(dir.path().join(".trama")).unwrap().count();
        let error = run(
            "new",
            dir.path(),
            None,
            json!({ "title": "Work", "summary": "A line.", "tasks": ["bbbbbbbb", "cccccccc", "dddddddd", "zzzzzzzz", "aaaaaaaa"] }),
        )
        .unwrap_err();
        assert_eq!((error.code.as_str(), error.exit_code()), (code::REFUSED, 1));
        for expected in [
            "tasks[0]",
            "To do",
            "tasks[1]",
            "in progress",
            "tasks[2]",
            "without changes",
            "tasks[3]",
        ] {
            assert!(
                error.message.contains(expected),
                "{expected} missing in {}",
                error.message
            );
        }
        assert!(!error.message.contains("tasks[4]"));
        assert!(
            !dir.path().join(".trama/history").exists(),
            "a refusal writes nothing"
        );
        assert_eq!(
            fs::read_dir(dir.path().join(".trama")).unwrap().count(),
            before
        );
        assert!(dir.path().join(".trama/tasks/aaaaaaaa-job.md").is_file());
    }

    #[test]
    fn new_records_the_tasks_and_prints_the_entry_and_the_tasks_recorded() {
        let dir = project();
        put(dir.path(), "task-images/aaaaaaaa/a.png", "png");
        let reply = run("new", dir.path(), None, json!({ "title": "Work done", "summary": "A line.", "body": "More.", "tasks": ["aaaaaaaa"] })).unwrap();
        assert_eq!(reply.json["recorded"], json!(["aaaaaaaa"]));
        assert_eq!(reply.json["title"], "Work done");
        assert!(reply.json["number"].is_null());
        let file = reply.json["file"].as_str().unwrap();
        assert!(file.starts_with("history/") && file.ends_with("-work-done.md"));
        assert!(dir.path().join(".trama").join(file).is_file());
        assert!(!dir.path().join(".trama/tasks/aaaaaaaa-job.md").exists());
        assert!(!dir.path().join(".trama/task-images/aaaaaaaa").exists());
        assert!(reply.markdown.contains("Recorded in History"));

        let id = reply.json["id"].as_str().unwrap();
        let shown = run("show", dir.path(), Some(id), json!({})).unwrap();
        assert_eq!(shown.json["tasks"][0]["id"], "aaaaaaaa");
        assert_eq!(shown.json["tasks"][0]["title"], "Task aaaaaaaa");
        assert_eq!(
            shown.json["tasks"][0]["finished"],
            "2026-11-02T08:00:00.000Z"
        );
        assert_eq!(shown.json["body"], "More.");
        let listed = run("list", dir.path(), None, json!({})).unwrap();
        assert_eq!(listed.json["entries"][0]["id"], id);
        assert_eq!(listed.json["entries"][0]["tasks"], 1);
    }

    #[test]
    fn edit_adds_tasks_and_replaces_fields_and_never_removes_a_task() {
        let dir = project();
        task(
            dir.path(),
            "eeeeeeee",
            "done",
            "finished: 2026-11-02T08:20:00.000Z\n",
        );
        let created = run(
            "new",
            dir.path(),
            None,
            json!({ "title": "Work", "summary": "A line.", "tasks": ["aaaaaaaa"] }),
        )
        .unwrap();
        let id = created.json["id"].as_str().unwrap().to_owned();
        let edited = run(
            "edit",
            dir.path(),
            Some(&id),
            json!({ "summary": "Another line.", "tasks": ["eeeeeeee"] }),
        )
        .unwrap();
        assert_eq!(edited.json["recorded"], json!(["eeeeeeee"]));
        let shown = run("show", dir.path(), Some(&id), json!({})).unwrap();
        let ids: Vec<&str> = shown.json["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["aaaaaaaa", "eeeeeeee"]);
        assert_eq!(shown.json["summary"], "Another line.");
        let refused = run(
            "edit",
            dir.path(),
            Some(&id),
            json!({ "tasks": ["bbbbbbbb"] }),
        )
        .unwrap_err();
        assert!(refused.message.contains("tasks[0]"), "{}", refused.message);
    }

    #[test]
    fn a_task_that_cannot_be_copied_stays_and_is_said_on_the_note() {
        let dir = project();
        task(dir.path(), "ffffffff", "done", ""); // no `finished`
        let reply = run(
            "new",
            dir.path(),
            None,
            json!({ "title": "Work", "summary": "A line.", "tasks": ["ffffffff", "aaaaaaaa"] }),
        )
        .unwrap();
        assert_eq!(reply.json["recorded"], json!(["aaaaaaaa"]));
        assert_eq!(reply.json["kept"], json!(["ffffffff"]));
        assert!(reply
            .note
            .as_deref()
            .is_some_and(|n| n.contains("ffffffff") && n.contains("finished")));
        assert!(dir.path().join(".trama/tasks/ffffffff-job.md").is_file());
    }

    #[test]
    fn list_in_a_project_without_entries_says_so() {
        let dir = project();
        let reply = run("list", dir.path(), None, json!({})).unwrap();
        assert!(reply.markdown.contains("No History entry yet"));
        assert_eq!(reply.json, json!({ "entries": [] }));
    }
}
