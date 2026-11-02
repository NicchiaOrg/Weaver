//! The Requirements page and the Specs page, with the labels of an overlay.
//!
//! Sections are compared by Area and title, whatever fragment holds them: a section that moved to
//! another fragment with the same text is not a change, only `moved from <fragment>`.

use serde::Serialize;
use ts_rs::TS;

use crate::project::refs::GENERAL_SECTION;
use crate::project::requirements::{Area, Fragment, Section};
use crate::project::specs::{SpecEntry, Specs, NOTES_FIELD, SPECS_FIELDS};

use super::lines::Locator;
use super::{aggregate, Change};

/// A fragment that only the shown image has (`new`) or only the current one has (`removed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum FragmentChange {
    New,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SectionView {
    /// `General`, or one element reference.
    pub title: String,
    /// What follows the heading, Markdown.
    pub body: String,
    /// The body before the overlay, when it changed.
    pub before: Option<String>,
    pub change: Option<Change>,
    /// The fragment the section was in, when the overlay moved it.
    pub moved_from: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FragmentView {
    /// The file name without `.md`.
    pub name: String,
    pub title: String,
    pub description: String,
    pub change: Option<FragmentChange>,
    pub file: Option<String>,
    /// `General` first, then in file order.
    pub sections: Vec<SectionView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AreaView {
    pub id: String,
    pub title: String,
    pub description: String,
    /// The prose of `area.md` after its title, Markdown.
    pub intro: String,
    pub file: Option<String>,
    pub fragments: Vec<FragmentView>,
}

impl AreaView {
    pub fn has_section(&self, title: &str) -> bool {
        self.fragments
            .iter()
            .flat_map(|f| &f.sections)
            .any(|s| s.title == title)
    }
}

fn general_first(sections: &[Section]) -> Vec<&Section> {
    let mut ordered: Vec<&Section> = sections
        .iter()
        .filter(|s| s.title == GENERAL_SECTION)
        .collect();
    ordered.extend(sections.iter().filter(|s| s.title != GENERAL_SECTION));
    ordered
}

fn section_view(
    loc: &Locator,
    area: &str,
    fragment: &str,
    section: &Section,
    change: Option<Change>,
) -> SectionView {
    let path = Locator::fragment_path(area, fragment);
    let place = loc.place(&path, change == Some(Change::Removed), |d| {
        d.section(&section.title)
    });
    SectionView {
        title: section.title.clone(),
        body: section.body.clone(),
        before: None,
        change,
        moved_from: None,
        file: place.as_ref().map(|p| p.file.clone()),
        line: place.and_then(|p| p.line),
    }
}

fn fragment_view(
    loc: &Locator,
    area: &str,
    fragment: &Fragment,
    change: Option<FragmentChange>,
    sections: Vec<SectionView>,
) -> FragmentView {
    let place = loc.place(
        &Locator::fragment_path(area, &fragment.name),
        change == Some(FragmentChange::Removed),
        |d| d.title(),
    );
    FragmentView {
        name: fragment.name.clone(),
        title: fragment.title.clone(),
        description: fragment.description.clone(),
        change,
        file: place.map(|p| p.file),
        sections,
    }
}

/// The Areas of the shown image (and, under an overlay, what only the current one has).
pub fn requirements(base: &[Area], shown: &[Area], overlaid: bool, loc: &Locator) -> Vec<AreaView> {
    let mut ids: Vec<&str> = shown.iter().map(|a| a.id.as_str()).collect();
    if overlaid {
        ids.extend(base.iter().map(|a| a.id.as_str()));
    }
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .filter_map(|id| {
            let now = shown.iter().find(|a| a.id == id);
            let before = if overlaid {
                base.iter().find(|a| a.id == id)
            } else {
                None
            };
            area_view(before, now, overlaid, loc)
        })
        .collect()
}

fn area_view(
    base: Option<&Area>,
    shown: Option<&Area>,
    overlaid: bool,
    loc: &Locator,
) -> Option<AreaView> {
    let head = shown.or(base)?;
    let removed_area = shown.is_none();
    let area_place = loc.place(&Locator::area_path(&head.id), removed_area, |d| d.title());
    let mut fragments: Vec<FragmentView> = Vec::new();

    if let Some(now) = shown {
        for fragment in &now.fragments {
            let before_fragment =
                base.and_then(|b| b.fragments.iter().find(|f| f.name == fragment.name));
            let change = (overlaid && before_fragment.is_none()).then_some(FragmentChange::New);
            let mut sections: Vec<SectionView> = Vec::new();
            for section in general_first(&fragment.sections) {
                let mut view = section_view(loc, &now.id, &fragment.name, section, None);
                if overlaid {
                    let found =
                        base.and_then(|b| b.sections().find(|(_, s)| s.title == section.title));
                    match found {
                        None => view.change = Some(Change::Added),
                        Some((home, old)) => {
                            if old.body != section.body {
                                view.change = Some(Change::Changed);
                                view.before = Some(old.body.clone());
                            }
                            if home.name != fragment.name {
                                view.moved_from = Some(home.name.clone());
                            }
                        }
                    }
                }
                sections.push(view);
            }
            if let Some(old) = before_fragment {
                for section in general_first(&old.sections) {
                    if !now.has_section(&section.title) {
                        sections.push(section_view(
                            loc,
                            &now.id,
                            &old.name,
                            section,
                            Some(Change::Removed),
                        ));
                    }
                }
            }
            fragments.push(fragment_view(loc, &now.id, fragment, change, sections));
        }
    }
    if let Some(old) = base {
        for fragment in old
            .fragments
            .iter()
            .filter(|f| !shown.is_some_and(|n| n.fragments.iter().any(|g| g.name == f.name)))
        {
            let sections = general_first(&fragment.sections)
                .into_iter()
                .filter(|s| !shown.is_some_and(|n| n.has_section(&s.title)))
                .map(|s| section_view(loc, &old.id, &fragment.name, s, Some(Change::Removed)))
                .collect();
            fragments.push(fragment_view(
                loc,
                &old.id,
                fragment,
                Some(FragmentChange::Removed),
                sections,
            ));
        }
    }
    fragments.sort_by(|a, b| a.name.cmp(&b.name));
    Some(AreaView {
        id: head.id.clone(),
        title: head.title.clone(),
        description: head.description.clone(),
        intro: head.intro.clone(),
        file: area_place.map(|p| p.file),
        fragments,
    })
}

// ---------------------------------------------------------------------------------------------
// Specs
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpecEntryView {
    pub name: String,
    pub version: Option<String>,
    pub note: Option<String>,
    /// Why the agent chose it; `Some` marks an entry the agent chose.
    pub chosen_reason: Option<String>,
    pub change: Option<Change>,
    /// How it read before the overlay, when it changed.
    pub before: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpecsFieldView {
    pub name: String,
    pub change: Option<Change>,
    pub entries: Vec<SpecEntryView>,
    /// The Markdown of `notes`.
    pub notes: Option<String>,
    pub before_notes: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpecsView {
    /// In the fixed order; empty fields are left out.
    pub fields: Vec<SpecsFieldView>,
}

fn entry_text(entry: &SpecEntry) -> String {
    let mut text = entry.name.clone();
    if let Some(version) = &entry.version {
        text.push(' ');
        text.push_str(version);
    }
    if let Some(note) = &entry.note {
        text.push_str(" — ");
        text.push_str(note);
    }
    text
}

fn entry_view(
    entry: &SpecEntry,
    change: Option<Change>,
    before: Option<&SpecEntry>,
) -> SpecEntryView {
    SpecEntryView {
        name: entry.name.clone(),
        version: entry.version.clone(),
        note: entry.note.clone(),
        chosen_reason: entry.chosen_by_agent.clone(),
        change,
        before: before.map(entry_text),
    }
}

fn merge_entries(base: &[SpecEntry], shown: &[SpecEntry], overlaid: bool) -> Vec<SpecEntryView> {
    let same = |a: &SpecEntry, b: &SpecEntry| a.name.to_lowercase() == b.name.to_lowercase();
    let mut out: Vec<SpecEntryView> = shown
        .iter()
        .map(|entry| {
            if !overlaid {
                return entry_view(entry, None, None);
            }
            match base.iter().find(|b| same(b, entry)) {
                None => entry_view(entry, Some(Change::Added), None),
                Some(old) if old != entry => entry_view(entry, Some(Change::Changed), Some(old)),
                Some(_) => entry_view(entry, None, None),
            }
        })
        .collect();
    if overlaid {
        out.extend(
            base.iter()
                .filter(|b| !shown.iter().any(|e| same(b, e)))
                .map(|b| entry_view(b, Some(Change::Removed), None)),
        );
    }
    out
}

/// The Specs page; `None` when neither image has Specs.
pub fn specs(
    base: Option<&Specs>,
    shown: Option<&Specs>,
    overlaid: bool,
    loc: &Locator,
) -> Option<SpecsView> {
    let base = if overlaid { base } else { None };
    if base.is_none() && shown.is_none() {
        return None;
    }
    let mut fields = Vec::new();
    for name in SPECS_FIELDS {
        let old_entries = base.and_then(|s| s.entries(name)).unwrap_or_default();
        let new_entries = shown.and_then(|s| s.entries(name)).unwrap_or_default();
        let (old_notes, new_notes) = (
            base.and_then(|s| s.notes.as_ref()),
            shown.and_then(|s| s.notes.as_ref()),
        );
        let field = if name == NOTES_FIELD {
            if old_notes.is_none() && new_notes.is_none() {
                continue;
            }
            let change = match (old_notes, new_notes) {
                (None, Some(_)) if overlaid => Some(Change::Added),
                (Some(_), None) => Some(Change::Removed),
                (Some(a), Some(b)) if a != b => Some(Change::Changed),
                _ => None,
            };
            SpecsFieldView {
                name: name.to_owned(),
                change,
                entries: Vec::new(),
                notes: new_notes.or(old_notes).cloned(),
                before_notes: (change == Some(Change::Changed))
                    .then(|| old_notes.cloned())
                    .flatten(),
                file: None,
                line: None,
            }
        } else {
            if old_entries.is_empty() && new_entries.is_empty() {
                continue;
            }
            let entries = merge_entries(old_entries, new_entries, overlaid);
            let change = if !overlaid {
                None
            } else if old_entries.is_empty() {
                Some(Change::Added)
            } else if new_entries.is_empty() {
                Some(Change::Removed)
            } else {
                aggregate(entries.iter().map(|e| e.change)).map(|_| Change::Changed)
            };
            SpecsFieldView {
                name: name.to_owned(),
                change,
                entries,
                notes: None,
                before_notes: None,
                file: None,
                line: None,
            }
        };
        let place = loc.place(
            &Locator::specs_path(),
            field.change == Some(Change::Removed),
            |d| d.specs_field(name),
        );
        fields.push(SpecsFieldView {
            file: place.as_ref().map(|p| p.file.clone()),
            line: place.and_then(|p| p.line),
            ..field
        });
    }
    Some(SpecsView { fields })
}
