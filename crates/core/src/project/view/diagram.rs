//! The levels of the Diagram, worked out from the merged Model.
//!
//! Levels: the root (a grid of `Whole System`, the Actors and the Features no Actor reaches), the
//! Whole System, an Actor, a Feature, a Machine and a State. Every node and edge has an id that
//! is the same on every level that shows it, so a selection survives going up or down.
//!
//! Keys of levels: `""` (root), `system`, `actor:<id>`, `feature:<id>`, `machine:<id>`,
//! `state:<machine>/<State>`. Ids of nodes: `whole`, `actor:<id>`, `feature:<id>`,
//! `machine:<id>`, `state:<machine>/<State>`, and `ext-actor:`, `ext-machine:`, `ext-state:`
//! for the compact ends that belong to another container.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use ts_rs::TS;

use crate::project::model::{event_ref_text, Marker};
use crate::project::refs::ElementRef;

use super::lines::Locator;
use super::merge::{MMachine, MState, MTransition, MergedModel};
use super::{aggregate, CardMention, Change, Concerning, ModelView};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum LevelKind {
    Root,
    System,
    Actor,
    Feature,
    Machine,
    State,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum NodeKind {
    WholeSystem,
    Actor,
    Feature,
    Machine,
    State,
    /// An Actor of another container, drawn compact and dashed.
    ExternalActor,
    /// A Machine of another Feature, drawn compact and dashed.
    ExternalMachine,
    /// The other end of a Transition on a State level.
    ExternalState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum DetailKind {
    System,
    Actor,
    Feature,
    Machine,
    State,
    /// One Transition.
    Transition,
    /// Every Event that goes from one element to another.
    Flow,
}

/// A Transition as the details list it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TransitionLine {
    /// `<sender>.<EVENT>`.
    pub event: String,
    /// The Machine, when the line does not belong to the Machine the details are about.
    pub machine: Option<String>,
    /// The State that reacts.
    pub from: String,
    /// The State it goes to.
    pub target: String,
    pub prose: String,
    pub change: Option<Change>,
    pub file: Option<String>,
    pub line: Option<u32>,
}

/// An Event an Actor or a Machine declares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EventLine {
    pub name: String,
    pub description: String,
    pub change: Option<Change>,
    pub line: Option<u32>,
}

/// What the right panel shows about a node or an edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    pub kind: DetailKind,
    pub title: String,
    /// Markdown.
    pub description: String,
    pub change: Option<Change>,
    pub transitions: Vec<TransitionLine>,
    pub events: Vec<EventLine>,
    /// Open Drifts that concern it.
    pub drifts: Vec<CardMention>,
    /// Live Tasks that concern it.
    pub tasks: Vec<CardMention>,
    pub file: Option<String>,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DiagramNode {
    pub id: String,
    /// The key of its details in [`ModelView::details`].
    pub reference: Option<String>,
    pub kind: NodeKind,
    pub title: String,
    pub subtitle: Option<String>,
    /// The level a double click or Enter opens; `None` when it cannot be opened.
    pub opens: Option<String>,
    /// A Machine on a Feature level: its States can be shown inside it.
    pub expandable: bool,
    /// The node that holds this one (a State in a Machine or in a State).
    pub parent: Option<String>,
    pub change: Option<Change>,
    /// Open Drifts that concern it.
    pub drifts: u32,
    pub initial: bool,
    #[serde(rename = "final")]
    #[ts(rename = "final")]
    pub terminal: bool,
    pub compact: bool,
    /// Drawn with a dashed border (an end that belongs to another container, an Actor nobody
    /// listens to).
    pub dashed: bool,
    pub muted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DiagramEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: String,
    pub detail: Detail,
    /// The Event names it carries, for a focus on an Event.
    pub events: Vec<String>,
    pub change: Option<Change>,
    /// The Machine node whose States this edge joins, on a Feature level: it shows only while
    /// that Machine is expanded.
    pub inside: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Level {
    pub key: String,
    pub kind: LevelKind,
    /// The name in the breadcrumb.
    pub title: String,
    /// The container a link to this level starts from.
    pub parent: Option<String>,
    /// Laid out as a grid with no edges.
    pub grid: bool,
    pub nodes: Vec<DiagramNode>,
    pub edges: Vec<DiagramEdge>,
}

/// Nodes and edges of one level that hold an element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Anchor {
    pub level: String,
    pub nodes: Vec<String>,
    pub edges: Vec<String>,
}

