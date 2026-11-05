//! The text and JSON of `weaver model show`: the System, an element with the Requirement
//! sections that apply to it, an Area or one of its sections, the Specs.

use serde_json::{json, Value};

use super::browse::{self, Located};
use super::image::ProjectImage;
use super::model::{event_ref_text, ElementKind, Machine, Marker, State};
use super::refs::{parse_reference, ElementRef, Reference, GENERAL_SECTION};
use super::requirements::Area;
use super::specs::{SpecEntry, Specs};

/// What a view is: the Markdown, and the data of `--json` (without the Requirements, which
/// [`show`] adds).
struct View {
    markdown: String,
    data: Value,
}

/// The view of `reference` (`None` or `system`: the System).
pub fn show(image: &ProjectImage, reference: Option<&str>) -> Result<(String, Value), String> {
    let Some(text) = reference.filter(|text| *text != "system") else {
        let view = system_view(image);
        return Ok((view.markdown, view.data));
    };
    let parsed = parse_reference(text)?;
    image.resolver().resolve(&parsed)?;
    let (view, applicable) = match &parsed {
        Reference::Element(element) => {
            let titles = browse::applicable_titles(image.model(), element);
            (
                element_view(image, element),
                browse::applicable(image.areas(), &titles),
            )
        }
        Reference::Requirement { area, section } => (
            requirement_view(image, area, section.as_deref()),
            Vec::new(),
        ),
        Reference::Specs { field } => (
            specs_view(image.specs().expect("resolved"), field.as_deref()),
            Vec::new(),
        ),
    };
    let mut markdown = view.markdown;
    if !applicable.is_empty() {
        markdown.push_str("\n## Requirements\n");
        for located in &applicable {
            markdown.push_str(&format!(
                "\n### `requirements/{}#{}`\n\nArea `{}`, fragment `{}`.\n\n{}\n",
                located.area.id,
                located.section.title,
                located.area.id,
                located.fragment.name,
                located.section.body
            ));
        }
    }
    let mut data = view.data;
    data["reference"] = json!(text);
    data["requirements"] = json!(applicable.iter().map(located_json).collect::<Vec<_>>());
    Ok((markdown, data))
}

fn located_json(located: &Located) -> Value {
    json!({
        "area": located.area.id,
        "fragment": located.fragment.name,
        "section": located.section.title,
        "body": located.section.body,
    })
}

fn title_line(heading: &str, title: &str) -> String {
    format!("# {heading}: {title}\n")
}

fn with_description(mut markdown: String, description: &str) -> String {
    if !description.trim().is_empty() {
        markdown.push_str(&format!("\n{}\n", description.trim()));
    }
    markdown
}

// ---------------------------------------------------------------------------------------------
// The System

