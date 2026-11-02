//! References to elements, Requirement sections and Specs fields, as `refs` fields and CLI
//! arguments write them:
//!
//! | Reference | Names |
//! |---|---|
//! | `checkout`, `order`, `user` | a Feature, Machine or Actor |
//! | `order/Paid` | a State, at any depth |
//! | `order/Paid/user.CANCEL` | the Transitions of that State on that Event |
//! | `user.CANCEL` | an Event of a Machine or Actor |
//! | `requirements/ui`, `requirements/ui#order/Paid`, `requirements/ui#General` | an Area, or one of its sections |
//! | `specs`, `specs#frameworks` | the Specs, or one field |
//!
//! [`parse_reference`] checks the syntax; [`Resolver::resolve`] checks that the target exists.

use super::model::{is_element_id, is_event_name, Model};
use super::requirements::Area;
use super::specs::{Specs, SPECS_FIELDS};

/// A reference to a Model element.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElementRef {
    /// A Feature, Machine or Actor.
    Element(String),
    State {
        machine: String,
        state: String,
    },
    /// The Transitions of `state` reacting to `owner.event`.
    Transitions {
        machine: String,
        state: String,
        owner: String,
        event: String,
    },
    Event {
        owner: String,
        event: String,
    },
}

/// Any reference a Task, a Drift or a Requirement section may hold.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reference {
    Element(ElementRef),
    /// `requirements/<area>`, or with `#<section>` (`General` or an element reference).
    Requirement {
        area: String,
        section: Option<String>,
    },
    /// `specs`, or with `#<field>`.
    Specs {
        field: Option<String>,
    },
}

/// The heading of the General Requirement section.
pub const GENERAL_SECTION: &str = "General";

impl std::fmt::Display for ElementRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElementRef::Element(id) => f.write_str(id),
            ElementRef::State { machine, state } => write!(f, "{machine}/{state}"),
            ElementRef::Transitions {
                machine,
                state,
                owner,
                event,
            } => write!(f, "{machine}/{state}/{owner}.{event}"),
            ElementRef::Event { owner, event } => write!(f, "{owner}.{event}"),
        }
    }
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reference::Element(e) => e.fmt(f),
            Reference::Requirement {
                area,
                section: None,
            } => write!(f, "requirements/{area}"),
            Reference::Requirement {
                area,
                section: Some(s),
            } => write!(f, "requirements/{area}#{s}"),
            Reference::Specs { field: None } => f.write_str("specs"),
            Reference::Specs { field: Some(field) } => write!(f, "specs#{field}"),
        }
    }
}

/// Parses an element reference (the title of a Requirement section is one).
pub fn parse_element_ref(text: &str) -> Result<ElementRef, String> {
    if text.is_empty() || text != text.trim() {
        return Err(format!(
            "`{text}` is not a reference: it is empty or has spaces at its ends"
        ));
    }
    if text.contains('/') {
        let parts: Vec<&str> = text.split('/').collect();
        if parts.iter().any(|p| p.is_empty()) || parts.len() > 3 {
            return Err(format!("`{text}` is not a reference: expected `machine/State` or `machine/State/owner.EVENT`"));
        }
        let machine = parts[0];
        if !is_element_id(machine) {
            return Err(format!(
                "`{machine}` in `{text}` is not a valid id (kebab-case)"
            ));
        }
        return match parts.as_slice() {
            [_, state] => Ok(ElementRef::State {
                machine: machine.to_owned(),
                state: (*state).to_owned(),
            }),
            [_, state, event] => {
                let (owner, event) = split_event(event)
                    .ok_or_else(|| format!("`{event}` in `{text}` is not `owner.EVENT`"))?;
                Ok(ElementRef::Transitions {
                    machine: machine.to_owned(),
                    state: (*state).to_owned(),
                    owner: owner.to_owned(),
                    event: event.to_owned(),
                })
            }
            _ => unreachable!("length checked"),
        };
    }
    if text.contains('.') {
        let (owner, event) =
            split_event(text).ok_or_else(|| format!("`{text}` is not `owner.EVENT`"))?;
        return Ok(ElementRef::Event {
            owner: owner.to_owned(),
            event: event.to_owned(),
        });
    }
    if is_element_id(text) {
        Ok(ElementRef::Element(text.to_owned()))
    } else {
        Err(format!("`{text}` is not a valid id (kebab-case)"))
    }
}

fn split_event(text: &str) -> Option<(&str, &str)> {
    let (owner, event) = text.split_once('.')?;
    (is_element_id(owner) && is_event_name(event)).then_some((owner, event))
}