pub const ROOT: &str = "";
pub const SYSTEM: &str = "system";

pub fn actor_key(id: &str) -> String {
    format!("actor:{id}")
}
pub fn feature_key(id: &str) -> String {
    format!("feature:{id}")
}
pub fn machine_key(id: &str) -> String {
    format!("machine:{id}")
}
pub fn state_key(machine: &str, state: &str) -> String {
    format!("state:{machine}/{state}")
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// One Transition of a Machine, seen from the whole Model.
struct Flow<'a> {
    machine: &'a MMachine,
    state: &'a MState,
    index: usize,
    transition: &'a MTransition,
    sender: String,
}

impl Flow<'_> {
    fn event_text(&self) -> String {
        format!("{}.{}", self.sender, self.transition.event.event)
    }
}

struct Ctx<'a> {
    model: &'a MergedModel,
    loc: &'a Locator<'a>,
    concerning: &'a Concerning<'a>,
    flows: Vec<Flow<'a>>,
    details: BTreeMap<String, Detail>,
}

pub fn build(model: &MergedModel, loc: &Locator, concerning: &Concerning) -> ModelView {
    let mut flows = Vec::new();
    for machine in &model.machines {
        for state in machine.all_states() {
            for (index, transition) in state.transitions.iter().enumerate() {
                let sender = machine.sender_of(&transition.event).to_owned();
                flows.push(Flow {
                    machine,
                    state,
                    index,
                    transition,
                    sender,
                });
            }
        }
    }
    let mut ctx = Ctx {
        model,
        loc,
        concerning,
        flows,
        details: BTreeMap::new(),
    };
    ctx.fill_details();
    let levels = ctx.levels();
    ModelView {
        levels,
        details: ctx.details,
    }
}

impl<'a> Ctx<'a> {
    // -- details ----------------------------------------------------------------------------

    fn mentions(&self, element: &ElementRef) -> (Vec<CardMention>, Vec<CardMention>) {
        self.concerning.of(element)
    }

    fn event_lines(&self, path: &str, events: &[super::merge::MEvent]) -> Vec<EventLine> {
        events
            .iter()
            .map(|e| EventLine {
                name: e.name.clone(),
                description: e.description.clone(),
                change: e.change,
                line: self
                    .loc
                    .place(path, e.change == Some(Change::Removed), |d| {
                        d.event(&e.name)
                    })
                    .and_then(|p| p.line),
            })
            .collect()
    }

