//! The decisions of the developer on an Open Drift: `developer.FIX_DRIFT_CODE`,
//! `developer.ACCEPT_DRIFT_PROPOSAL`, `developer.DISMISS_DRIFT` and `developer.REFINE_DRIFT_PROPOSAL`
//! (which decides nothing).
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
//! * Dismiss → `Dismissed`, refused while the Drift is in a resolution being written, refined or
//!   waiting for a decision, or while its attached Proposal is being refined; the attached Proposal
//!   is discarded; the text fingerprint is recorded.
//! * Refine the Proposal → the Drift stays `Open`: only the stale attached Proposal (Open), or one
//!   a refinement left half-way (Draft), becomes `Refining`, as `Approve` on a Proposal does
//!   ([`transitions::plan_approve`] with [`Decider::Drift`]); the caller starts the refinement job
//!   ([`Decided::refine`]). The Drift is not written.
//!
//! * Discard the Proposal → the Drift stays `Open`: only an attached Draft a refinement left
//!   half-way is discarded, through [`transitions::plan_discard`] with [`Decider::Drift`]. The
//!   Drift is not written.
//!
//! Fixing the code, accepting and dismissing are refused while the attached Proposal is Refining.

use std::path::Path;

use crate::project::board::{
    AgentFacts, Refused, ResolutionRow, REASON_AGENT_FINISHING, REASON_BEING_REFINED,
};
use crate::project::cards::{Card, CardKind};
use crate::project::changes::{self, Change, ChangeSet, MutationError};
use crate::project::frontmatter;
use crate::project::image::ProjectImage;
use crate::project::proposal::analysis;
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
    /// `developer.REFINE_DRIFT_PROPOSAL`: adapts the attached Proposal and decides nothing.
    RefineProposal,
    /// Discards the attached Draft a refinement left half-way; the Drift stays open.
    DiscardProposal,
}

