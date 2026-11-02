use std::collections::BTreeMap;
use std::path::Path;

use super::*;
use crate::project::changes::ChangeSet;
use crate::project::model::{ORDER_MACHINE, USER_ACTOR};
use crate::project::shortnum;
use crate::project::testing::put;

const SHOP: &str = "# Shop\n\nA shop where people buy things.\n";
const FEATURE: &str = "# Shopping\n\nEverything about buying.\n";
const AREA: &str = "---\ndescription: d\n---\n# UI\n\nHow the app looks.\n";
const MAIN: &str = "---\ndescription: d\n---\n# Main\n\n## General\n\nGeneral text.\n\n## order/Paid\n\nPaid text. More.\n";
const SPECS: &str = "languages:\n  - name: Rust\n    version: \"1.80\"\n  - name: Go\n    chosenByAgent: Fast to build\nnotes: |\n  Some notes.\n";
const DRIFT: &str =
    "---\nstatus: open\nrefs:\n  - order/Paid\ncreated: 2026-11-02T07:00:03.000Z\n---\n# A drift\n";
const TASK: &str = "---\ncolumn: todo\ntechnical: false\nrefs:\n  - order\ncreated: 2026-11-02T07:00:01.000Z\n---\n# A task\n";

fn sample(root: &Path) {
    put(root, "model/system.md", SHOP);
    put(root, "model/shop/feature.md", FEATURE);
    put(root, "model/shop/order.md", ORDER_MACHINE);
    put(root, "model/actors/user.md", USER_ACTOR);
    put(root, "requirements/ui/area.md", AREA);
    put(root, "requirements/ui/main.md", MAIN);
    put(root, "specs.yaml", SPECS);
    put(root, "drifts/dddddddd-a-drift.md", DRIFT);
    put(root, "tasks/tttttttt-a-task.md", TASK);
}

fn read(root: &Path, rel: &str) -> Option<String> {
    std::fs::read_to_string(root.join(".trama").join(rel)).ok()
}

struct Built {
    data: ViewData,
}

fn build_for(root: &Path, shown: Option<&ProjectImage>, extra: &[String]) -> Built {
    let current = ProjectImage::load(root);
    let numbers = shortnum::peek(root, current.cards());
    let shown_image = shown.unwrap_or(&current);
    let on_disk = |rel: &str| read(root, rel);
    let shown_text =
        |rel: &str| -> Option<(String, String)> { read(root, rel).map(|t| (rel.to_owned(), t)) };
    let input = ViewInput {
        current: &current,
        shown: shown_image,
        overlaid: shown.is_some(),
        numbers: &numbers,
        files: Files {
            current: &on_disk,
            shown: &shown_text,
        },
        extra_refs: extra,
    };
    Built {
        data: build(&input),
    }
}

fn level<'a>(data: &'a ViewData, key: &str) -> &'a Level {
    data.model
        .levels
        .iter()
        .find(|l| l.key == key)
        .unwrap_or_else(|| panic!("no level {key}"))
}

fn node<'a>(level: &'a Level, id: &str) -> &'a DiagramNode {
    level
        .nodes
        .iter()
        .find(|n| n.id == id)
        .unwrap_or_else(|| panic!("no node {id} in {}", level.key))
}

#[test]
fn the_root_lists_whole_system_the_actors_and_the_features_no_actor_reaches() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    put(dir.path(), "model/lonely/feature.md", "# Lonely\n");
    let data = build_for(dir.path(), None, &[]).data;
    let root = level(&data, "");
    assert!(root.grid && root.edges.is_empty());
    let ids: Vec<&str> = root.nodes.iter().map(|n| n.id.as_str()).collect();
    // The shop is reached by the user (user.CANCEL), the lonely Feature by nobody.
    assert_eq!(ids, ["whole", "actor:user", "feature:lonely"]);
    assert_eq!(
        node(root, "actor:user").subtitle.as_deref(),
        Some("1 feature")
    );
    assert_eq!(
        node(root, "feature:lonely").subtitle.as_deref(),
        Some("0 machines")
    );
    assert_eq!(
        node(root, "actor:user").opens.as_deref(),
        Some("actor:user")
    );
}

