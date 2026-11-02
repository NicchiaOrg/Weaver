//! What the Diagram, Requirements and Specs views show, worked out whole from the project
//! images: the interface draws it and never reads a file or derives a graph.
//!
//! Two images go in: the *current* one (what is on disk, the one open Drifts and Tasks refer to)
//! and the *shown* one (the same, or the current one with a Proposal applied on top of the
//! Proposals it requires). Everything that differs between them carries a [`Change`]; what only
//! the current image has is still drawn, as `removed`.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`merge`] | The Model of both images as one tree, every element with its change |
//! | [`lines`] | The file and line of an element, found in the text of its file |
//! | [`diagram`] | The levels of the Diagram: nodes, edges, details, where each reference is drawn |
//! | [`docs`] | The Requirements and Specs pages, with the overlay labels |

pub mod diagram;
pub mod docs;
pub mod lines;
pub mod merge;

use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;

use super::cards::{Card, CardKind};
use super::image::ProjectImage;
use super::layout;
use super::refs::{parse_element_ref, parse_reference, ElementRef, Reference, GENERAL_SECTION};
use super::shortnum::ShortNumbers;

pub use diagram::{
    Anchor, Detail, DetailKind, DiagramEdge, DiagramNode, EventLine, Level, LevelKind, NodeKind,
    TransitionLine,
};
pub use docs::{
    AreaView, FragmentChange, FragmentView, SectionView, SpecEntryView, SpecsFieldView, SpecsView,
};

/// How an element differs from the current Model when a Proposal is drawn over it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum Change {
    Added,
    Changed,
    Removed,
}

impl Change {
    /// The text label shown wherever the change is (details, hover box, accessible name).
    pub fn label(self) -> &'static str {
        match self {
            Change::Added => "added",
            Change::Changed => "changed",
            Change::Removed => "removed",
        }
    }
}

/// The change of something made of parts: all added, all removed, or changed when the parts
/// differ in any way; `None` when none changed.
pub fn aggregate(parts: impl IntoIterator<Item = Option<Change>>) -> Option<Change> {
    let mut total = 0;
    let (mut added, mut removed, mut other) = (0, 0, 0);
    for part in parts {
        total += 1;
        match part {
            Some(Change::Added) => added += 1,
            Some(Change::Removed) => removed += 1,
            Some(Change::Changed) => other += 1,
            None => {}
        }
    }
    if total == 0 || (added == 0 && removed == 0 && other == 0) {
        None
    } else if added == total {
        Some(Change::Added)
    } else if removed == total {
        Some(Change::Removed)
    } else {
        Some(Change::Changed)
    }
}

/// A card named by a view: an open Drift or a live Task that concerns an element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CardMention {
    pub id: String,
    /// `D3` or `T4`, when the card is live and has a number.
    pub short_number: Option<String>,
    pub title: String,
}

/// A Proposal the views can draw over the Model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OverlayOption {
    pub id: String,
    /// `P3`, when the Proposal has a number.
    pub short_number: Option<String>,
    pub title: String,
}

/// An error of the Model, the Requirements or the Specs, with where to open it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ViewError {
    pub message: String,
    /// The file, relative to the project folder (`.trama/model/...`).
    pub file: String,
    pub line: Option<u32>,
}

/// The views a reference may be focused in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum TargetView {
    Diagram,
    Requirements,
    Specs,
}

/// A Requirements section, or (with no `section`) a whole Area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SectionAnchor {
    pub area: String,
    pub section: Option<String>,
}

/// Where a reference is shown in each view, and which views it concerns at all. A reference a
/// view concerns and finds nothing for is `not found` there.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RefTargets {
    pub relevant: Vec<TargetView>,
    pub diagram: Vec<Anchor>,
    pub requirements: Vec<SectionAnchor>,
    /// Specs fields.
    pub specs: Vec<String>,
}

/// The Model as the Diagram draws it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModelView {
    /// In Model order: root, Whole System, Actors, then each Feature followed by its Machines and
    /// their States.
    pub levels: Vec<Level>,
    /// The details of each element, by the reference its nodes carry.
    pub details: BTreeMap<String, Detail>,
}

/// Everything the three views show.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ViewData {
    pub errors: Vec<ViewError>,
    pub model: ModelView,
    pub requirements: Vec<AreaView>,
    /// `None` when the project has no Specs.
    pub specs: Option<SpecsView>,
    /// The focus of every reference the project's cards, the Requirements and the scope name.
    pub targets: BTreeMap<String, RefTargets>,
}

