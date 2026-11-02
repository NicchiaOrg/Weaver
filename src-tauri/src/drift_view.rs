//! What the card of a Drift and its details dialog show, worked out in the core and sent whole:
//! the two sides, the fix options, the files, the earlier Drift it repeats, the attached Proposal
//! and the reasons each control is disabled for, in the fixed order of the control
//! (`requirements/ui#drift`). The UI never decides anything: it shows these.

use serde::Serialize;
use ts_rs::TS;
use weaver_core::project::board::AgentFacts;
use weaver_core::project::cards::{Card, CardKind};
use weaver_core::project::drift::places::places;
use weaver_core::project::drift::{self, prepared, reasons};
use weaver_core::project::image::ProjectImage;
use weaver_core::project::proposal::analysis;
use weaver_core::project::proposal::data::{self, Status};
use weaver_core::project::proposal::files::Folders;
use weaver_core::project::proposal::transitions::gate_of;
use weaver_core::project::shortnum::ShortNumbers;
use weaver_core::project::tasks;

use crate::board::TitledRef;
use crate::proposal_view::ProposalFiles;

/// The reason `Accept Proposal` lists while the attached Proposal is still a Draft.
pub const REASON_PROPOSAL_IS_DRAFT: &str = "The attached Proposal is still a Draft";

/// One way to fix the code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FixOption {
    pub title: String,
    /// Markdown.
    pub text: String,
}

/// A file named under `Where in the code`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CodePlace {
    /// Relative to the project, or absolute.
    pub file: String,
    /// 1-based, when the text gives one.
    pub line: Option<u32>,
}

/// The Proposal attached to a Drift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AttachedProposal {
    pub id: String,
    /// `P3`, when the Proposal is live and has a number.
    pub short_number: Option<String>,
    pub title: String,
    /// `Draft` or `Open`.
    pub state: String,
    /// The Markdown of the Proposal after its title.
    pub body: String,
    pub files: ProposalFiles,
    /// Why `Accept Proposal` is disabled besides the agent still finishing the Drift, one per line
    /// in the fixed order; empty when the Proposal could be approved.
    pub accept_reasons: Vec<String>,
}

/// The rest of the card of a Drift, beyond the head every card has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DriftView {
    /// What it concerns: Model elements, Requirement sections and Specs fields.
    pub refs: Vec<String>,
    /// Markdown.
    pub happens: String,
    /// Markdown.
    pub should: String,
    /// The fix options, the recommended first; none when no change of the code can respect both
    /// the Model and the Requirements.
    pub options: Vec<FixOption>,
    /// The text of `Where in the code`, Markdown.
    pub where_in_code: String,
    /// The files that text names, one row each.
    pub places: Vec<CodePlace>,
    /// The Drift it repeats (`recurring`).
    pub previous: Option<TitledRef>,
    /// A resolve job works on it or a resolution Draft lists it (`resolving`).
    pub resolving: bool,
    /// A valid prepared question: `Resolve` asks it at once (`prepared`).
    pub prepared: bool,
    /// Why `Resolve` is disabled, one per line in the fixed order; empty = it works.
    pub resolve_reasons: Vec<String>,
    /// Why `Dismiss Drift` is disabled, one per line in the fixed order; empty = it works.
    pub dismiss_reasons: Vec<String>,
    pub attached: Option<AttachedProposal>,
}

fn name_of(numbers: &ShortNumbers) -> impl Fn(CardKind, &str) -> String + '_ {
    move |kind, id| numbers.label_of(kind, id).unwrap_or_else(|| id.to_owned())
}

fn titled(image: &ProjectImage, numbers: &ShortNumbers, kind: CardKind, id: &str) -> TitledRef {
    TitledRef {
        id: id.to_owned(),
        short_number: numbers.label_of(kind, id),
        title: image
            .card(kind, id)
            .map(|c| c.title.clone())
            .unwrap_or_default(),
    }
}

fn attached(
    image: &ProjectImage,
    folders: &Folders,
    numbers: &ShortNumbers,
    facts: &AgentFacts,
    drift: &Card,
) -> Option<AttachedProposal> {
    let proposal = drift
        .str_key("proposal")
        .and_then(|id| image.card(CardKind::Proposal, id))?;
    let status = data::status_of(proposal)?;
    if !matches!(status, Status::Draft | Status::Open) {
        return None;
    }
    let analysed = analysis::analyse(image, folders, numbers, &proposal.id)?;
    let mut accept_reasons: Vec<String> = analysed
        .approve_reasons(gate_of(facts, proposal))
        .iter()
        .map(|r| r.text(numbers))
        .collect();
    if status == Status::Draft {
        accept_reasons.push(REASON_PROPOSAL_IS_DRAFT.to_owned());
    }
    Some(AttachedProposal {
        id: proposal.id.clone(),
        short_number: numbers.label_of(CardKind::Proposal, &proposal.id),
        title: proposal.title.clone(),
        state: if status == Status::Draft {
            "Draft"
        } else {
            "Open"
        }
        .to_owned(),
        body: tasks::body_after_title(proposal),
        files: ProposalFiles {
            added: analysed.files.added.clone(),
            changed: analysed.files.changed.clone(),
            removed: analysed.files.removed.clone(),
        },
        accept_reasons,
    })
}

/// The view of one Drift of the Board.
pub fn view(
    image: &ProjectImage,
    folders: &Folders,
    numbers: &ShortNumbers,
    facts: &AgentFacts,
    drift: &Card,
) -> DriftView {
    let where_in_code = drift::section_text(&drift.body, drift::WHERE_IN_CODE);
    let name = name_of(numbers);
    DriftView {
        refs: drift::refs_of(drift),
        happens: drift::section_text(&drift.body, drift::HAPPENS),
        should: drift::section_text(&drift.body, drift::SHOULD),
        options: drift::drift_options(drift)
            .into_iter()
            .map(|o| FixOption {
                title: o.title,
                text: o.text,
            })
            .collect(),
        places: places(&where_in_code)
            .into_iter()
            .map(|p| CodePlace {
                file: p.file,
                line: p.line,
            })
            .collect(),
        where_in_code,
        previous: drift
            .str_key("previous")
            .map(|id| titled(image, numbers, CardKind::Drift, id)),
        resolving: reasons::is_resolving(image, facts, drift),
        prepared: prepared::check(image, drift).is_valid(),
        resolve_reasons: reasons::resolve_reasons(image, facts, &name, drift),
        dismiss_reasons: reasons::dismiss_reasons(image, facts, drift),
        attached: attached(image, folders, numbers, facts, drift),
    }
}

/// The Task that found a Drift, with its title.
pub fn origin_of(image: &ProjectImage, numbers: &ShortNumbers, drift: &Card) -> Option<TitledRef> {
    drift
        .str_key("origin")
        .map(|task| titled(image, numbers, CardKind::Task, task))
}
