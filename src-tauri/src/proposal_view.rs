//! What the card of a Proposal and its details dialog show, worked out in the core and sent whole:
//! the badges, the files and Tasks lines, the required Proposals and the reasons each control is
//! disabled for, in the fixed order of the control (`requirements/ui#proposal`). The UI never
//! decides anything: it shows these.

use serde::Serialize;
use ts_rs::TS;
use weaver_core::project::board::{AgentFacts, REASON_BEING_REFINED};
use weaver_core::project::cards::{Card, CardKind};
use weaver_core::project::image::ProjectImage;
use weaver_core::project::proposal::analysis::{self, Analysis, Gate, Reason, RequiredProblem};
use weaver_core::project::proposal::data::{self, Kind, Status};
use weaver_core::project::proposal::files::Folders;
use weaver_core::project::proposal::models::ProposalModels;
use weaver_core::project::proposal::transitions::gate_of;
use weaver_core::project::shortnum::ShortNumbers;

/// The tooltip of `Refine` on a stale Open Proposal.
pub const TOOLTIP_REFINE: &str = "Refine on the current Model, then approve";
/// The tooltip of `Refine` when the hand-over does not approve: the Proposal is attached to a
/// Drift, or a required Proposal is still being written or not approved yet.
pub const TOOLTIP_REFINE_ONLY: &str = "Refine on the current Model";
/// The tooltip of `Refine` on a Draft a refinement left half-way.
pub const TOOLTIP_CONTINUE: &str = "Continue refining, then approve";
/// The tooltip of `Refine` on a Draft left half-way when the hand-over does not approve.
pub const TOOLTIP_CONTINUE_ONLY: &str = "Continue refining";

/// A Task a Proposal creates: its title and its body, for the compact row and the nested Dialog
/// of the details dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProposalTask {
    /// Stable among the Tasks of the Proposal, so that Tasks with the same title stay apart: the
    /// identity of a Task it carries, `removal:<ref>` for a code-removal Task.
    pub id: String,
    pub title: String,
    /// The Markdown after the title; empty for a code-removal Task.
    pub body: String,
}

/// A Proposal named in `requires`, with its state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RequiredProposal {
    pub id: String,
    /// `P2`, when the Proposal is live and has a number.
    pub short_number: Option<String>,
    pub title: String,
    /// `Approved`, `Open`, `Draft`, `Refining`, `Rejected`, `Discarded` or `Missing`.
    pub state: String,
    /// What is wrong with it for the approval (`is still being written`, `forms a cycle`...).
    pub problem: Option<String>,
}

/// The paths a Proposal adds, changes and removes, relative to `.weaver/`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProposalFiles {
    pub added: Vec<String>,
    pub changed: Vec<String>,
    pub removed: Vec<String>,
}

/// A button: why it is disabled, the tooltip it has while enabled and whether a click starts a
/// refinement job (`Refine`, solid yellow) instead of approving.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProposalControl {
    pub reasons: Vec<String>,
    pub tooltip: Option<String>,
    /// A click starts a refinement job: the control is `Refine`, not `Approve`. Decided here, never
    /// guessed from the tooltip.
    pub refines: bool,
}

/// The rest of the card of a Proposal, beyond the head every card has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProposalView {
    pub label: Option<String>,
    /// A file it changes is no longer the one it started from: the `stale` badge.
    pub stale: bool,
    /// Every reason it is blocked, one per line, for the popover of the `blocked` badge; empty
    /// when nothing blocks it.
    pub blocked: Vec<String>,
    /// The model the developer chose for the jobs born from it; never in the repository.
    pub model: Option<String>,
    pub files: ProposalFiles,
    /// It carries Tasks (an ordinary change or a setup): the Tasks line, the label badge and the
    /// model control exist.
    pub carries_tasks: bool,
    /// The Tasks it creates when approved, code-removal Tasks included.
    pub creates: u32,
    pub tasks: Vec<ProposalTask>,
    /// The code-removal Tasks Weaver adds on approval.
    pub removal_tasks: Vec<ProposalTask>,
    pub requires: Vec<RequiredProposal>,
    pub approve: ProposalControl,
    /// Reject, Rework with feedback… and Discard: the agent still finishing, or being refined.
    pub reject_reasons: Vec<String>,
    /// It is not itself a rework: `Rework with feedback…` exists.
    pub can_rework: bool,
    /// The references of its scope, for `View requirements`.
    pub scope_refs: Vec<String>,
}

