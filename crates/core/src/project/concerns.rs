//! "Concerns the same thing": the one function behind the blocking of Tasks, the slots and
//! groups of a Run, and the lookup of the Drifts that touch a Task.
//!
//! Every reference has a [`Key`]: a Model element is itself; `requirements/<area>#<element>` is
//! the element; `requirements/<area>#General` only itself; `requirements/<area>` the Area;
//! `specs#<field>` the field; `specs` the Specs.
//!
//! Containment: a Feature contains its Machines; a Machine its States, Events and their
//! contents; a State its sub-States and the Transitions leaving it; an Actor its Events; an Area
//! only its General section; `specs` each of its fields.
//!
//! Two references concern the same thing when their keys are equal or one contains the other;
//! two cards when any reference of one concerns any reference of the other. A card without
//! references concerns nothing.

use super::cards::Card;
use super::model::{ElementKind, Model, State};
use super::refs::{parse_element_ref, parse_reference, ElementRef, Reference, GENERAL_SECTION};

/// What a reference names, for the comparison of two references.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Element(ElementRef),
    /// `requirements/<area>`: the Area.
    Area(String),
    /// `requirements/<area>#General`: that section only.
    General(String),
    /// `specs`: the Specs.
    Specs,
    /// `specs#<field>`.
    SpecsField(String),
}

/// The key of a reference; `None` for a Requirement section that is not a valid element reference.
pub fn key_of(reference: &Reference) -> Option<Key> {
    Some(match reference {
        Reference::Element(element) => Key::Element(element.clone()),
        Reference::Requirement {
            area,
            section: None,
        } => Key::Area(area.clone()),
        Reference::Requirement {
            area,
            section: Some(section),
        } if section == GENERAL_SECTION => Key::General(area.clone()),
        Reference::Requirement {
            section: Some(section),
            ..
        } => Key::Element(parse_element_ref(section).ok()?),
        Reference::Specs { field: None } => Key::Specs,
        Reference::Specs { field: Some(field) } => Key::SpecsField(field.clone()),
    })
}

/// The keys of the references a card holds in its `refs`; one that does not parse is skipped.
pub fn card_keys(card: &Card) -> Vec<Key> {
    card.list_key("refs")
        .unwrap_or_default()
        .iter()
        .filter_map(|text| parse_reference(text).ok())
        .filter_map(|reference| key_of(&reference))
        .collect()
}

/// Whether `outer` contains `inner` (a key does not contain itself).
pub fn contains(model: &Model, outer: &Key, inner: &Key) -> bool {
    match (outer, inner) {
        (Key::Area(area), Key::General(general)) => area == general,
        (Key::Specs, Key::SpecsField(_)) => true,
        (Key::Element(outer), Key::Element(inner)) => element_contains(model, outer, inner),
        _ => false,
    }
}

/// Whether two keys name the same thing: they are equal or one contains the other.
pub fn keys_concern(model: &Model, a: &Key, b: &Key) -> bool {
    a == b || contains(model, a, b) || contains(model, b, a)
}

/// Whether two lists of keys concern the same thing: any key of one concerns any of the other.
/// An empty list concerns nothing.
pub fn key_lists_concern(model: &Model, a: &[Key], b: &[Key]) -> bool {
    a.iter()
        .any(|x| b.iter().any(|y| keys_concern(model, x, y)))
}

/// Whether two references concern the same thing.
pub fn references_concern(model: &Model, a: &Reference, b: &Reference) -> bool {
    match (key_of(a), key_of(b)) {
        (Some(a), Some(b)) => keys_concern(model, &a, &b),
        _ => false,
    }
}

/// Whether two cards concern the same thing, by their `refs`.
pub fn cards_concern(model: &Model, a: &Card, b: &Card) -> bool {
    key_lists_concern(model, &card_keys(a), &card_keys(b))
}

