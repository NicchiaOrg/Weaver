//! What can be said about a Proposal from the project: staleness, validity, scope and coverage,
//! and every reason Approve or the hand-over is refused.
//!
//! Reasons Approve is refused, all of them, in this order:
//!
//! 1. `The agent is still finishing`;
//! 2. `A run is in progress`;
//! 3. each required Proposal, in `requires` order: still being written, not approved yet, was
//!    rejected, was discarded, missing, forms a cycle;
//! 4. stale, naming each file;
//! 5. invalid with it applied, naming each error;
//! 6. each uncovered scope entry, each Task reference that would not exist, a missing label;
//! 7. for a resolution, each Drift whose code fix could no longer be applied.
//!
//! The hand-over is refused, naming every problem, for 5–7 and for what only the hand-over
//! checks (an unregistered file, Task files where there must be none, a Task whose `technical`
//! disagrees with its references, an `## Already present` reference that would not exist, the
//! rules of a resolution); never for staleness or required Proposals.

use std::collections::BTreeSet;

use crate::project::board::{REASON_AGENT_FINISHING, REASON_RUN_IN_PROGRESS};
use crate::project::cards::{Card, CardKind};
use crate::project::changes::{Change, ChangeSet};
use crate::project::diagnostic::Diagnostic;
use crate::project::image::ProjectImage;
use crate::project::layout::{classify, FileClass};
use crate::project::refs::{parse_reference, Reference};
use crate::project::shortnum::ShortNumbers;
use crate::project::tasks;
use crate::project::text;

use super::data::{self, Kind, Status};
use super::files::{self, Folder, Folders};
use super::label::validate_label;
use super::resolution;
use super::scope::{self, CoveredEntry, RemovalTask};

/// Why a required Proposal does not allow the approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredProblem {
    BeingWritten,
    NotApproved,
    Rejected,
    Discarded,
    Missing,
    Cycle,
}

impl RequiredProblem {
    pub fn text(self) -> &'static str {
        match self {
            RequiredProblem::BeingWritten => "is still being written",
            RequiredProblem::NotApproved => "is not approved yet",
            RequiredProblem::Rejected => "was rejected",
            RequiredProblem::Discarded => "was discarded",
            RequiredProblem::Missing => "is missing",
            RequiredProblem::Cycle => "forms a cycle",
        }
    }
}

/// A Proposal named in `requires`, and what is wrong with it, if anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Required {
    pub id: String,
    pub problem: Option<RequiredProblem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleHow {
    /// The file is not the one the Proposal started from.
    Changed,
    /// The file did not exist and does now.
    NowExists,
    /// The file existed and is gone.
    Gone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleFile {
    pub path: String,
    pub how: StaleHow,
}

/// A Task the Proposal carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInfo {
    pub id: String,
    pub title: String,
    pub technical: bool,
    pub refs: Vec<String>,
}

/// One reason Approve is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    AgentFinishing,
    RunInProgress,
    Required {
        id: String,
        problem: RequiredProblem,
    },
    Stale(StaleFile),
    Invalid {
        path: String,
        message: String,
    },
    Uncovered {
        reference: String,
        why: scope::Reason,
    },
    UnknownReference {
        task: String,
        reference: String,
        why: String,
    },
    Label(String),
    FixNotApplicable {
        drift: String,
        why: String,
    },
}

