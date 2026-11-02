//! Card files in general: Proposals, Tasks, Drifts and History entries.
//!
//! This module holds what every card shares: the identity and slug read from the file or
//! folder name, the front matter as it was written (unknown keys included), the H1 title, the
//! body and the creation instant. The rules of each kind of card (columns, statuses, checks
//! of its keys) belong to the code that handles that card, on top of [`Card`].

use serde::Serialize;
use serde_yaml::Mapping;

use super::clock::is_instant;
use super::frontmatter;
use super::md;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CardKind {
    /// `proposals/<id>-<slug>/proposal.md`.
    Proposal,
    /// `proposals/<id>-<slug>/tasks/<id>-<slug>.md`: a Task waiting for its Proposal's approval.
    ProposalTask,
    /// `tasks/<id>-<slug>.md`.
    Task,
    /// `drifts/<id>-<slug>.md`.
    Drift,
    /// `history/<id>-<slug>.md`.
    History,
}

impl CardKind {
    /// The letter of the short number of this kind (`P`, `T`, `D`), when it has one.
    pub fn short_letter(self) -> Option<char> {
        match self {
            CardKind::Proposal => Some('P'),
            CardKind::Task => Some('T'),
            CardKind::Drift => Some('D'),
            CardKind::ProposalTask | CardKind::History => None,
        }
    }

    /// Whether the front matter of this kind carries `created`.
    pub fn has_created(self) -> bool {
        self != CardKind::ProposalTask
    }
}

/// A card file as read: only what every kind shares.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Card {
    pub kind: CardKind,
    /// Read from the file or folder name.
    pub id: String,
    /// Read from the file or folder name; fixed when the card is created.
    pub slug: String,
    /// The Proposal identity, for a `ProposalTask`.
    pub parent: Option<String>,
    /// Path relative to `.trama/`: the card file (`proposal.md` for a Proposal).
    pub path: String,
    /// The first H1 of the body.
    pub title: String,
    /// Everything after the front matter.
    pub body: String,
    /// The `created` instant, for the kinds that have one.
    pub created: Option<String>,
    #[serde(skip)]
    pub front: Mapping,
}

impl Card {
    pub fn str_key(&self, key: &str) -> Option<&str> {
        frontmatter::get_str(&self.front, key)
    }

    pub fn bool_key(&self, key: &str) -> Option<bool> {
        frontmatter::get_bool(&self.front, key)
    }

    pub fn list_key(&self, key: &str) -> Option<Vec<String>> {
        frontmatter::get_str_list(&self.front, key)
    }

    /// The Task identities of the `cleared` map of a History entry.
    pub fn cleared_task_ids(&self) -> Vec<String> {
        match self.front.get("cleared") {
            Some(serde_yaml::Value::Mapping(map)) => map
                .keys()
                .filter_map(|k| k.as_str().map(str::to_owned))
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// Where a card file was found; the identity and slug come from here.
#[derive(Debug, Clone, Copy)]
pub struct CardName<'a> {
    pub kind: CardKind,
    pub id: &'a str,
    pub slug: &'a str,
    pub parent: Option<&'a str>,
    pub path: &'a str,
}

/// Reads a card from normalized text: front matter, H1 title and, for the kinds that have
/// it, a valid `created` instant.
pub fn parse_card(name: CardName, text: &str) -> Result<Card, Vec<String>> {
    let doc = frontmatter::parse(text).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let outline = md::outline(&doc.body);
    let title = match outline.headings.iter().find(|h| h.level == 1) {
        Some(h) if !h.text.is_empty() => h.text.clone(),
        _ => {
            errors.push("the card needs an H1 with its title".to_owned());
            String::new()
        }
    };
    let created = if name.kind.has_created() {
        match frontmatter::get_str(&doc.front, "created") {
            Some(c) if is_instant(c) => Some(c.to_owned()),
            Some(c) => {
                errors.push(format!("`created` is `{c}`, not a UTC ISO 8601 instant with milliseconds (2026-11-02T07:07:19.522Z)"));
                None
            }
            None => {
                errors.push("front matter needs `created`".to_owned());
                None
            }
        }
    } else {
        None
    };
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Card {
        kind: name.kind,
        id: name.id.to_owned(),
        slug: name.slug.to_owned(),
        parent: name.parent.map(str::to_owned),
        path: name.path.to_owned(),
        title,
        body: doc.body,
        created,
        front: doc.front,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(kind: CardKind) -> CardName<'static> {
        CardName {
            kind,
            id: "vxdhoylk",
            slug: "core",
            parent: None,
            path: "tasks/vxdhoylk-core.md",
        }
    }

    #[test]
    fn identity_and_slug_come_from_the_name_not_the_content() {
        let text = "---\nid: zzzzzzzz\ncolumn: todo\ncreated: 2026-11-02T07:07:19.522Z\n---\n# Core\n\nBody\n";
        let card = parse_card(name(CardKind::Task), text).unwrap();
        assert_eq!(card.id, "vxdhoylk");
        assert_eq!(card.slug, "core");
        assert_eq!(card.title, "Core");
        assert_eq!(card.created.as_deref(), Some("2026-11-02T07:07:19.522Z"));
        assert_eq!(card.str_key("column"), Some("todo"));
        assert_eq!(card.str_key("id"), Some("zzzzzzzz"));
        assert_eq!(card.body, "# Core\n\nBody\n");
    }

    #[test]
    fn created_is_required_and_exact_except_for_tasks_of_a_proposal() {
        let missing =
            parse_card(name(CardKind::Drift), "---\nstatus: open\n---\n# T\n").unwrap_err();
        assert!(missing[0].contains("needs `created`"));
        let bad = parse_card(
            name(CardKind::Drift),
            "---\ncreated: 1970-01-01\n---\n# T\n",
        )
        .unwrap_err();
        assert!(bad[0].contains("not a UTC ISO 8601"));
        let ok = parse_card(
            name(CardKind::ProposalTask),
            "---\ntechnical: true\n---\n# T\n",
        )
        .unwrap();
        assert_eq!(ok.created, None);
    }

    #[test]
    fn a_card_needs_a_title_and_valid_front_matter() {
        let e = parse_card(
            name(CardKind::ProposalTask),
            "---\ntechnical: true\n---\nno title\n",
        )
        .unwrap_err();
        assert!(e[0].contains("H1"));
        let e = parse_card(name(CardKind::Task), "---\ncolumn: [\n---\n# T\n").unwrap_err();
        assert!(e[0].contains("invalid front matter"));
    }

    #[test]
    fn cleared_copies_list_their_task_identities() {
        let text = "---\ncreated: 2026-11-02T07:07:19.522Z\nupdated: 2026-11-02T07:07:19.522Z\ncleared:\n  abcd1234:\n    title: T\n  wxyz9876:\n    title: U\n---\n# H\n";
        let card = parse_card(name(CardKind::History), text).unwrap();
        assert_eq!(card.cleared_task_ids(), ["abcd1234", "wxyz9876"]);
    }

    #[test]
    fn short_letters() {
        assert_eq!(CardKind::Proposal.short_letter(), Some('P'));
        assert_eq!(CardKind::Task.short_letter(), Some('T'));
        assert_eq!(CardKind::Drift.short_letter(), Some('D'));
        assert_eq!(CardKind::History.short_letter(), None);
        assert_eq!(CardKind::ProposalTask.short_letter(), None);
    }
}