/// The text of the files the views point into.
pub struct Files<'a> {
    /// The text of `rel` (relative to `.trama/`) as it is on disk: for what only the current
    /// Model has (what a Proposal removes).
    pub current: &'a dyn Fn(&str) -> Option<String>,
    /// The file that holds `rel` in the shown image and its text: the file itself, or the copy a
    /// Proposal keeps of it. The path is relative to `.trama/`.
    pub shown: &'a dyn Fn(&str) -> Option<(String, String)>,
}

/// What [`build`] works from.
pub struct ViewInput<'a> {
    pub current: &'a ProjectImage,
    pub shown: &'a ProjectImage,
    /// The shown image has a Proposal applied: differences are changes.
    pub overlaid: bool,
    pub numbers: &'a ShortNumbers,
    pub files: Files<'a>,
    /// References to resolve besides those of the cards and of the Requirements (the scope of
    /// the Proposal drawn).
    pub extra_refs: &'a [String],
}

/// The Proposals that can be drawn over the Model: every open Proposal not attached to a Drift.
/// Never a Draft, a Proposal being refined or an attached one.
pub fn eligible_overlays(image: &ProjectImage, numbers: &ShortNumbers) -> Vec<OverlayOption> {
    let attached: Vec<&str> = image
        .cards_of(CardKind::Drift)
        .filter_map(|d| d.str_key("proposal"))
        .collect();
    image
        .cards_of(CardKind::Proposal)
        .filter(|p| p.str_key("status") == Some("open") && !attached.contains(&p.id.as_str()))
        .map(|p| OverlayOption {
            id: p.id.clone(),
            short_number: numbers.label_of(CardKind::Proposal, &p.id),
            title: p.title.clone(),
        })
        .collect()
}

/// The errors of the Model, the Requirements and the Specs (unreadable files included), each
/// with the line the message names when it can be found.
pub fn view_errors(image: &ProjectImage, files: &Files) -> Vec<ViewError> {
    let loc = lines::Locator::new(files);
    image
        .diagnostics()
        .iter()
        .filter(|d| d.is_error() && is_view_file(&d.path))
        .map(|d| ViewError {
            message: d.message.clone(),
            file: format!("{}/{}", layout::TRAMA_DIR, d.path),
            line: loc.guess_line(&d.path, &d.message),
        })
        .collect()
}

fn is_view_file(path: &str) -> bool {
    let top = path.split('/').next().unwrap_or("");
    top == layout::MODEL_DIR || top == layout::REQUIREMENTS_DIR || path == layout::SPECS_FILE
}

/// Works out the three views.
pub fn build(input: &ViewInput) -> ViewData {
    let base_model = input.current.model();
    let shown_model = input.shown.model();
    let merged = merge::merge(base_model, shown_model, input.overlaid);
    let loc = lines::Locator::new(&input.files);
    let concerning = Concerning::new(input);

    let model = diagram::build(&merged, &loc, &concerning);
    let requirements = docs::requirements(
        input.current.areas(),
        input.shown.areas(),
        input.overlaid,
        &loc,
    );
    let specs = docs::specs(
        input.current.specs(),
        input.shown.specs(),
        input.overlaid,
        &loc,
    );

    let mut wanted: Vec<String> = Vec::new();
    for card in input.current.cards() {
        if matches!(
            card.kind,
            CardKind::Task | CardKind::Drift | CardKind::ProposalTask
        ) {
            wanted.extend(card.list_key("refs").unwrap_or_default());
        }
    }
    wanted.extend(input.extra_refs.iter().cloned());
    for area in input.shown.areas() {
        wanted.extend(
            area.sections()
                .filter(|(_, s)| s.title != GENERAL_SECTION)
                .map(|(_, s)| s.title.clone()),
        );
    }
    wanted.sort();
    wanted.dedup();
    let targets = wanted
        .into_iter()
        .map(|text| {
            let found = targets_of(
                &text,
                &merged,
                shown_model,
                &model,
                &requirements,
                specs.as_ref(),
            );
            (text, found)
        })
        .collect();

    ViewData {
        errors: view_errors(input.current, &input.files),
        model,
        requirements,
        specs,
        targets,
    }
}

