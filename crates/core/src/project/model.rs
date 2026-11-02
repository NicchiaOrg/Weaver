//! The Model: System, Actors, Features and Machines, parsed exactly as the `trama-design`
//! skill's `FORMAT.md` describes them.
//!
//! The per-file parsers ([`parse_actor`], [`parse_machine`], [`parse_titled`]) check every
//! rule that can be decided inside one file and return all the errors they find. Rules that
//! need several files (the shared namespace, the owner of a Transition declaring its Event)
//! are checked by [`Model::assemble`].

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::diagnostic::Diagnostic;
use super::frontmatter;
use super::md::{self, Outline};

/// Identifiers no Feature, Machine or Actor may use.
pub const RESERVED_IDS: [&str; 4] = ["system", "feature", "actors", "requirements"];

/// The characters a State name cannot contain (a backtick is refused too).
pub const STATE_NAME_FORBIDDEN: [char; 6] = ['/', ':', '(', ')', '→', '`'];

/// `system.md` or `feature.md`: a display name and prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Titled {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventDecl {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Actor {
    pub id: String,
    pub title: String,
    pub description: String,
    pub events: Vec<EventDecl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Marker {
    None,
    Initial,
    Final,
}

/// `owner.EVENT` of a Transition; `owner` is `None` for the bare `EVENT` of the same Machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventRef {
    pub owner: Option<String>,
    pub event: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Transition {
    pub event: EventRef,
    /// Name of the target State, in the same Machine.
    pub target: String,
    /// Condition and consequence, as written after the `:`.
    pub prose: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct State {
    pub name: String,
    pub marker: Marker,
    pub description: String,
    pub transitions: Vec<Transition>,
    pub children: Vec<State>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Machine {
    pub id: String,
    pub feature: String,
    pub title: String,
    pub description: String,
    pub states: Vec<State>,
    pub events: Vec<EventDecl>,
}

impl Machine {
    /// Every State, parents before their children, with its path of names from the top level.
    pub fn walk_states(&self) -> Vec<(Vec<&str>, &State)> {
        fn visit<'a>(
            states: &'a [State],
            path: &mut Vec<&'a str>,
            out: &mut Vec<(Vec<&'a str>, &'a State)>,
        ) {
            for state in states {
                path.push(&state.name);
                out.push((path.clone(), state));
                visit(&state.children, path, out);
                path.pop();
            }
        }
        let mut out = Vec::new();
        visit(&self.states, &mut Vec::new(), &mut out);
        out
    }

    /// The State called `name` (compared exactly), at any depth.
    pub fn find_state(&self, name: &str) -> Option<&State> {
        self.walk_states()
            .into_iter()
            .map(|(_, s)| s)
            .find(|s| s.name == name)
    }

    /// The owner of a Transition's Event: the written owner, or this Machine for a bare Event.
    pub fn owner_of<'a>(&'a self, event: &'a EventRef) -> &'a str {
        event.owner.as_deref().unwrap_or(&self.id)
    }
}

/// Features, Machines and Actors share one namespace; this is what is in it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Model {
    pub system: Option<Titled>,
    pub actors: BTreeMap<String, Actor>,
    /// Feature id → its `feature.md`.
    pub features: BTreeMap<String, Titled>,
    pub machines: BTreeMap<String, Machine>,
}

/// What a name of the shared namespace is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    Feature,
    Machine,
    Actor,
}

/// An id of a Feature, Machine or Actor: kebab-case.
pub fn is_element_id(id: &str) -> bool {
    !id.is_empty()
        && id.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// An Event name: `UPPER_SNAKE_CASE`.
pub fn is_event_name(name: &str) -> bool {
    !name.is_empty()
        && name.split('_').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        })
        && name.as_bytes()[0].is_ascii_uppercase()
}