    fn fill_details(&mut self) {
        let model = self.model;
        if let Some((system, change)) = &model.system {
            let place = self.loc.place(
                &Locator::system_path(),
                *change == Some(Change::Removed),
                |d| d.title(),
            );
            self.details.insert(
                SYSTEM.to_owned(),
                Detail {
                    kind: DetailKind::System,
                    title: "Whole System".to_owned(),
                    description: system.description.clone(),
                    change: *change,
                    transitions: Vec::new(),
                    events: Vec::new(),
                    drifts: Vec::new(),
                    tasks: Vec::new(),
                    file: place.as_ref().map(|p| p.file.clone()),
                    line: place.and_then(|p| p.line),
                },
            );
        }
        for actor in &model.actors {
            let path = Locator::actor_path(&actor.id);
            let place = self
                .loc
                .place(&path, actor.change == Some(Change::Removed), |d| d.title());
            let (drifts, tasks) = self.mentions(&ElementRef::Element(actor.id.clone()));
            let events = self.event_lines(&path, &actor.events);
            self.details.insert(
                actor.id.clone(),
                Detail {
                    kind: DetailKind::Actor,
                    title: actor.title.clone(),
                    description: actor.description.clone(),
                    change: actor.change,
                    transitions: Vec::new(),
                    events,
                    drifts,
                    tasks,
                    file: place.as_ref().map(|p| p.file.clone()),
                    line: place.and_then(|p| p.line),
                },
            );
        }
        for feature in &model.features {
            let place = self.loc.place(
                &Locator::feature_path(&feature.id),
                feature.change == Some(Change::Removed),
                |d| d.title(),
            );
            let (drifts, tasks) = self.mentions(&ElementRef::Element(feature.id.clone()));
            self.details.insert(
                feature.id.clone(),
                Detail {
                    kind: DetailKind::Feature,
                    title: feature.title.clone(),
                    description: feature.description.clone(),
                    change: feature.change,
                    transitions: Vec::new(),
                    events: Vec::new(),
                    drifts,
                    tasks,
                    file: place.as_ref().map(|p| p.file.clone()),
                    line: place.and_then(|p| p.line),
                },
            );
        }
        for machine in &model.machines {
            let path = Locator::machine_path(&machine.feature, &machine.id);
            let removed = machine.change == Some(Change::Removed);
            let place = self.loc.place(&path, removed, |d| d.title());
            let (drifts, tasks) = self.mentions(&ElementRef::Element(machine.id.clone()));
            let events = self.event_lines(&path, &machine.events);
            self.details.insert(
                machine.id.clone(),
                Detail {
                    kind: DetailKind::Machine,
                    title: machine.title.clone(),
                    description: machine.description.clone(),
                    change: machine.change,
                    transitions: Vec::new(),
                    events,
                    drifts,
                    tasks,
                    file: place.as_ref().map(|p| p.file.clone()),
                    line: place.and_then(|p| p.line),
                },
            );
            for state in machine.all_states() {
                let place = self
                    .loc
                    .place(&path, state.change == Some(Change::Removed), |d| {
                        d.state(&state.name)
                    });
                let (drifts, tasks) = self.mentions(&ElementRef::State {
                    machine: machine.id.clone(),
                    state: state.name.clone(),
                });
                let transitions = state
                    .transitions
                    .iter()
                    .map(|t| self.transition_line(machine, state, t, false))
                    .collect();
                self.details.insert(
                    format!("{}/{}", machine.id, state.name),
                    Detail {
                        kind: DetailKind::State,
                        title: state.name.clone(),
                        description: state.description.clone(),
                        change: state.change,
                        transitions,
                        events: Vec::new(),
                        drifts,
                        tasks,
                        file: place.as_ref().map(|p| p.file.clone()),
                        line: place.and_then(|p| p.line),
                    },
                );
            }
        }
    }

    fn transition_line(
        &self,
        machine: &MMachine,
        state: &MState,
        t: &MTransition,
        name_machine: bool,
    ) -> TransitionLine {
        let path = Locator::machine_path(&machine.feature, &machine.id);
        let removed = t.change == Some(Change::Removed);
        let event = event_ref_text(&t.event);
        let place = self.loc.place(&path, removed, |d| {
            d.transition(&state.name, &event, &t.target)
        });
        TransitionLine {
            event: format!("{}.{}", machine.sender_of(&t.event), t.event.event),
            machine: name_machine.then(|| machine.id.clone()),
            from: state.name.clone(),
            target: t.target.clone(),
            prose: t.prose.clone(),
            change: t.change,
            file: place.as_ref().map(|p| p.file.clone()),
            line: place.and_then(|p| p.line),
        }
    }

    fn drifts_of(&self, reference: &str) -> u32 {
        self.details
            .get(reference)
            .map_or(0, |d| d.drifts.len() as u32)
    }

    fn change_of(&self, reference: &str) -> Option<Change> {
        self.details.get(reference).and_then(|d| d.change)
    }

    // -- reach ------------------------------------------------------------------------------

    /// The Features whose Machines react to an Event of `actor`, in Model order.
    fn features_reached(&self, actor: &str) -> Vec<String> {
        let reached: BTreeSet<&str> = self
            .flows
            .iter()
            .filter(|f| f.sender == actor)
            .map(|f| f.machine.feature.as_str())
            .collect();
        self.model
            .features
            .iter()
            .filter(|f| reached.contains(f.id.as_str()))
            .map(|f| f.id.clone())
            .collect()
    }

    fn is_actor(&self, id: &str) -> bool {
        self.model.actor(id).is_some()
    }

    // -- nodes ------------------------------------------------------------------------------