#[test]
fn an_actor_nobody_listens_to_cannot_be_opened() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    put(
        dir.path(),
        "model/actors/clock.md",
        "# Clock\n\nTime.\n\n## Events\n\n### TICK\nEvery second.\n",
    );
    let data = build_for(dir.path(), None, &[]).data;
    let clock = node(level(&data, ""), "actor:clock");
    assert_eq!(clock.subtitle.as_deref(), Some("No events received"));
    assert!(clock.opens.is_none() && clock.dashed && clock.muted);
    assert!(data.model.levels.iter().all(|l| l.key != "actor:clock"));
}

#[test]
fn whole_system_edges_count_events_and_an_actor_level_names_them() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    let data = build_for(dir.path(), None, &[]).data;
    let system = level(&data, "system");
    let edge = system
        .edges
        .iter()
        .find(|e| e.source == "actor:user" && e.target == "feature:shop")
        .expect("edge");
    assert_eq!(edge.label, "1");
    let actor = level(&data, "actor:user");
    assert_eq!(actor.edges[0].label, "CANCEL");
    assert_eq!(
        actor
            .nodes
            .iter()
            .map(|n| n.id.as_str())
            .collect::<Vec<_>>(),
        ["actor:user", "feature:shop"]
    );
}

#[test]
fn a_machine_level_has_states_with_markers_and_labelled_transitions() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    let data = build_for(dir.path(), None, &[]).data;
    let machine = level(&data, "machine:order");
    assert!(node(machine, "state:order/Draft").initial);
    assert!(node(machine, "state:order/Shipped").terminal);
    assert_eq!(
        node(machine, "state:order/Preparing").parent.as_deref(),
        Some("state:order/Paid")
    );
    let cancel = machine
        .edges
        .iter()
        .find(|e| e.label == "user.CANCEL" && e.target == "state:order/Cancelled")
        .expect("cancel edge");
    assert_eq!(cancel.source, "state:order/Paid");
    assert_eq!(
        cancel.detail.description,
        "only if nothing has been prepared yet; refunds the customer."
    );
    // The Transition's line is found in the file.
    let line = cancel.detail.line.expect("line");
    let text = ORDER_MACHINE.lines().nth(line as usize - 1).unwrap();
    assert!(
        text.contains("user.CANCEL") && text.contains("Cancelled"),
        "{text}"
    );
    assert_eq!(
        cancel.detail.file.as_deref(),
        Some(".trama/model/shop/order.md")
    );
}

#[test]
fn a_state_level_shows_entering_and_leaving_transitions_and_their_other_ends() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    let data = build_for(dir.path(), None, &[]).data;
    let state = level(&data, "state:order/Awaiting payment");
    let ids: Vec<&str> = state.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "state:order/Awaiting payment",
            "ext-state:order/Cancelled",
            "ext-state:order/Draft",
            "ext-state:order/Paid"
        ]
    );
    assert!(
        state.nodes[1].compact
            && state.nodes[1].dashed
            && state.nodes[1].kind == NodeKind::ExternalState
    );
    assert_eq!(state.edges.len(), 3);
    assert_eq!(state.parent.as_deref(), Some("machine:order"));
}

#[test]
fn a_feature_level_holds_its_machines_with_states_inside_and_outside_senders_as_compact_nodes() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    let data = build_for(dir.path(), None, &[]).data;
    let feature = level(&data, "feature:shop");
    let order = node(feature, "machine:order");
    assert!(order.expandable);
    assert_eq!(
        node(feature, "state:order/Draft").parent.as_deref(),
        Some("machine:order")
    );
    let outside = node(feature, "ext-actor:user");
    assert!(outside.compact && outside.dashed);
    let inside = feature
        .edges
        .iter()
        .filter(|e| e.inside.as_deref() == Some("machine:order"))
        .count();
    assert!(inside > 5);
    assert!(feature.edges.iter().any(|e| e.source == "ext-actor:user"
        && e.target == "machine:order"
        && e.label == "CANCEL"));
}

