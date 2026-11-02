//! The scope of an ordinary change or a setup, and what covers it.
//!
//! Scope is computed live, against the Model, the Requirements and the Specs with the required
//! Proposals applied below, and never stored:
//!
//! * `changed`: each added or changed State, Transition and Event; a new Machine as one entry;
//!   a Machine, Feature or Actor whose description changed; each added or changed Requirement
//!   section, General included; each top-level Specs field whose value changed (`specs#<field>`);
//! * `leads into a change`: each Transition, in any Machine, entering a changed State or
//!   reacting to a changed Event;
//! * `removed, its container`: a removed element counts as its nearest existing container
//!   (Transition → State → Machine → Feature; Event → Machine or Actor). Removed Requirement
//!   sections add nothing.
//!
//! Each entry is covered by a Task reference equal to it or containing it (`task` covers
//! `task/To do`; a section by that section or its Area; `specs#<field>` by itself or `specs`),
//! or by a line under `## Already present`. `removed, its container` entries are covered by the
//! code-removal Tasks Weaver creates when the Proposal is approved: one per topmost removed
//! Model element, ``Remove code of `<ref>` ``, referencing the nearest surviving container; a
//! removed Feature or Actor gives a Technical Task with no reference.

use std::collections::{BTreeMap, BTreeSet};

use crate::project::concerns::{self, Key};
use crate::project::image::ProjectImage;
use crate::project::model::{Machine, Model};
use crate::project::refs::{parse_reference, Reference};
use crate::project::specs::SPECS_FIELDS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    Changed,
    LeadsIntoChange,
    RemovedContainer,
}

impl Reason {
    pub fn text(self) -> &'static str {
        match self {
            Reason::Changed => "changed",
            Reason::LeadsIntoChange => "leads into a change",
            Reason::RemovedContainer => "removed, its container",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeEntry {
    pub reference: String,
    pub reason: Reason,
}

/// A code-removal Task Weaver creates when the Proposal is approved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovalTask {
    /// The removed element, as a reference.
    pub removed: String,
    /// The nearest surviving container; `None` for a Feature or an Actor (a Technical Task).
    pub container: Option<String>,
}

impl RemovalTask {
    pub fn title(&self) -> String {
        format!("Remove code of `{}`", self.removed)
    }

    pub fn technical(&self) -> bool {
        self.container.is_none()
    }

    pub fn refs(&self) -> Vec<String> {
        self.container.iter().cloned().collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    pub entries: Vec<ScopeEntry>,
    pub removals: Vec<RemovalTask>,
}

/// What covers a scope entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Coverage {
    /// A Task of the Proposal (its identity).
    Task(String),
    /// A line of `## Already present` (its reference).
    AlreadyPresent(String),
    /// The code-removal Tasks created on approval.
    Removal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoveredEntry {
    pub entry: ScopeEntry,
    pub coverage: Option<Coverage>,
}

#[derive(Default)]
struct Builder {
    entries: Vec<ScopeEntry>,
}

impl Builder {
    fn add(&mut self, reference: String, reason: Reason) {
        match self.entries.iter_mut().find(|e| e.reference == reference) {
            Some(existing) => {
                // `changed` is the strongest reason: it never turns into a weaker one.
                if existing.reason != Reason::Changed && reason == Reason::Changed {
                    existing.reason = Reason::Changed;
                }
            }
            None => self.entries.push(ScopeEntry { reference, reason }),
        }
    }
}

#[derive(PartialEq)]
struct StateShape<'a> {
    parent: Vec<&'a str>,
    marker: crate::project::model::Marker,
    description: &'a str,
}

fn state_shapes(machine: &Machine) -> BTreeMap<&str, StateShape<'_>> {
    machine
        .walk_states()
        .into_iter()
        .map(|(path, state)| {
            let parent = path[..path.len() - 1].to_vec();
            (
                state.name.as_str(),
                StateShape {
                    parent,
                    marker: state.marker,
                    description: state.description.as_str(),
                },
            )
        })
        .collect()
}

/// `(State name, "owner.EVENT")` → the Transitions of that State on that Event, `(target, prose)`.
type Groups = BTreeMap<(String, String), Vec<(String, String)>>;

fn transition_groups(machine: &Machine) -> Groups {
    let mut groups: Groups = BTreeMap::new();
    for (_, state) in machine.walk_states() {
        for transition in &state.transitions {
            let event = format!(
                "{}.{}",
                machine.owner_of(&transition.event),
                transition.event.event
            );
            groups
                .entry((state.name.clone(), event))
                .or_default()
                .push((transition.target.clone(), transition.prose.clone()));
        }
    }
    groups
}

