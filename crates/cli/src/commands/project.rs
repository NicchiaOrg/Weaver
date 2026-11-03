//! `weaver project new | edit | show`.

use std::path::Path;

use serde_json::{json, Value};
use weaver_core::command::CommandReply;
use weaver_core::project::board::{self, ActiveEntry, BacklogEntry, ProposalState};
use weaver_core::project::cards::{Card, CardKind};
use weaver_core::project::layout::WEAVER_DIR;
use weaver_core::project::proposal::data;
use weaver_core::project::shortnum::{self, ShortNumbers};
use weaver_core::project_files::{self, InitialiseError, SyncOptions, SyncReport};

use super::{card_with, cards, reply, written};
use crate::spec::*;

pub fn commands() -> Vec<Command> {
    vec![
        Command::new("project", "new", "Initialises `.weaver/` in the folder and installs the agent files.", new)
            .notes(&[
                "Creates `.weaver/` with the System and the starting Requirement Areas, and writes the agent files (skills, sub-agent definitions, advisor instructions) in the project. It is refused, creating nothing, when `.weaver/` exists, or when an agent file to install exists with other content: every such file is listed, and `force` overwrites them.",
            ])
            .input(vec![opt("force", boolean(), "Overwrite the agent files that differ from the ones to install.")])
            .output(vec![
                req("root", line(), "The project folder."),
                req("files", list(line()), "The files created in `.weaver/`, relative to it."),
                req("areas", list(line()), "The starting Requirement Areas."),
                req("agentFiles", list(line()), "The agent files written, relative to the project folder."),
            ])
            .place(written)
            .example("weaver project new --input '{}'"),
        Command::new("project", "edit", "Updates the installed skills.", edit)
            .notes(&[
                "Update skills: writes again every installed file that differs from the one this version of Weaver carries, creates the starting Requirement Areas when the Requirements are missing, and generates the Requirement skills again. It always reports what it changed, or the error.",
            ])
            .output(vec![
                req(
                    "changed",
                    list(object(vec![
                        req("path", line(), "The file, relative to the project folder."),
                        req("action", one_of(&["created", "rewritten", "removed"]), "What was done to it."),
                    ])),
                    "The files written or removed; empty when everything was up to date.",
                ),
                req("areas", list(line()), "The Requirement Areas that have a generated skill."),
                req("generated", Ty::Any, "The result of the generation: `mode` (`fragment` or `index`), `budget` used out of `limit` (4,000), and `switched` (`from`, `to`) when the mode changed."),
            ])
            .place(written)
            .example("weaver project edit"),
        Command::new("project", "show", "Prints the Board.", show)
            .output(vec![
                cards("Open Proposals and open Drifts.", "toDecide"),
                req("inWriting", list(object(card_with(vec![req("job", nullable(line()), "The job writing it.")]))), "Draft Proposals with their job."),
                cards("Tasks in To do.", "todo"),
                req("doing", list(object(card_with(vec![
                    req("holder", nullable(line()), "The job that holds the Task."),
                    req("blockedBy", list(line()), "The Drifts that block it."),
                ]))), "Tasks in progress."),
            ])
            .example("weaver project show"),
    ]
}

fn new(call: &Call) -> Result<CommandReply, Failure> {
    let root: &Path = call.root;
    let force = call
        .input
        .get("force")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let done = project_files::initialise(root, force).map_err(|error| match error {
        InitialiseError::Exists => {
            Failure::refused(format!("`{WEAVER_DIR}` already exists in {}: the project is initialised", root.display()))
        }
        InitialiseError::Conflicts(conflicts) => {
            let list: Vec<String> = conflicts.iter().map(|path| format!("  - {path}")).collect();
            Failure::refused(format!(
                "These agent files exist with other content:\n{}\nNothing was created. Run again with `{{\"force\":true}}` to overwrite them.",
                list.join("\n")
            ))
        }
        InitialiseError::Failed(reason) => Failure::failed(format!("Could not initialise: {reason}.")),
    })?;
    let mut markdown = format!(
        "# Project initialised\n\nCreated `{WEAVER_DIR}/` in `{}` with the System (`model/system.md`) and the Requirement Areas {}.\n",
        root.display(),
        done.areas.iter().map(|a| format!("`{a}`")).collect::<Vec<_>>().join(", ")
    );
    if done.agent_files.is_empty() {
        markdown.push_str("\nThe agent files were already in place.\n");
    } else {
        markdown.push_str(&format!(
            "\nWrote {} agent files:\n\n",
            done.agent_files.len()
        ));
        for path in &done.agent_files {
            markdown.push_str(&format!("- `{path}`\n"));
        }
    }
    Ok(reply(
        markdown,
        json!({ "root": root.display().to_string(), "files": done.files, "areas": done.areas, "agentFiles": done.agent_files }),
    ))
}