/// Parses any reference.
pub fn parse_reference(text: &str) -> Result<Reference, String> {
    if let Some(rest) = text.strip_prefix("requirements/") {
        let (area, section) = match rest.split_once('#') {
            Some((area, section)) => (area, Some(section)),
            None => (rest, None),
        };
        if !is_element_id(area) {
            return Err(format!(
                "`{area}` in `{text}` is not an Area id (kebab-case)"
            ));
        }
        let section = match section {
            None => None,
            Some(GENERAL_SECTION) => Some(GENERAL_SECTION.to_owned()),
            Some(s) => {
                parse_element_ref(s).map_err(|e| format!("section of `{text}`: {e}"))?;
                Some(s.to_owned())
            }
        };
        return Ok(Reference::Requirement {
            area: area.to_owned(),
            section,
        });
    }
    if text == "specs" {
        return Ok(Reference::Specs { field: None });
    }
    if let Some(field) = text.strip_prefix("specs#") {
        if !SPECS_FIELDS.contains(&field) {
            return Err(format!(
                "`{field}` is not a Specs field ({})",
                SPECS_FIELDS.join(", ")
            ));
        }
        return Ok(Reference::Specs {
            field: Some(field.to_owned()),
        });
    }
    parse_element_ref(text).map(Reference::Element)
}

/// What a reference points at, once found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolved {
    Feature,
    Machine,
    Actor,
    State,
    Transitions,
    Event,
    Area,
    Section,
    Specs,
    SpecsField,
}

/// Looks references up in the Model, the Requirements and the Specs.
pub struct Resolver<'a> {
    pub model: &'a Model,
    pub areas: &'a [Area],
    pub specs: Option<&'a Specs>,
}