fn requirement_sections(image: &ProjectImage) -> BTreeMap<(String, String), String> {
    let mut sections = BTreeMap::new();
    for area in image.areas() {
        for (_, section) in area.sections() {
            sections.insert(
                (area.id.clone(), section.title.clone()),
                section.body.trim().to_owned(),
            );
        }
    }
    sections
}

/// The scope and the code-removal Tasks of the change that takes `before` to `after`.
pub fn compute(before: &ProjectImage, after: &ProjectImage) -> Scope {
    let (old, new) = (before.model(), after.model());
    let mut scope = Builder::default();
    let mut removals: Vec<RemovalTask> = Vec::new();
    // For the Transitions that lead into a change.
    let mut changed_states: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut changed_events: BTreeSet<String> = BTreeSet::new();

    // --- Features and Actors ---
    for (id, feature) in &new.features {
        if old
            .features
            .get(id)
            .is_none_or(|b| b.description != feature.description)
        {
            scope.add(id.clone(), Reason::Changed);
        }
    }
    for (id, actor) in &new.actors {
        match old.actors.get(id) {
            None => {
                scope.add(id.clone(), Reason::Changed);
                changed_events.extend(actor.events.iter().map(|e| format!("{id}.{}", e.name)));
            }
            Some(before) => {
                if before.description != actor.description {
                    scope.add(id.clone(), Reason::Changed);
                }
                for event in &actor.events {
                    if before
                        .events
                        .iter()
                        .find(|b| b.name == event.name)
                        .is_none_or(|b| b.description != event.description)
                    {
                        scope.add(format!("{id}.{}", event.name), Reason::Changed);
                        changed_events.insert(format!("{id}.{}", event.name));
                    }
                }
            }
        }
    }

    // --- Machines ---
    let mut new_machines: BTreeSet<&str> = BTreeSet::new();
    for (id, machine) in &new.machines {
        let Some(before_machine) = old.machines.get(id) else {
            new_machines.insert(id.as_str());
            scope.add(id.clone(), Reason::Changed);
            changed_events.extend(machine.events.iter().map(|e| format!("{id}.{}", e.name)));
            continue;
        };
        if before_machine.description != machine.description {
            scope.add(id.clone(), Reason::Changed);
        }
        let (old_states, new_states) = (state_shapes(before_machine), state_shapes(machine));
        for (name, shape) in &new_states {
            if old_states.get(name) != Some(shape) {
                scope.add(format!("{id}/{name}"), Reason::Changed);
                changed_states.entry(id.as_str()).or_default().insert(*name);
            }
        }
        let (old_groups, new_groups) = (
            transition_groups(before_machine),
            transition_groups(machine),
        );
        for (key, group) in &new_groups {
            if old_groups.get(key) != Some(group) {
                scope.add(format!("{id}/{}/{}", key.0, key.1), Reason::Changed);
            }
        }
        for event in &machine.events {
            if before_machine
                .events
                .iter()
                .find(|b| b.name == event.name)
                .is_none_or(|b| b.description != event.description)
            {
                scope.add(format!("{id}.{}", event.name), Reason::Changed);
                changed_events.insert(format!("{id}.{}", event.name));
            }
        }

        // Removed: the topmost removed States, Transitions and Events of a Machine that stays.
        for (name, shape) in &old_states {
            if new_states.contains_key(name) {
                continue;
            }
            let parent_removed = shape
                .parent
                .last()
                .is_some_and(|p| !new_states.contains_key(p));
            if parent_removed {
                continue;
            }
            let container = shape
                .parent
                .iter()
                .rev()
                .find(|p| new_states.contains_key(**p))
                .map(|p| format!("{id}/{p}"));
            removals.push(RemovalTask {
                removed: format!("{id}/{name}"),
                container: Some(container.unwrap_or_else(|| id.clone())),
            });
        }
        for key in old_groups.keys() {
            if !new_groups.contains_key(key) && new_states.contains_key(key.0.as_str()) {
                removals.push(RemovalTask {
                    removed: format!("{id}/{}/{}", key.0, key.1),
                    container: Some(format!("{id}/{}", key.0)),
                });
            }
        }
        for event in &before_machine.events {
            if !machine.events.iter().any(|e| e.name == event.name) {
                removals.push(RemovalTask {
                    removed: format!("{id}.{}", event.name),
                    container: Some(id.clone()),
                });
            }
        }
    }

    // --- Removed Features, Actors and Machines ---
    for id in old.features.keys() {
        if !new.features.contains_key(id) {
            removals.push(RemovalTask {
                removed: id.clone(),
                container: None,
            });
        }
    }
    for (id, actor) in &old.actors {
        match new.actors.get(id) {
            None => removals.push(RemovalTask {
                removed: id.clone(),
                container: None,
            }),
            Some(now) => {
                for event in &actor.events {
                    if !now.events.iter().any(|e| e.name == event.name) {
                        removals.push(RemovalTask {
                            removed: format!("{id}.{}", event.name),
                            container: Some(id.clone()),
                        });
                    }
                }
            }
        }
    }
    for (id, machine) in &old.machines {
        if !new.machines.contains_key(id) && new.features.contains_key(&machine.feature) {
            removals.push(RemovalTask {
                removed: id.clone(),
                container: Some(machine.feature.clone()),
            });
        }
    }

    // --- Requirements and Specs ---
    let (old_sections, new_sections) = (requirement_sections(before), requirement_sections(after));
    for (key, body) in &new_sections {
        if old_sections.get(key) != Some(body) {
            scope.add(format!("requirements/{}#{}", key.0, key.1), Reason::Changed);
        }
    }
    for field in SPECS_FIELDS {
        let value = |image: &ProjectImage| {
            image.specs().map(|specs| {
                (
                    specs.has_field(field),
                    specs.entries(field).map(<[_]>::to_vec),
                    if field == "notes" {
                        specs.notes.clone()
                    } else {
                        None
                    },
                )
            })
        };
        let (old_value, new_value) = (value(before), value(after));
        let present = |v: &Option<(bool, _, _)>| v.as_ref().is_some_and(|(has, _, _)| *has);
        if (present(&old_value) || present(&new_value)) && old_value != new_value {
            scope.add(format!("specs#{field}"), Reason::Changed);
        }
    }

    // --- Leads into a change ---
    for (id, machine) in &new.machines {
        if new_machines.contains(id.as_str()) {
            continue;
        }
        for ((state, event), group) in &transition_groups(machine) {
            let enters = group.iter().any(|(target, _)| {
                changed_states
                    .get(id.as_str())
                    .is_some_and(|s| s.contains(target.as_str()))
            });
            if enters || changed_events.contains(event) {
                scope.add(format!("{id}/{state}/{event}"), Reason::LeadsIntoChange);
            }
        }
    }

    let mut entries = scope.entries;
    for removal in &removals {
        if let Some(container) = &removal.container {
            if !entries
                .iter()
                .any(|e| &e.reference == container && e.reason == Reason::RemovedContainer)
            {
                entries.push(ScopeEntry {
                    reference: container.clone(),
                    reason: Reason::RemovedContainer,
                });
            }
        }
    }
    Scope { entries, removals }
}