/// The text the command line and the app show for a report of Update skills.
pub fn skills_markdown(report: &SyncReport) -> String {
    let mut markdown = String::from("# Skills updated\n\n");
    if report.is_unchanged() {
        markdown.push_str("Everything was already up to date.\n");
    } else {
        markdown.push_str(&format!("Updated {} files:\n\n", report.changes.len()));
        for change in &report.changes {
            let action = serde_json::to_value(change.action)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            markdown.push_str(&format!("- `{}` {action}\n", change.path));
        }
    }
    markdown.push_str(&format!("\nRequirement skills: {}.\n", report.mode_line()));
    if let Some(switch) = &report.generated.switched {
        markdown.push_str(&format!(
            "The Requirement skills switched from {} mode to {} mode.\n",
            serde_json::to_value(switch.from)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
            serde_json::to_value(switch.to)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default()
        ));
    }
    markdown
}

fn edit(call: &Call) -> Result<CommandReply, Failure> {
    let report = project_files::sync(call.root, SyncOptions::update_skills())
        .map_err(|error| Failure::failed(format!("Could not update the agent files: {error}.")))?;
    let json = json!({
        "changed": serde_json::to_value(&report.changes).map_err(|e| Failure::failed(e.to_string()))?,
        "areas": report.areas,
        "generated": serde_json::to_value(&report.generated).map_err(|e| Failure::failed(e.to_string()))?,
    });
    Ok(reply(skills_markdown(&report), json))
}

fn name(numbers: &ShortNumbers, kind: CardKind, id: &str) -> String {
    numbers.label_of(kind, id).unwrap_or_else(|| id.to_owned())
}

fn card_json(numbers: &ShortNumbers, kind: CardKind, card: &Card) -> Value {
    json!({ "id": card.id, "number": numbers.number_of(kind, &card.id), "title": card.title })
}

