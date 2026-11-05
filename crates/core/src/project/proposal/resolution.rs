//! A resolution Proposal: the decisions it records on open Drifts, and what approving it does
//! to them.
//!
//! A resolution lists one decision per Drift in its `drifts` key: `fix-code` (with the fix
//! option, from 1, when the Drift has options, and a note), `proposal` (the Model is adapted by
//! the files of the resolution) or `dismissed`. Approving it gives every listed Drift that is
//! still open its decision: `Code to be fixed` with a fix Task, `Proposal accepted` or
//! `Dismissed`.

use std::collections::BTreeSet;

use serde_yaml::{Mapping, Value};

use crate::project::board::{resolution_rows, ResolutionRow};
use crate::project::cards::{Card, CardKind};
use crate::project::changes::ChangeSet;
use crate::project::drift::{fingerprint, DRIFT_KEYS};
use crate::project::frontmatter;
use crate::project::ids::{card_stem, draw_identity, slugify};
use crate::project::image::ProjectImage;
use crate::project::layout::TASKS_DIR;
use crate::project::md::outline;
use crate::project::model::Model;
use crate::project::random::RandomSource;
use crate::project::refs::{parse_reference, ElementRef, Reference};
use crate::project::tasks::{fix_task_body, render_task, TaskDraft};

use super::data::{self, Kind, Status};

/// One way to fix the code, as the Drift offers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixOption {
    pub title: String,
    pub text: String,
}

/// The text of the section `## <heading>` of a body, without the heading.
pub fn section_text(body: &str, heading: &str) -> String {
    let headings = outline(body).headings;
    let Some(start) = headings
        .iter()
        .position(|h| h.level == 2 && h.text == heading)
    else {
        return String::new();
    };
    let from = headings[start].range.end;
    let to = headings[start + 1..]
        .iter()
        .find(|h| h.level <= 2)
        .map_or(body.len(), |h| h.range.start);
    body[from..to].trim().to_owned()
}

/// The fix options of a Drift: one `### <title>` per option under `## Fix options`, in order.
pub fn drift_options(drift: &Card) -> Vec<FixOption> {
    let headings = outline(&drift.body).headings;
    let Some(start) = headings
        .iter()
        .position(|h| h.level == 2 && h.text == "Fix options")
    else {
        return Vec::new();
    };
    let end = headings[start + 1..]
        .iter()
        .find(|h| h.level <= 2)
        .map_or(drift.body.len(), |h| h.range.start);
    let options: Vec<_> = headings[start + 1..]
        .iter()
        .take_while(|h| h.range.start < end && h.level >= 2)
        .filter(|h| h.level == 3)
        .collect();
    options
        .iter()
        .enumerate()
        .map(|(index, heading)| {
            let to = options.get(index + 1).map_or(end, |next| next.range.start);
            FixOption {
                title: heading.text.clone(),
                text: drift.body[heading.range.end..to].trim().to_owned(),
            }
        })
        .collect()
}

/// The Model elements above `element` that could replace it, nearest first (Transition → State
/// → Machine → Feature; Event → Machine or Actor). `earlier` is the Model as it was, to find
/// the Feature of a Machine that is gone.
fn containers(earlier: &Model, element: &ElementRef) -> Vec<ElementRef> {
    let feature_of = |machine: &str| {
        earlier
            .machines
            .get(machine)
            .map(|m| ElementRef::Element(m.feature.clone()))
    };
    let mut chain = Vec::new();
    match element {
        ElementRef::Transitions { machine, state, .. } => {
            chain.push(ElementRef::State {
                machine: machine.clone(),
                state: state.clone(),
            });
            chain.extend(parent_states(earlier, machine, state));
            chain.push(ElementRef::Element(machine.clone()));
            chain.extend(feature_of(machine));
        }
        ElementRef::State { machine, state } => {
            chain.extend(parent_states(earlier, machine, state));
            chain.push(ElementRef::Element(machine.clone()));
            chain.extend(feature_of(machine));
        }
        ElementRef::Event { owner, .. } => {
            chain.push(ElementRef::Element(owner.clone()));
            chain.extend(feature_of(owner));
        }
        ElementRef::Element(id) => chain.extend(feature_of(id)),
    }
    chain
}

