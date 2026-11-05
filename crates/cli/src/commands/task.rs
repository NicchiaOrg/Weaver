//! `weaver task new | show | list | move | delete`. The rules (blocking, slots, groups, taking,
//! finishing, forgetting) live in `weaver_core::project::tasks`; this file reads the input, applies
//! the changes and prints.

use std::collections::BTreeSet;
use std::convert::Infallible;

use serde_json::{json, Map, Value};
use weaver_core::command::CommandReply;
use weaver_core::project::board;
use weaver_core::project::cards::{Card, CardKind};
use weaver_core::project::changes::{self, ChangeSet, MutationError};
use weaver_core::project::clock::{now_instant, SystemClock};
use weaver_core::project::history;
use weaver_core::project::image::ProjectImage;
use weaver_core::project::model_view;
use weaver_core::project::random::OsRandom;
use weaver_core::project::resolve::resolve_card;
use weaver_core::project::shortnum::{self, ShortNumbers};
use weaver_core::project::tasks::{
    self, Column, FinishPlan, HeldBack, Outcome, Refusal, TaskDraft,
};

use super::{card_with, cards, local, reply, written};
use crate::spec::*;

pub fn commands() -> Vec<Command> {
    vec![
        Command::new("task", "new", "Files a Task in To do.", new)
            .notes(&["`technical: true` marks a Task with no Model reference. `announcement` is the identity printed by `weaver announcement new`. The job that files it is recorded when `WEAVER_JOB` is set."])
            .input(vec![
                req("title", line(), "The title."),
                opt("refs", list(line()), "Model elements, Requirement sections and Specs fields it implements."),
                opt("technical", boolean(), "Whether it has no Model reference."),
                opt("body", text(), "What to do, in Markdown."),
                opt("announcement", line(), "The announcement of the card."),
            ])
            .output(card_with(vec![req("column", line(), "The column: `todo`.")]))
            .place(written)
            .example(r#"weaver task new --input '{"title":"Cancel an order","refs":["order/Paid"]}'"#),
        Command::new("task", "show", "Prints a Task with the Model and the Requirements it refers to.", show)
            .arg(Arg::Card(CardKind::Task))
            .output(card_with(vec![
                req("column", line(), "The column."),
                req("technical", boolean(), "Whether it is technical."),
                req("refs", list(line()), "What it refers to."),
                req("blockedBy", list(line()), "The Drifts that block it."),
            ]))
            .example("weaver task show t3"),
        Command::new("task", "list", "Lists Tasks, or picks the runnable ones.", list_tasks)
            .notes(&[
                "`runnable` gives the next runnable Task, or up to `max` (at least 1) for free places, plus `waiting` (Tasks in To do with blockers) and, with `heldBack`, those left out. `groups` prints the runnable Tasks in groups of Tasks that concern the same thing, for one sub-agent each. `proposals` restricts the Tasks to those of these Proposals and `notProposals` excludes them.",
            ])
            .input(vec![
                opt("column", one_of(&["todo", "doing", "done"]), "Only the Tasks of this column."),
                opt("tasks", list(line()), "Only these Tasks."),
                opt("runnable", boolean(), "Pick the runnable Tasks."),
                opt("max", int_min(1), "At most this many Tasks (or groups with `groups`); with `runnable`."),
                opt("groups", boolean(), "Print the runnable Tasks in groups."),
                opt("proposals", list(line()), "Only the Tasks of these Proposals."),
                opt("notProposals", list(line()), "Not the Tasks of these Proposals."),
                opt("heldBack", boolean(), "Also list the runnable Tasks left out and why."),
            ])
            .output(vec![
                req("tasks", list(object(card_with(vec![req("column", line(), "The column.")]))), "The Tasks."),
                cards("Tasks in To do that have blockers.", "waiting"),
                opt("groups", list(object(vec![req("tasks", list(object(card_with(vec![]))), "The Tasks of the group.")])), "The groups, largest first."),
                opt("heldBack", list(object(card_with(vec![req("reason", line(), "Why it is left out.")]))), "The runnable Tasks left out."),
            ])
            .check(list_needs_runnable)
            .place(local)
            .example(r#"weaver task list --input '{"runnable":true,"max":3,"groups":true}'"#),
        Command::new("task", "move", "Takes a Task, releases it or finishes it.", move_task)
            .notes(&[
                "Bound to a job. `doing` takes the Task (refused naming its blocker), `todo` releases it and `done` finishes it. `unchanged`, a non-blank note, with `done` closes the fix Task of a Drift when the code already does what should happen. Only the job that holds a Task moves it out of `doing`.",
            ])
            .arg(Arg::Card(CardKind::Task))
            .states(&["doing", "todo", "done"])
            .access(Access::Job)
            .input(vec![opt("unchanged", nonblank(), "Closes the fix Task of a Drift without changes; with `done`.")])
            .output(card_with(vec![req("column", line(), "The column after the move.")]))
            .example("weaver task move t3 doing"),
        Command::new("task", "delete", "Deletes a Task in To do.", delete)
            .notes(&["A decision of the developer: the agent runs it only when the developer explicitly asked for that decision in the conversation."])
            .arg(Arg::Card(CardKind::Task))
            .output(card_with(vec![]))
            .place(written)
            .example("weaver task delete t3"),
    ]
}

// ---------------------------------------------------------------------------------------------
// Shared
// ---------------------------------------------------------------------------------------------

fn strings(input: &Map<String, Value>, key: &str) -> Vec<String> {
    input
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

fn flag(input: &Map<String, Value>, key: &str) -> bool {
    input.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn refused(refusal: Refusal) -> Failure {
    Failure::refused(refusal.0)
}

/// Writes `changes`; a failure of the files is `failed`, nothing is refused here.
fn write(call: &Call, changes: &ChangeSet) -> Result<(), Failure> {
    changes::apply(call.root, changes, |_| Ok::<(), Infallible>(()))
        .map(|_| ())
        .map_err(|error| match error {
            MutationError::Refused(never) => match never {},
            other => Failure::failed(other.to_string()),
        })
}

fn now() -> Result<String, Failure> {
    now_instant(&SystemClock).map_err(|e| Failure::failed(e.to_string()))
}

fn name(numbers: &ShortNumbers, kind: CardKind, id: &str) -> String {
    numbers.label_of(kind, id).unwrap_or_else(|| id.to_owned())
}

fn card_json(numbers: &ShortNumbers, task: &Card) -> Value {
    json!({
        "id": task.id,
        "number": numbers.number_of(CardKind::Task, &task.id),
        "title": task.title,
        "column": tasks::column_of(task).map_or("", Column::as_str),
    })
}

fn plain_json(numbers: &ShortNumbers, kind: CardKind, card: &Card) -> Value {
    json!({ "id": card.id, "number": numbers.number_of(kind, &card.id), "title": card.title })
}

fn task_line(numbers: &ShortNumbers, task: &Card) -> String {
    let column = tasks::column_of(task).map_or("?", Column::state_name);
    format!(
        "- `{}` {} ({column}) {}",
        name(numbers, CardKind::Task, &task.id),
        task.id,
        task.title
    )
}

// ---------------------------------------------------------------------------------------------
// new
// ---------------------------------------------------------------------------------------------

fn new(call: &Call) -> Result<CommandReply, Failure> {
    let draft = TaskDraft {
        title: call
            .input
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        body: call
            .input
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        refs: strings(call.input, "refs"),
        technical: flag(call.input, "technical"),
        proposal: None,
        drift: None,
        job: call.caller.job.clone(),
    };
    let problems = tasks::validate_draft(call.image, &draft);
    if !problems.is_empty() {
        let mut message = String::from("Nothing was written:");
        for problem in &problems {
            message.push_str(&format!("\n  - {}: {}", problem.path, problem.message));
        }
        return Err(Failure::refused(message));
    }
    let created = now()?;
    let planned = tasks::plan_new_task(call.image, &draft, &created, &mut OsRandom);
    let after = changes::apply(call.root, &planned.changes, |_| Ok::<(), Infallible>(())).map_err(
        |error| match error {
            MutationError::Refused(never) => match never {},
            other => Failure::failed(other.to_string()),
        },
    )?;
    let numbers = after.short_numbers();
    let task = after.card(CardKind::Task, &planned.id).ok_or_else(|| {
        Failure::failed(format!(
            "the Task file {} was written but cannot be read back",
            planned.path
        ))
    })?;
    let label = name(&numbers, CardKind::Task, &task.id);
    Ok(reply(
        format!(
            "# Task {label} filed\n\n`{}` {}\n\nFile: `{}`\n",
            task.id, task.title, planned.path
        ),
        card_json(&numbers, task),
    ))
}

// ---------------------------------------------------------------------------------------------
// show
// ---------------------------------------------------------------------------------------------

fn show(call: &Call) -> Result<CommandReply, Failure> {
    let id = call.arg.expect("a Task is named");
    let task = call
        .image
        .card(CardKind::Task, id)
        .ok_or_else(|| Failure::refused(format!("no Task has the identity {id}")))?;
    let numbers = shortnum::peek(call.root, call.image.cards());
    let blocking = tasks::blocking_of(call.image, task);
    let refs = tasks::refs_of(task);
    let drift_names: Vec<String> = blocking
        .drifts
        .iter()
        .map(|d| name(&numbers, CardKind::Drift, d))
        .collect();

    let mut markdown = format!(
        "# {} {}\n\n`{}` · {}{}{}\n",
        name(&numbers, CardKind::Task, &task.id),
        task.title,
        task.id,
        tasks::column_of(task).map_or("unknown column", Column::state_name),
        if tasks::is_technical(task) {
            " · technical"
        } else {
            ""
        },
        task.str_key("holder")
            .map(|h| format!(" · held by job `{h}`"))
            .unwrap_or_default(),
    );
    if !blocking.drifts.is_empty() {
        let list: Vec<String> = blocking
            .drifts
            .iter()
            .map(|d| {
                format!(
                    "{} {}",
                    name(&numbers, CardKind::Drift, d),
                    call.image
                        .card(CardKind::Drift, d)
                        .map_or("", |c| c.title.as_str())
                )
            })
            .collect();
        markdown.push_str(&format!("\nBlocked by Drift {}.\n", list.join("; ")));
    }
    if !blocking.fix_tasks.is_empty() {
        let list: Vec<String> = blocking
            .fix_tasks
            .iter()
            .map(|t| {
                format!(
                    "{} {}",
                    name(&numbers, CardKind::Task, t),
                    call.image
                        .card(CardKind::Task, t)
                        .map_or("", |c| c.title.as_str())
                )
            })
            .collect();
        markdown.push_str(&format!(
            "\nWaiting for the fix Task {}.\n",
            list.join("; ")
        ));
    }
    let body = tasks::body_after_title(task);
    if !body.is_empty() {
        markdown.push_str(&format!("\n{body}\n"));
    }
    if let Some(note) = task.str_key("unchanged") {
        markdown.push_str(&format!("\nClosed without changes: {note}\n"));
    }
    if !refs.is_empty() {
        markdown.push_str("\n## References\n");
        for reference in &refs {
            match model_view::show(call.image, Some(reference)) {
                Ok((view, _)) => {
                    markdown.push_str(&format!("\n### `{reference}`\n\n{}\n", view.trim_end()))
                }
                Err(why) => {
                    markdown.push_str(&format!("\n### `{reference}`\n\nCannot be read: {why}\n"))
                }
            }
        }
    }
    let mut json = card_json(&numbers, task);
    json["technical"] = json!(tasks::is_technical(task));
    json["refs"] = json!(refs);
    json["blockedBy"] = json!(drift_names);
    Ok(reply(markdown, json))
}

// ---------------------------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------------------------

fn list_needs_runnable(input: &Map<String, Value>) -> Vec<Problem> {
    if flag(input, "runnable") {
        return Vec::new();
    }
    ["max", "groups", "heldBack"]
        .into_iter()
        .filter(|key| input.contains_key(*key))
        .map(|key| Problem::new(key, "goes with `runnable`"))
        .collect()
}

fn resolve_all(
    call: &Call,
    key: &str,
    kind: CardKind,
) -> Result<Option<BTreeSet<String>>, Failure> {
    if !call.input.contains_key(key) {
        return Ok(None);
    }
    let mut set = BTreeSet::new();
    for (index, given) in strings(call.input, key).iter().enumerate() {
        let id = resolve_card(call.image, kind, given)
            .map_err(|e| Failure::refused(format!("{key}[{index}]: {e}")))?;
        set.insert(id);
    }
    Ok(Some(set))
}

fn list_tasks(call: &Call) -> Result<CommandReply, Failure> {
    let numbers = shortnum::peek(call.root, call.image.cards());
    let only = resolve_all(call, "tasks", CardKind::Task)?;
    let proposals = resolve_all(call, "proposals", CardKind::Proposal)?;
    let not_proposals = resolve_all(call, "notProposals", CardKind::Proposal)?;
    let eligible = |task: &Card| {
        let design = task.str_key("proposal");
        only.as_ref().is_none_or(|ids| ids.contains(&task.id))
            && proposals
                .as_ref()
                .is_none_or(|ids| design.is_some_and(|p| ids.contains(p)))
            && not_proposals
                .as_ref()
                .is_none_or(|ids| !design.is_some_and(|p| ids.contains(p)))
    };
    let runnable = flag(call.input, "runnable");
    let max = call
        .input
        .get("max")
        .and_then(Value::as_u64)
        .map(|n| n as usize);
    let want_groups = flag(call.input, "groups");
    let want_held_back = flag(call.input, "heldBack");

    let mut picked: Vec<&Card> = Vec::new();
    let mut groups: Vec<Vec<&Card>> = Vec::new();
    let mut held_back: Vec<(&Card, HeldBack)> = Vec::new();
    if runnable {
        if want_groups {
            (groups, held_back) = tasks::pick_groups(call.image, max.unwrap_or(1), &eligible);
            picked = groups.iter().flatten().copied().collect();
        } else if let Some(max) = max {
            let slots = tasks::pick_slots(call.image, max, &eligible);
            picked = slots.picked;
            held_back = slots.held_back;
        } else {
            let mut next = tasks::runnable(call.image)
                .into_iter()
                .filter(|t| eligible(t));
            picked.extend(next.next());
            held_back = next.map(|t| (t, HeldBack::NoPlace)).collect();
        }
    } else {
        let column = call
            .input
            .get("column")
            .and_then(Value::as_str)
            .and_then(Column::parse);
        picked = call
            .image
            .cards_of(CardKind::Task)
            .filter(|t| column.is_none_or(|c| tasks::column_of(t) == Some(c)))
            .filter(|t| eligible(t))
            .collect();
    }
    let index = tasks::BlockingIndex::new(call.image);
    let waiting: Vec<&Card> = tasks::tasks_in(call.image, Column::Todo)
        .into_iter()
        .filter(|t| eligible(t) && index.of(t).is_blocked())
        .collect();

    let mut markdown = String::from(if runnable {
        "# Runnable Tasks\n\n"
    } else {
        "# Tasks\n\n"
    });
    if want_groups {
        if groups.is_empty() {
            markdown.push_str("No Task is runnable.\n");
        }
        for (number, group) in groups.iter().enumerate() {
            markdown.push_str(&format!("## Group {}\n\n", number + 1));
            for task in group {
                markdown.push_str(&task_line(&numbers, task));
                markdown.push('\n');
            }
            markdown.push('\n');
        }
    } else if picked.is_empty() {
        markdown.push_str(if runnable {
            "No Task is runnable.\n"
        } else {
            "No Task.\n"
        });
    } else {
        for task in &picked {
            markdown.push_str(&task_line(&numbers, task));
            markdown.push('\n');
        }
    }
    if !waiting.is_empty() {
        markdown.push_str("\n## Waiting (in To do, with blockers)\n\n");
        for task in &waiting {
            markdown.push_str(&task_line(&numbers, task));
            markdown.push('\n');
        }
    }
    if want_held_back && !held_back.is_empty() {
        markdown.push_str("\n## Left out\n\n");
        for (task, why) in &held_back {
            markdown.push_str(&format!(
                "{}: {}\n",
                task_line(&numbers, task),
                why.text(&numbers)
            ));
        }
    }

    let mut json = json!({
        "tasks": picked.iter().map(|t| card_json(&numbers, t)).collect::<Vec<_>>(),
        "waiting": waiting.iter().map(|t| plain_json(&numbers, CardKind::Task, t)).collect::<Vec<_>>(),
    });
    if want_groups {
        json["groups"] = json!(groups
            .iter()
            .map(|g| json!({ "tasks": g.iter().map(|t| plain_json(&numbers, CardKind::Task, t)).collect::<Vec<_>>() }))
            .collect::<Vec<_>>());
    }
    if want_held_back {
        json["heldBack"] = json!(held_back
            .iter()
            .map(|(t, why)| {
                let mut value = plain_json(&numbers, CardKind::Task, t);
                value["reason"] = json!(why.text(&numbers));
                value
            })
            .collect::<Vec<_>>());
    }
    Ok(reply(markdown, json))
}

// ---------------------------------------------------------------------------------------------
// move
// ---------------------------------------------------------------------------------------------

fn move_task(call: &Call) -> Result<CommandReply, Failure> {
    let id = call.arg.expect("a Task is named");
    let state = call.state.expect("a state is named");
    let job = call.caller.job.as_deref().ok_or_else(|| {
        Failure::new(
            weaver_core::command::code::FORBIDDEN,
            "Only the agent of a job that Weaver started can run `weaver task move`.",
        )
    })?;
    let unchanged = call.input.get("unchanged").and_then(Value::as_str);
    if unchanged.is_some() && state != "done" {
        return Err(Failure::invalid(
            "Invalid input:\n  - unchanged: goes with the state `done`",
        ));
    }
    let numbers = shortnum::peek(call.root, call.image.cards());
    let label = name(&numbers, CardKind::Task, id);

    let (changes, outcome): (ChangeSet, Option<FinishPlan>) = match state {
        "doing" => (
            tasks::plan_take(call.image, &numbers, id, job).map_err(refused)?,
            None,
        ),
        "todo" => (
            tasks::plan_release(call.image, &numbers, id, job).map_err(refused)?,
            None,
        ),
        _ => {
            let plan = tasks::plan_finish(call.image, &numbers, id, job, &now()?, unchanged)
                .map_err(refused)?;
            (plan.changes.clone(), Some(plan))
        }
    };
    write(call, &changes)?;

    let mut markdown = match (state, outcome.as_ref().map(|p| p.outcome)) {
        ("doing", _) => format!("{label} is now in progress, held by this job.\n"),
        ("todo", _) => format!("{label} is back in To do.\n"),
        (_, Some(Outcome::ClosedUnchanged)) => {
            format!("{label} is Done, closed without changes.\n")
        }
        _ => format!("{label} is Done.\n"),
    };
    let mut note = None;
    if let Some(plan) = &outcome {
        if let Some(drift) = &plan.drift {
            let status = if plan.outcome == Outcome::Finished {
                "Fixed"
            } else {
                "Dismissed"
            };
            markdown.push_str(&format!(
                "\nDrift {} is now {status}.\n",
                name(&numbers, CardKind::Drift, drift)
            ));
        }
        if !plan.forgotten.is_empty() {
            history::remove_task_folders(call.root, &plan.forgotten);
            note = Some(format!(
                "{} older Done Task(s) were forgotten.",
                plan.forgotten.len()
            ));
        }
    }
    let after = ProjectImage::load(call.root);
    let after = shortnum::peek(call.root, after.cards());
    let column = match state {
        "doing" => Column::Doing,
        "todo" => Column::Todo,
        _ => Column::Done,
    };
    let title = call
        .image
        .card(CardKind::Task, id)
        .map_or(String::new(), |t| t.title.clone());
    let mut result = reply(
        markdown,
        json!({
            "id": id,
            "number": after.number_of(CardKind::Task, id),
            "title": title,
            "column": column.as_str(),
        }),
    );
    result.note = note;
    Ok(result)
}

// ---------------------------------------------------------------------------------------------
// delete
// ---------------------------------------------------------------------------------------------

fn delete(call: &Call) -> Result<CommandReply, Failure> {
    let id = call.arg.expect("a Task is named");
    // The app says which jobs work (a Task the working job filed stays); with no app, none does.
    let changes =
        board::plan_delete_task(call.image, &call.caller.agent, id).map_err(|refused| {
            let notice = refused.notice;
            Failure::refused(
                notice
                    .strip_prefix("Could not delete ")
                    .map_or(notice.clone(), |rest| format!("cannot delete {rest}")),
            )
        })?;
    let title = call
        .image
        .card(CardKind::Task, id)
        .map_or(String::new(), |t| t.title.clone());
    let numbers = shortnum::peek(call.root, call.image.cards());
    let label = name(&numbers, CardKind::Task, id);
    write(call, &changes)?;
    history::remove_task_folders(call.root, &[id.to_owned()]);
    Ok(reply(
        format!("Task {label} \"{title}\" is deleted.\n"),
        json!({ "id": id, "number": null, "title": title }),
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use weaver_core::command::code;
    use weaver_core::project::image::ProjectImage;

    use super::*;

    const ORDER: &str = "# Order\n\nAn order.\n\n## States\n\n### Draft (initial)\nChoosing.\n- `cart.CONFIRM` → Paid\n\n### Paid\nPaid.\n";

    fn put(root: &Path, rel: &str, text: &str) {
        let path = root.join(".weaver").join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), "model/system.md", "# Shop\n\nA shop.\n");
        put(dir.path(), "model/shop/feature.md", "# Shopping\n");
        put(dir.path(), "model/shop/order.md", ORDER);
        dir
    }

    fn task(root: &Path, id: &str, created: u32, column: &str, refs: &[&str], extra: &str) {
        let refs: String = refs.iter().map(|r| format!("  - {r}\n")).collect();
        let refs = if refs.is_empty() {
            "refs: []\n".to_owned()
        } else {
            format!("refs:\n{refs}")
        };
        put(
            root,
            &format!("tasks/{id}-t.md"),
            &format!("---\ncolumn: {column}\ntechnical: false\n{refs}{extra}created: 2026-11-02T07:00:00.{created:03}Z\n---\n# Task {id}\n\nDone when.\n"),
        );
    }

    fn drift(root: &Path, id: &str, created: u32, status: &str, refs: &[&str], extra: &str) {
        let refs: String = refs.iter().map(|r| format!("  - {r}\n")).collect();
        put(
            root,
            &format!("drifts/{id}-d.md"),
            &format!("---\nstatus: {status}\nrefs:\n{refs}{extra}created: 2026-11-02T07:00:00.{created:03}Z\n---\n# Drift {id}\n\nBody.\n"),
        );
    }

    fn run_as(
        verb: &str,
        root: &Path,
        arg: Option<&str>,
        state: Option<&str>,
        job: Option<&str>,
        input: Value,
    ) -> Result<CommandReply, Failure> {
        let all = commands();
        let command = all.iter().find(|c| c.verb == verb).unwrap();
        let image = ProjectImage::load(root);
        let input: Map<String, Value> = input.as_object().unwrap().clone();
        let caller = Caller {
            job: job.map(str::to_owned),
            ..Caller::default()
        };
        (command.run)(&Call {
            command,
            root,
            image: &image,
            arg,
            state,
            input: &input,
            caller: &caller,
        })
    }

    fn run(
        verb: &str,
        root: &Path,
        arg: Option<&str>,
        input: Value,
    ) -> Result<CommandReply, Failure> {
        run_as(verb, root, arg, None, None, input)
    }

    fn ids(reply: &CommandReply, key: &str) -> Vec<String> {
        reply.json[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["id"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn new_files_a_task_in_todo_recording_the_job_and_gives_its_number() {
        let dir = project();
        let reply = run_as("new", dir.path(), None, None, Some("jjjjjjjj"), json!({ "title": "Cancel an order", "refs": ["order/Paid"], "body": "It is cancelled." })).unwrap();
        assert_eq!(reply.json["column"], "todo");
        assert_eq!(reply.json["number"], 1);
        let id = reply.json["id"].as_str().unwrap();
        let image = ProjectImage::load(dir.path());
        let card = image.card(CardKind::Task, id).unwrap();
        assert_eq!(card.str_key("job"), Some("jjjjjjjj"));
        assert_eq!(card.list_key("refs"), Some(vec!["order/Paid".to_owned()]));
        assert!(dir
            .path()
            .join(format!(".weaver/tasks/{id}-cancel-an-order.md"))
            .is_file());
    }

    #[test]
    fn new_refuses_a_task_without_a_model_reference_and_a_technical_one_with_it_writing_nothing() {
        let dir = project();
        let none = run("new", dir.path(), None, json!({ "title": "Nothing" })).unwrap_err();
        assert_eq!(none.code, code::REFUSED);
        assert!(none.message.contains("refs:"), "{}", none.message);
        let technical = run(
            "new",
            dir.path(),
            None,
            json!({ "title": "Tech", "refs": ["order"], "technical": true }),
        )
        .unwrap_err();
        assert!(
            technical.message.contains("technical:"),
            "{}",
            technical.message
        );
        let bad = run(
            "new",
            dir.path(),
            None,
            json!({ "title": "Bad", "refs": ["order", "nowhere"] }),
        )
        .unwrap_err();
        assert!(bad.message.contains("refs[1]"), "{}", bad.message);
        assert!(!dir.path().join(".weaver/tasks").exists());
        assert!(run(
            "new",
            dir.path(),
            None,
            json!({ "title": "Fine", "technical": true })
        )
        .is_ok());
    }

    #[test]
    fn show_prints_the_task_its_blockers_and_what_it_refers_to() {
        let dir = project();
        task(dir.path(), "aaaaaaaa", 1, "todo", &["order/Paid"], "");
        drift(dir.path(), "dddddddd", 2, "open", &["order"], "");
        let reply = run("show", dir.path(), Some("aaaaaaaa"), json!({})).unwrap();
        assert_eq!(reply.json["column"], "todo");
        assert_eq!(reply.json["technical"], false);
        assert_eq!(reply.json["refs"], json!(["order/Paid"]));
        assert_eq!(reply.json["blockedBy"], json!(["D1"]));
        assert_eq!(reply.json["number"], 1);
        assert!(reply.markdown.contains("Done when."));
        assert!(reply.markdown.contains("### `order/Paid`"));
        assert!(
            reply.markdown.contains("D1 Drift dddddddd"),
            "{}",
            reply.markdown
        );
    }

    #[test]
    fn list_without_input_lists_every_task_and_the_waiting_ones() {
        let dir = project();
        let empty = run("list", dir.path(), None, json!({})).unwrap();
        assert_eq!(empty.json, json!({ "tasks": [], "waiting": [] }));
        assert!(empty.markdown.contains("No Task."));
        task(dir.path(), "aaaaaaaa", 1, "todo", &["order"], "");
        task(
            dir.path(),
            "bbbbbbbb",
            2,
            "done",
            &["order"],
            "finished: 2026-11-02T08:00:00.000Z\n",
        );
        drift(dir.path(), "dddddddd", 3, "open", &["order/Paid"], "");
        let all = run("list", dir.path(), None, json!({})).unwrap();
        assert_eq!(ids(&all, "tasks"), ["aaaaaaaa", "bbbbbbbb"]);
        assert_eq!(ids(&all, "waiting"), ["aaaaaaaa"]);
        let done = run("list", dir.path(), None, json!({ "column": "done" })).unwrap();
        assert_eq!(ids(&done, "tasks"), ["bbbbbbbb"]);
    }

    #[test]
    fn list_runnable_gives_the_next_task_the_free_places_the_groups_and_what_was_left_out() {
        let dir = project();
        put(
            dir.path(),
            "model/actors/user.md",
            "# User\n\nA person.\n\n## Events\n\n### CANCEL\nAsks.\n",
        );
        task(dir.path(), "aaaaaaaa", 1, "todo", &["order/Paid"], "");
        task(dir.path(), "bbbbbbbb", 2, "todo", &["order"], "");
        task(dir.path(), "cccccccc", 3, "todo", &["user"], "");
        let next = run("list", dir.path(), None, json!({ "runnable": true })).unwrap();
        assert_eq!(ids(&next, "tasks"), ["aaaaaaaa"]);
        let places = run(
            "list",
            dir.path(),
            None,
            json!({ "runnable": true, "max": 3, "heldBack": true }),
        )
        .unwrap();
        assert_eq!(ids(&places, "tasks"), ["aaaaaaaa", "cccccccc"]);
        assert_eq!(ids(&places, "heldBack"), ["bbbbbbbb"]);
        assert!(places.json["heldBack"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("same thing as T1"));
        let grouped = run(
            "list",
            dir.path(),
            None,
            json!({ "runnable": true, "max": 2, "groups": true }),
        )
        .unwrap();
        let sizes: Vec<usize> = grouped.json["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g["tasks"].as_array().unwrap().len())
            .collect();
        assert_eq!(sizes, [2, 1]);
        let one = run(
            "list",
            dir.path(),
            None,
            json!({ "runnable": true, "tasks": ["cccccccc"] }),
        )
        .unwrap();
        assert_eq!(ids(&one, "tasks"), ["cccccccc"]);
    }

    #[test]
    fn list_filters_by_the_proposal_of_the_design() {
        let dir = project();
        put(
            dir.path(),
            "proposals/pppppppp-p/proposal.md",
            "---\nstatus: approved\ncreated: 2026-11-02T06:00:00.000Z\n---\n# P\n",
        );
        task(
            dir.path(),
            "aaaaaaaa",
            1,
            "todo",
            &["order"],
            "proposal: pppppppp\n",
        );
        task(dir.path(), "bbbbbbbb", 2, "todo", &["order"], "");
        let with = run(
            "list",
            dir.path(),
            None,
            json!({ "proposals": ["pppppppp"] }),
        )
        .unwrap();
        assert_eq!(ids(&with, "tasks"), ["aaaaaaaa"]);
        let without = run(
            "list",
            dir.path(),
            None,
            json!({ "notProposals": ["pppppppp"] }),
        )
        .unwrap();
        assert_eq!(ids(&without, "tasks"), ["bbbbbbbb"]);
    }

    #[test]
    fn list_options_that_need_runnable_are_invalid_input() {
        let problems =
            list_needs_runnable(json!({ "max": 2, "groups": true }).as_object().unwrap());
        assert_eq!(
            problems.iter().map(|p| p.path.as_str()).collect::<Vec<_>>(),
            ["max", "groups"]
        );
        assert!(
            list_needs_runnable(json!({ "runnable": true, "max": 2 }).as_object().unwrap())
                .is_empty()
        );
    }

    #[test]
    fn a_job_takes_a_free_task_and_releases_or_finishes_it_and_nobody_else_can() {
        let dir = project();
        task(dir.path(), "aaaaaaaa", 1, "todo", &["order"], "");
        let taken = run_as(
            "move",
            dir.path(),
            Some("aaaaaaaa"),
            Some("doing"),
            Some("jjjjjjjj"),
            json!({}),
        )
        .unwrap();
        assert_eq!(taken.json["column"], "doing");
        let image = ProjectImage::load(dir.path());
        assert_eq!(
            image
                .card(CardKind::Task, "aaaaaaaa")
                .unwrap()
                .str_key("holder"),
            Some("jjjjjjjj")
        );

        let stranger = run_as(
            "move",
            dir.path(),
            Some("aaaaaaaa"),
            Some("todo"),
            Some("kkkkkkkk"),
            json!({}),
        )
        .unwrap_err();
        assert_eq!(stranger.code, code::REFUSED);
        let released = run_as(
            "move",
            dir.path(),
            Some("aaaaaaaa"),
            Some("todo"),
            Some("jjjjjjjj"),
            json!({}),
        )
        .unwrap();
        assert_eq!(released.json["column"], "todo");
        run_as(
            "move",
            dir.path(),
            Some("aaaaaaaa"),
            Some("doing"),
            Some("jjjjjjjj"),
            json!({}),
        )
        .unwrap();
        let done = run_as(
            "move",
            dir.path(),
            Some("aaaaaaaa"),
            Some("done"),
            Some("jjjjjjjj"),
            json!({}),
        )
        .unwrap();
        assert_eq!(done.json["column"], "done");
        let image = ProjectImage::load(dir.path());
        let card = image.card(CardKind::Task, "aaaaaaaa").unwrap();
        assert_eq!(card.str_key("column"), Some("done"));
        assert!(card.str_key("finished").is_some());
        assert_eq!(card.str_key("holder"), None);
    }

    #[test]
    fn taking_a_blocked_task_is_refused_naming_the_drift_and_nothing_changes() {
        let dir = project();
        task(dir.path(), "aaaaaaaa", 1, "todo", &["order"], "");
        drift(dir.path(), "dddddddd", 2, "open", &["order/Paid"], "");
        let before = fs::read_to_string(dir.path().join(".weaver/tasks/aaaaaaaa-t.md")).unwrap();
        let error = run_as(
            "move",
            dir.path(),
            Some("aaaaaaaa"),
            Some("doing"),
            Some("jjjjjjjj"),
            json!({}),
        )
        .unwrap_err();
        assert_eq!(error.code, code::REFUSED);
        assert!(error.message.contains("Drift D1"), "{}", error.message);
        assert_eq!(
            fs::read_to_string(dir.path().join(".weaver/tasks/aaaaaaaa-t.md")).unwrap(),
            before
        );
    }

    #[test]
    fn the_fix_task_closed_unchanged_dismisses_its_drift_and_one_finished_fixes_it() {
        for (input, status) in [
            (json!({}), "fixed"),
            (
                json!({ "unchanged": "The code already does it." }),
                "dismissed",
            ),
        ] {
            let dir = project();
            task(dir.path(), "aaaaaaaa", 1, "todo", &["order"], "");
            task(
                dir.path(),
                "ffffffff",
                2,
                "doing",
                &["order"],
                "drift: dddddddd\nholder: jjjjjjjj\n",
            );
            drift(
                dir.path(),
                "dddddddd",
                3,
                "code-to-be-fixed",
                &["order"],
                "origin: aaaaaaaa\ntask: ffffffff\n",
            );
            let waiting = run_as(
                "move",
                dir.path(),
                Some("aaaaaaaa"),
                Some("doing"),
                Some("jjjjjjjj"),
                json!({}),
            )
            .unwrap_err();
            assert!(waiting.message.contains("fix Task"), "{}", waiting.message);
            let reply = run_as(
                "move",
                dir.path(),
                Some("ffffffff"),
                Some("done"),
                Some("jjjjjjjj"),
                input,
            )
            .unwrap();
            assert!(
                reply.markdown.contains(if status == "fixed" {
                    "Fixed"
                } else {
                    "Dismissed"
                }),
                "{}",
                reply.markdown
            );
            let image = ProjectImage::load(dir.path());
            assert_eq!(
                image
                    .card(CardKind::Drift, "dddddddd")
                    .unwrap()
                    .str_key("status"),
                Some(status)
            );
            // The origin is free once the fix Task is Done.
            run_as(
                "move",
                dir.path(),
                Some("aaaaaaaa"),
                Some("doing"),
                Some("jjjjjjjj"),
                json!({}),
            )
            .unwrap();
        }
    }

    #[test]
    fn unchanged_goes_with_done_and_only_with_a_fix_task() {
        let dir = project();
        task(
            dir.path(),
            "aaaaaaaa",
            1,
            "doing",
            &["order"],
            "holder: jjjjjjjj\n",
        );
        let wrong_state = run_as(
            "move",
            dir.path(),
            Some("aaaaaaaa"),
            Some("todo"),
            Some("jjjjjjjj"),
            json!({ "unchanged": "x" }),
        )
        .unwrap_err();
        assert_eq!(wrong_state.code, code::INVALID_INPUT);
        let not_fix = run_as(
            "move",
            dir.path(),
            Some("aaaaaaaa"),
            Some("done"),
            Some("jjjjjjjj"),
            json!({ "unchanged": "x" }),
        )
        .unwrap_err();
        assert_eq!(not_fix.code, code::REFUSED);
    }

    #[test]
    fn delete_removes_a_task_in_todo_and_refuses_the_origin_of_an_open_drift() {
        let dir = project();
        task(dir.path(), "aaaaaaaa", 1, "todo", &["order"], "");
        task(dir.path(), "bbbbbbbb", 2, "todo", &["order"], "");
        put(dir.path(), "task-images/aaaaaaaa/a.png", "png");
        drift(
            dir.path(),
            "dddddddd",
            3,
            "open",
            &["user"],
            "origin: bbbbbbbb\n",
        );
        let refused = run("delete", dir.path(), Some("bbbbbbbb"), json!({})).unwrap_err();
        assert_eq!(refused.code, code::REFUSED);
        assert!(
            refused.message.contains("origin of open Drift D1"),
            "{}",
            refused.message
        );
        assert!(dir.path().join(".weaver/tasks/bbbbbbbb-t.md").is_file());
        let deleted = run("delete", dir.path(), Some("aaaaaaaa"), json!({})).unwrap();
        assert_eq!(deleted.json["id"], "aaaaaaaa");
        assert!(!dir.path().join(".weaver/tasks/aaaaaaaa-t.md").exists());
        assert!(!dir.path().join(".weaver/task-images/aaaaaaaa").exists());
    }
}