#[test]
fn levels_come_in_model_order() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    let data = build_for(dir.path(), None, &[]).data;
    let keys: Vec<&str> = data
        .model
        .levels
        .iter()
        .map(|l| l.key.as_str())
        .take(6)
        .collect();
    assert_eq!(
        keys,
        [
            "",
            "system",
            "actor:user",
            "feature:shop",
            "machine:order",
            "state:order/Draft"
        ]
    );
}

#[test]
fn open_drifts_and_tasks_that_concern_an_element_are_listed_and_counted() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    let data = build_for(dir.path(), None, &[]).data;
    let paid = &data.model.details["order/Paid"];
    assert_eq!(
        paid.drifts
            .iter()
            .map(|d| d.short_number.as_deref())
            .collect::<Vec<_>>(),
        [Some("D1")]
    );
    // The Task names the Machine, which contains the State.
    assert_eq!(
        paid.tasks
            .iter()
            .map(|t| t.short_number.as_deref())
            .collect::<Vec<_>>(),
        [Some("T1")]
    );
    let machine = level(&data, "machine:order");
    assert_eq!(node(machine, "state:order/Paid").drifts, 1);
    assert_eq!(node(machine, "state:order/Draft").drifts, 0);
    // The Machine holds the State, so its badge counts the Drift too.
    assert_eq!(
        node(level(&data, "feature:shop"), "machine:order").drifts,
        1
    );
}

#[test]
fn references_are_found_on_the_level_that_holds_them() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    let refs = [
        "order/Paid",
        "order/Paid/user.CANCEL",
        "user.CANCEL",
        "order",
        "shop",
        "user",
        "nothing",
        "requirements/ui",
        "requirements/ui#order/Paid",
        "requirements/ui#General",
        "specs#languages",
        "specs",
    ]
    .map(String::from);
    let data = build_for(dir.path(), None, &refs).data;
    let t = &data.targets;

    assert_eq!(t["order/Paid"].diagram[0].level, "machine:order");
    assert_eq!(t["order/Paid"].diagram[0].nodes, ["state:order/Paid"]);
    assert_eq!(
        t["order/Paid"].requirements,
        [SectionAnchor {
            area: "ui".into(),
            section: Some("order/Paid".into())
        }]
    );

    let transitions = &t["order/Paid/user.CANCEL"].diagram[0];
    assert_eq!(transitions.edges.len(), 2);
    assert_eq!(t["user.CANCEL"].diagram[0].level, "actor:user");
    assert_eq!(t["user.CANCEL"].diagram[0].edges.len(), 1);
    assert_eq!(t["order"].diagram[0].level, "feature:shop");
    assert_eq!(t["shop"].diagram[0].level, "system");
    assert_eq!(t["user"].diagram[0].level, "");

    assert!(
        t["nothing"].diagram.is_empty() && t["nothing"].relevant.contains(&TargetView::Diagram)
    );
    assert_eq!(t["requirements/ui"].relevant, [TargetView::Requirements]);
    assert_eq!(
        t["requirements/ui"].requirements,
        [SectionAnchor {
            area: "ui".into(),
            section: None
        }]
    );
    assert_eq!(
        t["requirements/ui#General"].requirements[0]
            .section
            .as_deref(),
        Some("General")
    );
    assert_eq!(
        t["requirements/ui#order/Paid"].diagram[0].nodes,
        ["state:order/Paid"]
    );
    assert_eq!(t["specs#languages"].specs, ["languages"]);
    assert_eq!(t["specs"].specs, ["languages", "notes"]);
    assert_eq!(t["specs"].relevant, [TargetView::Specs]);
    // The refs of the cards and the titles of the sections are resolved without being asked.
    assert!(t.contains_key("order/Paid"));
}

