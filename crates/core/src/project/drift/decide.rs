//! The decisions of the developer on an Open Drift: `developer.FIX_DRIFT_CODE`,
//! `developer.ACCEPT_DRIFT_PROPOSAL` and `developer.DISMISS_DRIFT`.
//!
//! Every `plan_*` function is pure and returns the [`ChangeSet`] of the decision or a [`Refusal`]
//! naming every reason. [`decide`] runs the plan under the project lock through
//! [`changes::mutate`]: it reads the project again, plans, validates and writes the whole change or
//! nothing, so a condition is checked when the decision is applied, never trusted from when its
//! button was enabled.
//!
//! * Fix the code → `Code to be fixed`: an option is required when the Drift offers some;
//!   references that are gone are replaced by their nearest container; a fix Task is created with
//!   both sides, the option, the note and where in the code; the Task named as origin goes back
//!   to To do; the attached Proposal is discarded (Draft) or rejected (Open).
//! * Accept the Proposal → `Proposal accepted`: only with an attached Proposal that could be
//!   approved; approving it is the whole decision.
//! * Dismiss → `Dismissed`, refused while the Drift is in a resolution being written or waiting for
//!   a decision; the attached Proposal is discarded; the text fingerprint is recorded.

use std::path::Path;

use crate::project::board::{AgentFacts, Refused, ResolutionRow, REASON_AGENT_FINISHING};
use crate::project::cards::{Card, CardKind};
use crate::project::changes::{self, Change, ChangeSet, MutationError};
use crate::project::frontmatter;
use crate::project::image::ProjectImage;
use crate::project::proposal::data::{self, set, Status};
use crate::project::proposal::files::{self, Folders};
use crate::project::proposal::resolution;
use crate::project::proposal::transitions::{self, Approval, Decider, Refusal};
use crate::project::random::RandomSource;
use crate::project::shortnum::{self, ShortNumbers};
use crate::project_files::SyncReport;

use super::reasons;
use super::{drift_options, is_open, status_of, DRIFT_KEYS};

/// What the developer decides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// `developer.FIX_DRIFT_CODE`: the fix option, from 1, and the developer's note.
    FixCode {
        option: Option<u32>,
        note: Option<String>,
    },
    /// `developer.ACCEPT_DRIFT_PROPOSAL`.
    AcceptProposal,
    /// `developer.DISMISS_DRIFT`.
    Dismiss,
}

impl Decision {
    /// The verb of a notice: `Could not <verb> D3: …`.
    pub fn verb(&self) -> &'static str {
        match self {
            Decision::FixCode { .. } => "fix the code of",
            Decision::AcceptProposal => "accept the Proposal of",
            Decision::Dismiss => "dismiss",
        }
    }
}

/// What a decision does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decided {
    pub changes: ChangeSet,
    /// The status of the Drift after the decision.
    pub status: &'static str,
    /// The fix Task created.
    pub fix_task: Option<String>,
    /// The Requirements changed: the Requirement skills are generated again in the same step.
    pub requirements_changed: bool,
    /// The Requirement skills generated again after the decision was written, when
    /// `requirements_changed`: the report, or why they could not be written (the decision stands).
    pub skills: Option<Result<SyncReport, String>>,
}

fn refuse<T>(reason: impl Into<String>) -> Result<T, Refusal> {
    Err(Refusal(vec![reason.into()]))
}

fn merge(into: &mut ChangeSet, from: &ChangeSet) {
    for (rel, change) in from.iter() {
        match change {
            Change::Write(content) => into.write(rel, content.clone()),
            Change::Delete => into.delete(rel),
        }
    }
}

fn name_of(numbers: &ShortNumbers, drift: &Card) -> String {
    numbers
        .label_of(CardKind::Drift, &drift.id)
        .unwrap_or_else(|| drift.id.clone())
}

/// Plans a decision on the Drift `id`.
#[allow(clippy::too_many_arguments)]
pub fn plan_decision(
    image: &ProjectImage,
    folders: &Folders,
    numbers: &ShortNumbers,
    facts: &AgentFacts,
    id: &str,
    decision: &Decision,
    now: &str,
    rng: &mut dyn RandomSource,
) -> Result<Decided, Refusal> {
    let Some(drift) = image.card(CardKind::Drift, id) else {
        return refuse(format!("no Drift has the identity {id}"));
    };
    if !is_open(drift) {
        return refuse(format!(
            "the Drift is {}",
            status_of(drift).unwrap_or("not open")
        ));
    }
    if facts.is_finishing(drift) {
        return refuse(REASON_AGENT_FINISHING);
    }
    match decision {
        Decision::FixCode { option, note } => {
            plan_fix(image, drift, *option, note.as_deref(), now, rng)
        }
        Decision::Dismiss => plan_dismiss(image, facts, drift, now, rng),
        Decision::AcceptProposal => plan_accept(image, folders, numbers, facts, drift, now, rng),
    }
}