/// Where `text` (a reference) is shown in each view.
fn targets_of(
    text: &str,
    merged: &merge::MergedModel,
    shown_model: &super::model::Model,
    model: &ModelView,
    requirements: &[AreaView],
    specs: Option<&SpecsView>,
) -> RefTargets {
    use TargetView::{Diagram, Requirements, Specs};
    let Ok(reference) = parse_reference(text) else {
        return RefTargets {
            relevant: vec![Diagram, Requirements, Specs],
            ..RefTargets::default()
        };
    };
    let mut targets = RefTargets::default();
    match &reference {
        Reference::Element(element) => {
            targets.relevant = vec![Diagram, Requirements];
            targets.diagram = diagram::anchors_of(merged, model, element);
            let titles = super::browse::applicable_titles(shown_model, element);
            targets.requirements = sections_titled(requirements, &titles);
        }
        Reference::Requirement { area, section } => {
            targets.relevant = vec![Requirements];
            let exists = requirements.iter().any(|a| a.id == *area);
            match section {
                None if exists => targets.requirements.push(SectionAnchor {
                    area: area.clone(),
                    section: None,
                }),
                Some(section)
                    if requirements
                        .iter()
                        .any(|a| a.id == *area && a.has_section(section)) =>
                {
                    targets.requirements.push(SectionAnchor {
                        area: area.clone(),
                        section: Some(section.clone()),
                    });
                }
                _ => {}
            }
            if let Some(section) = section.as_deref().filter(|s| *s != GENERAL_SECTION) {
                if let Ok(element) = parse_element_ref(section) {
                    targets.relevant.insert(0, Diagram);
                    targets.diagram = diagram::anchors_of(merged, model, &element);
                }
            }
        }
        Reference::Specs { field } => {
            targets.relevant = vec![Specs];
            if let Some(specs) = specs {
                targets.specs = specs
                    .fields
                    .iter()
                    .filter(|f| field.as_deref().is_none_or(|wanted| f.name == wanted))
                    .map(|f| f.name.clone())
                    .collect();
            }
        }
    }
    targets
}

/// The sections whose title is one of `titles` (an element and what contains it), in page order.
fn sections_titled(areas: &[AreaView], titles: &[String]) -> Vec<SectionAnchor> {
    let mut found = Vec::new();
    for area in areas {
        for fragment in &area.fragments {
            for section in &fragment.sections {
                if section.title != GENERAL_SECTION && titles.contains(&section.title) {
                    found.push(SectionAnchor {
                        area: area.id.clone(),
                        section: Some(section.title.clone()),
                    });
                }
            }
        }
    }
    found
}

/// The open Drifts and live Tasks that concern an element, from the *current* Model.
pub struct Concerning<'a> {
    input: &'a ViewInput<'a>,
    drifts: Vec<(&'a Card, Vec<super::concerns::Key>)>,
    tasks: Vec<(&'a Card, Vec<super::concerns::Key>)>,
}

impl<'a> Concerning<'a> {
    fn new(input: &'a ViewInput<'a>) -> Self {
        let keyed = |kind: CardKind,
                     live: &dyn Fn(&Card) -> bool|
         -> Vec<(&'a Card, Vec<super::concerns::Key>)> {
            input
                .current
                .cards_of(kind)
                .filter(|c| live(c))
                .map(|c| (c, super::concerns::card_keys(c)))
                .filter(|(_, keys)| !keys.is_empty())
                .collect()
        };
        Self {
            input,
            drifts: keyed(CardKind::Drift, &|d| d.str_key("status") == Some("open")),
            tasks: keyed(CardKind::Task, &|t| {
                matches!(
                    super::tasks::column_of(t),
                    Some(super::tasks::Column::Todo | super::tasks::Column::Doing)
                )
            }),
        }
    }

    /// The open Drifts and live Tasks whose references concern `element`.
    pub fn of(&self, element: &ElementRef) -> (Vec<CardMention>, Vec<CardMention>) {
        let key = super::concerns::Key::Element(element.clone());
        let model = self.input.current.model();
        let pick =
            |cards: &[(&Card, Vec<super::concerns::Key>)], kind: CardKind| -> Vec<CardMention> {
                cards
                    .iter()
                    .filter(|(_, keys)| {
                        keys.iter()
                            .any(|k| super::concerns::keys_concern(model, k, &key))
                    })
                    .map(|(c, _)| CardMention {
                        id: c.id.clone(),
                        short_number: self.input.numbers.label_of(kind, &c.id),
                        title: c.title.clone(),
                    })
                    .collect()
            };
        (
            pick(&self.drifts, CardKind::Drift),
            pick(&self.tasks, CardKind::Task),
        )
    }
}

#[cfg(test)]
mod tests;