impl Resolver<'_> {
    /// Finds the target of `reference`, or says why it does not exist.
    pub fn resolve(&self, reference: &Reference) -> Result<Resolved, String> {
        match reference {
            Reference::Element(e) => self.resolve_element(e),
            Reference::Requirement { area, section } => {
                let found = self
                    .areas
                    .iter()
                    .find(|a| a.id == *area)
                    .ok_or_else(|| format!("there is no Requirements Area `{area}`"))?;
                match section {
                    None => Ok(Resolved::Area),
                    Some(section) if found.has_section(section) => Ok(Resolved::Section),
                    Some(section) => Err(format!("Area `{area}` has no section `{section}`")),
                }
            }
            Reference::Specs { field } => match (self.specs, field) {
                (None, _) => Err("the project has no Specs".to_owned()),
                (Some(_), None) => Ok(Resolved::Specs),
                (Some(specs), Some(field)) if specs.has_field(field) => Ok(Resolved::SpecsField),
                (Some(_), Some(field)) => Err(format!("the Specs have no `{field}` field")),
            },
        }
    }

    /// Parses `text` and resolves it.
    pub fn resolve_text(&self, text: &str) -> Result<Resolved, String> {
        self.resolve(&parse_reference(text)?)
    }

    pub fn resolve_element(&self, element: &ElementRef) -> Result<Resolved, String> {
        let model = self.model;
        let machine_of = |id: &str| {
            model
                .machines
                .get(id)
                .ok_or_else(|| match model.element_kind(id) {
                    Some(_) => format!("`{id}` is not a Machine"),
                    None => format!("there is no Machine `{id}`"),
                })
        };
        match element {
            ElementRef::Element(id) => match model.element_kind(id) {
                Some(super::model::ElementKind::Feature) => Ok(Resolved::Feature),
                Some(super::model::ElementKind::Machine) => Ok(Resolved::Machine),
                Some(super::model::ElementKind::Actor) => Ok(Resolved::Actor),
                None => Err(format!("there is no Feature, Machine or Actor `{id}`")),
            },
            ElementRef::State { machine, state } => {
                let m = machine_of(machine)?;
                m.find_state(state)
                    .map(|_| Resolved::State)
                    .ok_or_else(|| format!("Machine `{machine}` has no State `{state}`"))
            }
            ElementRef::Transitions {
                machine,
                state,
                owner,
                event,
            } => {
                let m = machine_of(machine)?;
                let s = m
                    .find_state(state)
                    .ok_or_else(|| format!("Machine `{machine}` has no State `{state}`"))?;
                if s.transitions
                    .iter()
                    .any(|t| m.owner_of(&t.event) == owner && t.event.event == *event)
                {
                    Ok(Resolved::Transitions)
                } else {
                    Err(format!(
                        "State `{machine}/{state}` has no Transition on `{owner}.{event}`"
                    ))
                }
            }
            ElementRef::Event { owner, event } => match model.declared_events(owner) {
                None => Err(format!("there is no Machine or Actor `{owner}`")),
                Some(events) if events.iter().any(|e| e.name == *event) => Ok(Resolved::Event),
                Some(_) => Err(format!("`{owner}` does not declare the Event `{event}`")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::{
        parse_actor, parse_machine, ModelFiles, ORDER_MACHINE, USER_ACTOR,
    };
    use crate::project::requirements::{assemble_areas, parse_area_doc, parse_fragment};

    #[test]
    fn element_references_parse_and_print_back() {
        let cases = [
            ("checkout", ElementRef::Element("checkout".into())),
            (
                "order/Paid",
                ElementRef::State {
                    machine: "order".into(),
                    state: "Paid".into(),
                },
            ),
            (
                "order/Ready to ship",
                ElementRef::State {
                    machine: "order".into(),
                    state: "Ready to ship".into(),
                },
            ),
            (
                "order/Paid/user.CANCEL",
                ElementRef::Transitions {
                    machine: "order".into(),
                    state: "Paid".into(),
                    owner: "user".into(),
                    event: "CANCEL".into(),
                },
            ),
            (
                "user.CANCEL",
                ElementRef::Event {
                    owner: "user".into(),
                    event: "CANCEL".into(),
                },
            ),
        ];
        for (text, expected) in cases {
            let parsed = parse_element_ref(text).unwrap();
            assert_eq!(parsed, expected, "{text}");
            assert_eq!(parsed.to_string(), text);
        }
    }

    #[test]
    fn malformed_element_references_are_refused() {
        for bad in [
            "",
            " order",
            "order ",
            "Order",
            "order/",
            "/Paid",
            "order//x",
            "a/b/c/d",
            "order/Paid/CANCEL",
            "order/Paid/user.cancel",
            "user.cancel",
            "User.CANCEL",
            "a.b.C",
            "bad_id",
        ] {
            assert!(parse_element_ref(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn requirement_and_specs_references() {
        assert_eq!(
            parse_reference("requirements/ui").unwrap(),
            Reference::Requirement {
                area: "ui".into(),
                section: None
            }
        );
        assert_eq!(
            parse_reference("requirements/ui#order/Paid")
                .unwrap()
                .to_string(),
            "requirements/ui#order/Paid"
        );
        assert_eq!(
            parse_reference("requirements/ui#General").unwrap(),
            Reference::Requirement {
                area: "ui".into(),
                section: Some("General".into())
            }
        );
        assert_eq!(
            parse_reference("specs").unwrap(),
            Reference::Specs { field: None }
        );
        assert_eq!(
            parse_reference("specs#frameworks").unwrap(),
            Reference::Specs {
                field: Some("frameworks".into())
            }
        );
        for bad in [
            "requirements/",
            "requirements/Bad",
            "requirements/ui#",
            "requirements/ui#bad_id",
            "specs#nope",
            "specs#",
        ] {
            assert!(parse_reference(bad).is_err(), "{bad}");
        }
    }

    fn fixture() -> (Model, Vec<Area>, Specs) {
        let order = parse_machine("shop", "order", ORDER_MACHINE).unwrap();
        let user = parse_actor("user", USER_ACTOR).unwrap();
        let feature = crate::project::model::parse_titled("# Shop\n").unwrap();
        let files = ModelFiles {
            system: None,
            actors: vec![("model/actors/user.md", &user)],
            features: vec![("model/shop/feature.md", "shop", &feature)],
            machines: vec![("model/shop/order.md", &order)],
        };
        let (model, _) = Model::assemble(&files);
        let area = parse_area_doc("---\ndescription: d\n---\n# Interface\n").unwrap();
        let frag = parse_fragment(
            "main",
            "---\ndescription: d\n---\n# Main\n\n## General\n- x\n\n## order/Paid\n- y\n",
        )
        .unwrap();
        let (areas, diags) = assemble_areas(
            &[("requirements/ui/area.md", "ui", &area)],
            &[("requirements/ui/main.md", "ui", &frag)],
        );
        assert!(diags.is_empty(), "{diags:?}");
        let specs = crate::project::specs::parse_specs("frameworks:\n  - name: React\n").unwrap();
        (model, areas, specs)
    }

    #[test]
    fn references_resolve_against_the_project() {
        let (model, areas, specs) = fixture();
        let r = Resolver {
            model: &model,
            areas: &areas,
            specs: Some(&specs),
        };
        let ok = [
            ("shop", Resolved::Feature),
            ("order", Resolved::Machine),
            ("user", Resolved::Actor),
            ("order/Paid", Resolved::State),
            ("order/Preparing", Resolved::State),
            ("order/Paid/user.CANCEL", Resolved::Transitions),
            ("order/Draft/cart.CONFIRM", Resolved::Transitions),
            ("user.CANCEL", Resolved::Event),
            ("order.CONFIRMED", Resolved::Event),
            ("requirements/ui", Resolved::Area),
            ("requirements/ui#General", Resolved::Section),
            ("requirements/ui#order/Paid", Resolved::Section),
            ("specs", Resolved::Specs),
            ("specs#frameworks", Resolved::SpecsField),
        ];
        for (text, expected) in ok {
            assert_eq!(r.resolve_text(text), Ok(expected), "{text}");
        }
        let missing = [
            "nothing",
            "order/Nowhere",
            "shop/Paid",
            "order/Paid/user.NOPE",
            "order/Draft/user.CANCEL",
            "user.NOPE",
            "ghost.CANCEL",
            "requirements/api",
            "requirements/ui#order/Draft",
            "specs#languages",
        ];
        for text in missing {
            assert!(r.resolve_text(text).is_err(), "{text}");
        }
        let none = Resolver {
            model: &model,
            areas: &areas,
            specs: None,
        };
        assert!(none.resolve_text("specs").is_err());
    }
}
