//! What the Requirements and Specs views show, worked out whole from the project images: the
//! interface draws it and never reads a file or derives anything.
//!
//! Two images go in: the *current* one (what is on disk, the one open Drifts and Tasks refer to)
//! and the *shown* one (the same, or the current one with a Proposal applied on top of the
//! Proposals it requires). Everything that differs between them carries a [`Change`]; what only
//! the current image has is still drawn, as `removed`.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`lines`] | The file and line of an element, found in the text of its file |
//! | [`docs`] | The Requirements and Specs pages, with the overlay labels |

pub mod docs;
pub mod lines;

use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;

use super::cards::CardKind;
use super::image::ProjectImage;
use super::layout;
use super::refs::{parse_reference, Reference, GENERAL_SECTION};
use super::shortnum::ShortNumbers;

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
    /// The file, relative to the project folder (`.weaver/model/...`).
    pub file: String,
    pub line: Option<u32>,
}

/// The views a reference may be focused in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum TargetView {
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
    pub requirements: Vec<SectionAnchor>,
    /// Specs fields.
    pub specs: Vec<String>,
}

/// Everything the two views show.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ViewData {
    pub errors: Vec<ViewError>,
    pub requirements: Vec<AreaView>,
    /// `None` when the project has no Specs.
    pub specs: Option<SpecsView>,
    /// The focus of every reference the project's cards, the Requirements and the scope name.
    pub targets: BTreeMap<String, RefTargets>,
}

/// The text of the files the views point into.
pub struct Files<'a> {
    /// The text of `rel` (relative to `.weaver/`) as it is on disk: for what only the current
    /// Model has (what a Proposal removes).
    pub current: &'a dyn Fn(&str) -> Option<String>,
    /// The file that holds `rel` in the shown image and its text: the file itself, or the copy a
    /// Proposal keeps of it. The path is relative to `.weaver/`.
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
            file: format!("{}/{}", layout::WEAVER_DIR, d.path),
            line: loc.guess_line(&d.path, &d.message),
        })
        .collect()
}

fn is_view_file(path: &str) -> bool {
    let top = path.split('/').next().unwrap_or("");
    top == layout::MODEL_DIR || top == layout::REQUIREMENTS_DIR || path == layout::SPECS_FILE
}

/// Works out the Requirements and Specs views.
pub fn build(input: &ViewInput) -> ViewData {
    let shown_model = input.shown.model();
    let loc = lines::Locator::new(&input.files);
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
            let found = targets_of(&text, shown_model, &requirements, specs.as_ref());
            (text, found)
        })
        .collect();

    ViewData {
        errors: view_errors(input.current, &input.files),
        requirements,
        specs,
        targets,
    }
}

/// Where `text` (a reference) is shown in each view.
fn targets_of(
    text: &str,
    shown_model: &super::model::Model,
    requirements: &[AreaView],
    specs: Option<&SpecsView>,
) -> RefTargets {
    use TargetView::{Requirements, Specs};
    let Ok(reference) = parse_reference(text) else {
        return RefTargets {
            relevant: vec![Requirements, Specs],
            ..RefTargets::default()
        };
    };
    let mut targets = RefTargets::default();
    match &reference {
        Reference::Element(element) => {
            targets.relevant = vec![Requirements];
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

#[cfg(test)]
mod tests;