fn row(drift: &Card, decision: &str, option: Option<u32>, note: Option<&str>) -> ResolutionRow {
    ResolutionRow {
        drift: drift.id.clone(),
        decision: decision.to_owned(),
        option,
        note: note.map(str::to_owned),
    }
}

fn plan_fix(
    image: &ProjectImage,
    drift: &Card,
    option: Option<u32>,
    note: Option<&str>,
    now: &str,
    rng: &mut dyn RandomSource,
) -> Result<Decided, Refusal> {
    let options = drift_options(drift);
    match (options.len(), option) {
        (0, Some(_)) => return refuse("the Drift offers no fix option: leave `option` out"),
        (0, None) => {}
        (n, None) => {
            return refuse(format!(
                "an option is required: the Drift offers {n} (from 1 to {n})"
            ))
        }
        (n, Some(o)) if o < 1 || o as usize > n => {
            return refuse(format!("option {o} does not exist (from 1 to {n})"))
        }
        _ => {}
    }
    let mut used = image.used_ids(CardKind::Task);
    let planned = resolution::plan_decision(
        image,
        image,
        drift,
        &row(drift, "fix-code", option, note),
        now,
        rng,
        &mut used,
    )
    .map_err(|why| Refusal(vec![why]))?;
    Ok(Decided {
        changes: planned.changes,
        status: "code-to-be-fixed",
        fix_task: planned.fix_tasks.into_iter().next(),
        ..Decided::default()
    })
}

fn plan_dismiss(
    image: &ProjectImage,
    facts: &AgentFacts,
    drift: &Card,
    now: &str,
    rng: &mut dyn RandomSource,
) -> Result<Decided, Refusal> {
    let reasons = reasons::dismiss_reasons(image, facts, drift);
    if !reasons.is_empty() {
        return Err(Refusal(reasons));
    }
    let mut used = image.used_ids(CardKind::Task);
    let planned = resolution::plan_decision(
        image,
        image,
        drift,
        &row(drift, "dismissed", None, None),
        now,
        rng,
        &mut used,
    )
    .map_err(|why| Refusal(vec![why]))?;
    Ok(Decided {
        changes: planned.changes,
        status: "dismissed",
        ..Decided::default()
    })
}

fn plan_accept(
    image: &ProjectImage,
    folders: &Folders,
    numbers: &ShortNumbers,
    facts: &AgentFacts,
    drift: &Card,
    now: &str,
    rng: &mut dyn RandomSource,
) -> Result<Decided, Refusal> {
    let Some(proposal) = drift
        .str_key("proposal")
        .and_then(|id| image.card(CardKind::Proposal, id))
    else {
        return refuse("the Drift has no attached Proposal");
    };
    if !matches!(
        data::status_of(proposal),
        Some(Status::Draft | Status::Open)
    ) {
        return refuse("its attached Proposal is no longer in Draft or Open");
    }
    let gate = transitions::gate_of(facts, proposal);
    let approval = transitions::plan_approve(
        image,
        folders,
        numbers,
        &proposal.id,
        gate,
        now,
        rng,
        Decider::Drift,
    )?;
    let Approval::Approve(plan) = approval else {
        return refuse("its attached Proposal cannot be approved now");
    };
    let mut changes = ChangeSet::new();
    merge(&mut changes, &plan.changes);
    let mut front = drift.front.clone();
    set(&mut front, "status", "proposal-accepted");
    set(&mut front, "decided", now);
    changes.write(
        &drift.path,
        frontmatter::render(&front, &DRIFT_KEYS, &drift.body),
    );
    Ok(Decided {
        changes,
        status: "proposal-accepted",
        requirements_changed: plan.requirements_changed,
        ..Decided::default()
    })
}

/// `Fix the code`, `Accept Proposal` and `Dismiss Drift` under the project lock: reads the project
/// and the Proposal folders again, plans, writes the whole change or nothing. Returns the image
/// after it and what the decision did.
pub fn decide(
    project_root: &Path,
    facts: &AgentFacts,
    id: &str,
    decision: &Decision,
    now: &str,
    rng: &mut dyn RandomSource,
) -> Result<(ProjectImage, Decided), MutationError<Refused>> {
    let (image, mut decided) = changes::mutate(
        project_root,
        |image| {
            let folders = files::read_folders(project_root, image);
            let numbers = shortnum::peek(project_root, image.cards());
            let name = image
                .card(CardKind::Drift, id)
                .map_or_else(|| id.to_owned(), |d| name_of(&numbers, d));
            let decided = plan_decision(image, &folders, &numbers, facts, id, decision, now, rng)
                .map_err(|refusal| Refused {
                notice: format!(
                    "Could not {} {name}: {}.",
                    decision.verb(),
                    refusal.0.join("; ")
                ),
            })?;
            Ok((decided.changes.clone(), decided))
        },
        |_| Ok(()),
    )?;
    if decided.requirements_changed {
        decided.skills = transitions::regenerate_skills(project_root, &image, true);
    }
    Ok((image, decided))
}