fn system_view(image: &ProjectImage) -> View {
    let model = image.model();
    let mut markdown = match &model.system {
        Some(system) => with_description(title_line("System", &system.title), &system.description),
        None => "# System\n\nThe System has no `model/system.md` yet.\n".to_owned(),
    };
    if !model.actors.is_empty() {
        markdown.push_str("\n## Actors\n\n");
        for actor in model.actors.values() {
            markdown.push_str(&format!("- `{}`: {}\n", actor.id, actor.title));
        }
    }
    if !model.features.is_empty() {
        markdown.push_str("\n## Features\n\n");
        for (id, feature) in &model.features {
            let machines: Vec<String> = model
                .machines
                .values()
                .filter(|m| m.feature == *id)
                .map(|m| format!("`{}`", m.id))
                .collect();
            if machines.is_empty() {
                markdown.push_str(&format!("- `{id}`: {}\n", feature.title));
            } else {
                markdown.push_str(&format!(
                    "- `{id}`: {}. Machines: {}\n",
                    feature.title,
                    machines.join(", ")
                ));
            }
        }
    }
    if model.is_empty() {
        markdown.push_str("\nThe Model has no Feature and no Actor yet.\n");
    }
    if !image.areas().is_empty() {
        markdown.push_str("\n## Requirements\n\n");
        for area in image.areas() {
            markdown.push_str(&format!(
                "- `requirements/{}`: {}. {}\n",
                area.id, area.title, area.description
            ));
        }
    }
    if let Some(specs) = image.specs() {
        let fields: Vec<&str> = specs
            .lists
            .iter()
            .map(|(name, _)| name.as_str())
            .chain(specs.notes.iter().map(|_| "notes"))
            .collect();
        markdown.push_str(&format!(
            "\n## Specs\n\n- `specs`: {}\n",
            if fields.is_empty() {
                "no field".to_owned()
            } else {
                fields.join(", ")
            }
        ));
    }
    let data = json!({
        "kind": "system",
        "title": model.system.as_ref().map(|s| s.title.as_str()),
        "description": model.system.as_ref().map(|s| s.description.as_str()),
        "actors": model.actors.values().map(|a| json!({ "ref": a.id, "title": a.title })).collect::<Vec<_>>(),
        "features": model.features.iter().map(|(id, f)| json!({
            "ref": id,
            "title": f.title,
            "machines": model.machines.values().filter(|m| m.feature == *id).map(|m| m.id.as_str()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "areas": image.areas().iter().map(|a| json!({
            "area": a.id,
            "title": a.title,
            "description": a.description,
            "fragments": a.fragments.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "specs": image.specs().is_some(),
    });
    View { markdown, data }
}

// ---------------------------------------------------------------------------------------------
// Elements

fn element_view(image: &ProjectImage, element: &ElementRef) -> View {
    let model = image.model();
    match element {
        ElementRef::Element(id) => match model.element_kind(id).expect("resolved") {
            ElementKind::Feature => {
                let feature = &model.features[id];
                let mut markdown = with_description(
                    title_line(&format!("Feature `{id}`"), &feature.title),
                    &feature.description,
                );
                let machines: Vec<&Machine> = model
                    .machines
                    .values()
                    .filter(|m| m.feature == *id)
                    .collect();
                if !machines.is_empty() {
                    markdown.push_str("\n## Machines\n\n");
                    for machine in &machines {
                        markdown.push_str(&format!("- `{}`: {}\n", machine.id, machine.title));
                    }
                }
                let data = json!({
                    "kind": "feature",
                    "ref": id,
                    "title": feature.title,
                    "description": feature.description,
                    "machines": machines.iter().map(|m| json!({ "ref": m.id, "title": m.title })).collect::<Vec<_>>(),
                });
                View { markdown, data }
            }
            ElementKind::Machine => machine_view(image, &model.machines[id]),
            ElementKind::Actor => {
                let actor = &model.actors[id];
                let mut markdown = with_description(
                    title_line(&format!("Actor `{id}`"), &actor.title),
                    &actor.description,
                );
                markdown.push_str(&events_text(image, &actor.id, &actor.events));
                let data = json!({
                    "kind": "actor",
                    "ref": id,
                    "title": actor.title,
                    "description": actor.description,
                    "events": events_json(image, &actor.id, &actor.events),
                });
                View { markdown, data }
            }
        },
        ElementRef::State { machine, state } => state_view(image, machine, state, None),
        ElementRef::Transitions {
            machine,
            state,
            owner,
            event,
        } => state_view(image, machine, state, Some((owner, event))),
        ElementRef::Event { owner, event } => {
            let declared = image
                .model()
                .declared_events(owner)
                .expect("resolved")
                .iter()
                .find(|e| e.name == *event)
                .expect("resolved");
            let mut markdown = format!("# Event `{owner}.{event}`\n");
            markdown = with_description(markdown, &declared.description);
            let receivers = receivers_of(image, owner, event);
            markdown.push_str(&receivers_text(&receivers));
            let data = json!({
                "kind": "event",
                "owner": owner,
                "event": event,
                "description": declared.description,
                "receivers": receivers_json(&receivers),
            });
            View { markdown, data }
        }
    }
}

/// A Transition that reacts to an Event.
struct Receiver {
    machine: String,
    state: String,
    target: String,
    prose: String,
}

fn receivers_of(image: &ProjectImage, owner: &str, event: &str) -> Vec<Receiver> {
    let mut found = Vec::new();
    for machine in image.model().machines.values() {
        for (_, state) in machine.walk_states() {
            for transition in &state.transitions {
                if machine.owner_of(&transition.event) == owner && transition.event.event == event {
                    found.push(Receiver {
                        machine: machine.id.clone(),
                        state: state.name.clone(),
                        target: transition.target.clone(),
                        prose: transition.prose.clone(),
                    });
                }
            }
        }
    }
    found
}

fn receivers_json(receivers: &[Receiver]) -> Vec<Value> {
    receivers
        .iter()
        .map(|r| json!({ "state": format!("{}/{}", r.machine, r.state), "target": r.target, "prose": r.prose }))
        .collect()
}

fn receivers_text(receivers: &[Receiver]) -> String {
    if receivers.is_empty() {
        return "\nNo Transition reacts to it.\n".to_owned();
    }
    let mut text = String::from("\nReceived by:\n\n");
    for r in receivers {
        text.push_str(&format!(
            "- `{}/{}` → {}{}\n",
            r.machine,
            r.state,
            r.target,
            prose_suffix(&r.prose)
        ));
    }
    text
}

fn prose_suffix(prose: &str) -> String {
    if prose.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", prose.trim())
    }
}

fn events_text(image: &ProjectImage, owner: &str, events: &[super::model::EventDecl]) -> String {
    if events.is_empty() {
        return "\n## Events\n\nNo Event declared.\n".to_owned();
    }
    let mut text = String::from("\n## Events\n");
    for event in events {
        text.push_str(&format!("\n### `{owner}.{}`\n", event.name));
        if !event.description.trim().is_empty() {
            text.push_str(&format!("\n{}\n", event.description.trim()));
        }
        text.push_str(&receivers_text(&receivers_of(image, owner, &event.name)));
    }
    text
}

fn events_json(
    image: &ProjectImage,
    owner: &str,
    events: &[super::model::EventDecl],
) -> Vec<Value> {
    events
        .iter()
        .map(|e| {
            json!({
                "name": e.name,
                "description": e.description,
                "receivers": receivers_json(&receivers_of(image, owner, &e.name)),
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Machines and States

fn marker_text(marker: Marker) -> &'static str {
    match marker {
        Marker::None => "",
        Marker::Initial => " (initial)",
        Marker::Final => " (final)",
    }
}

fn state_lines(state: &State, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    let description = if state.description.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", state.description.trim())
    };
    out.push_str(&format!(
        "{pad}- **{}**{}{description}\n",
        state.name,
        marker_text(state.marker)
    ));
    for transition in &state.transitions {
        out.push_str(&format!(
            "{pad}  - `{}` → {}{}\n",
            event_ref_text(&transition.event),
            transition.target,
            prose_suffix(&transition.prose)
        ));
    }
    for child in &state.children {
        state_lines(child, depth + 1, out);
    }
}

fn machine_view(image: &ProjectImage, machine: &Machine) -> View {
    let mut markdown = with_description(
        title_line(&format!("Machine `{}`", machine.id), &machine.title),
        &machine.description,
    );
    markdown.push_str(&format!(
        "\nFeature: `{}`.\n\n## States\n\n",
        machine.feature
    ));
    for state in &machine.states {
        state_lines(state, 0, &mut markdown);
    }
    markdown.push_str(&events_text(image, &machine.id, &machine.events));
    let data = json!({
        "kind": "machine",
        "ref": machine.id,
        "feature": machine.feature,
        "title": machine.title,
        "description": machine.description,
        "states": machine.states,
        "events": events_json(image, &machine.id, &machine.events),
    });
    View { markdown, data }
}

fn state_view(
    image: &ProjectImage,
    machine_id: &str,
    state_name: &str,
    only_event: Option<(&String, &String)>,
) -> View {
    let machine = &image.model().machines[machine_id];
    let walked = machine.walk_states();
    let (path, state) = walked
        .iter()
        .find(|(_, s)| s.name == state_name)
        .expect("resolved");
    let heading = match only_event {
        Some((owner, event)) => format!("Transitions `{machine_id}/{state_name}/{owner}.{event}`"),
        None => format!("State `{machine_id}/{state_name}`"),
    };
    let mut markdown = format!("# {heading}\n");
    if only_event.is_none() {
        markdown.push_str(&format!(
            "\nIn Machine `{machine_id}`, path {}{}.\n",
            path.iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(" › "),
            marker_text(state.marker)
        ));
        markdown = with_description(markdown, &state.description);
    }
    let shown: Vec<_> = state
        .transitions
        .iter()
        .filter(|t| {
            only_event.is_none_or(|(owner, event)| {
                machine.owner_of(&t.event) == owner && t.event.event == *event
            })
        })
        .collect();
    if shown.is_empty() {
        markdown.push_str("\nNo Transition leaves this State.\n");
    } else {
        markdown.push_str("\nTransitions:\n\n");
        for transition in &shown {
            markdown.push_str(&format!(
                "- `{}` → {}{}\n",
                event_ref_text(&transition.event),
                transition.target,
                prose_suffix(&transition.prose)
            ));
        }
    }
    if only_event.is_none() {
        if !state.children.is_empty() {
            markdown.push_str("\nSub-States:\n\n");
            for child in &state.children {
                markdown.push_str(&format!(
                    "- `{machine_id}/{}`{}\n",
                    child.name,
                    marker_text(child.marker)
                ));
            }
        }
        let entered_from: Vec<(String, String)> = walked
            .iter()
            .flat_map(|(_, s)| {
                s.transitions
                    .iter()
                    .filter(|t| t.target == state_name)
                    .map(move |t| (s.name.clone(), event_ref_text(&t.event)))
            })
            .collect();
        if !entered_from.is_empty() {
            markdown.push_str("\nEntered from:\n\n");
            for (from, event) in &entered_from {
                markdown.push_str(&format!("- `{machine_id}/{from}` on `{event}`\n"));
            }
        }
    }
    let data = json!({
        "kind": if only_event.is_some() { "transitions" } else { "state" },
        "machine": machine_id,
        "state": state_name,
        "path": path,
        "marker": state.marker,
        "description": state.description,
        "transitions": shown,
    });
    View { markdown, data }
}

// ---------------------------------------------------------------------------------------------
// Requirements and Specs

fn requirement_view(image: &ProjectImage, area_id: &str, section: Option<&str>) -> View {
    let area: &Area = image
        .areas()
        .iter()
        .find(|a| a.id == area_id)
        .expect("resolved");
    match section {
        None => {
            let mut markdown = format!(
                "# Area `{}`: {}\n\n{}\n",
                area.id, area.title, area.description
            );
            if !area.intro.trim().is_empty() {
                markdown.push_str(&format!("\n{}\n", area.intro.trim()));
            }
            if area.fragments.is_empty() {
                markdown.push_str("\nThe Area has no fragment yet.\n");
            }
            for fragment in &area.fragments {
                markdown.push_str(&format!(
                    "\n## Fragment `{}`: {}\n\n{} ({} bytes)\n",
                    fragment.name, fragment.title, fragment.description, fragment.size
                ));
                if !fragment.sections.is_empty() {
                    markdown.push('\n');
                    for s in &fragment.sections {
                        markdown.push_str(&format!("- `requirements/{}#{}`\n", area.id, s.title));
                    }
                }
            }
            let data = json!({
                "kind": "area",
                "area": area.id,
                "title": area.title,
                "description": area.description,
                "intro": area.intro,
                "fragments": area.fragments,
            });
            View { markdown, data }
        }
        Some(title) => {
            let (fragment, found) = area
                .sections()
                .find(|(_, s)| s.title == title)
                .expect("resolved");
            let located = Located {
                area,
                fragment,
                section: found,
            };
            let heading = if title == GENERAL_SECTION {
                "General Requirements"
            } else {
                "Requirements"
            };
            let markdown = format!(
                "# {heading} `requirements/{}#{title}`\n\nArea `{}`, fragment `{}`.\n\n{}\n",
                area.id, area.id, fragment.name, found.body
            );
            let data = json!({ "kind": "section", "section": located_json(&located) });
            View { markdown, data }
        }
    }
}

fn entry_line(entry: &SpecEntry) -> String {
    let mut line = format!("- {}", entry.name);
    if let Some(version) = &entry.version {
        line.push_str(&format!(" {version}"));
    }
    if let Some(note) = &entry.note {
        line.push_str(&format!(": {note}"));
    }
    if let Some(by) = &entry.chosen_by_agent {
        line.push_str(&format!(" (chosen by the agent: {by})"));
    }
    line
}

fn specs_view(specs: &Specs, field: Option<&str>) -> View {
    let mut markdown = String::from("# Specs\n");
    let mut data = serde_json::Map::new();
    data.insert("kind".into(), json!("specs"));
    for (name, entries) in &specs.lists {
        if field.is_some_and(|f| f != name) {
            continue;
        }
        markdown.push_str(&format!("\n## {name}\n\n"));
        if entries.is_empty() {
            markdown.push_str("No entry.\n");
        }
        for entry in entries {
            markdown.push_str(&format!("{}\n", entry_line(entry)));
        }
        data.insert(name.clone(), json!(entries));
    }
    if let Some(notes) = &specs.notes {
        if field.is_none_or(|f| f == "notes") {
            markdown.push_str(&format!("\n## notes\n\n{notes}\n"));
            data.insert("notes".into(), json!(notes));
        }
    }
    View {
        markdown,
        data: Value::Object(data),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::project::changes::ChangeSet;

    const ORDER_MACHINE: &str = "# Order

An order placed by a customer, from composition to delivery.

## States

### Draft (initial)
The customer is still choosing what to buy.
- `cart.CONFIRM` → Awaiting payment: only if the cart is not empty.

### Awaiting payment
- `payment.SUCCEEDED` → Paid
- `payment.DECLINED` → Cancelled: tells the customer by email.

### Paid
- `user.CANCEL` → Cancelled: only if nothing has been prepared yet; refunds the customer.
- `user.CANCEL` → Refunding: once preparation has started.

#### Preparing (initial)
- `warehouse.PACKAGE_READY` → Ready to ship

#### Ready to ship
- `courier.PICKED_UP` → Shipped

### Refunding
- `payment.REFUNDED` → Cancelled

### Shipped (final)

### Cancelled (final)

## Events

### CONFIRMED
Emitted when the order leaves Draft.
";

    use super::*;

    fn image() -> ProjectImage {
        let mut changes = ChangeSet::new();
        changes.write("model/system.md", "# Shop\n\nSells things.\n");
        changes.write("model/checkout/feature.md", "# Checkout\n\nBuying.\n");
        changes.write("model/checkout/order.md", ORDER_MACHINE);
        for (actor, event) in [
            ("user", "CANCEL"),
            ("cart", "CONFIRM"),
            ("payment", "SUCCEEDED"),
            ("warehouse", "PACKAGE_READY"),
            ("courier", "PICKED_UP"),
        ] {
            changes.write(
                &format!("model/actors/{actor}.md"),
                format!("# {actor}\n\nActor.\n\n## Events\n\n### {event}\nHappens.\n"),
            );
        }
        changes.write(
            "requirements/ui/area.md",
            "---\ndescription: Use when building screens.\n---\n# Interface\n",
        );
        changes.write(
            "requirements/ui/checkout.md",
            "---\ndescription: Use when building checkout.\n---\n# Checkout\n\n## General\n- Material 3.\n\n## order\n- Order number in the header.\n\n## order/Paid\n- A green badge.\n",
        );
        ProjectImage::empty(Path::new("/p")).with_changes(&changes)
    }

    #[test]
    fn without_a_reference_the_system_is_shown_with_its_actors_features_and_areas() {
        let image = image();
        let (markdown, data) = show(&image, None).unwrap();
        assert!(markdown.starts_with("# System: Shop\n\nSells things.\n"));
        assert!(markdown.contains("## Features\n\n- `checkout`: Checkout. Machines: `order`"));
        assert!(markdown.contains("- `requirements/ui`: Interface. Use when building screens."));
        assert_eq!(data["kind"], "system");
        assert_eq!(show(&image, Some("system")).unwrap().1["kind"], "system");
    }

    #[test]
    fn a_machine_is_followed_by_the_requirements_that_apply() {
        let (markdown, data) = show(&image(), Some("order")).unwrap();
        assert!(markdown.starts_with("# Machine `order`: Order\n"));
        assert!(
            markdown.contains("- **Draft** (initial): The customer is still choosing what to buy.")
        );
        assert!(markdown
            .contains("  - `cart.CONFIRM` → Awaiting payment: only if the cart is not empty."));
        assert!(markdown.contains("\n## Requirements\n"));
        assert!(markdown.contains("### `requirements/ui#order`"));
        assert!(
            !markdown.contains("A green badge"),
            "a State's section does not apply to its Machine"
        );
        assert_eq!(data["kind"], "machine");
        assert_eq!(data["requirements"][0]["section"], "order");
    }

    #[test]
    fn a_state_shows_its_routes_and_the_sections_of_what_contains_it() {
        let (markdown, data) = show(&image(), Some("order/Paid")).unwrap();
        assert!(markdown.starts_with("# State `order/Paid`\n"));
        assert!(markdown.contains("- `user.CANCEL` → Cancelled: only if nothing has been prepared yet; refunds the customer."));
        assert!(
            markdown.contains("Entered from:\n\n- `order/Awaiting payment` on `payment.SUCCEEDED`")
        );
        assert!(
            markdown.contains("A green badge") && markdown.contains("Order number in the header")
        );
        assert_eq!(data["kind"], "state");
        assert_eq!(data["path"], json!(["Paid"]));
    }

    #[test]
    fn an_event_shows_who_reacts_to_it() {
        let (markdown, data) = show(&image(), Some("user.CANCEL")).unwrap();
        assert!(markdown.contains("Received by:"));
        assert!(markdown.contains("- `order/Paid` → Cancelled"));
        assert_eq!(data["receivers"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn a_transition_reference_shows_only_those_transitions() {
        let (markdown, data) = show(&image(), Some("order/Paid/user.CANCEL")).unwrap();
        assert!(markdown.starts_with("# Transitions `order/Paid/user.CANCEL`\n"));
        assert_eq!(data["transitions"].as_array().unwrap().len(), 2);
        assert!(!markdown.contains("Ready to ship"));
    }

    #[test]
    fn an_area_and_its_general_section_can_be_read() {
        let image = image();
        let (area, _) = show(&image, Some("requirements/ui")).unwrap();
        assert!(area.contains("## Fragment `checkout`: Checkout"));
        assert!(area.contains("- `requirements/ui#General`"));
        let (general, data) = show(&image, Some("requirements/ui#General")).unwrap();
        assert!(general.contains("- Material 3."));
        assert_eq!(data["section"]["area"], "ui");
        assert!(show(&image, Some("requirements/ui#order/Paid"))
            .unwrap()
            .0
            .contains("A green badge"));
    }

    #[test]
    fn unknown_or_malformed_references_are_refused() {
        let image = image();
        for bad in [
            "nothing",
            "order/Nothing",
            "requirements/none",
            "requirements/ui#nothing",
            "specs",
            "Bad Ref",
        ] {
            let error = show(&image, Some(bad)).unwrap_err();
            assert!(!error.is_empty(), "{bad}");
        }
    }
}
