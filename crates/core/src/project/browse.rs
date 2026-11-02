//! Reading the Model and the Requirements: which Requirement sections apply to an element, and
//! the text search of `weaver model list`.
//!
//! A Requirement section is titled with one element reference and applies to that element and
//! everything inside it: the sections of a Machine apply to all its States, those of a Feature
//! to all its Machines. `## General` applies everywhere and is not part of [`applicable`]: it is
//! read by Area (`requirements/<area>#General`).

use serde::Serialize;

use super::image::ProjectImage;
use super::model::{event_ref_text, ElementKind, Model};
use super::refs::{ElementRef, GENERAL_SECTION};
use super::requirements::{Area, Fragment, Section};
use super::specs::NOTES_FIELD;

/// A Requirement section with the Area and the fragment that hold it.
#[derive(Debug, Clone, Copy)]
pub struct Located<'a> {
    pub area: &'a Area,
    pub fragment: &'a Fragment,
    pub section: &'a Section,
}

/// The titles of the sections that apply to `reference`: its own first, then those of what
/// contains it, up to the Feature.
pub fn applicable_titles(model: &Model, reference: &ElementRef) -> Vec<String> {
    let mut titles = vec![reference.to_string()];
    match reference {
        ElementRef::Element(id) => push_element_chain(model, id, &mut titles),
        ElementRef::State { machine, state } => {
            push_state_chain(model, machine, state, &mut titles)
        }
        ElementRef::Transitions { machine, state, .. } => {
            push_state_chain(model, machine, state, &mut titles)
        }
        ElementRef::Event { owner, .. } => push_element_chain(model, owner, &mut titles),
    }
    titles.dedup();
    titles
}

/// `id` (already pushed by the caller when it is the reference itself) and its containers.
fn push_element_chain(model: &Model, id: &str, titles: &mut Vec<String>) {
    if titles.last().map(String::as_str) != Some(id) {
        titles.push(id.to_owned());
    }
    if model.element_kind(id) == Some(ElementKind::Machine) {
        if let Some(machine) = model.machines.get(id) {
            titles.push(machine.feature.clone());
        }
    }
}

/// The State `state` of `machine`, then the States that contain it, then the Machine chain.
fn push_state_chain(model: &Model, machine_id: &str, state: &str, titles: &mut Vec<String>) {
    if let Some(machine) = model.machines.get(machine_id) {
        let walked = machine.walk_states();
        if let Some((path, _)) = walked.iter().find(|(path, _)| path.last() == Some(&state)) {
            for name in path.iter().rev() {
                titles.push(format!("{machine_id}/{name}"));
            }
        }
    }
    push_element_chain(model, machine_id, titles);
}

/// The sections of `areas` whose title is one of `titles`, in the order of `titles` (and by
/// Area within one title).
pub fn applicable<'a>(areas: &'a [Area], titles: &[String]) -> Vec<Located<'a>> {
    let mut out: Vec<Located<'a>> = Vec::new();
    for title in titles {
        for area in areas {
            for (fragment, section) in area.sections() {
                let already = out
                    .iter()
                    .any(|l| l.area.id == area.id && l.section.title == section.title);
                if section.title == *title && !already {
                    out.push(Located {
                        area,
                        fragment,
                        section,
                    });
                }
            }
        }
    }
    out
}

/// The `General` sections of an Area (one in the whole Area, in whichever fragment).
pub fn general_sections(area: &Area) -> Vec<Located<'_>> {
    area.sections()
        .filter(|(_, section)| section.title == GENERAL_SECTION)
        .map(|(fragment, section)| Located {
            area,
            fragment,
            section,
        })
        .collect()
}

/// What a search found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hit {
    /// The reference to pass to `weaver model show`; `system` for the System.
    pub reference: String,
    pub kind: &'static str,
    pub title: String,
    /// The first line that holds the text, shortened.
    pub excerpt: String,
}

const EXCERPT_CHARS: usize = 120;