impl Decision {
    /// The verb of a notice: `Could not <verb> D3: …`.
    pub fn verb(&self) -> &'static str {
        match self {
            Decision::FixCode { .. } => "fix the code of",
            Decision::AcceptProposal => "accept the Proposal of",
            Decision::Dismiss => "dismiss",
            Decision::RefineProposal => "refine the Proposal of",
            Decision::DiscardProposal => "discard the Proposal of",
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
    /// The attached Proposal that is now Refining: the caller starts a refinement job on it.
    pub refine: Option<String>,
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
    if matches!(
        decision,
        Decision::FixCode { .. } | Decision::AcceptProposal
    ) && reasons::attached_being_refined(image, drift)
    {
        return refuse(REASON_BEING_REFINED);
    }
    match decision {
        Decision::FixCode { option, note } => {
            plan_fix(image, drift, *option, note.as_deref(), now, rng)
        }
        Decision::Dismiss => plan_dismiss(image, facts, drift, now, rng),
        Decision::AcceptProposal => plan_accept(image, folders, numbers, facts, drift, now, rng),
        Decision::RefineProposal => plan_refine(image, folders, numbers, facts, drift, now, rng),
        Decision::DiscardProposal => plan_discard_attached(image, numbers, facts, drift),
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
    match data::status_of(proposal) {
        Some(Status::Refining) => return refuse(REASON_BEING_REFINED),
        Some(Status::Draft | Status::Open) => {}
        _ => return refuse("its attached Proposal is no longer in Draft or Open"),
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

/// `Discard` on the attached Draft a refinement left half-way, decided by the Drift
/// (`plan_discard` with [`Decider::Drift`]): the Proposal is discarded, the Drift stays open and
/// is not written.
fn plan_discard_attached(
    image: &ProjectImage,
    numbers: &ShortNumbers,
    facts: &AgentFacts,
    drift: &Card,
) -> Result<Decided, Refusal> {
    let Some(proposal) = drift
        .str_key("proposal")
        .and_then(|id| image.card(CardKind::Proposal, id))
    else {
        return refuse("the Drift has no attached Proposal");
    };
    match data::status_of(proposal) {
        Some(Status::Refining) => return refuse(REASON_BEING_REFINED),
        Some(Status::Draft) if data::is_interrupted(proposal) => {}
        _ => return refuse("its attached Proposal is not a Draft left half-way by a refinement"),
    }
    let changes = transitions::plan_discard(
        image,
        numbers,
        &proposal.id,
        transitions::gate_of(facts, proposal),
        Decider::Drift,
    )?;
    Ok(Decided {
        changes,
        status: "open",
        ..Decided::default()
    })
}

/// `Refine` on the attached Proposal: it becomes Refining (a stale Open one, or a Draft a
/// refinement left half-way) and a refinement job adapts it. The Drift is not written.
fn plan_refine(
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
    match data::status_of(proposal) {
        Some(Status::Refining) => return refuse(REASON_BEING_REFINED),
        Some(Status::Open) => {
            let stale = analysis::analyse(image, folders, numbers, &proposal.id)
                .is_some_and(|analysed| !analysed.stale.is_empty());
            if !stale {
                return refuse("its attached Proposal is not stale");
            }
        }
        Some(Status::Draft) if data::is_interrupted(proposal) => {}
        Some(Status::Draft) => {
            return refuse("its attached Proposal is still being written by the agent")
        }
        _ => return refuse("its attached Proposal is no longer in Draft or Open"),
    }
    let approval = transitions::plan_approve(
        image,
        folders,
        numbers,
        &proposal.id,
        transitions::gate_of(facts, proposal),
        now,
        rng,
        Decider::Drift,
    )?;
    match approval {
        Approval::Refine(changes) | Approval::Continue(changes) => Ok(Decided {
            changes,
            status: "open",
            refine: Some(proposal.id.clone()),
            ..Decided::default()
        }),
        Approval::Approve(_) => refuse("its attached Proposal needs no refinement"),
    }
}

/// `Fix the code`, `Accept Proposal`, `Dismiss Drift` and `Refine` under the project lock: reads
/// the project and the Proposal folders again, plans, writes the whole change or nothing. Returns
/// the image after it and what the decision did.
pub fn decide(
    project_root: &Path,
    facts: &AgentFacts,
    id: &str,
    decision: &Decision,
    now: &str,
    rng: &mut dyn RandomSource,
) -> Result<(ProjectImage, Decided), MutationError<Refused>> {
    decide_with(project_root, facts, id, decision, now, rng, &|| Ok(()))
}

/// [`decide`] for the app: a `Refine` first asks `jobs` whether a refinement job can start, and is
/// refused, writing nothing, when it cannot.
pub fn decide_with(
    project_root: &Path,
    facts: &AgentFacts,
    id: &str,
    decision: &Decision,
    now: &str,
    rng: &mut dyn RandomSource,
    jobs: transitions::JobCheck,
) -> Result<(ProjectImage, Decided), MutationError<Refused>> {
    let (image, mut decided) = changes::mutate(
        project_root,
        |image| {
            let folders = files::read_folders(project_root, image);
            let numbers = shortnum::peek(project_root, image.cards());
            let name = image
                .card(CardKind::Drift, id)
                .map_or_else(|| id.to_owned(), |d| name_of(&numbers, d));
            let refused = |reasons: Vec<String>| Refused {
                notice: format!(
                    "Could not {} {name}: {}.",
                    decision.verb(),
                    reasons.join("; ")
                ),
            };
            let decided = plan_decision(image, &folders, &numbers, facts, id, decision, now, rng)
                .map_err(|refusal| refused(refusal.0))?;
            if decided.refine.is_some() {
                jobs().map_err(|why| refused(vec![why]))?;
            }
            Ok((decided.changes.clone(), decided))
        },
        |_| Ok(()),
    )?;
    if decided.requirements_changed {
        decided.skills = transitions::regenerate_skills(project_root, &image, true);
    }
    Ok((image, decided))
}
