//! The Model of the current and of the shown image as one tree.
//!
//! Without an overlay the tree is the Model with no change. With one, an element only the shown
//! image has is `added`, one only the current image has is `removed` (and stays in the tree, so
//! it can be drawn), and one both have is `changed` when its text differs. A container is
//! `changed` only when its own text changed; the changes inside it show on what changed.

use std::collections::BTreeMap;

use crate::project::model::{
    Actor, EventDecl, EventRef, Machine, Marker, Model, State, Titled, Transition,
};

use super::Change;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MEvent {
    pub name: String,
    pub description: String,
    pub change: Option<Change>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MTransition {
    pub event: EventRef,
    pub target: String,
    pub prose: String,
    pub change: Option<Change>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MState {
    pub name: String,
    pub marker: Marker,
    pub description: String,
    pub change: Option<Change>,
    pub transitions: Vec<MTransition>,
    pub children: Vec<MState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MMachine {
    pub id: String,
    pub feature: String,
    pub title: String,
    pub description: String,
    pub change: Option<Change>,
    pub states: Vec<MState>,
    pub events: Vec<MEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MActor {
    pub id: String,
    pub title: String,
    pub description: String,
    pub change: Option<Change>,
    pub events: Vec<MEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MFeature {
    pub id: String,
    pub title: String,
    pub description: String,
    pub change: Option<Change>,
}

/// The whole tree, in Model order: Actors and Features by id, Machines by Feature then id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergedModel {
    pub system: Option<(Titled, Option<Change>)>,
    pub actors: Vec<MActor>,
    pub features: Vec<MFeature>,
    pub machines: Vec<MMachine>,
}

impl MState {
    /// This State and every State inside it, parents first.
    pub fn walk(&self) -> Vec<&MState> {
        let mut out = vec![self];
        for child in &self.children {
            out.extend(child.walk());
        }
        out
    }

    pub fn find(&self, name: &str) -> Option<&MState> {
        self.walk().into_iter().find(|s| s.name == name)
    }
}

impl MMachine {
    /// Every State, parents before their children.
    pub fn all_states(&self) -> Vec<&MState> {
        self.states.iter().flat_map(MState::walk).collect()
    }

    pub fn state(&self, name: &str) -> Option<&MState> {
        self.all_states().into_iter().find(|s| s.name == name)
    }

    /// The name of the State that holds `name`, when it is inside another.
    pub fn parent_of(&self, name: &str) -> Option<&str> {
        fn find<'a>(
            states: &'a [MState],
            name: &str,
            parent: Option<&'a str>,
        ) -> Option<Option<&'a str>> {
            for state in states {
                if state.name == name {
                    return Some(parent);
                }
                if let Some(found) = find(&state.children, name, Some(&state.name)) {
                    return Some(found);
                }
            }
            None
        }
        find(&self.states, name, None).flatten()
    }

    /// The sender of a Transition's Event: the written owner, or this Machine.
    pub fn sender_of<'a>(&'a self, event: &'a EventRef) -> &'a str {
        event.owner.as_deref().unwrap_or(&self.id)
    }
}

impl MergedModel {
    pub fn machine(&self, id: &str) -> Option<&MMachine> {
        self.machines.iter().find(|m| m.id == id)
    }

    pub fn actor(&self, id: &str) -> Option<&MActor> {
        self.actors.iter().find(|a| a.id == id)
    }

    pub fn feature(&self, id: &str) -> Option<&MFeature> {
        self.features.iter().find(|f| f.id == id)
    }

    pub fn machines_of<'a>(&'a self, feature: &'a str) -> impl Iterator<Item = &'a MMachine> {
        self.machines.iter().filter(move |m| m.feature == feature)
    }

    pub fn is_empty(&self) -> bool {
        self.actors.is_empty() && self.features.is_empty()
    }
}

fn events(list: &[EventDecl], change: Option<Change>) -> Vec<MEvent> {
    list.iter()
        .map(|e| MEvent {
            name: e.name.clone(),
            description: e.description.clone(),
            change,
        })
        .collect()
}

fn merge_events(base: &[EventDecl], shown: &[EventDecl]) -> Vec<MEvent> {
    let mut out: Vec<MEvent> = shown
        .iter()
        .map(|e| {
            let change = match base.iter().find(|b| b.name == e.name) {
                None => Some(Change::Added),
                Some(b) if b.description != e.description => Some(Change::Changed),
                Some(_) => None,
            };
            MEvent {
                name: e.name.clone(),
                description: e.description.clone(),
                change,
            }
        })
        .collect();
    out.extend(events(
        &base
            .iter()
            .filter(|b| !shown.iter().any(|e| e.name == b.name))
            .cloned()
            .collect::<Vec<_>>(),
        Some(Change::Removed),
    ));
    out
}

fn transitions(list: &[Transition], change: Option<Change>) -> Vec<MTransition> {
    list.iter()
        .map(|t| MTransition {
            event: t.event.clone(),
            target: t.target.clone(),
            prose: t.prose.clone(),
            change,
        })
        .collect()
}

fn lift_state(state: &State, change: Option<Change>) -> MState {
    MState {
        name: state.name.clone(),
        marker: state.marker,
        description: state.description.clone(),
        change,
        transitions: transitions(&state.transitions, change),
        children: state
            .children
            .iter()
            .map(|c| lift_state(c, change))
            .collect(),
    }
}

fn same_transition(a: &Transition, b: &Transition) -> bool {
    a.event == b.event && a.target == b.target
}

fn merge_transitions(base: &[Transition], shown: &[Transition]) -> Vec<MTransition> {
    let mut taken = vec![false; base.len()];
    let mut out = Vec::new();
    for t in shown {
        let found = base
            .iter()
            .enumerate()
            .find(|(i, b)| !taken[*i] && same_transition(b, t));
        let change = match found {
            None => Some(Change::Added),
            Some((i, b)) => {
                taken[i] = true;
                (b.prose != t.prose).then_some(Change::Changed)
            }
        };
        out.push(MTransition {
            event: t.event.clone(),
            target: t.target.clone(),
            prose: t.prose.clone(),
            change,
        });
    }
    for (i, b) in base.iter().enumerate().filter(|(i, _)| !taken[*i]) {
        let _ = i;
        out.push(MTransition {
            event: b.event.clone(),
            target: b.target.clone(),
            prose: b.prose.clone(),
            change: Some(Change::Removed),
        });
    }
    out
}

fn merge_states(base: &[State], shown: &[State]) -> Vec<MState> {
    let mut out: Vec<MState> = shown
        .iter()
        .map(|s| match base.iter().find(|b| b.name == s.name) {
            None => lift_state(s, Some(Change::Added)),
            Some(b) => MState {
                name: s.name.clone(),
                marker: s.marker,
                description: s.description.clone(),
                change: (b.description != s.description || b.marker != s.marker)
                    .then_some(Change::Changed),
                transitions: merge_transitions(&b.transitions, &s.transitions),
                children: merge_states(&b.children, &s.children),
            },
        })
        .collect();
    out.extend(
        base.iter()
            .filter(|b| !shown.iter().any(|s| s.name == b.name))
            .map(|b| lift_state(b, Some(Change::Removed))),
    );
    out
}

fn lift_machine(machine: &Machine, change: Option<Change>) -> MMachine {
    MMachine {
        id: machine.id.clone(),
        feature: machine.feature.clone(),
        title: machine.title.clone(),
        description: machine.description.clone(),
        change,
        states: machine
            .states
            .iter()
            .map(|s| lift_state(s, change))
            .collect(),
        events: events(&machine.events, change),
    }
}

fn lift_actor(actor: &Actor, change: Option<Change>) -> MActor {
    MActor {
        id: actor.id.clone(),
        title: actor.title.clone(),
        description: actor.description.clone(),
        change,
        events: events(&actor.events, change),
    }
}

fn lift_feature(id: &str, titled: &Titled, change: Option<Change>) -> MFeature {
    MFeature {
        id: id.to_owned(),
        title: titled.title.clone(),
        description: titled.description.clone(),
        change,
    }
}

/// Merges two Models. With `overlaid` false, `shown` is the Model as it is and nothing changes.
pub fn merge(base: &Model, shown: &Model, overlaid: bool) -> MergedModel {
    if !overlaid {
        return MergedModel {
            system: shown.system.clone().map(|s| (s, None)),
            actors: shown.actors.values().map(|a| lift_actor(a, None)).collect(),
            features: shown
                .features
                .iter()
                .map(|(id, f)| lift_feature(id, f, None))
                .collect(),
            machines: sorted_machines(
                shown
                    .machines
                    .values()
                    .map(|m| lift_machine(m, None))
                    .collect(),
            ),
        };
    }
    let system = match (&base.system, &shown.system) {
        (_, Some(s)) => {
            let change = match &base.system {
                None => Some(Change::Added),
                Some(b) => (b != s).then_some(Change::Changed),
            };
            Some((s.clone(), change))
        }
        (Some(b), None) => Some((b.clone(), Some(Change::Removed))),
        (None, None) => None,
    };

    let mut actors: BTreeMap<&str, MActor> = BTreeMap::new();
    for (id, a) in &shown.actors {
        actors.insert(
            id,
            match base.actors.get(id) {
                None => lift_actor(a, Some(Change::Added)),
                Some(b) => MActor {
                    id: a.id.clone(),
                    title: a.title.clone(),
                    description: a.description.clone(),
                    change: (b.title != a.title || b.description != a.description)
                        .then_some(Change::Changed),
                    events: merge_events(&b.events, &a.events),
                },
            },
        );
    }
    for (id, b) in base
        .actors
        .iter()
        .filter(|(id, _)| !shown.actors.contains_key(*id))
    {
        actors.insert(id, lift_actor(b, Some(Change::Removed)));
    }

    let mut features: BTreeMap<&str, MFeature> = BTreeMap::new();
    for (id, f) in &shown.features {
        let change = match base.features.get(id) {
            None => Some(Change::Added),
            Some(b) => (b != f).then_some(Change::Changed),
        };
        features.insert(id, lift_feature(id, f, change));
    }
    for (id, b) in base
        .features
        .iter()
        .filter(|(id, _)| !shown.features.contains_key(*id))
    {
        features.insert(id, lift_feature(id, b, Some(Change::Removed)));
    }

    let mut machines: Vec<MMachine> = Vec::new();
    for (id, m) in &shown.machines {
        machines.push(match base.machines.get(id) {
            None => lift_machine(m, Some(Change::Added)),
            Some(b) => MMachine {
                id: m.id.clone(),
                feature: m.feature.clone(),
                title: m.title.clone(),
                description: m.description.clone(),
                change: (b.title != m.title || b.description != m.description)
                    .then_some(Change::Changed),
                states: merge_states(&b.states, &m.states),
                events: merge_events(&b.events, &m.events),
            },
        });
    }
    machines.extend(
        base.machines
            .iter()
            .filter(|(id, _)| !shown.machines.contains_key(*id))
            .map(|(_, m)| lift_machine(m, Some(Change::Removed))),
    );

    MergedModel {
        system,
        actors: actors.into_values().collect(),
        features: features.into_values().collect(),
        machines: sorted_machines(machines),
    }
}

fn sorted_machines(mut machines: Vec<MMachine>) -> Vec<MMachine> {
    machines.sort_by(|a, b| a.feature.cmp(&b.feature).then_with(|| a.id.cmp(&b.id)));
    machines
}