impl Reason {
    /// The text of the reason, as the card, the details and the commands show it.
    pub fn text(&self, numbers: &ShortNumbers) -> String {
        let name =
            |kind: CardKind, id: &str| numbers.label_of(kind, id).unwrap_or_else(|| id.to_owned());
        match self {
            Reason::AgentFinishing => REASON_AGENT_FINISHING.to_owned(),
            Reason::RunInProgress => REASON_RUN_IN_PROGRESS.to_owned(),
            Reason::Required { id, problem } => format!(
                "Proposal {} {}",
                name(CardKind::Proposal, id),
                problem.text()
            ),
            Reason::Stale(StaleFile { path, how }) => match how {
                StaleHow::Changed => format!("Stale: `{path}` changed since the Proposal started"),
                StaleHow::NowExists => {
                    format!("Stale: `{path}` did not exist when the Proposal started and does now")
                }
                StaleHow::Gone => format!("Stale: `{path}` is gone"),
            },
            Reason::Invalid { path, message } => {
                format!("Invalid with it applied: {path}: {message}")
            }
            Reason::Uncovered { reference, why } => {
                format!("`{reference}` ({}) is not covered by a Task", why.text())
            }
            Reason::UnknownReference {
                task,
                reference,
                why,
            } => format!("Task \"{task}\" names `{reference}`, which would not exist: {why}"),
            Reason::Label(text) => text.clone(),
            Reason::FixNotApplicable { drift, why } => {
                format!(
                    "Drift {}: the code fix can no longer be applied: {why}",
                    name(CardKind::Drift, drift)
                )
            }
        }
    }

    pub fn is_stale(&self) -> bool {
        matches!(self, Reason::Stale(_))
    }
}

/// What only the running app knows, for the first two reasons.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Gate {
    /// The job that handed the Proposal over is still working.
    pub agent_finishing: bool,
    /// A Run works in the project.
    pub run_in_progress: bool,
}

/// Everything computed about one Proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    pub id: String,
    pub kind: Kind,
    pub status: Option<Status>,
    pub requires: Vec<Required>,
    pub files: files::FileSummary,
    pub stale: Vec<StaleFile>,
    /// Reason 5.
    pub invalid: Vec<Reason>,
    /// The scope with what covers each entry; empty for an import and a resolution.
    pub scope: Vec<CoveredEntry>,
    pub removals: Vec<RemovalTask>,
    pub tasks: Vec<TaskInfo>,
    /// Reason 6.
    pub uncovered: Vec<Reason>,
    /// Reason 7.
    pub fixes: Vec<Reason>,
    /// What only the hand-over checks, already worded.
    pub handover_only: Vec<String>,
    /// Whether a Draft, Open or Refining Proposal is attached to a Drift (decided through it).
    pub attached: bool,
}

impl Analysis {
    /// Reasons 3 to 7: what blocks the approval whatever the agent and the Runs do.
    pub fn blocking(&self) -> Vec<Reason> {
        let mut reasons: Vec<Reason> = self
            .requires
            .iter()
            .filter_map(|r| {
                r.problem.map(|problem| Reason::Required {
                    id: r.id.clone(),
                    problem,
                })
            })
            .collect();
        reasons.extend(self.stale.iter().cloned().map(Reason::Stale));
        reasons.extend(self.invalid.iter().cloned());
        reasons.extend(self.uncovered.iter().cloned());
        reasons.extend(self.fixes.iter().cloned());
        reasons
    }

    /// Every reason Approve is refused, in order.
    pub fn approve_reasons(&self, gate: Gate) -> Vec<Reason> {
        let mut reasons = Vec::new();
        if gate.agent_finishing {
            reasons.push(Reason::AgentFinishing);
        }
        if gate.run_in_progress {
            reasons.push(Reason::RunInProgress);
        }
        reasons.extend(self.blocking());
        reasons
    }

    /// Staleness is the only thing that blocks it, a Run apart: Approve refines it.
    pub fn refinable(&self, gate: Gate) -> bool {
        !gate.agent_finishing
            && self.kind == Kind::Change
            && !self.attached
            && !self.stale.is_empty()
            && self.blocking().iter().all(Reason::is_stale)
    }

    /// The reasons the hand-over is refused: 5 to 7, then what only the hand-over checks.
    pub fn handover_refusals(&self, numbers: &ShortNumbers) -> Vec<String> {
        self.invalid
            .iter()
            .chain(&self.uncovered)
            .chain(&self.fixes)
            .map(|r| r.text(numbers))
            .chain(self.handover_only.iter().cloned())
            .collect()
    }
}

// ---------------------------------------------------------------------------------------------
// Required Proposals
// ---------------------------------------------------------------------------------------------