fn parent_states(earlier: &Model, machine: &str, state: &str) -> Vec<ElementRef> {
    let Some(machine_model) = earlier.machines.get(machine) else {
        return Vec::new();
    };
    let Some((path, _)) = machine_model
        .walk_states()
        .into_iter()
        .find(|(_, s)| s.name == state)
    else {
        return Vec::new();
    };
    path[..path.len() - 1]
        .iter()
        .rev()
        .map(|name| ElementRef::State {
            machine: machine.to_owned(),
            state: (*name).to_owned(),
        })
        .collect()
}

/// The references of a fix Task for `drift`: each reference that still exists, a Model element
/// that is gone replaced by its nearest container that exists (none: it is dropped), a missing
/// Requirement section or Specs field dropped. `after` is the project once the decision is
/// applied; `earlier` is where the containers of vanished elements are looked up.
pub fn fix_refs(after: &ProjectImage, earlier: &ProjectImage, drift: &Card) -> Vec<String> {
    let resolver = after.resolver();
    let mut refs: Vec<String> = Vec::new();
    let mut keep = |text: String| {
        if !refs.contains(&text) {
            refs.push(text);
        }
    };
    for text in drift.list_key("refs").unwrap_or_default() {
        match parse_reference(&text) {
            Err(_) => {}
            Ok(Reference::Element(element)) => {
                if resolver.resolve_element(&element).is_ok() {
                    keep(element.to_string());
                } else if let Some(container) = containers(earlier.model(), &element)
                    .into_iter()
                    .find(|c| resolver.resolve_element(c).is_ok())
                {
                    keep(container.to_string());
                }
            }
            Ok(other) => {
                if resolver.resolve(&other).is_ok() {
                    keep(text);
                }
            }
        }
    }
    refs
}

/// Whether a list of references names at least one Model element.
pub fn has_element(refs: &[String]) -> bool {
    refs.iter()
        .any(|r| matches!(parse_reference(r), Ok(Reference::Element(_))))
}

/// The references of the fix Task of `drift` and whether the Task is technical. A Drift that
/// concerns Model elements needs one to be left (the nearest container replaces one that is
/// gone); a Drift on Requirements only needs a section to be left, and its Task is technical.
/// `Err` says why the code cannot be fixed; nothing is written then.
pub fn fix_task_refs(
    after: &ProjectImage,
    earlier: &ProjectImage,
    drift: &Card,
) -> Result<(Vec<String>, bool), String> {
    let had_elements = has_element(&drift.list_key("refs").unwrap_or_default());
    let refs = fix_refs(after, earlier, drift);
    match (had_elements, has_element(&refs)) {
        (true, false) => Err("no Model element it concerns is left".to_owned()),
        (true, true) => Ok((refs, false)),
        (false, _) if refs.is_empty() => {
            Err("no Requirement section it concerns is left".to_owned())
        }
        (false, _) => Ok((refs, true)),
    }
}

/// What is wrong with the decisions of a resolution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Problems {
    /// Refused at hand-over only.
    pub handover: Vec<String>,
    /// `(Drift identity, why)`: a code fix that could no longer be applied.
    pub fixes: Vec<(String, String)>,
}