fn state_word(card: Option<&Card>) -> String {
    match card.and_then(data::status_of) {
        Some(Status::Approved) => "Approved",
        Some(Status::Open) => "Open",
        Some(Status::Draft) => "Draft",
        Some(Status::Refining) => "Refining",
        Some(Status::Rejected) => "Rejected",
        Some(Status::Discarded) => "Discarded",
        None => "Missing",
    }
    .to_owned()
}

fn required_rows(
    image: &ProjectImage,
    numbers: &ShortNumbers,
    analysis: &Analysis,
) -> Vec<RequiredProposal> {
    analysis
        .requires
        .iter()
        .map(|required| {
            let card = image.card(CardKind::Proposal, &required.id);
            RequiredProposal {
                id: required.id.clone(),
                short_number: numbers.label_of(CardKind::Proposal, &required.id),
                title: card.map(|c| c.title.clone()).unwrap_or_default(),
                state: state_word(card),
                problem: required
                    .problem
                    .map(|p: RequiredProblem| p.text().to_owned()),
            }
        })
        .collect()
}

/// The approve control and the reasons of the others, for the Proposal `card` in its standing.
fn controls(
    analysis: &Analysis,
    numbers: &ShortNumbers,
    card: &Card,
    gate: Gate,
) -> (ProposalControl, Vec<String>) {
    let finishing = || {
        gate.agent_finishing
            .then(|| Reason::AgentFinishing.text(numbers))
    };
    let status = data::status_of(card);
    let mut reject: Vec<String> = finishing().into_iter().collect();
    if status == Some(Status::Refining) {
        reject.push(REASON_BEING_REFINED.to_owned());
    }
    let refinement = || -> Vec<String> {
        analysis
            .refinement_refusals(gate)
            .into_iter()
            .map(|r| r.text().to_owned())
            .collect()
    };
    // A hand-over that cannot approve: attached to a Drift (the developer accepts it there), or a
    // required Proposal still to be written or approved.
    let hands_back_open = analysis.attached
        || analysis.requires.iter().any(|r| {
            matches!(
                r.problem,
                Some(RequiredProblem::BeingWritten | RequiredProblem::NotApproved)
            )
        });
    let approve = match status {
        Some(Status::Draft) => ProposalControl {
            reasons: refinement(),
            tooltip: Some(
                if hands_back_open {
                    TOOLTIP_CONTINUE_ONLY
                } else {
                    TOOLTIP_CONTINUE
                }
                .to_owned(),
            ),
            refines: true,
        },
        Some(Status::Refining) => {
            let mut reasons: Vec<String> = finishing().into_iter().collect();
            if gate.run_in_progress {
                reasons.push(Reason::RunInProgress.text(numbers));
            }
            reasons.push(REASON_BEING_REFINED.to_owned());
            reasons.extend(analysis.blocking().iter().map(|r| r.text(numbers)));
            ProposalControl {
                reasons,
                tooltip: None,
                refines: false,
            }
        }
        // Any stale Open Proposal is refined, whatever its kind and whatever else blocks it: only
        // the agent finishing and a resolution without an open Drift disable it.
        _ if !analysis.stale.is_empty() => ProposalControl {
            reasons: refinement(),
            tooltip: Some(
                if hands_back_open {
                    TOOLTIP_REFINE_ONLY
                } else {
                    TOOLTIP_REFINE
                }
                .to_owned(),
            ),
            refines: true,
        },
        _ => ProposalControl {
            reasons: analysis
                .approve_reasons(gate)
                .iter()
                .map(|r| r.text(numbers))
                .collect(),
            tooltip: None,
            refines: false,
        },
    };
    (approve, reject)
}