fn show(call: &Call) -> Result<CommandReply, Failure> {
    let image = call.image;
    let numbers = shortnum::peek(call.root, image.cards());
    let board = board::compute(image, &call.caller.agent);
    let kind_of = |id: &str| {
        if image.card(CardKind::Drift, id).is_some() {
            CardKind::Drift
        } else {
            CardKind::Proposal
        }
    };

    // Open Proposals and open Drifts wait for the developer; a Proposal an agent still writes is in writing.
    let mut to_decide: Vec<(CardKind, &Card)> = Vec::new();
    let mut in_writing: Vec<&Card> = Vec::new();
    for entry in &board.backlog {
        let id = entry.id();
        let kind = kind_of(id);
        let Some(card) = image.card(kind, id) else {
            continue;
        };
        match entry {
            BacklogEntry::Drift { .. } => to_decide.push((kind, card)),
            BacklogEntry::Proposal { state, .. } | BacklogEntry::Resolution { state, .. } => {
                match state {
                    ProposalState::Open | ProposalState::Interrupted => {
                        to_decide.push((kind, card))
                    }
                    ProposalState::Refining => in_writing.push(card),
                }
            }
        }
    }
    for card in image.cards_of(CardKind::Proposal) {
        if data::status_of(card) == Some(data::Status::Draft)
            && card.bool_key("refining") != Some(true)
        {
            in_writing.push(card);
        }
    }
    let task_of = |entry: &ActiveEntry| image.card(CardKind::Task, &entry.id);
    let todo: Vec<&Card> = board
        .active
        .iter()
        .filter(|e| !e.doing)
        .filter_map(task_of)
        .collect();
    let doing: Vec<(&ActiveEntry, &Card)> = board
        .active
        .iter()
        .filter(|e| e.doing)
        .filter_map(|e| task_of(e).map(|t| (e, t)))
        .collect();

    let mut markdown = String::from("# Board\n");
    let mut section = |title: &str, lines: Vec<String>| {
        markdown.push_str(&format!("\n## {title}\n\n"));
        if lines.is_empty() {
            markdown.push_str("_none_\n");
        }
        for line in lines {
            markdown.push_str(&line);
            markdown.push('\n');
        }
    };
    section(
        "To decide",
        to_decide
            .iter()
            .map(|(kind, card)| {
                format!(
                    "- `{}` {} {} ({})",
                    name(&numbers, *kind, &card.id),
                    card.id,
                    card.title,
                    if *kind == CardKind::Drift {
                        "Drift"
                    } else {
                        "Proposal"
                    }
                )
            })
            .collect(),
    );
    section(
        "In writing",
        in_writing
            .iter()
            .map(|card| {
                let job = data::job_of(card)
                    .map(|j| format!(" · job `{j}`"))
                    .unwrap_or_default();
                format!(
                    "- `{}` {} {}{job}",
                    name(&numbers, CardKind::Proposal, &card.id),
                    card.id,
                    card.title
                )
            })
            .collect(),
    );
    section(
        "To do",
        todo.iter()
            .map(|t| {
                format!(
                    "- `{}` {} {}",
                    name(&numbers, CardKind::Task, &t.id),
                    t.id,
                    t.title
                )
            })
            .collect(),
    );
    section(
        "In progress",
        doing
            .iter()
            .map(|(entry, task)| {
                let holder = task
                    .str_key("holder")
                    .map(|h| format!(" · held by job `{h}`"))
                    .unwrap_or_default();
                let blockers: Vec<String> = entry
                    .blocked_by
                    .iter()
                    .map(|d| name(&numbers, CardKind::Drift, d))
                    .collect();
                let blocked = if blockers.is_empty() {
                    String::new()
                } else {
                    format!(" · blocked by Drift {}", blockers.join(", "))
                };
                format!(
                    "- `{}` {} {}{holder}{blocked}",
                    name(&numbers, CardKind::Task, &task.id),
                    task.id,
                    task.title
                )
            })
            .collect(),
    );

    let json = json!({
        "toDecide": to_decide.iter().map(|(kind, card)| card_json(&numbers, *kind, card)).collect::<Vec<_>>(),
        "inWriting": in_writing.iter().map(|card| {
            let mut value = card_json(&numbers, CardKind::Proposal, card);
            value["job"] = json!(data::job_of(card));
            value
        }).collect::<Vec<_>>(),
        "todo": todo.iter().map(|t| card_json(&numbers, CardKind::Task, t)).collect::<Vec<_>>(),
        "doing": doing.iter().map(|(entry, task)| {
            let mut value = card_json(&numbers, CardKind::Task, task);
            value["holder"] = json!(task.str_key("holder"));
            value["blockedBy"] = json!(entry.blocked_by);
            value
        }).collect::<Vec<_>>(),
    });
    Ok(reply(markdown, json))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::Map;
    use weaver_core::project::image::ProjectImage;

    use super::*;
    use crate::spec::Caller;

    fn run(root: &Path, input: Value) -> Result<CommandReply, Failure> {
        let command = &commands()[0];
        let image = ProjectImage::empty(root);
        let input: Map<String, Value> = input.as_object().unwrap().clone();
        new(&Call {
            command,
            root,
            image: &image,
            arg: None,
            state: None,
            input: &input,
            caller: &Caller::default(),
        })
    }

    #[test]
    fn a_new_project_gets_the_model_the_areas_and_the_agent_files() {
        let dir = tempfile::tempdir().unwrap();
        let reply = run(dir.path(), json!({})).unwrap();
        assert!(reply.markdown.starts_with("# Project initialised"));
        assert!(dir.path().join(".weaver/model/system.md").is_file());
        assert!(dir.path().join(".weaver/requirements/ui/area.md").is_file());
        assert!(dir
            .path()
            .join(".agents/skills/weaver-design/COMMANDS.md")
            .is_file());
        assert!(dir.path().join(".omp/WATCHDOG.yml").is_file());
        let image = ProjectImage::load(dir.path());
        assert!(image.diagnostics().is_empty(), "{:?}", image.diagnostics());
        assert_eq!(reply.json["areas"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn an_existing_weaver_folder_refuses_and_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".weaver")).unwrap();
        let error = run(dir.path(), json!({})).unwrap_err();
        assert_eq!(error.code, "refused");
        assert!(!dir.path().join(".agents").exists());
    }

    #[test]
    fn an_old_trama_folder_is_left_alone_and_weaver_is_created_beside_it() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join(".trama/model/system.md");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, "# Old\n").unwrap();
        run(dir.path(), json!({})).unwrap();
        assert!(dir.path().join(".weaver/model/system.md").is_file());
        assert_eq!(fs::read_to_string(&old).unwrap(), "# Old\n");
    }

    #[test]
    fn differing_agent_files_are_all_listed_and_nothing_is_created_unless_forced() {
        let dir = tempfile::tempdir().unwrap();
        for file in [".agents/skills/weaver-design/SKILL.md", ".omp/WATCHDOG.yml"] {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "mine").unwrap();
        }
        let error = run(dir.path(), json!({})).unwrap_err();
        assert!(
            error
                .message
                .contains("- .agents/skills/weaver-design/SKILL.md"),
            "{}",
            error.message
        );
        assert!(error.message.contains("- .omp/WATCHDOG.yml"));
        assert!(!dir.path().join(".weaver").exists());
        assert!(!dir.path().join(".agents/skills/weaver-setup").exists());
        assert_eq!(
            fs::read_to_string(dir.path().join(".omp/WATCHDOG.yml")).unwrap(),
            "mine"
        );

        run(dir.path(), json!({ "force": true })).unwrap();
        assert!(dir.path().join(".weaver").is_dir());
        assert_ne!(
            fs::read_to_string(dir.path().join(".omp/WATCHDOG.yml")).unwrap(),
            "mine"
        );
    }

    fn run_edit_or_show(index: usize, root: &Path) -> Result<CommandReply, Failure> {
        let command = &commands()[index];
        let image = ProjectImage::load(root);
        let input = Map::new();
        (command.run)(&Call {
            command,
            root,
            image: &image,
            arg: None,
            state: None,
            input: &input,
            caller: &Caller::default(),
        })
    }

    #[test]
    fn project_edit_rewrites_what_differs_and_then_reports_nothing_changed() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path(), json!({})).unwrap();
        let calm = run_edit_or_show(1, dir.path()).unwrap();
        assert_eq!(calm.json["changed"], json!([]));
        assert!(
            calm.markdown.contains("Everything was already up to date."),
            "{}",
            calm.markdown
        );
        assert_eq!(calm.json["generated"]["mode"], "fragment");
        assert_eq!(calm.json["generated"]["limit"], 4000);

        fs::write(dir.path().join(".omp/WATCHDOG.yml"), "mine").unwrap();
        let busy = run_edit_or_show(1, dir.path()).unwrap();
        assert_eq!(
            busy.json["changed"],
            json!([{ "path": ".omp/WATCHDOG.yml", "action": "rewritten" }])
        );
        assert!(
            busy.markdown.contains("Updated 1 files:")
                && busy.markdown.contains("`.omp/WATCHDOG.yml` rewritten"),
            "{}",
            busy.markdown
        );
        assert_ne!(
            fs::read_to_string(dir.path().join(".omp/WATCHDOG.yml")).unwrap(),
            "mine"
        );
    }

    #[test]
    fn project_show_prints_the_board_of_an_empty_project() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path(), json!({})).unwrap();
        let reply = run_edit_or_show(2, dir.path()).unwrap();
        assert_eq!(
            reply.json,
            json!({ "toDecide": [], "inWriting": [], "todo": [], "doing": [] })
        );
        assert!(reply.markdown.starts_with("# Board"), "{}", reply.markdown);
    }
}