/// Checks the decisions of `resolution` against the Drifts of `current`; `after` is the project
/// with the resolution's files applied.
pub fn problems(
    current: &ProjectImage,
    after: &ProjectImage,
    resolution: &Card,
    name: &dyn Fn(CardKind, &str) -> String,
) -> Problems {
    let rows = resolution_rows(resolution);
    let mut found = Problems::default();
    if rows.is_empty() {
        found
            .handover
            .push("The resolution lists no Drift".to_owned());
    }
    let mut seen = BTreeSet::new();
    for row in &rows {
        let label = name(CardKind::Drift, &row.drift);
        if !seen.insert(row.drift.as_str()) {
            found
                .handover
                .push(format!("Drift {label} is listed twice"));
            continue;
        }
        let Some(drift) = current.card(CardKind::Drift, &row.drift) else {
            found
                .handover
                .push(format!("Drift {} does not exist", row.drift));
            continue;
        };
        if drift.str_key("status") != Some("open") {
            found.handover.push(format!("Drift {label} is not open"));
            continue;
        }
        let elsewhere = current.cards_of(CardKind::Proposal).find(|other| {
            other.id != resolution.id
                && data::kind_of(other) == Kind::Resolution
                && matches!(
                    data::status_of(other),
                    Some(Status::Draft | Status::Open | Status::Refining)
                )
                && resolution_rows(other).iter().any(|r| r.drift == row.drift)
        });
        if let Some(other) = elsewhere {
            found.handover.push(format!(
                "Drift {label} is already in the resolution {}",
                name(CardKind::Proposal, &other.id)
            ));
        }
        let options = drift_options(drift);
        match row.decision.as_str() {
            "fix-code" => {
                match (options.len(), row.option) {
                    (0, Some(_)) => found.handover.push(format!("Drift {label} offers no fix option: remove `option`")),
                    (0, None) => {}
                    (n, None) => found.handover.push(format!("Drift {label}: `fix-code` needs `option`, from 1 to {n}")),
                    (n, Some(o)) if o < 1 || o as usize > n => {
                        found.handover.push(format!("Drift {label}: option {o} does not exist (from 1 to {n})"));
                    }
                    _ => {}
                }
                if let Err(why) = fix_task_refs(after, current, drift) {
                    found.fixes.push((drift.id.clone(), why));
                }
            }
            "proposal" | "dismissed" => {
                if row.option.is_some() {
                    found.handover.push(format!("Drift {label}: `option` goes only with `fix-code`"));
                }
                if row.note.is_some() {
                    found.handover.push(format!("Drift {label}: `note` goes only with `fix-code`"));
                }
            }
            other => found.handover.push(format!("Drift {label}: `{other}` is not a decision (`fix-code`, `proposal` or `dismissed`)")),
        }
    }
    found
}

/// Whether a Drift is still open.
pub fn is_open(drift: &Card) -> bool {
    drift.str_key("status") == Some("open")
}

/// Whether none of the Drifts a resolution lists is still open (a listed Drift that does not
/// exist is not open either, and a resolution listing nothing has none).
pub fn none_open(current: &ProjectImage, resolution: &Card) -> bool {
    !resolution_rows(resolution).iter().any(|row| {
        current
            .card(CardKind::Drift, &row.drift)
            .is_some_and(is_open)
    })
}

fn drift_text(drift: &Card, edit: impl FnOnce(&mut Mapping)) -> String {
    let mut front = drift.front.clone();
    edit(&mut front);
    frontmatter::render(&front, &DRIFT_KEYS, &drift.body)
}

fn set(front: &mut Mapping, key: &str, value: impl Into<Value>) {
    front.insert(Value::String(key.to_owned()), value.into());
}

/// What a resolution does to its Drifts when approved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decisions {
    pub changes: ChangeSet,
    /// `(Drift identity, its new status)`, in the order of the list.
    pub decided: Vec<(String, &'static str)>,
    /// The fix Tasks created.
    pub fix_tasks: Vec<String>,
}