/// Whether `target` can be reached from `from` through `requires`.
fn reaches(image: &ProjectImage, from: &str, target: &str) -> bool {
    let mut seen = BTreeSet::new();
    let mut stack = vec![from.to_owned()];
    while let Some(node) = stack.pop() {
        if node == target {
            return true;
        }
        if !seen.insert(node.clone()) {
            continue;
        }
        if let Some(card) = image.card(CardKind::Proposal, &node) {
            stack.extend(data::requires_of(card));
        }
    }
    false
}

fn required_states(image: &ProjectImage, card: &Card) -> Vec<Required> {
    data::requires_of(card)
        .into_iter()
        .map(|id| {
            let problem = match image.card(CardKind::Proposal, &id) {
                None => Some(RequiredProblem::Missing),
                Some(required) if reaches(image, &required.id, &card.id) => {
                    Some(RequiredProblem::Cycle)
                }
                Some(required) => match data::status_of(required) {
                    Some(Status::Approved) => None,
                    Some(Status::Draft | Status::Refining) => Some(RequiredProblem::BeingWritten),
                    Some(Status::Rejected) => Some(RequiredProblem::Rejected),
                    Some(Status::Discarded) => Some(RequiredProblem::Discarded),
                    Some(Status::Open) | None => Some(RequiredProblem::NotApproved),
                },
            };
            Required { id, problem }
        })
        .collect()
}

/// The Proposals `id` requires, transitively, each after the ones it requires; `id` itself and
/// a Proposal already seen are left out, so a cycle ends.
pub fn dependency_order(image: &ProjectImage, id: &str) -> Vec<String> {
    fn visit(
        image: &ProjectImage,
        node: &str,
        seen: &mut BTreeSet<String>,
        order: &mut Vec<String>,
    ) {
        let Some(card) = image.card(CardKind::Proposal, node) else {
            return;
        };
        for required in data::requires_of(card) {
            if seen.insert(required.clone()) {
                visit(image, &required, seen, order);
                order.push(required);
            }
        }
    }
    let mut seen = BTreeSet::from([id.to_owned()]);
    let mut order = Vec::new();
    visit(image, id, &mut seen, &mut order);
    order
}

/// The changes of the required Proposals applied below `id`, in dependency order: only those
/// still to be approved count (an approved one is already in the files; a rejected or discarded
/// one never happens).
pub fn applied_below(image: &ProjectImage, folders: &Folders, id: &str) -> ChangeSet {
    let empty = Folder::default();
    let mut below = ChangeSet::new();
    for required in dependency_order(image, id) {
        let Some(card) = image.card(CardKind::Proposal, &required) else {
            continue;
        };
        if !matches!(
            data::status_of(card),
            Some(Status::Draft | Status::Open | Status::Refining)
        ) {
            continue;
        }
        let changes = files::changes_of(card, folders.get(&required).unwrap_or(&empty));
        for (rel, change) in changes.iter() {
            match change {
                Change::Write(content) => below.write(rel, content.clone()),
                Change::Delete => below.delete(rel),
            }
        }
    }
    below
}

/// The image with the required Proposals applied below `id`, then the Proposal itself: its files
/// written and its removed paths deleted. Task files are not part of it.
pub fn overlay(image: &ProjectImage, folders: &Folders, id: &str) -> ProjectImage {
    let below = image.with_changes(&applied_below(image, folders, id));
    match image.card(CardKind::Proposal, id) {
        Some(card) => below.with_changes(&files::changes_of(
            card,
            folders.get(id).unwrap_or(&Folder::default()),
        )),
        None => below,
    }
}

// ---------------------------------------------------------------------------------------------
// Staleness
// ---------------------------------------------------------------------------------------------

/// The files of `base` that are no longer the ones the Proposal started from, counting the
/// required Proposals as applied (`below`).
fn stale_files(image: &ProjectImage, card: &Card, below: &ChangeSet) -> Vec<StaleFile> {
    let mut stale = Vec::new();
    for (path, recorded) in data::base_of(card) {
        let current: Option<String> = match below.get(&path) {
            Some(Change::Write(content)) => Some(text::sha256_hex(&text::file_content(content))),
            Some(Change::Delete) => None,
            None => image.file_hash(&path).map(str::to_owned),
        };
        let how = match (&current, recorded.as_str()) {
            (Some(_), data::NEW_FILE) => Some(StaleHow::NowExists),
            (None, data::NEW_FILE) => None,
            (None, _) => Some(StaleHow::Gone),
            (Some(hash), recorded) if hash != recorded => Some(StaleHow::Changed),
            _ => None,
        };
        if let Some(how) = how {
            stale.push(StaleFile { path, how });
        }
    }
    stale
}