/// The first line of `text` that contains `needle` (lowercase), cut to [`EXCERPT_CHARS`].
fn excerpt(fields: &[&str], needle: &str) -> Option<String> {
    for field in fields {
        for line in field.lines() {
            if line.to_lowercase().contains(needle) {
                let line = line.trim();
                let mut cut: String = line.chars().take(EXCERPT_CHARS).collect();
                if line.chars().count() > EXCERPT_CHARS {
                    cut.push('…');
                }
                return Some(cut);
            }
        }
    }
    None
}

/// Every element, Requirement section and Specs entry whose text contains `text`, compared
/// without regard to case; the whole project in a stable order (System, Actors, Features,
/// Machines with their Events and States, Areas with their sections, Specs).
pub fn search(image: &ProjectImage, text: &str) -> Vec<Hit> {
    let needle = text.trim().to_lowercase();
    let mut hits = Vec::new();
    let mut offer = |reference: String, kind: &'static str, title: &str, fields: &[&str]| {
        if let Some(excerpt) = excerpt(fields, &needle) {
            hits.push(Hit {
                reference,
                kind,
                title: title.to_owned(),
                excerpt,
            });
        }
    };
    let model = image.model();
    if let Some(system) = &model.system {
        offer(
            "system".to_owned(),
            "system",
            &system.title,
            &[&system.title, &system.description],
        );
    }
    for actor in model.actors.values() {
        offer(
            actor.id.clone(),
            "actor",
            &actor.title,
            &[&actor.id, &actor.title, &actor.description],
        );
        for event in &actor.events {
            offer(
                format!("{}.{}", actor.id, event.name),
                "event",
                &event.name,
                &[&event.name, &event.description],
            );
        }
    }
    for (id, feature) in &model.features {
        offer(
            id.clone(),
            "feature",
            &feature.title,
            &[id, &feature.title, &feature.description],
        );
    }
    for machine in model.machines.values() {
        offer(
            machine.id.clone(),
            "machine",
            &machine.title,
            &[&machine.id, &machine.title, &machine.description],
        );
        for event in &machine.events {
            offer(
                format!("{}.{}", machine.id, event.name),
                "event",
                &event.name,
                &[&event.name, &event.description],
            );
        }
        for (_, state) in machine.walk_states() {
            let mut fields: Vec<String> = vec![state.name.clone(), state.description.clone()];
            fields.extend(
                state
                    .transitions
                    .iter()
                    .map(|t| format!("{} → {}: {}", event_ref_text(&t.event), t.target, t.prose)),
            );
            let fields: Vec<&str> = fields.iter().map(String::as_str).collect();
            offer(
                format!("{}/{}", machine.id, state.name),
                "state",
                &state.name,
                &fields,
            );
        }
    }
    for area in image.areas() {
        offer(
            format!("requirements/{}", area.id),
            "area",
            &area.title,
            &[&area.id, &area.title, &area.description, &area.intro],
        );
        for (fragment, section) in area.sections() {
            offer(
                format!("requirements/{}#{}", area.id, section.title),
                "section",
                &format!("{} ({})", section.title, fragment.title),
                &[&section.title, &section.body],
            );
        }
    }
    if let Some(specs) = image.specs() {
        for (field, entries) in &specs.lists {
            for entry in entries {
                let fields = [
                    entry.name.as_str(),
                    entry.version.as_deref().unwrap_or(""),
                    entry.note.as_deref().unwrap_or(""),
                ];
                offer(format!("specs#{field}"), "specs", &entry.name, &fields);
            }
        }
        if let Some(notes) = &specs.notes {
            offer(
                format!("specs#{NOTES_FIELD}"),
                "specs",
                NOTES_FIELD,
                &[notes],
            );
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::project::changes::ChangeSet;
    use crate::project::model::ORDER_MACHINE;
    use crate::project::refs::parse_element_ref;

    fn image() -> ProjectImage {
        let mut changes = ChangeSet::new();
        changes.write("model/system.md", "# Shop\n\nSells things.\n");
        changes.write("model/checkout/feature.md", "# Checkout\n\nBuying.\n");
        changes.write("model/checkout/order.md", ORDER_MACHINE);
        changes.write(
            "model/actors/user.md",
            "# User\n\nA person.\n\n## Events\n\n### CANCEL\nThe user cancels.\n",
        );
        changes.write(
            "model/actors/cart.md",
            "# Cart\n\nThe cart.\n\n## Events\n\n### CONFIRM\nThe cart is confirmed.\n",
        );
        changes.write("model/actors/payment.md", "# Payment\n\nThe gateway.\n\n## Events\n\n### SUCCEEDED\nPaid.\n\n### DECLINED\nRefused.\n\n### REFUNDED\nRefunded.\n");
        changes.write(
            "model/actors/warehouse.md",
            "# Warehouse\n\nThe warehouse.\n\n## Events\n\n### PACKAGE_READY\nReady.\n",
        );
        changes.write(
            "model/actors/courier.md",
            "# Courier\n\nThe courier.\n\n## Events\n\n### PICKED_UP\nPicked up.\n",
        );
        changes.write(
            "requirements/ui/area.md",
            "---\ndescription: Use when building screens.\n---\n# Interface\n",
        );
        changes.write(
            "requirements/ui/checkout.md",
            "---\ndescription: Use when building the checkout screens.\n---\n# Checkout\n\n## General\n- Material 3.\n\n## checkout\n- Cart total at the bottom.\n\n## order\n- Order number in the header.\n\n## order/Paid\n- A green badge.\n\n## order/Preparing\n- A spinner.\n",
        );
        ProjectImage::empty(Path::new("/p")).with_changes(&changes)
    }

    fn titles(image: &ProjectImage, reference: &str) -> Vec<String> {
        applicable_titles(image.model(), &parse_element_ref(reference).unwrap())
    }

    #[test]
    fn the_fixture_has_no_errors() {
        assert!(!image().has_errors(), "{:?}", image().diagnostics());
    }

    #[test]
    fn sections_apply_from_the_element_up_to_its_feature() {
        let image = image();
        assert_eq!(titles(&image, "checkout"), ["checkout"]);
        assert_eq!(titles(&image, "order"), ["order", "checkout"]);
        assert_eq!(
            titles(&image, "order/Paid"),
            ["order/Paid", "order", "checkout"]
        );
        assert_eq!(
            titles(&image, "order/Preparing"),
            ["order/Preparing", "order/Paid", "order", "checkout"]
        );
        assert_eq!(titles(&image, "user.CANCEL"), ["user.CANCEL", "user"]);
        assert_eq!(
            titles(&image, "order/Paid/user.CANCEL"),
            ["order/Paid/user.CANCEL", "order/Paid", "order", "checkout"]
        );
    }

    #[test]
    fn applicable_sections_follow_the_order_of_the_titles_and_skip_general() {
        let image = image();
        let found = applicable(image.areas(), &titles(&image, "order/Paid"));
        let names: Vec<_> = found
            .iter()
            .map(|l| (l.area.id.as_str(), l.section.title.as_str()))
            .collect();
        assert_eq!(
            names,
            [("ui", "order/Paid"), ("ui", "order"), ("ui", "checkout")]
        );
        let ui = &image.areas()[0];
        let general = general_sections(ui);
        assert_eq!(general.len(), 1);
        assert!(general[0].section.body.contains("Material 3"));
    }

    #[test]
    fn search_finds_elements_states_sections_and_ignores_case() {
        let image = image();
        let refs = |text: &str| {
            search(&image, text)
                .into_iter()
                .map(|h| h.reference)
                .collect::<Vec<_>>()
        };
        assert_eq!(refs("sells"), ["system"]);
        assert!(refs("GREEN").contains(&"requirements/ui#order/Paid".to_owned()));
        assert!(refs("refund").contains(&"order/Refunding".to_owned()));
        assert!(refs("cancel").contains(&"user.CANCEL".to_owned()));
        assert!(refs("no such words anywhere").is_empty());
        let hit = &search(&image, "material")[0];
        assert_eq!(hit.kind, "section");
        assert_eq!(hit.excerpt, "- Material 3.");
    }
}