#[test]
fn requirements_have_areas_fragments_and_general_first() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    put(
        dir.path(),
        "requirements/ui/zeta.md",
        "---\ndescription: d\n---\n# Zeta\n\n## shop\n\nShop text.\n\n## General\n\nZ general.\n",
    );
    let data = build_for(dir.path(), None, &[]).data;
    let area = &data.requirements[0];
    assert_eq!((area.id.as_str(), area.title.as_str()), ("ui", "UI"));
    assert_eq!(
        area.fragments
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>(),
        ["main", "zeta"]
    );
    let zeta = &area.fragments[1];
    assert_eq!(
        zeta.sections
            .iter()
            .map(|s| s.title.as_str())
            .collect::<Vec<_>>(),
        ["General", "shop"]
    );
    let paid = &area.fragments[0].sections[1];
    assert_eq!(paid.body, "Paid text. More.");
    // The heading's line, past the front matter.
    assert_eq!(paid.line, Some(10));
    assert_eq!(paid.file.as_deref(), Some(".trama/requirements/ui/main.md"));
    assert!(paid.change.is_none() && area.fragments.iter().all(|f| f.change.is_none()));
}

#[test]
fn specs_fields_come_in_the_fixed_order_and_empty_ones_are_hidden() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    let data = build_for(dir.path(), None, &[]).data;
    let specs = data.specs.unwrap();
    assert_eq!(
        specs
            .fields
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>(),
        ["languages", "notes"]
    );
    let languages = &specs.fields[0];
    assert_eq!(languages.entries[0].version.as_deref(), Some("1.80"));
    assert_eq!(
        languages.entries[1].chosen_reason.as_deref(),
        Some("Fast to build")
    );
    assert_eq!(languages.line, Some(1));
    assert_eq!(
        specs.fields[1].notes.as_deref().map(str::trim),
        Some("Some notes.")
    );
}

#[test]
fn a_project_without_specs_has_none() {
    let dir = tempfile::tempdir().unwrap();
    put(dir.path(), "model/system.md", SHOP);
    assert!(build_for(dir.path(), None, &[]).data.specs.is_none());
}

fn overlay(root: &Path) -> ProjectImage {
    let current = ProjectImage::load(root);
    let mut changes = ChangeSet::new();
    // The Machine: `Paid` changed, `Refunding` removed, `Returned` added; a Transition changed.
    let machine = ORDER_MACHINE
        .replace("- `user.CANCEL` → Cancelled: only if nothing has been prepared yet; refunds the customer.", "- `user.CANCEL` → Cancelled: now always refunds.")
        .replace("### Refunding\n- `payment.REFUNDED` → Cancelled\n\n", "")
        .replace("- `user.CANCEL` → Refunding: once preparation has started.\n", "")
        .replace("### Shipped (final)", "### Returned\nThe customer sent it back.\n\n### Shipped (final)")
        .replace("The customer is still choosing what to buy.", "The customer picks goods.");
    changes.write("model/shop/order.md", machine);
    changes.write(
        "requirements/ui/main.md",
        MAIN.replace("Paid text. More.", "Paid text, changed.")
            .replace("## General", "## shop\n\nNew section.\n\n## General"),
    );
    changes.write(
        "requirements/ui/extra.md",
        "---\ndescription: d\n---\n# Extra\n\n## General\n\nExtra general.\n",
    );
    changes.write(
        "specs.yaml",
        SPECS
            .replace("version: \"1.80\"", "version: \"1.81\"")
            .replace(
                "  - name: Go\n    chosenByAgent: Fast to build\n",
                "  - name: Zig\n",
            ),
    );
    current.with_changes(&changes)
}

#[test]
fn an_overlay_marks_added_changed_and_removed_and_keeps_removed_elements_drawn() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    let shown = overlay(dir.path());
    let data = build_for(dir.path(), Some(&shown), &[]).data;

    let machine = level(&data, "machine:order");
    assert_eq!(
        node(machine, "state:order/Returned").change,
        Some(Change::Added)
    );
    assert_eq!(
        node(machine, "state:order/Refunding").change,
        Some(Change::Removed)
    );
    assert_eq!(
        node(machine, "state:order/Draft").change,
        Some(Change::Changed)
    );
    assert_eq!(node(machine, "state:order/Paid").change, None);
    let changed = machine
        .edges
        .iter()
        .find(|e| e.label == "user.CANCEL" && e.target == "state:order/Cancelled")
        .unwrap();
    assert_eq!(changed.change, Some(Change::Changed));
    let removed = machine
        .edges
        .iter()
        .find(|e| e.target == "state:order/Refunding" && e.label == "user.CANCEL")
        .unwrap();
    assert_eq!(removed.change, Some(Change::Removed));
    assert_eq!(removed.detail.title, "user.CANCEL");

    // The details of a removed element still name its file and line (in the file on disk).
    let refunding = &data.model.details["order/Refunding"];
    assert_eq!(refunding.change, Some(Change::Removed));
    assert_eq!(
        refunding.file.as_deref(),
        Some(".trama/model/shop/order.md")
    );
    let text = ORDER_MACHINE
        .lines()
        .nth(refunding.line.unwrap() as usize - 1)
        .unwrap();
    assert_eq!(text, "### Refunding");
}