/// The view of one Proposal of the Board; `None` when the image does not hold it.
pub fn view(
    image: &ProjectImage,
    folders: &Folders,
    numbers: &ShortNumbers,
    facts: &AgentFacts,
    models: &ProposalModels,
    id: &str,
) -> Option<ProposalView> {
    let card = image.card(CardKind::Proposal, id)?;
    let analysis = analysis::analyse(image, folders, numbers, id)?;
    let gate = gate_of(facts, card);
    let (approve, reject_reasons) = controls(&analysis, numbers, card, gate);
    let carries_tasks = analysis.kind.carries_tasks() && analysis.kind != Kind::Resolution;
    let tasks: Vec<ProposalTask> = data::tasks_of(image, id)
        .into_iter()
        .map(|t| ProposalTask {
            id: t.id.clone(),
            title: t.title.clone(),
            body: crate::board::body_after_title(t),
        })
        .collect();
    let removal_tasks: Vec<ProposalTask> = analysis
        .removals
        .iter()
        .map(|r| ProposalTask {
            id: format!("removal:{}", r.removed),
            title: r.title(),
            body: String::new(),
        })
        .collect();
    let blocked: Vec<String> = analysis
        .blocking()
        .iter()
        .map(|r| r.text(numbers))
        .collect();
    Some(ProposalView {
        label: data::label_of(card).map(str::to_owned),
        stale: !analysis.stale.is_empty(),
        blocked,
        model: models.get(id).map(str::to_owned),
        files: ProposalFiles {
            added: analysis.files.added.clone(),
            changed: analysis.files.changed.clone(),
            removed: analysis.files.removed.clone(),
        },
        carries_tasks,
        creates: (tasks.len() + removal_tasks.len()) as u32,
        tasks,
        removal_tasks,
        requires: required_rows(image, numbers, &analysis),
        approve,
        reject_reasons,
        can_rework: data::reworks_of(card).is_none(),
        scope_refs: analysis::scope_refs(image, folders, id),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use weaver_core::project::shortnum;

    use super::*;

    const ORDER: &str = "# Order\n\nAn order.\n\n## States\n\n### Draft (initial)\nChoosing.\n- `CONFIRM` → Paid\n\n### Paid\n\n## Events\n\n### CONFIRM\nConfirmed.\n";

    fn put(root: &Path, rel: &str, text: &str) {
        let path = root.join(".weaver").join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn hash(text: &str) -> String {
        weaver_core::project::text::sha256_hex(&weaver_core::project::text::file_content(text))
    }

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), "model/system.md", "# Shop\n\nA shop.\n");
        put(dir.path(), "model/shop/feature.md", "# Shopping\n");
        put(dir.path(), "model/shop/order.md", ORDER);
        dir
    }

    fn proposal(root: &Path, id: &str, front: &str, base: &str, tasks: bool) {
        put(
            root,
            &format!("proposals/{id}-t/proposal.md"),
            &format!(
                "---\n{front}label: Shop\ncreated: 2026-11-02T07:00:00.000Z\n---\n# Pay\n\nWhy.\n"
            ),
        );
        put(
            root,
            &format!("proposals/{id}-t/model/shop/order.md"),
            &ORDER.replace("### Paid\n", "### Paid\nPaid.\n"),
        );
        let front =
            fs::read_to_string(root.join(format!(".weaver/proposals/{id}-t/proposal.md"))).unwrap();
        put(
            root,
            &format!("proposals/{id}-t/proposal.md"),
            &front.replacen(
                "---\n",
                &format!("---\nbase:\n  model/shop/order.md: {base}\n"),
                1,
            ),
        );
        if tasks {
            put(
                root,
                &format!("proposals/{id}-t/tasks/tttttttt-pay.md"),
                "---\ntechnical: false\nrefs:\n  - order\n---\n# Pay\n\nDo it.\n",
            );
        }
    }

    fn view_of(root: &Path, facts: &AgentFacts, models: &ProposalModels) -> ProposalView {
        let image = ProjectImage::load(root);
        let folders = weaver_core::project::proposal::files::read_folders(root, &image);
        let numbers = shortnum::peek(root, image.cards());
        view(&image, &folders, &numbers, facts, models, "pppppppp").unwrap()
    }

    #[test]
    fn an_open_change_shows_its_files_tasks_label_and_chosen_model() {
        let dir = project();
        proposal(
            dir.path(),
            "pppppppp",
            "status: open\nkind: change\n",
            &hash(ORDER),
            true,
        );
        let mut models = ProposalModels::default();
        models.set("pppppppp", Some("anthropic/claude-opus"));
        let view = view_of(dir.path(), &AgentFacts::default(), &models);
        assert_eq!(view.label.as_deref(), Some("Shop"));
        assert_eq!(view.files.changed, ["model/shop/order.md"]);
        assert!(view.carries_tasks && !view.stale && view.blocked.is_empty());
        assert_eq!(
            (view.creates, view.tasks.len(), view.removal_tasks.len()),
            (1, 1, 0)
        );
        assert_eq!(
            view.tasks,
            [ProposalTask {
                id: "tttttttt".to_owned(),
                title: "Pay".to_owned(),
                body: "Do it.\n".to_owned(),
            }]
        );
        assert_eq!(view.model.as_deref(), Some("anthropic/claude-opus"));
        assert_eq!(view.approve, ProposalControl::default());
        assert!(view.reject_reasons.is_empty() && view.can_rework);
        assert_eq!(view.scope_refs, ["order/Paid", "order/Draft/order.CONFIRM"]);
    }

    #[test]
    fn approve_lists_every_reason_in_the_fixed_order_and_reject_only_the_agent_and_the_refinement()
    {
        let dir = project();
        proposal(
            dir.path(),
            "pppppppp",
            "status: open\nkind: change\nrequires:\n  - zzzzzzzz\n",
            &hash(ORDER),
            true,
        );
        let mut facts = AgentFacts::default();
        facts.working_jobs.insert("jjjjjjjj".into());
        facts.run_in_progress = true;
        let text = fs::read_to_string(dir.path().join(".weaver/proposals/pppppppp-t/proposal.md"))
            .unwrap();
        put(
            dir.path(),
            "proposals/pppppppp-t/proposal.md",
            &text.replacen("---\n", "---\njob: jjjjjjjj\n", 1),
        );
        let view = view_of(dir.path(), &facts, &ProposalModels::default());
        assert_eq!(
            view.approve.reasons,
            [
                "The agent is still finishing",
                "A run is in progress",
                "Proposal zzzzzzzz is missing"
            ]
        );
        assert_eq!(view.blocked, ["Proposal zzzzzzzz is missing"]);
        assert_eq!(view.reject_reasons, ["The agent is still finishing"]);
        assert_eq!(view.requires[0].state, "Missing");
        assert_eq!(view.requires[0].problem.as_deref(), Some("is missing"));
    }

    fn drift_file(status: &str) -> String {
        format!(
            "---\nstatus: {status}\nrefs:\n  - order/Paid\ncreated: 2026-11-02T06:00:00.000Z\n---\n# The code ships twice\n\n## What happens\n\nIt ships twice.\n\n## What should happen\n\nIt ships once.\n\n## Where in the code\n\n`src/ship.rs`\n"
        )
    }

    /// A Drift the Proposal `pppppppp` is attached to.
    fn attached_drift_file(status: &str) -> String {
        drift_file(status).replacen("created:", "proposal: pppppppp\ncreated:", 1)
    }

    #[test]
    fn a_stale_proposal_is_refined_whatever_else_blocks_it_also_during_a_run() {
        let dir = project();
        proposal(
            dir.path(),
            "pppppppp",
            "status: open\nkind: change\n",
            &hash("an older Model"),
            true,
        );
        let facts = AgentFacts {
            run_in_progress: true,
            ..AgentFacts::default()
        };
        let view = view_of(dir.path(), &facts, &ProposalModels::default());
        assert!(view.stale);
        assert_eq!(view.blocked.len(), 1);
        assert!(
            view.blocked[0].starts_with("Stale: `model/shop/order.md`"),
            "{:?}",
            view.blocked
        );
        assert_eq!(
            view.approve,
            ProposalControl {
                reasons: Vec::new(),
                tooltip: Some(TOOLTIP_REFINE.to_owned()),
                refines: true,
            }
        );

        // A missing required Proposal blocks it too: Refine stays, the reason stays in `blocked`.
        proposal(
            dir.path(),
            "pppppppp",
            "status: open\nkind: change\nrequires:\n  - zzzzzzzz\n",
            &hash("an older Model"),
            true,
        );
        let view = view_of(dir.path(), &facts, &ProposalModels::default());
        assert_eq!(view.blocked.len(), 2, "{:?}", view.blocked);
        assert_eq!(view.approve.reasons, Vec::<String>::new());
        assert!(view.approve.refines);
        assert_eq!(view.approve.tooltip.as_deref(), Some(TOOLTIP_REFINE));

        // A required Proposal not approved yet: the hand-over cannot approve, the tooltip says so.
        proposal(
            dir.path(),
            "qqqqqqqq",
            "status: open\nkind: change\n",
            &hash(ORDER),
            false,
        );
        proposal(
            dir.path(),
            "pppppppp",
            "status: open\nkind: change\nrequires:\n  - qqqqqqqq\n",
            &hash("an older Model"),
            true,
        );
        let view = view_of(dir.path(), &facts, &ProposalModels::default());
        assert!(
            view.blocked
                .iter()
                .any(|b| b.ends_with("is not approved yet")),
            "{:?}",
            view.blocked
        );
        assert!(view.approve.reasons.is_empty() && view.approve.refines);
        assert_eq!(view.approve.tooltip.as_deref(), Some(TOOLTIP_REFINE_ONLY));

        // Attached to a Drift: the same.
        proposal(
            dir.path(),
            "pppppppp",
            "status: open\nkind: change\n",
            &hash("an older Model"),
            true,
        );
        put(
            dir.path(),
            "drifts/dddddddd-d.md",
            &attached_drift_file("open"),
        );
        let view = view_of(dir.path(), &facts, &ProposalModels::default());
        assert!(view.approve.reasons.is_empty() && view.approve.refines);
        assert_eq!(view.approve.tooltip.as_deref(), Some(TOOLTIP_REFINE_ONLY));
        fs::remove_file(dir.path().join(".weaver/drifts/dddddddd-d.md")).unwrap();

        // Only the agent still finishing disables it.
        let text = fs::read_to_string(dir.path().join(".weaver/proposals/pppppppp-t/proposal.md"))
            .unwrap();
        put(
            dir.path(),
            "proposals/pppppppp-t/proposal.md",
            &text.replacen("---\n", "---\njob: jjjjjjjj\n", 1),
        );
        let mut finishing = facts.clone();
        finishing.working_jobs.insert("jjjjjjjj".into());
        let view = view_of(dir.path(), &finishing, &ProposalModels::default());
        assert_eq!(view.approve.reasons, ["The agent is still finishing"]);
        assert!(view.approve.refines);
    }

    #[test]
    fn a_stale_resolution_is_refined_only_while_one_of_its_drifts_is_still_open() {
        let dir = project();
        let drifts = "drifts:\n  - drift: dddddddd\n    decision: dismissed\n";
        proposal(
            dir.path(),
            "pppppppp",
            &format!("status: open\nkind: resolution\n{drifts}"),
            &hash("an older Model"),
            false,
        );
        put(dir.path(), "drifts/dddddddd-d.md", &drift_file("open"));
        let view = view_of(
            dir.path(),
            &AgentFacts::default(),
            &ProposalModels::default(),
        );
        assert!(view.stale && view.approve.refines);
        assert!(view.approve.reasons.is_empty());

        put(dir.path(), "drifts/dddddddd-d.md", &drift_file("dismissed"));
        let view = view_of(
            dir.path(),
            &AgentFacts::default(),
            &ProposalModels::default(),
        );
        assert!(view.approve.refines);
        assert_eq!(view.approve.reasons, ["None of its Drifts is still open"]);

        // A resolution with no file has no `base`: never stale, never refined.
        put(
            dir.path(),
            "proposals/pppppppp-t/proposal.md",
            &format!("---\nstatus: open\nkind: resolution\n{drifts}created: 2026-11-02T07:00:00.000Z\n---\n# R\n"),
        );
        fs::remove_file(
            dir.path()
                .join(".weaver/proposals/pppppppp-t/model/shop/order.md"),
        )
        .unwrap();
        put(dir.path(), "drifts/dddddddd-d.md", &drift_file("open"));
        let view = view_of(
            dir.path(),
            &AgentFacts::default(),
            &ProposalModels::default(),
        );
        assert!(!view.stale && !view.approve.refines);
    }

    #[test]
    fn a_refined_proposal_says_being_refined_and_an_interrupted_draft_can_continue() {
        let dir = project();
        proposal(
            dir.path(),
            "pppppppp",
            "status: refining\nrefining: true\nkind: change\n",
            &hash(ORDER),
            true,
        );
        let view = view_of(
            dir.path(),
            &AgentFacts::default(),
            &ProposalModels::default(),
        );
        assert_eq!(view.approve.reasons, ["Being refined"]);
        assert_eq!(view.reject_reasons, ["Being refined"]);

        proposal(
            dir.path(),
            "pppppppp",
            "status: draft\nrefining: true\nkind: change\n",
            &hash(ORDER),
            true,
        );
        let view = view_of(
            dir.path(),
            &AgentFacts::default(),
            &ProposalModels::default(),
        );
        assert!(view.approve.reasons.is_empty());
        assert_eq!(view.approve.tooltip.as_deref(), Some(TOOLTIP_CONTINUE));
        // The agent that handed it over is still finishing: disabled by that alone.
        let mut finishing = AgentFacts::default();
        finishing.working_jobs.insert("jjjjjjjj".into());
        let text = fs::read_to_string(dir.path().join(".weaver/proposals/pppppppp-t/proposal.md"))
            .unwrap();
        put(
            dir.path(),
            "proposals/pppppppp-t/proposal.md",
            &text.replacen("---\n", "---\njob: jjjjjjjj\n", 1),
        );
        let view = view_of(dir.path(), &finishing, &ProposalModels::default());
        assert_eq!(view.approve.reasons, ["The agent is still finishing"]);
        assert!(view.approve.refines);
        // Attached to a Drift: the hand-over never approves.
        put(
            dir.path(),
            "drifts/dddddddd-d.md",
            &attached_drift_file("open"),
        );
        let view = view_of(
            dir.path(),
            &AgentFacts::default(),
            &ProposalModels::default(),
        );
        assert_eq!(view.approve.tooltip.as_deref(), Some(TOOLTIP_CONTINUE_ONLY));
        assert!(view.approve.refines);
    }

    #[test]
    fn a_rework_cannot_be_reworked_again_and_an_import_carries_no_tasks() {
        let dir = project();
        proposal(
            dir.path(),
            "pppppppp",
            "status: open\nreworks: oooooooo\nkind: import\n",
            &hash(ORDER),
            false,
        );
        let view = view_of(
            dir.path(),
            &AgentFacts::default(),
            &ProposalModels::default(),
        );
        assert!(
            !view.can_rework
                && !view.carries_tasks
                && view.creates == 0
                && view.scope_refs.is_empty()
        );
    }
}