/// Splits the content of a Transition's code span: `owner.EVENT` or the bare `EVENT`.
pub fn parse_event_ref(text: &str) -> Option<EventRef> {
    match text.split_once('.') {
        Some((owner, event)) if is_element_id(owner) && is_event_name(event) => Some(EventRef {
            owner: Some(owner.to_owned()),
            event: event.to_owned(),
        }),
        None if is_event_name(text) => Some(EventRef {
            owner: None,
            event: text.to_owned(),
        }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------
// Per-file parsers
// ---------------------------------------------------------------------------------------

/// Parses `system.md` or `feature.md`: one H1 (the display name), then prose.
pub fn parse_titled(text: &str) -> Result<Titled, Vec<String>> {
    let (body, mut errors) = body_of(text);
    let outline = md::outline(&body);
    let title = single_h1(&outline, &mut errors);
    finish(errors, || {
        let (title, idx) = title.expect("checked");
        Titled {
            title,
            description: description_after_h1(&body, &outline, idx),
        }
    })
}

/// Parses an Actor file: H1, prose, and optionally `## Events`. Actors have no States and no
/// Transitions.
pub fn parse_actor(id: &str, text: &str) -> Result<Actor, Vec<String>> {
    let (body, mut errors) = body_of(text);
    let outline = md::outline(&body);
    let title = single_h1(&outline, &mut errors);
    let mut events = Vec::new();
    let mut seen_events = false;
    for (i, h) in outline.headings.iter().enumerate() {
        match h.level {
            1 => {}
            2 if h.text == "Events" => {
                if seen_events {
                    errors.push("`## Events` appears more than once".to_owned());
                }
                seen_events = true;
                events = parse_events(&body, &outline, i, &mut errors);
            }
            2 => errors.push(format!(
                "unexpected section `## {}`: an Actor has only `## Events` (no States, no Transitions)",
                h.text
            )),
            _ => {}
        }
    }
    finish(errors, || {
        let (title, idx) = title.expect("checked");
        Actor {
            id: id.to_owned(),
            title,
            description: description_after_h1(&body, &outline, idx),
            events,
        }
    })
}

/// Parses a Machine file (`## States` required, `## Events` optional, in that order).
pub fn parse_machine(feature: &str, id: &str, text: &str) -> Result<Machine, Vec<String>> {
    let (body, mut errors) = body_of(text);
    let outline = md::outline(&body);
    let title = single_h1(&outline, &mut errors);

    #[derive(PartialEq, Clone, Copy)]
    enum Section {
        None,
        States,
        Events,
    }
    struct Flat {
        level: u8,
        name: String,
        marker: Marker,
        heading: usize,
        description: String,
        transitions: Vec<Transition>,
    }

    let mut section = Section::None;
    let mut states_at: Option<usize> = None;
    let mut events_at: Option<usize> = None;
    let mut flat: Vec<Flat> = Vec::new();
    let mut flat_of_heading: BTreeMap<usize, usize> = BTreeMap::new();
    let mut prev_level: Option<u8> = None;

    for (i, h) in outline.headings.iter().enumerate() {
        match h.level {
            1 => {}
            2 => match h.text.as_str() {
                "States" => {
                    if states_at.is_some() {
                        errors.push("`## States` appears more than once".to_owned());
                    }
                    if events_at.is_some() {
                        errors.push("`## States` must come before `## Events`".to_owned());
                    }
                    states_at = Some(i);
                    section = Section::States;
                }
                "Events" => {
                    if events_at.is_some() {
                        errors.push("`## Events` appears more than once".to_owned());
                    }
                    events_at = Some(i);
                    section = Section::Events;
                }
                other => {
                    errors.push(format!("unexpected section `## {other}`: a Machine has only `## States` and `## Events`"));
                    section = Section::None;
                }
            },
            level => match section {
                Section::States => {
                    match prev_level {
                        None if level != 3 => {
                            errors.push(format!("State heading `{}` must be a `###` heading (top-level State)", h.text));
                        }
                        Some(prev) if level > prev + 1 => errors.push(format!(
                            "State heading `{}` skips a heading level (a `{}` heading follows a `{}` heading)",
                            h.text,
                            "#".repeat(level as usize),
                            "#".repeat(prev as usize)
                        )),
                        _ => {}
                    }
                    prev_level = Some(level);
                    let (name, marker) = split_marker(&h.text);
                    flat_of_heading.insert(i, flat.len());
                    flat.push(Flat {
                        level,
                        name,
                        marker,
                        heading: i,
                        description: String::new(),
                        transitions: Vec::new(),
                    });
                }
                Section::Events => {}
                Section::None => errors.push(format!(
                    "heading `{}` is outside `## States` and `## Events`",
                    h.text
                )),
            },
        }
    }

    // Descriptions and Transitions.
    let transition_ranges: Vec<_> = outline
        .items
        .iter()
        .filter(|it| it.code.is_some())
        .map(|it| it.range.clone())
        .collect();
    for f in &mut flat {
        let (start, end) = section_bounds(&body, &outline, f.heading);
        f.description = md::text_excluding(&body, start, end, &transition_ranges);
    }
    for item in outline.items.iter().filter(|it| it.code.is_some()) {
        let Some(heading) = outline
            .headings
            .iter()
            .rposition(|h| h.range.end <= item.range.start)
        else {
            continue;
        };
        let Some(&k) = flat_of_heading.get(&heading) else {
            // Under H1, `## Events` or an Event: prose, unless it sits in `## States` itself.
            if outline.headings[heading].level == 2 && outline.headings[heading].text == "States" {
                let code = &item.code.as_ref().expect("filtered").0;
                errors.push(format!("Transition `{code}` is not under a State"));
            }
            continue;
        };
        match parse_transition(&body, item) {
            Ok(t) => flat[k].transitions.push(t),
            Err(e) => errors.push(format!("State `{}`: {e}", flat[k].name)),
        }
    }

    if states_at.is_none() {
        errors.push("`## States` is required".to_owned());
    } else if flat.is_empty() {
        errors.push("`## States` has no State".to_owned());
    }

    let events = match events_at {
        Some(i) => parse_events(&body, &outline, i, &mut errors),
        None => Vec::new(),
    };

    // State names: allowed characters, unique within the Machine (case-insensitive).
    let mut seen_names = BTreeSet::new();
    for f in &flat {
        if f.name.is_empty() {
            errors.push("a State heading has no name".to_owned());
        } else if let Some(bad) = f.name.chars().find(|c| STATE_NAME_FORBIDDEN.contains(c)) {
            errors.push(format!(
                "State name `{}` contains `{bad}` (names cannot contain `/ : ( ) →` or a backtick)",
                f.name
            ));
        }
        if !f.name.is_empty() && !seen_names.insert(f.name.to_lowercase()) {
            errors.push(format!("State name `{}` is used twice (names are unique within the Machine, case-insensitive)", f.name));
        }
    }

    // Fold into the tree.
    let mut roots: Vec<State> = Vec::new();
    let mut stack: Vec<(u8, State)> = Vec::new();
    fn close(stack: &mut Vec<(u8, State)>, roots: &mut Vec<State>) {
        let (_, state) = stack.pop().expect("non-empty");
        match stack.last_mut() {
            Some((_, parent)) => parent.children.push(state),
            None => roots.push(state),
        }
    }
    for f in flat {
        while stack.last().is_some_and(|(level, _)| *level >= f.level) {
            close(&mut stack, &mut roots);
        }
        stack.push((
            f.level,
            State {
                name: f.name,
                marker: f.marker,
                description: f.description,
                transitions: f.transitions,
                children: Vec::new(),
            },
        ));
    }
    while !stack.is_empty() {
        close(&mut stack, &mut roots);
    }

    let machine = Machine {
        id: id.to_owned(),
        feature: feature.to_owned(),
        title: title.as_ref().map(|t| t.0.clone()).unwrap_or_default(),
        description: title
            .as_ref()
            .map(|(_, idx)| description_after_h1(&body, &outline, *idx))
            .unwrap_or_default(),
        states: roots,
        events,
    };
    check_machine_structure(&machine, &mut errors);
    if errors.is_empty() {
        Ok(machine)
    } else {
        Err(errors)
    }
}

fn check_machine_structure(machine: &Machine, errors: &mut Vec<String>) {
    fn levels(states: &[State], parent: Option<&str>, errors: &mut Vec<String>) {
        if states.is_empty() {
            return;
        }
        let initial = states
            .iter()
            .filter(|s| s.marker == Marker::Initial)
            .count();
        if initial != 1 {
            let place = parent.map_or_else(
                || "the top level".to_owned(),
                |p| format!("the sub-States of `{p}`"),
            );
            errors.push(format!(
                "{place} has {initial} `(initial)` States; exactly one is required"
            ));
        }
        for s in states {
            if s.marker == Marker::Final && !s.children.is_empty() {
                errors.push(format!(
                    "`(final)` State `{}` cannot have sub-States",
                    s.name
                ));
            }
            if s.marker == Marker::Final && !s.transitions.is_empty() {
                errors.push(format!(
                    "`(final)` State `{}` cannot have Transitions",
                    s.name
                ));
            }
            levels(&s.children, Some(&s.name), errors);
        }
    }
    levels(&machine.states, None, errors);

    let names: BTreeSet<&str> = machine
        .walk_states()
        .into_iter()
        .map(|(_, s)| s.name.as_str())
        .collect();
    for (_, state) in machine.walk_states() {
        for t in &state.transitions {
            if !names.contains(t.target.as_str()) {
                errors.push(format!(
                    "State `{}`: Transition on `{}` goes to `{}`, which is not a State of this Machine",
                    state.name,
                    event_ref_text(&t.event),
                    t.target
                ));
            }
        }
    }
}

/// `owner.EVENT`, or the bare `EVENT`.
pub fn event_ref_text(event: &EventRef) -> String {
    match &event.owner {
        Some(owner) => format!("{owner}.{}", event.event),
        None => event.event.clone(),
    }
}

fn parse_transition(body: &str, item: &md::Item) -> Result<Transition, String> {
    let (code, code_end) = item.code.as_ref().expect("filtered");
    let event = parse_event_ref(code).ok_or_else(|| {
        format!("`{code}` is not an Event: write `owner.EVENT` (owner a Machine or Actor id, EVENT in UPPER_SNAKE_CASE) or a bare `EVENT`")
    })?;
    let rest = body[*code_end..item.range.end].trim_start();
    let rest = rest
        .strip_prefix('→')
        .or_else(|| rest.strip_prefix("->"))
        .ok_or_else(|| format!("Transition `{code}` needs `→ Target State` after the Event"))?;
    let (first_line, more) = rest.split_once('\n').unwrap_or((rest, ""));
    let (target, prose_head) = match first_line.split_once(':') {
        Some((target, prose)) => (target.trim(), prose.trim()),
        None => (first_line.trim(), ""),
    };
    if target.is_empty() {
        return Err(format!("Transition `{code}` has no target State"));
    }
    let mut prose = prose_head.to_owned();
    for line in more.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if !prose.is_empty() {
            prose.push('\n');
        }
        prose.push_str(line);
    }
    Ok(Transition {
        event,
        target: target.to_owned(),
        prose,
    })
}

fn parse_events(
    body: &str,
    outline: &Outline,
    events_heading: usize,
    errors: &mut Vec<String>,
) -> Vec<EventDecl> {
    let mut events = Vec::new();
    let mut seen = BTreeSet::new();
    for (i, h) in outline.headings.iter().enumerate().skip(events_heading + 1) {
        if h.level <= 2 {
            break;
        }
        if h.level != 3 {
            errors.push(format!(
                "`{}` under `## Events` must be a `###` heading",
                h.text
            ));
            continue;
        }
        if !is_event_name(&h.text) {
            errors.push(format!("Event name `{}` must be UPPER_SNAKE_CASE", h.text));
        } else if !seen.insert(h.text.clone()) {
            errors.push(format!("Event `{}` is declared twice", h.text));
        }
        let (start, end) = section_bounds(body, outline, i);
        events.push(EventDecl {
            name: h.text.clone(),
            description: body[start..end].trim().to_owned(),
        });
    }
    events
}

/// `Name (initial)` → (`Name`, Initial). Only one trailing marker is recognised; anything else
/// stays in the name, where the character check rejects the parentheses.
fn split_marker(text: &str) -> (String, Marker) {
    for (suffix, marker) in [("(initial)", Marker::Initial), ("(final)", Marker::Final)] {
        if let Some(name) = text.strip_suffix(suffix) {
            if name.is_empty() || name.ends_with(' ') {
                return (name.trim().to_owned(), marker);
            }
        }
    }
    (text.trim().to_owned(), Marker::None)
}

fn body_of(text: &str) -> (String, Vec<String>) {
    match frontmatter::parse(text) {
        Ok(doc) => (doc.body, Vec::new()),
        Err(e) => (text.to_owned(), vec![e]),
    }
}

fn single_h1(outline: &Outline, errors: &mut Vec<String>) -> Option<(String, usize)> {
    let h1s: Vec<_> = outline
        .headings
        .iter()
        .enumerate()
        .filter(|(_, h)| h.level == 1)
        .collect();
    match h1s.as_slice() {
        [] => {
            errors.push("the file needs one H1 with the name".to_owned());
            None
        }
        [(i, h)] => {
            if h.text.is_empty() {
                errors.push("the H1 is empty".to_owned());
            }
            Some((h.text.clone(), *i))
        }
        _ => {
            errors.push(format!(
                "the file has {} H1 headings; exactly one is required",
                h1s.len()
            ));
            Some((h1s[0].1.text.clone(), h1s[0].0))
        }
    }
}

fn finish<T>(errors: Vec<String>, build: impl FnOnce() -> T) -> Result<T, Vec<String>> {
    if errors.is_empty() {
        Ok(build())
    } else {
        Err(errors)
    }
}

/// The source between the end of heading `index` and the next heading (any level).
fn section_bounds(body: &str, outline: &Outline, index: usize) -> (usize, usize) {
    let start = outline.headings[index].range.end;
    let end = outline
        .headings
        .get(index + 1)
        .map_or(body.len(), |h| h.range.start);
    (start, end)
}

/// Prose after the H1 up to the first H2 (or the end).
fn description_after_h1(body: &str, outline: &Outline, h1_index: usize) -> String {
    let start = outline.headings[h1_index].range.end;
    let end = outline
        .headings
        .iter()
        .skip(h1_index + 1)
        .find(|h| h.level <= 2)
        .map_or(body.len(), |h| h.range.start);
    body[start..end].trim().to_owned()
}

// ---------------------------------------------------------------------------------------
// Assembly and cross-file rules
// ---------------------------------------------------------------------------------------

/// The parsed Model files, by the `.trama/model/` path they came from.
#[derive(Debug, Default)]
pub struct ModelFiles<'a> {
    pub system: Option<(&'a str, &'a Titled)>,
    /// `(path, actor)`
    pub actors: Vec<(&'a str, &'a Actor)>,
    /// `(path, feature id, feature.md)`
    pub features: Vec<(&'a str, &'a str, &'a Titled)>,
    pub machines: Vec<(&'a str, &'a Machine)>,
}

impl Model {
    /// Builds the Model from the parsed files and returns what the files break together:
    /// reserved or duplicated names in the shared namespace, a Transition whose owner does not
    /// exist or does not declare the Event. Files that failed to parse are simply absent.
    pub fn assemble(files: &ModelFiles) -> (Model, Vec<Diagnostic>) {
        let mut diagnostics = Vec::new();
        let mut model = Model {
            system: files.system.map(|(_, t)| t.clone()),
            ..Model::default()
        };
        let mut owners: BTreeMap<String, (ElementKind, String)> = BTreeMap::new();

        let mut claim = |id: &str,
                         kind: ElementKind,
                         path: &str,
                         diagnostics: &mut Vec<Diagnostic>|
         -> bool {
            if !is_element_id(id) {
                diagnostics.push(Diagnostic::error(
                    path,
                    format!(
                        "`{id}` is not a valid id: use kebab-case (`order`, `payment-gateway`)"
                    ),
                ));
                return false;
            }
            if RESERVED_IDS.contains(&id) {
                diagnostics.push(Diagnostic::error(
                    path,
                    format!("`{id}` is a reserved name and cannot be a Feature, Machine or Actor"),
                ));
                return false;
            }
            if let Some((_, first)) = owners.get(id) {
                diagnostics.push(Diagnostic::error(
                    path,
                    format!("`{id}` is already used by `{first}`: Features, Machines and Actors share one namespace"),
                ));
                return false;
            }
            owners.insert(id.to_owned(), (kind, path.to_owned()));
            true
        };

        for (path, feature, titled) in &files.features {
            if claim(feature, ElementKind::Feature, path, &mut diagnostics) {
                model
                    .features
                    .insert((*feature).to_owned(), (*titled).clone());
            }
        }
        for (path, actor) in &files.actors {
            if claim(&actor.id, ElementKind::Actor, path, &mut diagnostics) {
                model.actors.insert(actor.id.clone(), (*actor).clone());
            }
        }
        for (path, machine) in &files.machines {
            if claim(&machine.id, ElementKind::Machine, path, &mut diagnostics) {
                model
                    .machines
                    .insert(machine.id.clone(), (*machine).clone());
            }
        }

        let paths: BTreeMap<&str, &str> = files
            .machines
            .iter()
            .map(|(p, m)| (m.id.as_str(), *p))
            .collect();
        for machine in model.machines.values() {
            let path = paths.get(machine.id.as_str()).copied().unwrap_or_default();
            for (_, state) in machine.walk_states() {
                for t in &state.transitions {
                    let owner = machine.owner_of(&t.event);
                    let declared = if let Some(m) = model.machines.get(owner) {
                        Some(m.events.iter().any(|e| e.name == t.event.event))
                    } else {
                        model
                            .actors
                            .get(owner)
                            .map(|a| a.events.iter().any(|e| e.name == t.event.event))
                    };
                    match declared {
                        None => diagnostics.push(Diagnostic::error(
                            path,
                            format!(
                                "State `{}`: `{}` names `{owner}`, which is not a Machine or an Actor",
                                state.name,
                                event_ref_text(&t.event)
                            ),
                        )),
                        Some(false) => diagnostics.push(Diagnostic::error(
                            path,
                            format!(
                                "State `{}`: `{owner}` does not declare the Event `{}` under its `## Events`",
                                state.name, t.event.event
                            ),
                        )),
                        Some(true) => {}
                    }
                }
            }
        }
        (model, diagnostics)
    }

    /// The kind of element `id` names in the shared namespace.
    pub fn element_kind(&self, id: &str) -> Option<ElementKind> {
        if self.features.contains_key(id) {
            Some(ElementKind::Feature)
        } else if self.machines.contains_key(id) {
            Some(ElementKind::Machine)
        } else if self.actors.contains_key(id) {
            Some(ElementKind::Actor)
        } else {
            None
        }
    }

    /// Whether the Model has no Feature and no Actor.
    pub fn is_empty(&self) -> bool {
        self.features.is_empty() && self.actors.is_empty()
    }

    /// Events declared by a Machine or an Actor.
    pub fn declared_events(&self, owner: &str) -> Option<&[EventDecl]> {
        self.machines
            .get(owner)
            .map(|m| m.events.as_slice())
            .or_else(|| self.actors.get(owner).map(|a| a.events.as_slice()))
    }
}

#[cfg(test)]
pub(crate) const ORDER_MACHINE: &str = "# Order

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

#[cfg(test)]
pub(crate) const USER_ACTOR: &str = "# User

A person browsing and buying from the shop.

## Events

### CANCEL
The user asks to cancel an order.
";

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(text: &str) -> Result<Machine, Vec<String>> {
        parse_machine("shop", "order", text)
    }

    fn errors(text: &str) -> String {
        machine(text).expect_err("must fail").join(" | ")
    }

    /// A Machine with one State, to which each test adds what it checks.
    fn with_states(states: &str) -> String {
        format!("# M\n\n## States\n\n{states}")
    }

    #[test]
    fn the_format_example_machine_parses_with_its_structure() {
        let m = machine(ORDER_MACHINE).unwrap();
        assert_eq!(m.title, "Order");
        assert_eq!(
            m.description,
            "An order placed by a customer, from composition to delivery."
        );
        assert_eq!(
            m.states.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            [
                "Draft",
                "Awaiting payment",
                "Paid",
                "Refunding",
                "Shipped",
                "Cancelled"
            ]
        );
        let paid = m.find_state("Paid").unwrap();
        assert_eq!(
            paid.children
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            ["Preparing", "Ready to ship"]
        );
        assert_eq!(paid.children[0].marker, Marker::Initial);
        assert_eq!(paid.transitions.len(), 2);
        assert_eq!(paid.transitions[1].target, "Refunding");
        assert_eq!(paid.transitions[1].prose, "once preparation has started.");
        assert_eq!(m.find_state("Shipped").unwrap().marker, Marker::Final);
        assert_eq!(
            m.find_state("Draft").unwrap().description,
            "The customer is still choosing what to buy."
        );
        let draft = &m.states[0];
        assert_eq!(
            draft.transitions[0].event,
            EventRef {
                owner: Some("cart".into()),
                event: "CONFIRM".into()
            }
        );
        assert_eq!(
            m.events,
            vec![EventDecl {
                name: "CONFIRMED".into(),
                description: "Emitted when the order leaves Draft.".into()
            }]
        );
        assert_eq!(m.feature, "shop");
        let awaiting = m.find_state("Awaiting payment").unwrap();
        assert_eq!(awaiting.transitions[0].prose, "");
    }

    #[test]
    fn the_format_example_actor_parses() {
        let a = parse_actor("user", USER_ACTOR).unwrap();
        assert_eq!(a.title, "User");
        assert_eq!(a.description, "A person browsing and buying from the shop.");
        assert_eq!(a.events[0].name, "CANCEL");
        assert_eq!(a.events[0].description, "The user asks to cancel an order.");
    }

    #[test]
    fn an_actor_without_events_is_fine() {
        assert!(parse_actor("time", "# Time\n\nPassing time.\n")
            .unwrap()
            .events
            .is_empty());
    }

    #[test]
    fn actors_have_no_states_and_no_transitions() {
        let e = parse_actor("user", "# User\n\n## States\n\n### S (initial)\n")
            .unwrap_err()
            .join("|");
        assert!(e.contains("only `## Events`"), "{e}");
    }

    #[test]
    fn system_and_feature_files_are_an_h1_and_prose() {
        let t = parse_titled("# Weaver\n\nA desktop app.\n\nSecond paragraph.\n").unwrap();
        assert_eq!(t.title, "Weaver");
        assert_eq!(t.description, "A desktop app.\n\nSecond paragraph.");
        assert!(parse_titled("no heading\n").is_err());
        assert!(parse_titled("# A\n\n# B\n")
            .unwrap_err()
            .join("")
            .contains("2 H1"));
    }

    #[test]
    fn a_machine_has_exactly_one_h1() {
        assert!(errors("## States\n\n### A (initial)\n").contains("needs one H1"));
        assert!(errors("# A\n# B\n\n## States\n\n### A (initial)\n").contains("2 H1"));
    }

    #[test]
    fn states_section_is_required_and_events_must_follow_it() {
        assert!(errors("# M\n\nprose\n").contains("`## States` is required"));
        assert!(
            errors("# M\n\n## Events\n\n### E\n\n## States\n\n### A (initial)\n")
                .contains("must come before")
        );
        assert!(
            errors("# M\n\n## States\n\n## States\n\n### A (initial)\n").contains("more than once")
        );
    }

    #[test]
    fn no_other_h2() {
        let e = errors(&with_states("### A (initial)\n\n## Notes\n"));
        assert!(e.contains("unexpected section `## Notes`"), "{e}");
    }

    #[test]
    fn heading_levels_nest_and_never_skip() {
        let e = errors(&with_states("### A (initial)\n\n##### Deep (initial)\n"));
        assert!(e.contains("skips a heading level"), "{e}");
        let e = errors("# M\n\n## States\n\n#### A (initial)\n");
        assert!(e.contains("must be a `###` heading"), "{e}");
        assert!(machine(&with_states(
            "### A (initial)\n#### B (initial)\n##### C (initial)\n###### D (initial)\n"
        ))
        .is_ok());
    }

    #[test]
    fn sub_states_attach_to_the_nearest_shallower_state() {
        let m = machine(&with_states(
            "### A (initial)\n#### A1 (initial)\n##### A1a (initial)\n#### A2\n### B\n",
        ))
        .unwrap();
        assert_eq!(m.states.len(), 2);
        assert_eq!(m.states[0].children.len(), 2);
        assert_eq!(m.states[0].children[0].children[0].name, "A1a");
        assert!(m.states[1].children.is_empty());
    }

    #[test]
    fn state_headings_accept_one_marker_only() {
        let e = errors(&with_states("### A (initial) (final)\n"));
        assert!(e.contains("contains `(`"), "{e}");
        let e = errors(&with_states("### A (initial)\n### B (maybe)\n"));
        assert!(e.contains("contains `(`"), "{e}");
    }

    #[test]
    fn state_names_cannot_contain_reserved_characters() {
        for bad in ["A/B", "A:B", "A→B", "A`B"] {
            let e = errors(&with_states(&format!("### {bad} (initial)\n")));
            assert!(e.contains("names cannot contain"), "{bad}: {e}");
        }
        assert!(machine(&with_states("### Ready to ship, v1.0 (initial)\n")).is_ok());
    }

    #[test]
    fn state_names_are_unique_case_insensitive_at_any_depth() {
        let e = errors(&with_states("### A (initial)\n#### a (initial)\n"));
        assert!(e.contains("used twice"), "{e}");
        let e = errors(&with_states("### A (initial)\n### B\n#### A (initial)\n"));
        assert!(e.contains("used twice"), "{e}");
    }

    #[test]
    fn exactly_one_initial_per_level() {
        assert!(errors(&with_states("### A\n### B\n")).contains("0 `(initial)`"));
        assert!(
            errors(&with_states("### A (initial)\n### B (initial)\n")).contains("2 `(initial)`")
        );
        let e = errors(&with_states("### A (initial)\n#### A1\n#### A2\n"));
        assert!(e.contains("sub-States of `A` has 0"), "{e}");
        let e = errors(&with_states(
            "### A (initial)\n#### A1 (initial)\n#### A2 (initial)\n",
        ));
        assert!(e.contains("sub-States of `A` has 2"), "{e}");
    }

    #[test]
    fn final_states_have_no_children_and_no_transitions() {
        let e = errors(&with_states(
            "### A (initial)\n### B (final)\n#### B1 (initial)\n",
        ));
        assert!(
            e.contains("`(final)` State `B` cannot have sub-States"),
            "{e}"
        );
        let e = errors(&with_states(
            "### A (initial)\n- `E` → B\n### B (final)\n- `E` → A\n",
        ));
        assert!(
            e.contains("`(final)` State `B` cannot have Transitions"),
            "{e}"
        );
    }

    #[test]
    fn transitions_are_list_items_starting_with_a_code_span() {
        let m = machine(&with_states(
            "### A (initial)\n- plain prose item\n- `cart.CONFIRM` → B: only if ok\n  continued here\n  - nested item\n### B\n",
        ))
        .unwrap();
        let a = &m.states[0];
        assert_eq!(a.transitions.len(), 1);
        assert_eq!(
            a.transitions[0].prose,
            "only if ok\ncontinued here\n- nested item"
        );
        assert!(a.description.contains("plain prose item"));
        assert!(!a.description.contains("cart.CONFIRM"));
    }

    #[test]
    fn the_ascii_arrow_is_accepted() {
        let m = machine(&with_states("### A (initial)\n- `E` -> B: x\n### B\n")).unwrap();
        assert_eq!(m.states[0].transitions[0].target, "B");
    }

    #[test]
    fn a_code_span_item_without_an_arrow_or_target_is_an_error() {
        assert!(errors(&with_states("### A (initial)\n- `E` goes to B\n"))
            .contains("needs `→ Target State`"));
        assert!(errors(&with_states("### A (initial)\n- `E` →\n")).contains("no target"));
        assert!(errors(&with_states("### A (initial)\n- `E` → : prose\n")).contains("no target"));
    }

    #[test]
    fn transition_events_are_owner_dot_event_or_a_bare_event() {
        assert!(
            errors(&with_states("### A (initial)\n- `payment.declined` → A\n"))
                .contains("is not an Event")
        );
        assert!(
            errors(&with_states("### A (initial)\n- `Payment.DECLINED` → A\n"))
                .contains("is not an Event")
        );
        assert!(
            errors(&with_states("### A (initial)\n- `some thing` → A\n"))
                .contains("is not an Event")
        );
        let m = machine(&with_states(
            "### A (initial)\n- `DONE` → A\n- `x-y.GO` → A\n",
        ))
        .unwrap();
        assert_eq!(m.states[0].transitions[0].event.owner, None);
        assert_eq!(m.owner_of(&m.states[0].transitions[0].event), "order");
        assert_eq!(
            m.states[0].transitions[1].event.owner.as_deref(),
            Some("x-y")
        );
    }

    #[test]
    fn the_target_must_be_a_state_of_the_same_machine_at_any_depth() {
        let e = errors(&with_states("### A (initial)\n- `E` → Nowhere\n"));
        assert!(e.contains("not a State of this Machine"), "{e}");
        assert!(machine(&with_states(
            "### A (initial)\n- `E` → A2\n#### A1 (initial)\n#### A2\n"
        ))
        .is_ok());
    }

    #[test]
    fn the_same_event_may_leave_a_state_towards_different_targets() {
        let m = machine(&with_states(
            "### A (initial)\n- `E` → B: if x\n- `E` → C: if y\n### B\n### C\n",
        ))
        .unwrap();
        assert_eq!(m.states[0].transitions.len(), 2);
    }

    #[test]
    fn transitions_outside_a_state_are_errors_but_prose_items_elsewhere_are_not() {
        let e = errors("# M\n\n## States\n\n- `E` → A\n\n### A (initial)\n");
        assert!(e.contains("not under a State"), "{e}");
        let m = machine("# M\n\n- `x` is a note\n\n## States\n\n### A (initial)\n\n## Events\n\n### GO\n- `x.Y` → not a transition\n").unwrap();
        assert_eq!(m.events[0].name, "GO");
    }

    #[test]
    fn headings_in_code_fences_are_not_states() {
        let m = machine(&with_states(
            "### A (initial)\n\n```\n### Fake (initial)\n- `E` → Z\n```\n",
        ))
        .unwrap();
        assert_eq!(m.states.len(), 1);
        assert!(m.states[0].transitions.is_empty());
    }

    #[test]
    fn event_names_are_upper_snake_case_and_unique_in_their_owner() {
        assert!(
            errors(&with_states("### A (initial)\n\n## Events\n\n### lower\n"))
                .contains("UPPER_SNAKE_CASE")
        );
        assert!(errors(&with_states(
            "### A (initial)\n\n## Events\n\n### Mixed_Case\n"
        ))
        .contains("UPPER_SNAKE_CASE"));
        assert!(errors(&with_states(
            "### A (initial)\n\n## Events\n\n### DONE\n\n### DONE\n"
        ))
        .contains("declared twice"));
        assert!(
            errors(&with_states("### A (initial)\n\n## Events\n\n#### DEEP\n"))
                .contains("must be a `###`")
        );
        for good in ["A", "A1", "A_B", "TASK_DONE_2"] {
            assert!(is_event_name(good), "{good}");
        }
        for bad in ["", "_A", "A_", "A__B", "1A", "a", "A-B"] {
            assert!(!is_event_name(bad), "{bad}");
        }
    }

    #[test]
    fn element_ids_are_kebab_case() {
        for good in ["order", "payment-gateway", "a1", "1a"] {
            assert!(is_element_id(good), "{good}");
        }
        for bad in ["", "Order", "a_b", "-a", "a-", "a--b", "a b"] {
            assert!(!is_element_id(bad), "{bad}");
        }
    }

    fn assemble(files: &[(&str, &str, &str, &str)]) -> (Model, Vec<Diagnostic>) {
        // (kind, feature, id, text): kind is `m`, `a`, `f`.
        let mut titled = Vec::new();
        let mut actors = Vec::new();
        let mut machines = Vec::new();
        for (kind, feature, id, text) in files {
            match *kind {
                "f" => titled.push((
                    format!("model/{id}/feature.md"),
                    (*id).to_owned(),
                    parse_titled(text).unwrap(),
                )),
                "a" => actors.push((
                    format!("model/actors/{id}.md"),
                    parse_actor(id, text).unwrap(),
                )),
                _ => machines.push((
                    format!("model/{feature}/{id}.md"),
                    parse_machine(feature, id, text).unwrap(),
                )),
            }
        }
        let files = ModelFiles {
            system: None,
            actors: actors.iter().map(|(p, a)| (p.as_str(), a)).collect(),
            features: titled
                .iter()
                .map(|(p, id, t)| (p.as_str(), id.as_str(), t))
                .collect(),
            machines: machines.iter().map(|(p, m)| (p.as_str(), m)).collect(),
        };
        Model::assemble(&files)
    }

    #[test]
    fn features_machines_and_actors_share_one_namespace() {
        let (model, diags) = assemble(&[
            ("f", "", "order", "# Order\n"),
            ("m", "order", "order", ORDER_MACHINE),
        ]);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("share one namespace")),
            "{diags:?}"
        );
        assert!(model.machines.is_empty());
        let (_, diags) = assemble(&[("a", "", "user", USER_ACTOR), ("f", "", "user", "# U\n")]);
        assert!(
            diags.iter().any(|d| d.message.contains("already used")),
            "{diags:?}"
        );
    }

    #[test]
    fn reserved_names_cannot_be_used() {
        for name in RESERVED_IDS {
            let (model, diags) = assemble(&[("f", "", name, "# X\n")]);
            assert!(model.features.is_empty());
            assert!(
                diags.iter().any(|d| d.message.contains("reserved")),
                "{name}: {diags:?}"
            );
        }
        let (_, diags) = assemble(&[("m", "shop", "feature", &with_states("### A (initial)\n"))]);
        assert!(diags.iter().any(|d| d.message.contains("reserved")));
    }

    #[test]
    fn a_transition_needs_its_owner_to_declare_the_event() {
        let (_, diags) = assemble(&[
            ("f", "", "shop", "# Shop\n"),
            ("m", "shop", "order", ORDER_MACHINE),
            ("a", "", "user", USER_ACTOR),
        ]);
        let messages: Vec<_> = diags.iter().map(|d| d.message.as_str()).collect();
        // cart, payment, warehouse and courier are not declared anywhere; user.CANCEL is.
        assert!(
            messages
                .iter()
                .any(|m| m.contains("`cart.CONFIRM` names `cart`")),
            "{messages:?}"
        );
        assert!(
            !messages.iter().any(|m| m.contains("user.CANCEL")),
            "{messages:?}"
        );

        let (_, diags) = assemble(&[
            (
                "m",
                "shop",
                "order",
                &with_states(
                    "### A (initial)\n- `user.NOPE` → A\n- `GONE` → A\n\n## Events\n\n### HERE\n",
                ),
            ),
            ("a", "", "user", USER_ACTOR),
        ]);
        let messages: Vec<_> = diags.iter().map(|d| d.message.as_str()).collect();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("`user` does not declare the Event `NOPE`")),
            "{messages:?}"
        );
        assert!(
            messages
                .iter()
                .any(|m| m.contains("`order` does not declare the Event `GONE`")),
            "{messages:?}"
        );
    }

    #[test]
    fn a_machine_may_react_to_another_machines_event() {
        let (_, diags) = assemble(&[
            (
                "m",
                "shop",
                "order",
                &with_states("### A (initial)\n- `payment.PAID` → A\n"),
            ),
            (
                "m",
                "shop",
                "payment",
                &with_states("### P (initial)\n\n## Events\n\n### PAID\n"),
            ),
        ]);
        assert!(diags.is_empty(), "{diags:?}");
    }
}