/// The decision of one Drift of a resolution: the Drift file, and for a code fix the fix Task.
pub fn plan_decision(
    current: &ProjectImage,
    after: &ProjectImage,
    drift: &Card,
    row: &ResolutionRow,
    now: &str,
    rng: &mut dyn RandomSource,
    used: &mut BTreeSet<String>,
) -> Result<Decisions, String> {
    let mut out = Decisions::default();
    let status = match row.decision.as_str() {
        "fix-code" => "code-to-be-fixed",
        "proposal" => "proposal-accepted",
        _ => "dismissed",
    };
    // What happens to the Proposal attached to the Drift: `drift.CODE_TO_BE_FIXED` discards a Draft
    // and rejects an Open one; `drift.DISMISSED` discards either. A resolution that adapts the
    // Model for the Drift carries the changes itself, so the attached Proposal is discarded too.
    let mut dispose = |status: Status| {
        if let Some(proposal) = drift
            .str_key("proposal")
            .and_then(|id| current.card(CardKind::Proposal, id))
        {
            if matches!(
                data::status_of(proposal),
                Some(Status::Draft | Status::Open)
            ) {
                let to = if data::status_of(proposal) == Some(Status::Open) {
                    status
                } else {
                    Status::Discarded
                };
                out.changes
                    .write(&proposal.path, data::with_status(proposal, to));
            }
        }
    };
    if status != "code-to-be-fixed" {
        dispose(Status::Discarded);
        let recorded = (status == "dismissed").then(|| fingerprint::stored_of(after, drift));
        out.changes.write(
            &drift.path,
            drift_text(drift, |front| {
                set(front, "status", status);
                set(front, "decided", now);
                if let Some(recorded) = &recorded {
                    set(front, "fingerprint", recorded.as_str());
                }
            }),
        );
        out.decided.push((drift.id.clone(), status));
        return Ok(out);
    }

    let (refs, technical) = fix_task_refs(after, current, drift)?;
    dispose(Status::Rejected);
    let options = drift_options(drift);
    let chosen = row
        .option
        .and_then(|o| options.get((o as usize).saturating_sub(1)));
    let body = fix_task_body(
        &section_text(&drift.body, "What happens"),
        &section_text(&drift.body, "What should happen"),
        chosen.map(|o| (o.title.as_str(), o.text.as_str())),
        row.note.as_deref(),
        &section_text(&drift.body, "Where in the code"),
    );
    let id = draw_identity(rng, &|candidate| used.contains(candidate));
    used.insert(id.clone());
    let title = format!("Fix the code: {}", drift.title);
    let draft = TaskDraft {
        title: title.clone(),
        body,
        refs,
        technical,
        proposal: None,
        drift: Some(drift.id.clone()),
        job: None,
    };
    out.changes.write(
        &format!("{TASKS_DIR}/{}.md", card_stem(&id, &slugify(&title))),
        render_task(&draft, now),
    );
    out.changes.write(
        &drift.path,
        drift_text(drift, |front| {
            set(front, "status", "code-to-be-fixed");
            set(front, "task", id.as_str());
            if let Some(option) = row.option {
                set(front, "option", u64::from(option));
            }
            if let Some(note) = row.note.as_deref().filter(|n| !n.trim().is_empty()) {
                set(front, "note", note.trim());
            }
            set(front, "decided", now);
        }),
    );
    if let Some(origin) = drift.str_key("origin") {
        let waits = crate::project::tasks::plan_origin_waits(current, origin);
        for (rel, change) in waits.iter() {
            if let crate::project::changes::Change::Write(content) = change {
                out.changes.write(rel, content.clone());
            }
        }
    }
    out.decided.push((drift.id.clone(), status));
    out.fix_tasks.push(id);
    Ok(out)
}

/// What approving `resolution` does to its Drifts: each listed Drift that is still open gets its
/// decision; the others are left as they are.
pub fn plan_decisions(
    current: &ProjectImage,
    after: &ProjectImage,
    resolution: &Card,
    now: &str,
    rng: &mut dyn RandomSource,
    used: &mut BTreeSet<String>,
) -> Result<Decisions, String> {
    let mut all = Decisions::default();
    for row in resolution_rows(resolution) {
        let Some(drift) = current.card(CardKind::Drift, &row.drift) else {
            continue;
        };
        if drift.str_key("status") != Some("open") {
            continue;
        }
        let one = plan_decision(current, after, drift, &row, now, rng, used).map_err(|why| {
            format!(
                "the decision on Drift {} cannot be applied: {why}",
                drift.id
            )
        })?;
        for (rel, change) in one.changes.iter() {
            if let crate::project::changes::Change::Write(content) = change {
                all.changes.write(rel, content.clone());
            }
        }
        all.decided.extend(one.decided);
        all.fix_tasks.extend(one.fix_tasks);
    }
    Ok(all)
}