    fn actor_node(&self, id: &str, reach: &[String]) -> DiagramNode {
        let actor = self.model.actor(id).expect("an Actor of the Model");
        let reaches = !reach.is_empty();
        DiagramNode {
            id: actor_key(id),
            reference: Some(id.to_owned()),
            kind: NodeKind::Actor,
            title: actor.title.clone(),
            subtitle: Some(if reaches {
                plural(reach.len(), "feature", "features")
            } else {
                "No events received".to_owned()
            }),
            opens: reaches.then(|| actor_key(id)),
            expandable: false,
            parent: None,
            change: actor.change,
            drifts: self.drifts_of(id),
            initial: false,
            terminal: false,
            compact: false,
            dashed: !reaches,
            muted: !reaches,
        }
    }

    fn feature_node(&self, id: &str) -> DiagramNode {
        let feature = self.model.feature(id).expect("a Feature of the Model");
        let machines = self.model.machines_of(id).count();
        DiagramNode {
            id: feature_key(id),
            reference: Some(id.to_owned()),
            kind: NodeKind::Feature,
            title: feature.title.clone(),
            subtitle: Some(plural(machines, "machine", "machines")),
            opens: Some(feature_key(id)),
            expandable: false,
            parent: None,
            change: feature.change,
            drifts: self.drifts_of(id),
            initial: false,
            terminal: false,
            compact: false,
            dashed: false,
            muted: false,
        }
    }

    fn whole_node(&self) -> DiagramNode {
        DiagramNode {
            id: "whole".to_owned(),
            reference: self.details.contains_key(SYSTEM).then(|| SYSTEM.to_owned()),
            kind: NodeKind::WholeSystem,
            title: "Whole System".to_owned(),
            subtitle: None,
            opens: Some(SYSTEM.to_owned()),
            expandable: false,
            parent: None,
            change: self.change_of(SYSTEM),
            drifts: 0,
            initial: false,
            terminal: false,
            compact: false,
            dashed: false,
            muted: false,
        }
    }

    fn machine_node(&self, machine: &MMachine, expandable: bool) -> DiagramNode {
        DiagramNode {
            id: machine_key(&machine.id),
            reference: Some(machine.id.clone()),
            kind: NodeKind::Machine,
            title: machine.title.clone(),
            subtitle: None,
            opens: Some(machine_key(&machine.id)),
            expandable,
            parent: None,
            change: machine.change,
            drifts: self.drifts_of(&machine.id),
            initial: false,
            terminal: false,
            compact: false,
            dashed: false,
            muted: false,
        }
    }

    fn state_node(
        &self,
        machine: &MMachine,
        state: &MState,
        parent: Option<String>,
    ) -> DiagramNode {
        let reference = format!("{}/{}", machine.id, state.name);
        DiagramNode {
            id: state_key(&machine.id, &state.name),
            drifts: self.drifts_of(&reference),
            reference: Some(reference),
            kind: NodeKind::State,
            title: state.name.clone(),
            subtitle: None,
            opens: Some(state_key(&machine.id, &state.name)),
            expandable: false,
            parent,
            change: state.change,
            initial: state.marker == Marker::Initial,
            terminal: state.marker == Marker::Final,
            compact: false,
            dashed: false,
            muted: false,
        }
    }

    /// The compact, dashed end of an edge that belongs to another container.
    fn external_node(&self, sender: &str) -> DiagramNode {
        let (kind, id, title, change, opens) = if let Some(actor) = self.model.actor(sender) {
            let reaches = !self.features_reached(sender).is_empty();
            (
                NodeKind::ExternalActor,
                format!("ext-actor:{sender}"),
                actor.title.clone(),
                actor.change,
                reaches.then(|| actor_key(sender)),
            )
        } else if let Some(machine) = self.model.machine(sender) {
            (
                NodeKind::ExternalMachine,
                format!("ext-machine:{sender}"),
                machine.title.clone(),
                machine.change,
                Some(machine_key(sender)),
            )
        } else {
            (
                NodeKind::ExternalMachine,
                format!("ext-machine:{sender}"),
                sender.to_owned(),
                None,
                None,
            )
        };
        DiagramNode {
            id,
            reference: self.details.contains_key(sender).then(|| sender.to_owned()),
            kind,
            title,
            subtitle: None,
            opens,
            expandable: false,
            parent: None,
            change,
            drifts: self.drifts_of(sender),
            initial: false,
            terminal: false,
            compact: true,
            dashed: true,
            muted: false,
        }
    }