/// The base a file of the Proposal has now: the hash of its current content with the required
/// Proposals applied below, or `new`. Used to record `base` the first time and again by a rebase.
pub fn current_base(image: &ProjectImage, folders: &Folders, id: &str, path: &str) -> String {
    match applied_below(image, folders, id).get(path) {
        Some(Change::Write(content)) => text::sha256_hex(&text::file_content(content)),
        Some(Change::Delete) => data::NEW_FILE.to_owned(),
        None => image
            .file_hash(path)
            .map_or_else(|| data::NEW_FILE.to_owned(), str::to_owned),
    }
}

// ---------------------------------------------------------------------------------------------
// The whole analysis
// ---------------------------------------------------------------------------------------------

fn is_model_file(path: &str) -> bool {
    !matches!(
        classify(path),
        FileClass::Ignored | FileClass::Misplaced(_) | FileClass::Card { .. }
    )
}

/// The references of the scope entries of `id`, deduplicated, in order; empty for an import and
/// a resolution (and for a Proposal that does not exist).
pub fn scope_refs(image: &ProjectImage, folders: &Folders, id: &str) -> Vec<String> {
    let Some(card) = image.card(CardKind::Proposal, id) else {
        return Vec::new();
    };
    if !data::kind_of(card).carries_tasks() {
        return Vec::new();
    }
    let before = image.with_changes(&applied_below(image, folders, id));
    let after = overlay(image, folders, id);
    let mut refs: Vec<String> = Vec::new();
    for entry in scope::compute(&before, &after).entries {
        if !refs.contains(&entry.reference) {
            refs.push(entry.reference);
        }
    }
    refs
}

