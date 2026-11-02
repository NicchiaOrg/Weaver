//! Rules that need several files at once and are not done by assembling them.
//!
//! Together with the per-file parsers and [`Model::assemble`] / [`assemble_areas`] these are
//! the rules of `trama model check`.
//!
//! [`assemble_areas`]: super::requirements::assemble_areas

use std::collections::BTreeMap;

use super::cards::{Card, CardKind};
use super::diagnostic::Diagnostic;
use super::layout::REQUIREMENTS_DIR;
use super::model::Model;
use super::refs::{parse_element_ref, Resolver, GENERAL_SECTION};
use super::requirements::Area;
use super::specs::Specs;

/// The path (relative to `.trama/`) of a fragment of an Area.
pub fn fragment_path(area: &str, fragment: &str) -> String {
    format!("{REQUIREMENTS_DIR}/{area}/{fragment}.md")
}

/// Every section other than `General` must name an element that exists in the Model.
pub fn check_sections(model: &Model, areas: &[Area], specs: Option<&Specs>) -> Vec<Diagnostic> {
    let resolver = Resolver {
        model,
        areas,
        specs,
    };
    let mut out = Vec::new();
    for area in areas {
        for (fragment, section) in area.sections() {
            if section.title == GENERAL_SECTION {
                continue;
            }
            let result = parse_element_ref(&section.title)
                .and_then(|r| resolver.resolve_element(&r).map(|_| ()));
            if let Err(reason) = result {
                out.push(Diagnostic::error(
                    fragment_path(&area.id, &fragment.name),
                    format!(
                        "section `{}` names an element that does not exist: {reason}",
                        section.title
                    ),
                ));
            }
        }
    }
    out
}

/// Two cards of the same type with the same identity: two files of `tasks/`, `drifts/` or
/// `history/` (or two Proposal folders, or two Tasks of one Proposal) whose names carry the
/// same identity. The same Task identity in `tasks/` and in a Proposal's `tasks/` is how an
/// approved Proposal's Task looks and is not reported.
pub fn check_duplicate_identities(cards: &[&Card]) -> Vec<Diagnostic> {
    let mut by_key: BTreeMap<(CardKind, Option<&str>, &str), Vec<&str>> = BTreeMap::new();
    for card in cards {
        by_key
            .entry((card.kind, card.parent.as_deref(), card.id.as_str()))
            .or_default()
            .push(card.path.as_str());
    }
    let mut out = Vec::new();
    for ((_, _, id), paths) in by_key {
        if paths.len() > 1 {
            for path in &paths {
                out.push(Diagnostic::error(
                    *path,
                    format!(
                        "identity `{id}` is used by {} files of the same type: {}",
                        paths.len(),
                        paths.join(", ")
                    ),
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::{parse_machine, parse_titled, ModelFiles, ORDER_MACHINE};
    use crate::project::requirements::{assemble_areas, parse_area_doc, parse_fragment};

    #[test]
    fn a_section_must_name_an_element_that_exists() {
        let order = parse_machine("shop", "order", ORDER_MACHINE).unwrap();
        let shop = parse_titled("# Shop\n").unwrap();
        let (model, _) = Model::assemble(&ModelFiles {
            system: None,
            actors: vec![],
            features: vec![("model/shop/feature.md", "shop", &shop)],
            machines: vec![("model/shop/order.md", &order)],
        });
        let area = parse_area_doc("---\ndescription: d\n---\n# A\n").unwrap();
        let frag = parse_fragment(
            "main",
            "---\ndescription: d\n---\n# F\n\n## General\n- ok\n\n## shop\n- ok\n\n## order/Paid\n- ok\n\n## order/Gone\n- bad\n\n## ghost\n- bad\n\n## order/Paid/user.CANCEL\n- bad: no user Actor\n",
        )
        .unwrap();
        let (areas, _) = assemble_areas(
            &[("requirements/a/area.md", "a", &area)],
            &[("requirements/a/main.md", "a", &frag)],
        );
        let diags = check_sections(&model, &areas, None);
        let named: Vec<_> = diags
            .iter()
            .map(|d| d.message.split('`').nth(1).unwrap())
            .collect();
        // `order/Paid/user.CANCEL` does exist as a Transition of the Machine, whoever owns the Event.
        assert_eq!(named, ["order/Gone", "ghost"], "{diags:?}");
        assert!(diags.iter().all(|d| d.path == "requirements/a/main.md"));
    }

    fn card(kind: CardKind, id: &str, path: &str, parent: Option<&str>) -> Card {
        Card {
            kind,
            id: id.to_owned(),
            slug: "x".into(),
            parent: parent.map(Into::into),
            path: path.to_owned(),
            title: "T".into(),
            body: String::new(),
            created: None,
            front: serde_yaml::Mapping::new(),
        }
    }

    #[test]
    fn duplicate_identities_are_reported_per_type_only() {
        let a = card(CardKind::Task, "aaaaaaaa", "tasks/aaaaaaaa-one.md", None);
        let b = card(CardKind::Task, "aaaaaaaa", "tasks/aaaaaaaa-two.md", None);
        // An approved Proposal's Task has the same identity in its own folder: fine.
        let c = card(
            CardKind::ProposalTask,
            "aaaaaaaa",
            "proposals/pppppppp-x/tasks/aaaaaaaa-one.md",
            Some("pppppppp"),
        );
        // A Drift may carry the identity of a Task: another type.
        let d = card(CardKind::Drift, "aaaaaaaa", "drifts/aaaaaaaa-x.md", None);
        let diags = check_duplicate_identities(&[&a, &b, &c, &d]);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].path, "tasks/aaaaaaaa-one.md");
        assert!(diags[0].message.contains("aaaaaaaa-two.md"));
        assert!(check_duplicate_identities(&[&a, &c, &d]).is_empty());
    }
}