    // -- edges ------------------------------------------------------------------------------

    /// The edge of one Transition: from the State that reacts to the State it goes to.
    fn transition_edge(
        &self,
        flow: &Flow,
        source: String,
        target: String,
        inside: Option<String>,
    ) -> DiagramEdge {
        let line = self.transition_line(flow.machine, flow.state, flow.transition, false);
        let label = flow.event_text();
        DiagramEdge {
            id: format!("t:{}/{}/{}", flow.machine.id, flow.state.name, flow.index),
            source,
            target,
            label: label.clone(),
            detail: Detail {
                kind: DetailKind::Transition,
                title: label,
                description: flow.transition.prose.clone(),
                change: flow.transition.change,
                transitions: vec![line.clone()],
                events: Vec::new(),
                drifts: Vec::new(),
                tasks: Vec::new(),
                file: line.file,
                line: line.line,
            },
            events: vec![flow.transition.event.event.clone()],
            change: flow.transition.change,
            inside,
        }
    }

    /// One edge for every Event that goes from `source` to `target`: the label is made by `label`.
    fn flow_edge(
        &self,
        id: String,
        source: String,
        target: String,
        title: String,
        flows: &[&Flow],
        label: String,
    ) -> DiagramEdge {
        let mut events: Vec<String> = Vec::new();
        for f in flows {
            if !events.contains(&f.transition.event.event) {
                events.push(f.transition.event.event.clone());
            }
        }
        let change = aggregate(flows.iter().map(|f| f.transition.change));
        DiagramEdge {
            id,
            source,
            target,
            label,
            detail: Detail {
                kind: DetailKind::Flow,
                title,
                description: String::new(),
                change,
                transitions: flows
                    .iter()
                    .map(|f| self.transition_line(f.machine, f.state, f.transition, true))
                    .collect(),
                events: Vec::new(),
                drifts: Vec::new(),
                tasks: Vec::new(),
                file: None,
                line: None,
            },
            events,
            change,
            inside: None,
        }
    }

    fn distinct_events(flows: &[&Flow]) -> usize {
        flows
            .iter()
            .map(|f| (f.sender.as_str(), f.transition.event.event.as_str()))
            .collect::<BTreeSet<_>>()
            .len()
    }

    fn title_of(&self, id: &str) -> String {
        self.details
            .get(id)
            .map_or_else(|| id.to_owned(), |d| d.title.clone())
    }

    // -- levels -----------------------------------------------------------------------------

    fn levels(&self) -> Vec<Level> {
        let model = self.model;
        let reach: BTreeMap<&str, Vec<String>> = model
            .actors
            .iter()
            .map(|a| (a.id.as_str(), self.features_reached(&a.id)))
            .collect();
        let reached_features: BTreeSet<&str> =
            reach.values().flatten().map(String::as_str).collect();

        let mut levels = Vec::new();

        // Root: Whole System, the Actors, the Features no Actor reaches.
        let mut nodes = vec![self.whole_node()];
        nodes.extend(
            model
                .actors
                .iter()
                .map(|a| self.actor_node(&a.id, &reach[a.id.as_str()])),
        );
        nodes.extend(
            model
                .features
                .iter()
                .filter(|f| !reached_features.contains(f.id.as_str()))
                .map(|f| self.feature_node(&f.id)),
        );
        levels.push(Level {
            key: ROOT.to_owned(),
            kind: LevelKind::Root,
            title: "System".to_owned(),
            parent: None,
            grid: true,
            nodes,
            edges: Vec::new(),
        });

        levels.push(self.system_level(&reach));

        for actor in &model.actors {
            let features = &reach[actor.id.as_str()];
            if !features.is_empty() {
                levels.push(self.actor_level(&actor.id, features));
            }
        }
        for feature in &model.features {
            levels.push(self.feature_level(&feature.id));
            for machine in model.machines_of(&feature.id) {
                levels.push(self.machine_level(machine));
                for state in machine.all_states() {
                    levels.push(self.state_level(machine, state));
                }
            }
        }
        levels
    }