#[test]
fn an_overlay_labels_requirements_fragments_and_sections() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    let shown = overlay(dir.path());
    let data = build_for(dir.path(), Some(&shown), &[]).data;
    let area = &data.requirements[0];
    let names: Vec<(&str, Option<FragmentChange>)> = area
        .fragments
        .iter()
        .map(|f| (f.name.as_str(), f.change))
        .collect();
    assert_eq!(
        names,
        [("extra", Some(FragmentChange::New)), ("main", None)]
    );
    let main = &area.fragments[1];
    let by_title: BTreeMap<&str, &SectionView> = main
        .sections
        .iter()
        .map(|s| (s.title.as_str(), s))
        .collect();
    assert_eq!(by_title["shop"].change, Some(Change::Added));
    let paid = by_title["order/Paid"];
    assert_eq!(paid.change, Some(Change::Changed));
    assert_eq!(paid.before.as_deref(), Some("Paid text. More."));
    assert_eq!(by_title["General"].change, None);
    // `General` of the new fragment is the same section as `General` of `main`, which exists
    // already: the new fragment's copy reads as moved.
    assert_eq!(
        area.fragments[0].sections[0].moved_from.as_deref(),
        Some("main")
    );
}

#[test]
fn an_overlay_labels_removed_sections_and_removed_fragments() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    let current = ProjectImage::load(dir.path());
    let mut changes = ChangeSet::new();
    changes.delete("requirements/ui/main.md");
    changes.write(
        "requirements/ui/other.md",
        "---\ndescription: d\n---\n# Other\n\n## General\n\nGeneral text.\n",
    );
    let shown = current.with_changes(&changes);
    let data = build_for(dir.path(), Some(&shown), &[]).data;
    let area = &data.requirements[0];
    let main = area.fragments.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.change, Some(FragmentChange::Removed));
    // `General` moved to `other`; only the section that vanished is listed as removed.
    assert_eq!(
        main.sections
            .iter()
            .map(|s| (s.title.as_str(), s.change))
            .collect::<Vec<_>>(),
        [("order/Paid", Some(Change::Removed))]
    );
    let other = area.fragments.iter().find(|f| f.name == "other").unwrap();
    assert_eq!(other.change, Some(FragmentChange::New));
    assert_eq!(other.sections[0].moved_from.as_deref(), Some("main"));
}

#[test]
fn an_overlay_labels_specs_entries() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    let shown = overlay(dir.path());
    let data = build_for(dir.path(), Some(&shown), &[]).data;
    let specs = data.specs.unwrap();
    let languages = &specs.fields[0];
    assert_eq!(languages.change, Some(Change::Changed));
    let by_name: BTreeMap<&str, &SpecEntryView> = languages
        .entries
        .iter()
        .map(|e| (e.name.as_str(), e))
        .collect();
    assert_eq!(by_name["Rust"].change, Some(Change::Changed));
    assert_eq!(by_name["Rust"].before.as_deref(), Some("Rust 1.80"));
    assert_eq!(by_name["Zig"].change, Some(Change::Added));
    assert_eq!(by_name["Go"].change, Some(Change::Removed));
}