fn has_descendant(state: &State, name: &str) -> bool {
    state
        .children
        .iter()
        .any(|child| child.name == name || has_descendant(child, name))
}

fn element_contains(model: &Model, outer: &ElementRef, inner: &ElementRef) -> bool {
    let machine_of_inner = |inner: &ElementRef| -> Option<String> {
        match inner {
            ElementRef::Element(id) => model.machines.contains_key(id).then(|| id.clone()),
            ElementRef::State { machine, .. } | ElementRef::Transitions { machine, .. } => {
                Some(machine.clone())
            }
            ElementRef::Event { owner, .. } => {
                model.machines.contains_key(owner).then(|| owner.clone())
            }
        }
    };
    match outer {
        ElementRef::Element(id) => match model.element_kind(id) {
            Some(ElementKind::Feature) => machine_of_inner(inner)
                .and_then(|machine| model.machines.get(&machine))
                .is_some_and(|machine| machine.feature == *id),
            Some(ElementKind::Machine) => match inner {
                ElementRef::Element(_) => false,
                ElementRef::State { machine, .. } | ElementRef::Transitions { machine, .. } => {
                    machine == id
                }
                ElementRef::Event { owner, .. } => owner == id,
            },
            Some(ElementKind::Actor) => {
                matches!(inner, ElementRef::Event { owner, .. } if owner == id)
            }
            None => false,
        },
        ElementRef::State { machine, state } => {
            let Some(found) = model
                .machines
                .get(machine)
                .and_then(|m| m.find_state(state))
            else {
                return false;
            };
            match inner {
                ElementRef::State {
                    machine: other_machine,
                    state: other,
                } => other_machine == machine && other != state && has_descendant(found, other),
                ElementRef::Transitions {
                    machine: other_machine,
                    state: other,
                    ..
                } => other_machine == machine && (other == state || has_descendant(found, other)),
                _ => false,
            }
        }
        ElementRef::Transitions { .. } | ElementRef::Event { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::cards::{parse_card, CardKind, CardName};
    use crate::project::model::{parse_actor, parse_machine, Titled, ORDER_MACHINE, USER_ACTOR};

    const BILLING: &str = "# Billing\n\nHow invoices are issued.\n\n## States\n\n### Open (initial)\n\n## Events\n\n### ISSUED\nEmitted when an invoice is issued.\n";

    fn model() -> Model {
        let mut model = Model::default();
        for feature in ["shop", "finance"] {
            model.features.insert(
                feature.to_owned(),
                Titled {
                    title: feature.to_owned(),
                    description: "A Feature.".to_owned(),
                },
            );
        }
        model.machines.insert(
            "order".to_owned(),
            parse_machine("shop", "order", ORDER_MACHINE).unwrap(),
        );
        model.machines.insert(
            "billing".to_owned(),
            parse_machine("finance", "billing", BILLING).unwrap(),
        );
        model
            .actors
            .insert("user".to_owned(), parse_actor("user", USER_ACTOR).unwrap());
        model
    }

    fn r(text: &str) -> Reference {
        parse_reference(text).unwrap()
    }

    fn concern(a: &str, b: &str) -> bool {
        let model = model();
        let forward = references_concern(&model, &r(a), &r(b));
        assert_eq!(
            forward,
            references_concern(&model, &r(b), &r(a)),
            "`{a}` and `{b}` must be symmetric"
        );
        forward
    }

    #[test]
    fn the_key_of_each_kind_of_reference() {
        assert_eq!(
            key_of(&r("order")),
            Some(Key::Element(ElementRef::Element("order".into())))
        );
        assert_eq!(
            key_of(&r("requirements/ui#order/Paid")),
            Some(Key::Element(ElementRef::State {
                machine: "order".into(),
                state: "Paid".into()
            }))
        );
        assert_eq!(
            key_of(&r("requirements/ui#user.CANCEL")),
            key_of(&r("user.CANCEL"))
        );
        assert_eq!(
            key_of(&r("requirements/ui#General")),
            Some(Key::General("ui".into()))
        );
        assert_eq!(key_of(&r("requirements/ui")), Some(Key::Area("ui".into())));
        assert_eq!(
            key_of(&r("specs#frameworks")),
            Some(Key::SpecsField("frameworks".into()))
        );
        assert_eq!(key_of(&r("specs")), Some(Key::Specs));
        let broken = Reference::Requirement {
            area: "ui".into(),
            section: Some("not valid".into()),
        };
        assert_eq!(key_of(&broken), None);
    }

    #[test]
    fn an_element_concerns_itself_and_not_another() {
        assert!(concern("order", "order"));
        assert!(concern("order/Paid", "order/Paid"));
        assert!(concern("order/Paid/user.CANCEL", "order/Paid/user.CANCEL"));
        assert!(concern("user.CANCEL", "user.CANCEL"));
        assert!(!concern("order/Paid", "order/Shipped"));
        assert!(!concern("order", "billing"));
        assert!(!concern("user.CANCEL", "order.CONFIRMED"));
    }

    #[test]
    fn a_feature_contains_its_machines_and_what_they_contain() {
        assert!(concern("shop", "order"));
        assert!(concern("shop", "order/Paid"));
        assert!(concern("shop", "order/Paid/user.CANCEL"));
        assert!(concern("shop", "order.CONFIRMED"));
        assert!(!concern("shop", "billing"));
        assert!(!concern("finance", "order/Paid"));
        assert!(!concern("shop", "finance"));
        assert!(!concern("shop", "user"), "an Actor is not inside a Feature");
    }

    #[test]
    fn a_machine_contains_its_states_events_and_their_contents() {
        assert!(concern("order", "order/Paid"));
        assert!(concern("order", "order/Preparing"));
        assert!(concern("order", "order/Paid/user.CANCEL"));
        assert!(concern("order", "order.CONFIRMED"));
        assert!(!concern("order", "billing.ISSUED"));
        assert!(
            !concern("order", "user.CANCEL"),
            "an Event of an Actor is not inside a Machine"
        );
        assert!(!concern("billing", "order/Paid"));
    }

    #[test]
    fn a_state_contains_its_sub_states_and_the_transitions_leaving_it() {
        assert!(concern("order/Paid", "order/Preparing"));
        assert!(concern("order/Paid", "order/Ready to ship"));
        assert!(concern("order/Paid", "order/Paid/user.CANCEL"));
        assert!(concern(
            "order/Paid",
            "order/Preparing/warehouse.PACKAGE_READY"
        ));
        assert!(concern(
            "order/Preparing",
            "order/Preparing/warehouse.PACKAGE_READY"
        ));
        assert!(
            concern("order/Preparing", "order/Paid"),
            "containment counts from either side"
        );
    }

    #[test]
    fn a_sub_state_does_not_reach_its_siblings_or_other_states() {
        assert!(!concern("order/Preparing", "order/Ready to ship"));
        assert!(!concern("order/Paid", "order/Refunding"));
        assert!(!concern("order/Paid", "order/Refunding/payment.REFUNDED"));
        assert!(!concern("order/Paid/user.CANCEL", "order/Preparing"));
    }

    #[test]
    fn an_actor_contains_its_events_only() {
        assert!(concern("user", "user.CANCEL"));
        assert!(!concern("user", "order"));
        assert!(
            !concern("user", "order/Paid/user.CANCEL"),
            "a Transition is inside its State, not inside the Event it reacts to"
        );
        assert!(!concern("user", "payment.SUCCEEDED"));
    }

    #[test]
    fn a_requirement_section_titled_with_an_element_concerns_that_element() {
        assert!(concern("requirements/ui#order/Paid", "order/Paid"));
        assert!(concern("requirements/ui#order/Paid", "order"));
        assert!(concern("requirements/ui#order/Paid", "shop"));
        assert!(concern(
            "requirements/ui#order/Paid",
            "requirements/data#order/Paid"
        ));
        assert!(concern("requirements/ui#order/Paid", "order/Preparing"));
        assert!(!concern("requirements/ui#order/Paid", "order/Refunding"));
        assert!(concern("requirements/ui#user.CANCEL", "user"));
    }

    #[test]
    fn a_general_section_concerns_only_itself_and_its_area() {
        assert!(concern(
            "requirements/ui#General",
            "requirements/ui#General"
        ));
        assert!(concern("requirements/ui#General", "requirements/ui"));
        assert!(!concern(
            "requirements/ui#General",
            "requirements/data#General"
        ));
        assert!(!concern("requirements/ui#General", "order"));
        assert!(!concern("requirements/ui#General", "shop"));
        assert!(!concern(
            "requirements/ui#General",
            "requirements/ui#order/Paid"
        ));
    }

    #[test]
    fn an_area_contains_only_its_general_section() {
        assert!(concern("requirements/ui", "requirements/ui"));
        assert!(!concern("requirements/ui", "requirements/data"));
        assert!(
            !concern("requirements/ui", "requirements/ui#order/Paid"),
            "a section titled with an element is that element, not part of the Area"
        );
        assert!(!concern("requirements/ui", "order"));
    }

    #[test]
    fn the_specs_contain_each_field() {
        assert!(concern("specs", "specs"));
        assert!(concern("specs", "specs#frameworks"));
        assert!(concern("specs#frameworks", "specs#frameworks"));
        assert!(!concern("specs#frameworks", "specs#languages"));
        assert!(!concern("specs", "requirements/ui#General"));
        assert!(!concern("specs#languages", "order"));
    }

    #[test]
    fn references_that_no_longer_exist_keep_their_container_and_nothing_else() {
        assert!(concern("order/Gone", "order/Gone"));
        assert!(
            concern("order/Gone", "order"),
            "a Machine contains its States, by name"
        );
        assert!(!concern("order/Gone", "order/Paid"));
        assert!(concern("ghost", "ghost"));
        assert!(!concern("ghost", "shop"));
    }

    fn card(refs: &[&str]) -> Card {
        let list: String = refs.iter().map(|r| format!("  - {r}\n")).collect();
        let front = if refs.is_empty() {
            "refs: []\n".to_owned()
        } else {
            format!("refs:\n{list}")
        };
        let text =
            format!("---\ncolumn: todo\n{front}created: 2026-11-02T07:00:01.000Z\n---\n# T\n");
        parse_card(
            CardName {
                kind: CardKind::Task,
                id: "aaaaaaaa",
                slug: "t",
                parent: None,
                path: "tasks/aaaaaaaa-t.md",
            },
            &text,
        )
        .unwrap()
    }

    #[test]
    fn two_cards_concern_the_same_thing_when_any_reference_of_one_concerns_any_of_the_other() {
        let model = model();
        assert!(cards_concern(
            &model,
            &card(&["billing", "order/Paid"]),
            &card(&["shop"])
        ));
        assert!(cards_concern(
            &model,
            &card(&["specs", "user"]),
            &card(&["order", "specs#frameworks"])
        ));
        assert!(!cards_concern(
            &model,
            &card(&["billing"]),
            &card(&["order/Paid", "user.CANCEL"])
        ));
    }

    #[test]
    fn a_card_without_references_concerns_nothing_not_even_itself() {
        let model = model();
        let empty = card(&[]);
        assert!(!cards_concern(&model, &empty, &empty));
        assert!(!cards_concern(&model, &empty, &card(&["order"])));
        assert!(!cards_concern(&model, &card(&["order"]), &empty));
    }

    #[test]
    fn a_reference_that_does_not_parse_is_ignored() {
        let model = model();
        let odd = card(&["'not a reference'"]);
        assert!(card_keys(&odd).is_empty());
        assert!(!cards_concern(&model, &odd, &card(&["order"])));
    }
}