    fn system_level(&self, reach: &BTreeMap<&str, Vec<String>>) -> Level {
        let model = self.model;
        let mut nodes: Vec<DiagramNode> = model
            .actors
            .iter()
            .map(|a| self.actor_node(&a.id, &reach[a.id.as_str()]))
            .collect();
        nodes.extend(model.features.iter().map(|f| self.feature_node(&f.id)));
        let feature_of = |id: &str| model.machine(id).map(|m| m.feature.clone());

        // Actor → Feature, and Feature → Feature, grouped.
        let mut groups: BTreeMap<(String, String), Vec<&Flow>> = BTreeMap::new();
        for flow in &self.flows {
            if flow.sender == flow.machine.id {
                continue;
            }
            let target = feature_key(&flow.machine.feature);
            let source = if self.is_actor(&flow.sender) {
                actor_key(&flow.sender)
            } else if let Some(f) = feature_of(&flow.sender) {
                if f == flow.machine.feature {
                    continue;
                }
                feature_key(&f)
            } else {
                continue;
            };
            groups.entry((source, target)).or_default().push(flow);
        }
        let edges = groups
            .into_iter()
            .map(|((source, target), flows)| {
                let title = format!(
                    "{} → {}",
                    self.node_title(&source),
                    self.node_title(&target)
                );
                let label = Self::distinct_events(&flows).to_string();
                self.flow_edge(
                    format!("e:{source}>{target}"),
                    source,
                    target,
                    title,
                    &flows,
                    label,
                )
            })
            .collect();
        Level {
            key: SYSTEM.to_owned(),
            kind: LevelKind::System,
            title: "Whole System".to_owned(),
            parent: Some(ROOT.to_owned()),
            grid: false,
            nodes,
            edges,
        }
    }

    fn node_title(&self, node_id: &str) -> String {
        let id = node_id.split_once(':').map_or(node_id, |(_, id)| id);
        self.title_of(id)
    }

    fn actor_level(&self, actor: &str, features: &[String]) -> Level {
        let reach = self.features_reached(actor);
        let mut nodes = vec![self.actor_node(actor, &reach)];
        nodes.extend(features.iter().map(|f| self.feature_node(f)));
        let mut edges = Vec::new();
        for feature in features {
            let flows: Vec<&Flow> = self
                .flows
                .iter()
                .filter(|f| f.sender == actor && f.machine.feature == *feature)
                .collect();
            let mut names: Vec<&str> = Vec::new();
            for f in &flows {
                if !names.contains(&f.transition.event.event.as_str()) {
                    names.push(&f.transition.event.event);
                }
            }
            let source = actor_key(actor);
            let target = feature_key(feature);
            let title = format!("{} → {}", self.title_of(actor), self.title_of(feature));
            edges.push(self.flow_edge(
                format!("e:{source}>{target}"),
                source,
                target,
                title,
                &flows,
                names.join(", "),
            ));
        }
        Level {
            key: actor_key(actor),
            kind: LevelKind::Actor,
            title: self.title_of(actor),
            parent: Some(ROOT.to_owned()),
            grid: false,
            nodes,
            edges,
        }
    }