#[test]
fn a_removed_element_is_still_found_by_its_reference() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    let shown = overlay(dir.path());
    let refs = vec!["order/Refunding".to_owned(), "order/Returned".to_owned()];
    let data = build_for(dir.path(), Some(&shown), &refs).data;
    assert_eq!(
        data.targets["order/Refunding"].diagram[0].nodes,
        ["state:order/Refunding"]
    );
    assert_eq!(
        data.targets["order/Returned"].diagram[0].nodes,
        ["state:order/Returned"]
    );
}

#[test]
fn only_open_proposals_not_attached_to_a_drift_can_be_drawn() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = |status: &str, n: u32| {
        format!("---\nstatus: {status}\nkind: change\ncreated: 2026-11-02T07:00:0{n}.000Z\n---\n# Proposal {n}\n")
    };
    put(
        dir.path(),
        "proposals/aaaaaaaa-one/proposal.md",
        &proposal("open", 1),
    );
    put(
        dir.path(),
        "proposals/bbbbbbbb-two/proposal.md",
        &proposal("draft", 2),
    );
    put(
        dir.path(),
        "proposals/cccccccc-three/proposal.md",
        &proposal("refining", 3),
    );
    put(
        dir.path(),
        "proposals/dddddddd-four/proposal.md",
        &proposal("open", 4),
    );
    put(dir.path(), "drifts/eeeeeeee-d.md", "---\nstatus: open\nrefs: []\nproposal: dddddddd\ncreated: 2026-11-02T07:00:05.000Z\n---\n# D\n");
    let image = ProjectImage::load(dir.path());
    let numbers = shortnum::peek(dir.path(), image.cards());
    let options = eligible_overlays(&image, &numbers);
    assert_eq!(options.len(), 1);
    assert_eq!(
        (options[0].id.as_str(), options[0].title.as_str()),
        ("aaaaaaaa", "Proposal 1")
    );
    assert!(options[0].short_number.is_some());
}

#[test]
fn errors_name_the_file_and_the_line_when_the_message_names_a_heading() {
    let dir = tempfile::tempdir().unwrap();
    sample(dir.path());
    // `Refunding` targets a State that does not exist.
    put(
        dir.path(),
        "model/shop/order.md",
        &ORDER_MACHINE.replace("→ Cancelled\n\n### Shipped", "→ Nowhere\n\n### Shipped"),
    );
    put(dir.path(), "model/shop/broken.md", "no heading at all\n");
    put(
        dir.path(),
        "tasks/zzzzzzzz-bad.md",
        "---\nbroken: [\n---\n# x\n",
    );
    let data = build_for(dir.path(), None, &[]).data;
    assert!(
        data.errors
            .iter()
            .any(|e| e.file == ".trama/model/shop/broken.md"),
        "{:?}",
        data.errors
    );
    // A broken Task file is not an error of the Model.
    assert!(data.errors.iter().all(|e| !e.file.contains("tasks/")));
    let order = data
        .errors
        .iter()
        .find(|e| e.file == ".trama/model/shop/order.md")
        .expect("order error");
    let line = order.line.expect("a line");
    assert!(ORDER_MACHINE
        .replace("→ Cancelled\n\n### Shipped", "→ Nowhere\n\n### Shipped")
        .lines()
        .nth(line as usize - 1)
        .unwrap()
        .starts_with('#'));
}

#[test]
fn an_empty_project_builds_empty_views() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".trama")).unwrap();
    let data = build_for(dir.path(), None, &[]).data;
    assert_eq!(data.model.levels.len(), 2);
    assert!(data.requirements.is_empty() && data.specs.is_none() && data.errors.is_empty());
}

#[test]
fn aggregate_is_added_removed_or_changed() {
    assert_eq!(aggregate([]), None);
    assert_eq!(aggregate([None, None]), None);
    assert_eq!(
        aggregate([Some(Change::Added), Some(Change::Added)]),
        Some(Change::Added)
    );
    assert_eq!(aggregate([Some(Change::Removed)]), Some(Change::Removed));
    assert_eq!(
        aggregate([Some(Change::Added), None]),
        Some(Change::Changed)
    );
    assert_eq!(
        aggregate([Some(Change::Added), Some(Change::Removed)]),
        Some(Change::Changed)
    );
}
