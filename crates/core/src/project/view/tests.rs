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
    std::fs::read_to_string(root.join(".weaver").join(rel)).ok()
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

#[test]
fn references_are_found_in_the_requirements_and_the_specs() {
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

    assert_eq!(t["order/Paid"].relevant, [TargetView::Requirements]);
    assert_eq!(
        t["order/Paid"].requirements,
        [SectionAnchor {
            area: "ui".into(),
            section: Some("order/Paid".into())
        }]
    );
    // A reference no section is titled with is relevant to the Requirements and found nowhere.
    assert!(
        t["nothing"].requirements.is_empty() && t["nothing"].relevant == [TargetView::Requirements]
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
        t["requirements/ui#order/Paid"].requirements,
        [SectionAnchor {
            area: "ui".into(),
            section: Some("order/Paid".into())
        }]
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
    assert_eq!(
        paid.file.as_deref(),
        Some(".weaver/requirements/ui/main.md")
    );
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
            .any(|e| e.file == ".weaver/model/shop/broken.md"),
        "{:?}",
        data.errors
    );
    // A broken Task file is not an error of the Model.
    assert!(data.errors.iter().all(|e| !e.file.contains("tasks/")));
    let order = data
        .errors
        .iter()
        .find(|e| e.file == ".weaver/model/shop/order.md")
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
    std::fs::create_dir_all(dir.path().join(".weaver")).unwrap();
    let data = build_for(dir.path(), None, &[]).data;
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