    fn feature_level(&self, feature: &str) -> Level {
        let model = self.model;
        let machines: Vec<&MMachine> = model.machines_of(feature).collect();
        let mut nodes: Vec<DiagramNode> = Vec::new();
        let mut edges: Vec<DiagramEdge> = Vec::new();
        for machine in &machines {
            nodes.push(self.machine_node(machine, !machine.states.is_empty()));
            let machine_node = machine_key(&machine.id);
            fn add_states(
                ctx: &Ctx,
                machine: &MMachine,
                states: &[MState],
                parent: &str,
                out: &mut Vec<DiagramNode>,
            ) {
                for state in states {
                    out.push(ctx.state_node(machine, state, Some(parent.to_owned())));
                    add_states(
                        ctx,
                        machine,
                        &state.children,
                        &state_key(&machine.id, &state.name),
                        out,
                    );
                }
            }
            add_states(self, machine, &machine.states, &machine_node, &mut nodes);
            edges.extend(self.state_edges(machine, Some(machine_node)));
        }

        // Events between Machines of this Feature, and the ones that come in from outside.
        let mut groups: BTreeMap<(String, String), Vec<&Flow>> = BTreeMap::new();
        let mut outside: BTreeSet<String> = BTreeSet::new();
        for flow in self
            .flows
            .iter()
            .filter(|f| f.machine.feature == feature && f.sender != f.machine.id)
        {
            let inside = model
                .machine(&flow.sender)
                .is_some_and(|m| m.feature == feature);
            let source = if inside {
                machine_key(&flow.sender)
            } else if self.is_actor(&flow.sender) {
                outside.insert(flow.sender.clone());
                format!("ext-actor:{}", flow.sender)
            } else if model.machine(&flow.sender).is_some() {
                outside.insert(flow.sender.clone());
                format!("ext-machine:{}", flow.sender)
            } else {
                continue;
            };
            groups
                .entry((source, machine_key(&flow.machine.id)))
                .or_default()
                .push(flow);
        }
        nodes.extend(outside.iter().map(|sender| self.external_node(sender)));
        for ((source, target), flows) in groups {
            let mut names: Vec<&str> = Vec::new();
            for f in &flows {
                if !names.contains(&f.transition.event.event.as_str()) {
                    names.push(&f.transition.event.event);
                }
            }
            let title = format!(
                "{} → {}",
                self.node_title(&source),
                self.node_title(&target)
            );
            edges.push(self.flow_edge(
                format!("e:{source}>{target}"),
                source,
                target,
                title,
                &flows,
                names.join(", "),
            ));
        }
        Level {
            key: feature_key(feature),
            kind: LevelKind::Feature,
            title: self.title_of(feature),
            parent: Some(ROOT.to_owned()),
            grid: false,
            nodes,
            edges,
        }
    }

    /// The Transitions of `machine` as edges between its States.
    fn state_edges(&self, machine: &MMachine, inside: Option<String>) -> Vec<DiagramEdge> {
        self.flows
            .iter()
            .filter(|f| f.machine.id == machine.id)
            .filter(|f| machine.state(&f.transition.target).is_some())
            .map(|f| {
                self.transition_edge(
                    f,
                    state_key(&machine.id, &f.state.name),
                    state_key(&machine.id, &f.transition.target),
                    inside.clone(),
                )
            })
            .collect()
    }

    fn machine_level(&self, machine: &MMachine) -> Level {
        let mut nodes = Vec::new();
        fn add(
            ctx: &Ctx,
            machine: &MMachine,
            states: &[MState],
            parent: Option<String>,
            out: &mut Vec<DiagramNode>,
        ) {
            for state in states {
                out.push(ctx.state_node(machine, state, parent.clone()));
                add(
                    ctx,
                    machine,
                    &state.children,
                    Some(state_key(&machine.id, &state.name)),
                    out,
                );
            }
        }
        add(self, machine, &machine.states, None, &mut nodes);
        Level {
            key: machine_key(&machine.id),
            kind: LevelKind::Machine,
            title: machine.title.clone(),
            parent: Some(feature_key(&machine.feature)),
            grid: false,
            nodes,
            edges: self.state_edges(machine, None),
        }
    }

    fn state_level(&self, machine: &MMachine, state: &MState) -> Level {
        let center = state_key(&machine.id, &state.name);
        let mut nodes = vec![self.state_node(machine, state, None)];
        let mut edges = Vec::new();
        let mut others: BTreeSet<String> = BTreeSet::new();
        let end_id = |name: &str| {
            if name == state.name {
                center.clone()
            } else {
                format!("ext-state:{}/{name}", machine.id)
            }
        };
        for flow in self.flows.iter().filter(|f| f.machine.id == machine.id) {
            let leaving = flow.state.name == state.name;
            let entering = flow.transition.target == state.name;
            if !(leaving || entering) || machine.state(&flow.transition.target).is_none() {
                continue;
            }
            if leaving {
                others.insert(flow.transition.target.clone());
            }
            if entering {
                others.insert(flow.state.name.clone());
            }
            edges.push(self.transition_edge(
                flow,
                end_id(&flow.state.name),
                end_id(&flow.transition.target),
                None,
            ));
        }
        for name in others.into_iter().filter(|n| *n != state.name) {
            if let Some(other) = machine.state(&name) {
                let mut node = self.state_node(machine, other, None);
                node.id = end_id(&name);
                node.kind = NodeKind::ExternalState;
                node.compact = true;
                node.dashed = true;
                nodes.push(node);
            }
        }
        Level {
            key: center.clone(),
            kind: LevelKind::State,
            title: state.name.clone(),
            parent: Some(machine_key(&machine.id)),
            grid: false,
            nodes,
            edges,
        }
    }
}

