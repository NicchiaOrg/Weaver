//! Fixtures of the tests of the Proposal modules.

use std::path::{Path, PathBuf};

use crate::project::cards::{parse_card, Card, CardKind, CardName};
use crate::project::text;

pub(crate) use crate::project::testing::put;

pub(crate) const SYSTEM: &str = "# Shop\n\nA shop.\n";
pub(crate) const FEATURE: &str = "# Shopping\n\nBuying things.\n";
pub(crate) const ORDER: &str = "# Order

An order placed by a customer.

## States

### Draft (initial)
The customer is choosing.
- `CONFIRM` → Paid: only if the cart is not empty.

### Paid
- `user.CANCEL` → Draft: refunds the customer.

### Shipped (final)

## Events

### CONFIRM
The customer confirms the order.
";
pub(crate) const USER: &str = "# User\n\nA person buying from the shop.\n\n## Events\n\n### CANCEL\nThe user asks to cancel an order.\n";
pub(crate) const AREA: &str = "---\ndescription: d\n---\n# UI\n";
pub(crate) const MAIN: &str =
    "---\ndescription: d\n---\n# Main\n\n## General\n- x\n\n## order/Paid\n- y\n";
pub(crate) const SPECS: &str = "languages:\n  - name: Rust\n";

/// The created instant of the Proposal the tests build.
pub(crate) const CREATED: &str = "2026-11-02T07:00:00.000Z";
pub(crate) const NOW: &str = "2026-11-02T09:00:00.000Z";

/// A valid project: a Feature with a Machine, an Actor, an Area and Specs.
pub(crate) fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    put(root, "model/system.md", SYSTEM);
    put(root, "model/shop/feature.md", FEATURE);
    put(root, "model/shop/order.md", ORDER);
    put(root, "model/actors/user.md", USER);
    put(root, "requirements/ui/area.md", AREA);
    put(root, "requirements/ui/main.md", MAIN);
    put(root, "specs.yaml", SPECS);
    dir
}

/// An empty project: only the System.
pub(crate) fn empty_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    put(dir.path(), "model/system.md", SYSTEM);
    dir
}

/// The hash `base` records for `content`.
pub(crate) fn hash(content: &str) -> String {
    text::sha256_hex(&text::file_content(content))
}

pub(crate) fn proposal_card(text: &str) -> Card {
    let name = CardName {
        kind: CardKind::Proposal,
        id: "pppppppp",
        slug: "t",
        parent: None,
        path: "proposals/pppppppp-t/proposal.md",
    };
    parse_card(name, &text::normalize(text)).unwrap()
}

/// Writes a Proposal folder: `proposal.md` (front matter lines, then the body), its held files
/// and its Task files `(id, slug, text)`.
pub(crate) fn proposal(
    root: &Path,
    id: &str,
    front: &str,
    body: &str,
    files: &[(&str, &str)],
    tasks: &[(&str, &str, &str)],
) -> PathBuf {
    let folder = format!("proposals/{id}-t");
    let created = if front.contains("created:") {
        String::new()
    } else {
        format!("created: {CREATED}\n")
    };
    put(
        root,
        &format!("{folder}/proposal.md"),
        &format!("---\n{front}{created}---\n{body}"),
    );
    for (path, content) in files {
        put(root, &format!("{folder}/{path}"), content);
    }
    for (task_id, slug, content) in tasks {
        put(
            root,
            &format!("{folder}/tasks/{task_id}-{slug}.md"),
            content,
        );
    }
    root.join(".weaver").join(folder)
}

/// A Task file of a Proposal.
pub(crate) fn task_file(title: &str, technical: bool, refs: &[&str]) -> String {
    let refs = if refs.is_empty() {
        "refs: []\n".to_owned()
    } else {
        format!(
            "refs:\n{}",
            refs.iter()
                .map(|r| format!("  - {r}\n"))
                .collect::<String>()
        )
    };
    format!("---\ntechnical: {technical}\n{refs}---\n# {title}\n\nDo it.\n")
}
