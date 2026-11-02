//! What the answer to the prepared question of a Drift becomes (`requirements/rules#drift`).
//!
//! * A fix option is `fix-code` with the free text as note; `proposal` or `dismissed` without
//!   text is that decision. Weaver writes the resolution `Resolve: <Drift title>` itself, with
//!   the decision of the Drift, and hands it over with the agent's checks ([`plan_resolution`]).
//! * A free answer next to a choice other than a code fix, or no choice at all, goes to a resolve
//!   job that starts with the answer already given.
//! * A skipped question leaves the Drift as it is.
//!
//! A resolution Weaver writes carries no short number: every card in it is named by identity.

use crate::job_instruction::ResolveAnswer;
use crate::project::board::ResolutionRow;
use crate::project::cards::{Card, CardKind};
use crate::project::changes::{Change, ChangeSet};
use crate::project::image::ProjectImage;
use crate::project::proposal::analysis;
use crate::project::proposal::create::{self, DriftDecision, EditRequest, NewProposal};
use crate::project::proposal::data::Kind;
use crate::project::proposal::files::{self, Folder, Folders};
use crate::project::random::RandomSource;
use crate::project::shortnum::ShortNumbers;
use crate::question::Answer;

use super::prepared::{Prepared, DISMISSED, FIX_CODE, PROPOSAL};

/// What the answer asks Weaver to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// Nothing: the question was skipped, or no answer exists.
    Nothing,
    /// Write and hand over a resolution with this decision.
    Resolution(ResolutionRow),
    /// Start a resolve job with the answer already given.
    Job(ResolveAnswer),
}

/// What the `answer` to the prepared question of `drift` asks for.
pub fn next_of(drift: &str, prepared: &Prepared, answer: &Answer) -> Next {
    let Answer::Answered { selected, text } = answer else {
        return Next::Nothing;
    };
    let text = text.as_deref().map(str::trim).filter(|t| !t.is_empty());
    let given = || ResolveAnswer {
        selected: selected.clone(),
        text: text.map(str::to_owned),
    };
    let Some(label) = selected.first() else {
        // A free answer with no choice: the agent reads it.
        return if text.is_some() {
            Next::Job(given())
        } else {
            Next::Nothing
        };
    };
    let Some(index) = prepared
        .question
        .options
        .iter()
        .position(|option| &option.label == label)
    else {
        return Next::Job(given());
    };
    let Some(decision) = prepared.decisions.get(index) else {
        return Next::Job(given());
    };
    match (decision.decision.as_str(), text) {
        (FIX_CODE, note) => Next::Resolution(ResolutionRow {
            drift: drift.to_owned(),
            decision: FIX_CODE.to_owned(),
            option: decision.option,
            note: note.map(str::to_owned),
        }),
        (PROPOSAL | DISMISSED, None) => Next::Resolution(ResolutionRow {
            drift: drift.to_owned(),
            decision: decision.decision.clone(),
            option: None,
            note: None,
        }),
        _ => Next::Job(given()),
    }
}

/// A resolution Draft, ready to be handed over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The identity of the resolution Proposal.
    pub id: String,
    pub changes: ChangeSet,
}

/// The title of the resolution Weaver writes for a Drift.
pub fn resolution_title(drift: &Card) -> String {
    format!("Resolve: {}", drift.title)
}

fn decision_text(row: &ResolutionRow, drift: &Card) -> String {
    let options = super::drift_options(drift);
    match row.decision.as_str() {
        FIX_CODE => {
            let chosen = row
                .option
                .and_then(|o| options.get((o as usize).saturating_sub(1)));
            match chosen {
                Some(option) => format!("fix the code: {}", option.title),
                None => "fix the code".to_owned(),
            }
        }
        PROPOSAL => "change the Model and the Requirements as the code does".to_owned(),
        _ => "let it be: nothing changes".to_owned(),
    }
}

/// Plans the resolution Weaver writes from an answer: a Draft of kind `resolution` titled
/// `Resolve: <Drift title>`, listing the Drift with its decision. For `proposal` it carries the
/// files of the Proposal attached to the Drift. `read` gives the text of a file of the project on
/// disk. Refused, with every reason, when it cannot be written (the Proposal attached has gone
/// stale, for instance): the caller then starts a resolve job with the answer.
#[allow(clippy::too_many_arguments)]
pub fn plan_resolution(
    image: &ProjectImage,
    folders: &Folders,
    numbers: &ShortNumbers,
    drift: &Card,
    row: &ResolutionRow,
    created: &str,
    read: &dyn Fn(&str) -> Option<String>,
    rng: &mut dyn RandomSource,
) -> Result<Written, Vec<String>> {
    // The files a `proposal` decision carries: those of the attached Proposal.
    let mut carried = ChangeSet::new();
    if row.decision == PROPOSAL {
        let Some(attached) = drift
            .str_key("proposal")
            .and_then(|id| image.card(CardKind::Proposal, id))
        else {
            return Err(vec![
                "the Drift has no attached Proposal whose changes the resolution could carry"
                    .to_owned(),
            ]);
        };
        let analysed = analysis::analyse(image, folders, numbers, &attached.id)
            .ok_or_else(|| vec![format!("no Proposal has the identity {}", attached.id)])?;
        if !analysed.stale.is_empty() {
            return Err(vec!["the attached Proposal is stale: its changes must be rewritten on the current Model".to_owned()]);
        }
        let empty = Folder::default();
        carried = files::changes_of(attached, folders.get(&attached.id).unwrap_or(&empty));
    }

    let body = format!(
        "Resolves Drift `{}` ({}): {}.",
        drift.id,
        drift.title,
        decision_text(row, drift)
    );
    let new = NewProposal {
        title: resolution_title(drift),
        body: Some(body),
        kind: Some(Kind::Resolution),
        ..NewProposal::default()
    };
    let draft = create::plan_new(image, numbers, &new, created, rng)?;
    let with_draft = image.with_changes(&draft.changes);
    let mut changes = draft.changes.clone();

    let edit = EditRequest {
        edit: carried
            .iter()
            .filter(|(_, change)| matches!(change, Change::Write(_)))
            .map(|(path, _)| path.clone())
            .collect(),
        delete: carried
            .iter()
            .filter(|(_, change)| matches!(change, Change::Delete))
            .map(|(path, _)| path.clone())
            .collect(),
        drifts: vec![DriftDecision {
            drift: drift.id.clone(),
            decision: row.decision.clone(),
            option: row.option,
            note: row.note.clone(),
        }],
        ..EditRequest::default()
    };
    let edited = create::plan_edit(&with_draft, &Folders::new(), &draft.id, &edit, read, rng)?;
    for (rel, change) in edited.changes.iter() {
        match change {
            Change::Write(content) => changes.write(rel, content.clone()),
            Change::Delete => changes.delete(rel),
        }
    }
    // The copies `edit` made hold the current files; the resolution carries the attached Proposal's.
    for file in &edited.files {
        if let Some(Change::Write(content)) = carried.get(&file.path) {
            changes.write(&file.file, content.clone());
        }
    }
    Ok(Written {
        id: draft.id,
        changes,
    })
}