/// Whether the reference `covering` equals `entry` or contains it.
pub fn covers(model: &Model, covering: &str, entry: &str) -> bool {
    let (Ok(covering), Ok(entry)) = (parse_reference(covering), parse_reference(entry)) else {
        return false;
    };
    if covering == entry {
        return true;
    }
    match (&covering, &entry) {
        (Reference::Element(outer), Reference::Element(inner)) => concerns::contains(
            model,
            &Key::Element(outer.clone()),
            &Key::Element(inner.clone()),
        ),
        (
            Reference::Requirement {
                area,
                section: None,
            },
            Reference::Requirement {
                area: other,
                section: Some(_),
            },
        ) => area == other,
        (Reference::Specs { field: None }, Reference::Specs { field: Some(_) }) => true,
        _ => false,
    }
}

/// The references listed under `## Already present`, one per line ``- `<ref>` — <where>``.
pub fn already_present(body: &str) -> Vec<String> {
    let mut inside = false;
    let mut in_fence = false;
    let mut refs = Vec::new();
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(heading) = line.strip_prefix("## ") {
            inside = heading.trim() == super::data::ALREADY_PRESENT;
            continue;
        }
        if inside {
            if let Some(rest) = line.trim_start().strip_prefix("- `") {
                if let Some((reference, _)) = rest.split_once('`') {
                    refs.push(reference.to_owned());
                }
            }
        }
    }
    refs
}