/// Where `element` is drawn: the level that holds it (the container it belongs to) and its nodes
/// and edges there. Empty when the Model has no such element.
pub fn anchors_of(merged: &MergedModel, view: &ModelView, element: &ElementRef) -> Vec<Anchor> {
    let level_of = |key: &str| view.levels.iter().find(|l| l.key == key);
    let edges_where = |key: &str, keep: &dyn Fn(&DiagramEdge) -> bool| -> Vec<String> {
        level_of(key)
            .map(|l| {
                l.edges
                    .iter()
                    .filter(|e| keep(e))
                    .map(|e| e.id.clone())
                    .collect()
            })
            .unwrap_or_default()
    };
    let one = |level: String, nodes: Vec<String>, edges: Vec<String>| {
        vec![Anchor {
            level,
            nodes,
            edges,
        }]
    };
    match element {
        ElementRef::Element(id) => {
            if merged.actor(id).is_some() {
                one(ROOT.to_owned(), vec![actor_key(id)], Vec::new())
            } else if merged.feature(id).is_some() {
                let node = feature_key(id);
                let on_root = level_of(ROOT).is_some_and(|l| l.nodes.iter().any(|n| n.id == node));
                one(
                    if on_root { ROOT } else { SYSTEM }.to_owned(),
                    vec![node],
                    Vec::new(),
                )
            } else if let Some(machine) = merged.machine(id) {
                one(
                    feature_key(&machine.feature),
                    vec![machine_key(id)],
                    Vec::new(),
                )
            } else {
                Vec::new()
            }
        }
        ElementRef::State { machine, state } => {
            match merged.machine(machine).and_then(|m| m.state(state)) {
                Some(_) => one(
                    machine_key(machine),
                    vec![state_key(machine, state)],
                    Vec::new(),
                ),
                None => Vec::new(),
            }
        }
        ElementRef::Transitions {
            machine,
            state,
            owner,
            event,
        } => {
            let Some(m) = merged.machine(machine) else {
                return Vec::new();
            };
            let Some(s) = m.state(state) else {
                return Vec::new();
            };
            let prefix = format!("t:{machine}/{state}/");
            let wanted: Vec<usize> = s
                .transitions
                .iter()
                .enumerate()
                .filter(|(_, t)| m.sender_of(&t.event) == owner && t.event.event == *event)
                .map(|(i, _)| i)
                .collect();
            let ids = edges_where(&machine_key(machine), &|e| {
                e.id.strip_prefix(&prefix)
                    .and_then(|i| i.parse::<usize>().ok())
                    .is_some_and(|i| wanted.contains(&i))
            });
            if ids.is_empty() {
                return Vec::new();
            }
            one(machine_key(machine), vec![state_key(machine, state)], ids)
        }
        ElementRef::Event { owner, event } => {
            if merged.actor(owner).is_some() {
                let key = actor_key(owner);
                if level_of(&key).is_some() {
                    let ids = edges_where(&key, &|e| e.events.contains(event));
                    return one(key, vec![actor_key(owner)], ids);
                }
                return one(ROOT.to_owned(), vec![actor_key(owner)], Vec::new());
            }
            let Some(machine) = merged.machine(owner) else {
                return Vec::new();
            };
            let key = feature_key(&machine.feature);
            let source = machine_key(owner);
            let ids = edges_where(&key, &|e| e.source == source && e.events.contains(event));
            one(key, vec![source], ids)
        }
    }
}