/// Analyses the Proposal `id`; `None` when no such Proposal is in the image.
pub fn analyse(
    image: &ProjectImage,
    folders: &Folders,
    numbers: &ShortNumbers,
    id: &str,
) -> Option<Analysis> {
    let card = image.card(CardKind::Proposal, id)?;
    let kind = data::kind_of(card);
    let name =
        |kind: CardKind, id: &str| numbers.label_of(kind, id).unwrap_or_else(|| id.to_owned());
    let empty = Folder::default();
    let folder = folders.get(id).unwrap_or(&empty);

    let requires = required_states(image, card);
    let below = applied_below(image, folders, id);
    let base_image = image.with_changes(&below);
    let after = base_image.with_changes(&files::changes_of(card, folder));
    let stale = stale_files(image, card, &below);

    // 5. Invalid with it applied: errors the files would add to those the project already has.
    let existing: BTreeSet<&Diagnostic> = base_image.diagnostics().iter().collect();
    let invalid: Vec<Reason> = after
        .diagnostics()
        .iter()
        .filter(|d| d.is_error() && !existing.contains(d))
        .map(|d| Reason::Invalid {
            path: d.path.clone(),
            message: d.message.clone(),
        })
        .collect();

    // The Tasks the Proposal carries.
    let own: Vec<TaskInfo> = data::tasks_of(image, id)
        .into_iter()
        .map(|t| TaskInfo {
            id: t.id.clone(),
            title: t.title.clone(),
            technical: tasks::is_technical(t),
            refs: tasks::refs_of(t),
        })
        .collect();

    // 6. Scope and coverage, references that would not exist, the label.
    let mut covered: Vec<CoveredEntry> = Vec::new();
    let mut removals: Vec<RemovalTask> = Vec::new();
    let mut uncovered: Vec<Reason> = Vec::new();
    let present = scope::already_present(&card.body);
    if kind.carries_tasks() {
        let computed = scope::compute(&base_image, &after);
        let refs: Vec<(String, Vec<String>)> =
            own.iter().map(|t| (t.id.clone(), t.refs.clone())).collect();
        covered = scope::coverage(&computed, after.model(), &refs, &present);
        removals = computed.removals;
        for entry in covered.iter().filter(|c| c.coverage.is_none()) {
            uncovered.push(Reason::Uncovered {
                reference: entry.entry.reference.clone(),
                why: entry.entry.reason,
            });
        }
    }
    let resolver = after.resolver();
    for task in &own {
        for reference in &task.refs {
            let why = match parse_reference(reference) {
                Err(why) => Some(why),
                Ok(parsed) => resolver.resolve(&parsed).err(),
            };
            if let Some(why) = why {
                uncovered.push(Reason::UnknownReference {
                    task: task.title.clone(),
                    reference: reference.clone(),
                    why,
                });
            }
        }
    }
    if !own.is_empty() || !removals.is_empty() {
        match data::label_of(card) {
            None => uncovered.push(Reason::Label("The label is missing".to_owned())),
            Some(label) => {
                if let Err(why) = validate_label(label) {
                    uncovered.push(Reason::Label(format!("The label is not valid: {why}")));
                }
            }
        }
    }

    // 7 and what only the hand-over checks.
    let mut fixes: Vec<Reason> = Vec::new();
    let mut handover_only: Vec<String> = Vec::new();
    let attached = data::attached_drift(image, id).is_some();

    for (path, why) in &folder.unreadable {
        handover_only.push(format!("`{path}` cannot be read: {why}"));
    }
    let base = data::base_of(card);
    for (path, _) in folder.held() {
        if !base.contains_key(path) {
            handover_only.push(format!("`{path}` is not registered: make a file editable with `weaver proposal edit`, never by creating it in the folder"));
        }
    }
    for path in base.keys() {
        if !is_model_file(path) {
            handover_only.push(format!(
                "`{path}` is not a file of the Model, the Requirements or the Specs"
            ));
        }
    }
    let folder_prefix = format!("{}/", data::folder_of(card));
    for diagnostic in image
        .diagnostics()
        .iter()
        .filter(|d| d.is_error() && d.path.starts_with(&folder_prefix))
    {
        handover_only.push(format!("`{}`: {}", diagnostic.path, diagnostic.message));
    }
    if !own.is_empty() {
        if !kind.carries_tasks() {
            let article = if kind == Kind::Import { "An" } else { "A" };
            handover_only.push(format!(
                "{article} {} carries no Task: remove its Task files",
                kind.as_str()
            ));
        } else if attached {
            handover_only.push(
                "A Proposal attached to a Drift carries no Task: remove its Task files".to_owned(),
            );
        }
    }
    for task in &own {
        let elements = task
            .refs
            .iter()
            .filter(|r| matches!(parse_reference(r), Ok(Reference::Element(_))))
            .count();
        if task.technical && elements > 0 {
            handover_only.push(format!("Task \"{}\" is technical but names a Model element: remove the element from `refs`, or make the Task not technical", task.title));
        }
        if !task.technical && elements == 0 {
            handover_only.push(format!("Task \"{}\" is not technical but names no Model element: add one to `refs`, or make the Task technical", task.title));
        }
    }
    for reference in &present {
        let why = match parse_reference(reference) {
            Err(why) => Some(why),
            Ok(parsed) => resolver.resolve(&parsed).err(),
        };
        if let Some(why) = why {
            handover_only.push(format!(
                "`## Already present` names `{reference}`, which would not exist: {why}"
            ));
        }
    }
    if kind == Kind::Resolution {
        let problems = resolution::problems(image, &after, card, &name);
        handover_only.extend(problems.handover);
        fixes.extend(
            problems
                .fixes
                .into_iter()
                .map(|(drift, why)| Reason::FixNotApplicable { drift, why }),
        );
    }

    Some(Analysis {
        id: id.to_owned(),
        kind,
        status: data::status_of(card),
        requires,
        files: files::summary_of(card),
        stale,
        invalid,
        scope: covered,
        removals,
        tasks: own,
        uncovered,
        fixes,
        handover_only,
        attached,
    })
}