/// Each entry of `scope` with what covers it: a Task of the Proposal (`tasks`: identity and
/// references), a line of `## Already present`, or the removal Tasks.
pub fn coverage(
    scope: &Scope,
    model: &Model,
    tasks: &[(String, Vec<String>)],
    present: &[String],
) -> Vec<CoveredEntry> {
    scope
        .entries
        .iter()
        .map(|entry| {
            let covered = if entry.reason == Reason::RemovedContainer {
                Some(Coverage::Removal)
            } else if let Some((id, _)) = tasks
                .iter()
                .find(|(_, refs)| refs.iter().any(|r| covers(model, r, &entry.reference)))
            {
                Some(Coverage::Task(id.clone()))
            } else {
                present
                    .iter()
                    .find(|r| covers(model, r, &entry.reference))
                    .map(|r| Coverage::AlreadyPresent(r.clone()))
            };
            CoveredEntry {
                entry: entry.clone(),
                coverage: covered,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::changes::ChangeSet;
    use crate::project::proposal::fixtures::{project, FEATURE, ORDER, USER};

    fn after(changes: &ChangeSet) -> (ProjectImage, ProjectImage) {
        let dir = project();
        let before = ProjectImage::load(dir.path());
        assert!(!before.has_errors(), "{:?}", before.diagnostics());
        let after = before.with_changes(changes);
        (before, after)
    }

    fn refs(scope: &Scope) -> Vec<(String, &'static str)> {
        scope
            .entries
            .iter()
            .map(|e| (e.reference.clone(), e.reason.text()))
            .collect()
    }

    #[test]
    fn nothing_changed_nothing_in_scope() {
        let (before, after) = after(&ChangeSet::new());
        assert_eq!(compute(&before, &after), Scope::default());
    }

    #[test]
    fn a_changed_state_a_transition_and_what_leads_into_it() {
        let mut changes = ChangeSet::new();
        // `Paid` gets a new description; `Draft` still enters it, so its Transition leads into a change.
        changes.write(
            "model/shop/order.md",
            ORDER.replace("### Paid\n", "### Paid\nThe customer paid.\n"),
        );
        let (before, after) = after(&changes);
        let scope = compute(&before, &after);
        assert_eq!(
            refs(&scope),
            [
                ("order/Paid".to_owned(), "changed"),
                (
                    "order/Draft/order.CONFIRM".to_owned(),
                    "leads into a change"
                )
            ]
        );
        assert!(scope.removals.is_empty());
    }

    #[test]
    fn a_changed_transition_event_and_machine_description() {
        let mut changes = ChangeSet::new();
        let text = ORDER
            .replace(
                "refunds the customer.",
                "refunds the customer and says sorry.",
            )
            .replace("The customer confirms the order.", "The customer confirms.")
            .replace("An order placed by a customer.", "An order.");
        changes.write("model/shop/order.md", text);
        let (before, after) = after(&changes);
        assert_eq!(
            refs(&compute(&before, &after)),
            [
                ("order".to_owned(), "changed"),
                ("order/Paid/user.CANCEL".to_owned(), "changed"),
                ("order.CONFIRM".to_owned(), "changed"),
                (
                    "order/Draft/order.CONFIRM".to_owned(),
                    "leads into a change"
                ),
            ]
        );
    }

    #[test]
    fn a_new_machine_is_one_entry_and_the_transitions_that_react_to_its_events_lead_into_a_change()
    {
        let mut changes = ChangeSet::new();
        changes.write("model/shop/payment.md", "# Payment\n\nPay.\n\n## States\n\n### Idle (initial)\n\n## Events\n\n### PAID\nPaid.\n");
        changes.write(
            "model/shop/order.md",
            ORDER.replace(
                "### Shipped (final)",
                "### Shipped (final)\n\n### Awaiting\n- `payment.PAID` → Paid\n",
            ),
        );
        let (before, after) = after(&changes);
        let got = refs(&compute(&before, &after));
        assert!(got.contains(&("payment".to_owned(), "changed")), "{got:?}");
        assert!(
            got.contains(&("order/Awaiting".to_owned(), "changed")),
            "{got:?}"
        );
        assert!(
            got.contains(&("order/Awaiting/payment.PAID".to_owned(), "changed")),
            "{got:?}"
        );
        assert!(
            !got.iter()
                .any(|(r, _)| r.starts_with("payment/") || r == "payment.PAID"),
            "{got:?}"
        );
    }

    #[test]
    fn features_actors_requirements_and_specs() {
        let mut changes = ChangeSet::new();
        changes.write(
            "model/shop/feature.md",
            FEATURE.replace("Buying things.", "Buying and returning."),
        );
        changes.write(
            "model/actors/user.md",
            USER.replace("The user asks to cancel an order.", "The user cancels."),
        );
        changes.write(
            "requirements/ui/main.md",
            "---\ndescription: d\n---\n# Main\n\n## General\n- x\n\n## order/Paid\n- changed\n",
        );
        changes.write(
            "requirements/ui/more.md",
            "---\ndescription: d\n---\n# More\n\n## order/Draft\n- new\n",
        );
        changes.write(
            "specs.yaml",
            "languages:\n  - name: Rust\nframeworks:\n  - name: Tauri\n",
        );
        let (before, after) = after(&changes);
        assert!(!after.has_errors(), "{:?}", after.diagnostics());
        let got = refs(&compute(&before, &after));
        assert_eq!(
            got.iter()
                .filter(|(_, r)| *r == "changed")
                .map(|(e, _)| e.as_str())
                .collect::<Vec<_>>(),
            [
                "shop",
                "user.CANCEL",
                "requirements/ui#order/Draft",
                "requirements/ui#order/Paid",
                "specs#frameworks",
            ]
        );
    }

    #[test]
    fn a_moved_section_and_a_vanishing_fragment_add_nothing() {
        let mut changes = ChangeSet::new();
        changes.write(
            "requirements/ui/main.md",
            "---\ndescription: d\n---\n# Main\n\n## General\n- x\n",
        );
        changes.write(
            "requirements/ui/other.md",
            "---\ndescription: other\n---\n# Other\n\n## order/Paid\n- y\n",
        );
        let (before, after) = after(&changes);
        assert_eq!(compute(&before, &after), Scope::default());
        let mut changes = ChangeSet::new();
        changes.delete("requirements/ui/main.md");
        let (before, after) = self::after(&changes);
        // Both sections vanish with their fragment: removed sections add nothing.
        assert_eq!(compute(&before, &after), Scope::default());
    }

    #[test]
    fn removed_elements_count_as_their_container_with_a_removal_task_each() {
        let mut changes = ChangeSet::new();
        // The State `Paid` and its Transition go away, the Event CONFIRM too.
        let text = ORDER
            .replace(
                "### Paid\n- `user.CANCEL` → Draft: refunds the customer.\n\n",
                "",
            )
            .replace("- `CONFIRM` → Paid: only if the cart is not empty.\n", "")
            .replace(
                "\n## Events\n\n### CONFIRM\nThe customer confirms the order.\n",
                "",
            );
        changes.write("model/shop/order.md", text);
        let (before, after) = after(&changes);
        // The Requirement section of `order/Paid` is left dangling here: scope does not care.
        let scope = compute(&before, &after);
        let titles: Vec<String> = scope.removals.iter().map(RemovalTask::title).collect();
        assert!(
            titles.contains(&"Remove code of `order/Paid`".to_owned()),
            "{titles:?}"
        );
        assert!(
            titles.contains(&"Remove code of `order/Draft/order.CONFIRM`".to_owned()),
            "{titles:?}"
        );
        assert!(
            titles.contains(&"Remove code of `order.CONFIRM`".to_owned()),
            "{titles:?}"
        );
        // `Paid` is topmost: its own Transition is not listed apart.
        assert!(
            !titles.iter().any(|t| t.contains("order/Paid/")),
            "{titles:?}"
        );
        assert_eq!(
            scope
                .entries
                .iter()
                .filter(|e| e.reason == Reason::RemovedContainer)
                .count(),
            2,
            "{:?}",
            scope.entries
        );
        let containers: BTreeSet<&str> = scope
            .removals
            .iter()
            .filter_map(|r| r.container.as_deref())
            .collect();
        assert_eq!(containers, BTreeSet::from(["order", "order/Draft"]));
    }

    #[test]
    fn a_removed_feature_or_actor_is_a_technical_removal_with_no_reference() {
        let mut changes = ChangeSet::new();
        changes.delete("model/actors/user.md");
        changes.write(
            "model/shop/order.md",
            ORDER.replace("user.CANCEL", "CONFIRM"),
        );
        let (before, after) = after(&changes);
        let scope = compute(&before, &after);
        let actor = scope
            .removals
            .iter()
            .find(|r| r.removed == "user")
            .expect("a removal Task");
        assert!(actor.technical() && actor.refs().is_empty());
        assert_eq!(actor.title(), "Remove code of `user`");
        // The Transition on the Actor's Event is replaced in State `Paid`, which stays.
        assert!(scope
            .removals
            .iter()
            .any(|r| r.removed == "order/Paid/user.CANCEL"
                && r.container.as_deref() == Some("order/Paid")));

        let mut changes = ChangeSet::new();
        changes.delete("model/shop/feature.md");
        changes.delete("model/shop/order.md");
        changes.write("model/actors/user.md", USER);
        let (before, after) = self::after(&changes);
        let scope = compute(&before, &after);
        // The Feature is topmost: its Machine is not listed apart.
        assert_eq!(
            scope.removals,
            [RemovalTask {
                removed: "shop".to_owned(),
                container: None
            }]
        );
    }

    #[test]
    fn a_removed_machine_counts_as_its_feature() {
        let mut changes = ChangeSet::new();
        changes.delete("model/shop/order.md");
        let (before, after) = after(&changes);
        let scope = compute(&before, &after);
        assert_eq!(
            scope.removals,
            [RemovalTask {
                removed: "order".to_owned(),
                container: Some("shop".to_owned())
            }]
        );
        assert_eq!(
            refs(&scope),
            [("shop".to_owned(), "removed, its container")]
        );
    }

    #[test]
    fn covering_follows_containment_areas_and_specs() {
        let dir = project();
        let image = ProjectImage::load(dir.path());
        let model = image.model();
        assert!(covers(model, "order", "order/Paid"));
        assert!(covers(model, "order/Paid", "order/Paid/user.CANCEL"));
        assert!(covers(model, "shop", "order"));
        assert!(covers(model, "user", "user.CANCEL"));
        assert!(covers(model, "order", "order.CONFIRM"));
        assert!(!covers(model, "order/Paid", "order"));
        assert!(!covers(model, "order/Draft", "order/Paid"));
        assert!(covers(
            model,
            "requirements/ui",
            "requirements/ui#order/Paid"
        ));
        assert!(covers(
            model,
            "requirements/ui#General",
            "requirements/ui#General"
        ));
        assert!(!covers(
            model,
            "requirements/ui#order/Paid",
            "requirements/ui"
        ));
        assert!(!covers(
            model,
            "requirements/other",
            "requirements/ui#General"
        ));
        assert!(covers(model, "specs", "specs#frameworks"));
        assert!(covers(model, "specs#frameworks", "specs#frameworks"));
        assert!(!covers(model, "specs#data", "specs#frameworks"));
        assert!(!covers(model, "order", "specs#data"));
    }

    #[test]
    fn already_present_lines_cover_like_task_references() {
        let body = "# T\n\nWhy.\n\n## Already present\n\n- `order/Paid` — src/order.rs\n- `specs#data` — Cargo.toml\nnot an item\n\n```\n- `fenced` — no\n```\n\n## Other\n\n- `ignored` — x\n";
        assert_eq!(already_present(body), ["order/Paid", "specs#data"]);
        assert!(already_present("# T\n").is_empty());

        let dir = project();
        let image = ProjectImage::load(dir.path());
        let scope = Scope {
            entries: vec![
                ScopeEntry {
                    reference: "order/Paid".into(),
                    reason: Reason::Changed,
                },
                ScopeEntry {
                    reference: "order".into(),
                    reason: Reason::Changed,
                },
                ScopeEntry {
                    reference: "shop".into(),
                    reason: Reason::RemovedContainer,
                },
            ],
            removals: Vec::new(),
        };
        let tasks = vec![("tttttttt".to_owned(), vec!["order".to_owned()])];
        let covered = coverage(&scope, image.model(), &tasks, &["order/Paid".to_owned()]);
        assert_eq!(
            covered[0].coverage,
            Some(Coverage::Task("tttttttt".into())),
            "a Task wins over a line"
        );
        assert_eq!(covered[1].coverage, Some(Coverage::Task("tttttttt".into())));
        assert_eq!(covered[2].coverage, Some(Coverage::Removal));
        let none = coverage(&scope, image.model(), &[], &["order/Paid".to_owned()]);
        assert_eq!(
            none[0].coverage,
            Some(Coverage::AlreadyPresent("order/Paid".into()))
        );
        assert_eq!(none[1].coverage, None);
    }
}
