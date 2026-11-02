//! Short numbers: `P`, `T` or `D` plus the lowest free positive integer of that type.
//!
//! Numbers go to live cards in creation order and are freed when the card closes. They are kept
//! only in the local cache `short-numbers/assigned` (`<P|T|D> <identity> <number>`, one line
//! per live card), written atomically. The cache is never trusted: a missing, unreadable or
//! conflicting one is rebuilt silently (valid lines kept, missing numbers assigned) and no
//! command fails because of it. Short numbers never appear in a versioned file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::cards::{Card, CardKind};
use super::ids::looks_like_short_number;
use super::layout::{ensure_local_folder, trama_dir, SHORT_NUMBERS_DIR};
use super::order::sort_by_creation;
use super::text;

/// Name of the cache file in `short-numbers/`.
pub const CACHE_FILE: &str = "assigned";

/// A card as the numbering sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortCard {
    /// `Proposal`, `Task` or `Drift`.
    pub kind: CardKind,
    pub id: String,
    pub created: String,
    /// Whether the card holds a number: it is live, or a Done Task of a design that still has
    /// Tasks left.
    pub live: bool,
}

/// Why an input was refused as a short number.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ShortError {
    #[error("`{given}` is a {given_kind} number, but a {expected_kind} is expected")]
    WrongLetter {
        given: String,
        given_kind: String,
        expected_kind: String,
    },
    #[error("`{given}` is not the number of any live {expected_kind}")]
    NotLive {
        given: String,
        expected_kind: String,
    },
}

/// The numbers of the live cards.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShortNumbers {
    /// `(letter, identity, number)`, in `P`, `T`, `D` order and by number.
    entries: Vec<(char, String, u32)>,
}

const LETTERS: [char; 3] = ['P', 'T', 'D'];

fn kind_name(letter: char) -> &'static str {
    match letter {
        'P' => "Proposal",
        'T' => "Task",
        'D' => "Drift",
        _ => "card",
    }
}

impl ShortNumbers {
    /// Assigns numbers to the live cards, keeping the valid lines of `cache`.
    ///
    /// A cache line is valid when its letter matches the kind of the card it names, that card
    /// is live, its number is at least 1 and neither the number nor the card appears on an
    /// earlier valid line. Live cards left without a number then take the lowest free integer
    /// of their kind in creation order.
    pub fn assign(cards: &[ShortCard], cache: &str) -> ShortNumbers {
        let live: BTreeSet<(char, &str)> = cards
            .iter()
            .filter(|c| c.live)
            .filter_map(|c| c.kind.short_letter().map(|l| (l, c.id.as_str())))
            .collect();
        let mut kept: BTreeMap<(char, String), u32> = BTreeMap::new();
        let mut used: BTreeMap<char, BTreeSet<u32>> = BTreeMap::new();
        for line in cache.lines() {
            let Some((letter, id, number)) = parse_line(line) else {
                continue;
            };
            let numbers = used.entry(letter).or_default();
            if live.contains(&(letter, id.as_str()))
                && number >= 1
                && !numbers.contains(&number)
                && !kept.contains_key(&(letter, id.clone()))
            {
                numbers.insert(number);
                kept.insert((letter, id), number);
            }
        }
        let mut waiting: Vec<&ShortCard> = cards
            .iter()
            .filter(|c| c.live)
            .filter(|c| {
                c.kind
                    .short_letter()
                    .is_some_and(|l| !kept.contains_key(&(l, c.id.clone())))
            })
            .collect();
        sort_by_creation(&mut waiting, |c| (c.created.as_str(), c.id.as_str()));
        for card in waiting {
            let letter = card.kind.short_letter().expect("filtered");
            let numbers = used.entry(letter).or_default();
            let number = (1u32..)
                .find(|n| !numbers.contains(n))
                .expect("an integer is free");
            numbers.insert(number);
            kept.insert((letter, card.id.clone()), number);
        }
        let mut entries: Vec<_> = kept
            .into_iter()
            .map(|((letter, id), number)| (letter, id, number))
            .collect();
        entries
            .sort_by_key(|(letter, _, number)| (LETTERS.iter().position(|l| l == letter), *number));
        ShortNumbers { entries }
    }

    /// The text of the cache file: one `<letter> <identity> <number>` line per live card.
    pub fn render(&self) -> String {
        self.entries
            .iter()
            .map(|(letter, id, number)| format!("{letter} {id} {number}\n"))
            .collect()
    }

    /// The number of the card `id` of `kind`.
    pub fn number_of(&self, kind: CardKind, id: &str) -> Option<u32> {
        let letter = kind.short_letter()?;
        self.entries
            .iter()
            .find(|(l, i, _)| *l == letter && i == id)
            .map(|(_, _, n)| *n)
    }

    /// The identity of the live card `letter` + `number`.
    pub fn id_of(&self, kind: CardKind, number: u32) -> Option<&str> {
        let letter = kind.short_letter()?;
        self.entries
            .iter()
            .find(|(l, _, n)| *l == letter && *n == number)
            .map(|(_, i, _)| i.as_str())
    }

    /// `P3`, `T12`, `D1`: the short number text of a card.
    pub fn label_of(&self, kind: CardKind, id: &str) -> Option<String> {
        Some(format!(
            "{}{}",
            kind.short_letter()?,
            self.number_of(kind, id)?
        ))
    }

    /// Reads a command argument expecting a card of `kind`.
    ///
    /// `Ok(None)`: the input is not a short number (it is not a letter followed by digits), so
    /// it is an identity. `Ok(Some(id))`: the identity of the live card. A letter followed by
    /// digits is always a short number: with another letter than `kind`'s, or a number no live
    /// card holds, it is refused. Letters are accepted in either case.
    pub fn resolve(&self, input: &str, kind: CardKind) -> Result<Option<String>, ShortError> {
        if !looks_like_short_number(input) {
            return Ok(None);
        }
        let expected = kind.short_letter().expect("a kind with short numbers");
        let letter = input
            .chars()
            .next()
            .expect("non-empty")
            .to_ascii_uppercase();
        if letter != expected {
            return Err(ShortError::WrongLetter {
                given: input.to_owned(),
                given_kind: kind_name(letter).to_owned(),
                expected_kind: kind_name(expected).to_owned(),
            });
        }
        let not_live = || ShortError::NotLive {
            given: input.to_owned(),
            expected_kind: kind_name(expected).to_owned(),
        };
        let number: u32 = input[1..].parse().map_err(|_| not_live())?;
        self.id_of(kind, number)
            .map(|id| Some(id.to_owned()))
            .ok_or_else(not_live)
    }
}

fn parse_line(line: &str) -> Option<(char, String, u32)> {
    let mut parts = line.split(' ');
    let (letter, id, number) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let letter = match letter {
        "P" => 'P',
        "T" => 'T',
        "D" => 'D',
        _ => return None,
    };
    Some((letter, id.to_owned(), number.parse().ok()?))
}

/// The cards that hold a number, from the cards of the project.
///
/// Live: a Proposal in `draft`, `open` or `refining`; a Task in `todo` or `doing`; a Drift in
/// `open` or `code-to-be-fixed`. A Done Task of a design (it has a `proposal`) keeps its number
/// until every Task of that design is Done.
pub fn short_cards(cards: &[Card]) -> Vec<ShortCard> {
    let unfinished_designs: BTreeSet<&str> = cards
        .iter()
        .filter(|c| c.kind == CardKind::Task && c.str_key("column") != Some("done"))
        .filter_map(|c| c.str_key("proposal"))
        .collect();
    cards
        .iter()
        .filter_map(|card| {
            let live = match card.kind {
                CardKind::Proposal => {
                    matches!(card.str_key("status"), Some("draft" | "open" | "refining"))
                }
                CardKind::Drift => {
                    matches!(card.str_key("status"), Some("open" | "code-to-be-fixed"))
                }
                CardKind::Task => match card.str_key("column") {
                    Some("todo" | "doing") => true,
                    Some("done") => card
                        .str_key("proposal")
                        .is_some_and(|p| unfinished_designs.contains(p)),
                    _ => false,
                },
                CardKind::ProposalTask | CardKind::History => return None,
            };
            Some(ShortCard {
                kind: card.kind,
                id: card.id.clone(),
                created: card.created.clone().unwrap_or_default(),
                live,
            })
        })
        .collect()
}

/// Reads the cache of the project, rebuilds what is wrong with it and writes it back when it
/// changed. Nothing here fails: a cache that cannot be read counts as empty, and one that
/// cannot be written is simply not updated.
pub fn load(project_root: &Path, cards: &[Card]) -> ShortNumbers {
    let path = trama_dir(project_root)
        .join(SHORT_NUMBERS_DIR)
        .join(CACHE_FILE);
    let current = text::read_text(&path).unwrap_or_default();
    let numbers = ShortNumbers::assign(&short_cards(cards), &current);
    let rendered = numbers.render();
    if current.trim_end() != rendered.trim_end() {
        let _ = ensure_local_folder(project_root, SHORT_NUMBERS_DIR)
            .and_then(|_| text::write_text(&path, &rendered));
    }
    numbers
}

/// The numbers [`load`] would give, without writing the cache back: for what reads only (the
/// Board's picture) and for a decision that may still be refused, which must write nothing.
pub fn peek(project_root: &Path, cards: &[Card]) -> ShortNumbers {
    let path = trama_dir(project_root)
        .join(SHORT_NUMBERS_DIR)
        .join(CACHE_FILE);
    ShortNumbers::assign(
        &short_cards(cards),
        &text::read_text(&path).unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(kind: CardKind, id: &str, created: &str, live: bool) -> ShortCard {
        ShortCard {
            kind,
            id: id.to_owned(),
            created: format!("2026-11-02T07:00:{created}.000Z"),
            live,
        }
    }

    fn t(id: &str, created: &str) -> ShortCard {
        card(CardKind::Task, id, created, true)
    }

    #[test]
    fn numbers_follow_creation_order_per_type() {
        let cards = [
            t("cccccccc", "03"),
            t("aaaaaaaa", "01"),
            card(CardKind::Proposal, "pppppppp", "02", true),
            card(CardKind::Drift, "dddddddd", "04", true),
            t("bbbbbbbb", "02"),
        ];
        let n = ShortNumbers::assign(&cards, "");
        assert_eq!(
            n.render(),
            "P pppppppp 1\nT aaaaaaaa 1\nT bbbbbbbb 2\nT cccccccc 3\nD dddddddd 1\n"
        );
        assert_eq!(
            n.label_of(CardKind::Task, "bbbbbbbb").as_deref(),
            Some("T2")
        );
        assert_eq!(n.number_of(CardKind::Drift, "aaaaaaaa"), None);
    }

    #[test]
    fn ties_in_creation_go_by_identity() {
        let n = ShortNumbers::assign(&[t("bbbbbbbb", "01"), t("aaaaaaaa", "01")], "");
        assert_eq!(n.number_of(CardKind::Task, "aaaaaaaa"), Some(1));
        assert_eq!(n.number_of(CardKind::Task, "bbbbbbbb"), Some(2));
    }

    #[test]
    fn a_closed_card_frees_its_number_for_the_next_card_of_its_type() {
        let first = [
            t("aaaaaaaa", "01"),
            t("bbbbbbbb", "02"),
            t("cccccccc", "03"),
        ];
        let cache = ShortNumbers::assign(&first, "").render();
        // `bbbbbbbb` closes; a new Task is created later.
        let second = [
            t("aaaaaaaa", "01"),
            card(CardKind::Task, "bbbbbbbb", "02", false),
            t("cccccccc", "03"),
            t("dddddddd", "04"),
        ];
        let n = ShortNumbers::assign(&second, &cache);
        assert_eq!(n.render(), "T aaaaaaaa 1\nT dddddddd 2\nT cccccccc 3\n");
    }

    #[test]
    fn numbers_already_held_are_never_reshuffled() {
        let cards = [t("aaaaaaaa", "01"), t("bbbbbbbb", "02")];
        let n = ShortNumbers::assign(&cards, "T bbbbbbbb 1\nT aaaaaaaa 2\n");
        assert_eq!(n.number_of(CardKind::Task, "bbbbbbbb"), Some(1));
        assert_eq!(n.number_of(CardKind::Task, "aaaaaaaa"), Some(2));
    }

    #[test]
    fn a_bad_cache_is_rebuilt_keeping_what_is_valid() {
        let cards = [
            t("aaaaaaaa", "01"),
            t("bbbbbbbb", "02"),
            t("cccccccc", "03"),
            card(CardKind::Drift, "dddddddd", "04", true),
        ];
        let cache = "\
T aaaaaaaa 5
T bbbbbbbb 5
T cccccccc 0
T gone0000 2
D aaaaaaaa 1
T aaaaaaaa 6
garbage line
T dddddddd 3
D dddddddd 2
X cccccccc 9
T cccccccc
";
        let n = ShortNumbers::assign(&cards, cache);
        // aaaaaaaa keeps 5 (first valid line), bbbbbbbb conflicts on 5 and is renumbered, the
        // zero, unknown card, wrong-letter and unreadable lines are dropped.
        assert_eq!(n.number_of(CardKind::Task, "aaaaaaaa"), Some(5));
        assert_eq!(n.number_of(CardKind::Task, "bbbbbbbb"), Some(1));
        assert_eq!(n.number_of(CardKind::Task, "cccccccc"), Some(2));
        assert_eq!(n.number_of(CardKind::Drift, "dddddddd"), Some(2));
        assert_eq!(
            n.render(),
            "T bbbbbbbb 1\nT cccccccc 2\nT aaaaaaaa 5\nD dddddddd 2\n"
        );
    }

    #[test]
    fn input_is_read_in_either_case() {
        let n = ShortNumbers::assign(&[t("aaaaaaaa", "01"), t("bbbbbbbb", "02")], "");
        assert_eq!(n.resolve("t2", CardKind::Task), Ok(Some("bbbbbbbb".into())));
        assert_eq!(n.resolve("T1", CardKind::Task), Ok(Some("aaaaaaaa".into())));
        assert_eq!(
            n.resolve("T01", CardKind::Task),
            Ok(Some("aaaaaaaa".into()))
        );
    }

    #[test]
    fn a_letter_and_digits_is_always_a_short_number() {
        let n = ShortNumbers::assign(&[t("aaaaaaaa", "01")], "");
        assert!(matches!(
            n.resolve("d1", CardKind::Task),
            Err(ShortError::WrongLetter { .. })
        ));
        assert!(matches!(
            n.resolve("x1", CardKind::Task),
            Err(ShortError::WrongLetter { .. })
        ));
        assert!(matches!(
            n.resolve("T9", CardKind::Task),
            Err(ShortError::NotLive { .. })
        ));
        assert!(matches!(
            n.resolve("T0", CardKind::Task),
            Err(ShortError::NotLive { .. })
        ));
        assert!(matches!(
            n.resolve("T99999999999999999999", CardKind::Task),
            Err(ShortError::NotLive { .. })
        ));
        // Even something the size of an identity.
        assert!(matches!(
            n.resolve("t1234567", CardKind::Task),
            Err(ShortError::NotLive { .. })
        ));
        // Anything else is an identity, not a short number.
        assert_eq!(n.resolve("aaaaaaaa", CardKind::Task), Ok(None));
        assert_eq!(n.resolve("t", CardKind::Task), Ok(None));
        assert_eq!(n.resolve("12345678", CardKind::Task), Ok(None));
    }

    #[test]
    fn a_refusal_names_what_was_wrong() {
        let n = ShortNumbers::assign(&[], "");
        let e = n.resolve("d3", CardKind::Task).unwrap_err().to_string();
        assert!(e.contains("Drift") && e.contains("Task"), "{e}");
        let e = n.resolve("t3", CardKind::Task).unwrap_err().to_string();
        assert!(e.contains("not the number of any live Task"), "{e}");
    }

    // --- from card files ---

    fn task(id: &str, column: &str, proposal: Option<&str>, created: &str) -> Card {
        let mut front = serde_yaml::Mapping::new();
        front.insert("column".into(), column.into());
        if let Some(p) = proposal {
            front.insert("proposal".into(), p.into());
        }
        Card {
            kind: CardKind::Task,
            id: id.to_owned(),
            slug: "x".into(),
            parent: None,
            path: format!("tasks/{id}-x.md"),
            title: "T".into(),
            body: String::new(),
            created: Some(format!("2026-11-02T07:00:{created}.000Z")),
            front,
        }
    }

    #[test]
    fn a_done_task_of_a_design_keeps_its_number_until_the_whole_design_is_done() {
        let cards = [
            task("aaaaaaaa", "done", Some("pppppppp"), "01"),
            task("bbbbbbbb", "doing", Some("pppppppp"), "02"),
            task("cccccccc", "done", None, "03"),
            task("dddddddd", "todo", None, "04"),
        ];
        let live: Vec<_> = short_cards(&cards)
            .into_iter()
            .filter(|c| c.live)
            .map(|c| c.id)
            .collect();
        assert_eq!(live, ["aaaaaaaa", "bbbbbbbb", "dddddddd"]);

        let all_done = [
            task("aaaaaaaa", "done", Some("pppppppp"), "01"),
            task("bbbbbbbb", "done", Some("pppppppp"), "02"),
            task("dddddddd", "todo", None, "04"),
        ];
        let live: Vec<_> = short_cards(&all_done)
            .into_iter()
            .filter(|c| c.live)
            .map(|c| c.id)
            .collect();
        assert_eq!(live, ["dddddddd"]);
    }

    #[test]
    fn live_statuses_per_type() {
        fn with(kind: CardKind, key: &str, value: &str) -> Card {
            let mut c = task("aaaaaaaa", "todo", None, "01");
            c.kind = kind;
            c.front.remove("column");
            c.front.insert(key.into(), value.into());
            c
        }
        for (kind, key, live, dead) in [
            (
                CardKind::Proposal,
                "status",
                ["draft", "open", "refining"].as_slice(),
                ["approved", "rejected", "discarded"].as_slice(),
            ),
            (
                CardKind::Drift,
                "status",
                ["open", "code-to-be-fixed"].as_slice(),
                ["fixed", "proposal-accepted", "dismissed"].as_slice(),
            ),
            (
                CardKind::Task,
                "column",
                ["todo", "doing"].as_slice(),
                ["done", "bogus"].as_slice(),
            ),
        ] {
            for value in live {
                assert!(
                    short_cards(&[with(kind, key, value)])[0].live,
                    "{kind:?} {value}"
                );
            }
            for value in dead {
                assert!(
                    !short_cards(&[with(kind, key, value)])[0].live,
                    "{kind:?} {value}"
                );
            }
        }
    }

    #[test]
    fn the_cache_file_is_written_rebuilt_and_never_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let cards = [
            task("aaaaaaaa", "todo", None, "01"),
            task("bbbbbbbb", "todo", None, "02"),
        ];
        let cache_path = trama_dir(dir.path()).join("short-numbers/assigned");

        // Missing: built and written, with the folder's `.gitignore`.
        let n = load(dir.path(), &cards);
        assert_eq!(n.number_of(CardKind::Task, "bbbbbbbb"), Some(2));
        assert_eq!(
            std::fs::read_to_string(&cache_path).unwrap(),
            "T aaaaaaaa 1\nT bbbbbbbb 2\n"
        );
        assert_eq!(
            std::fs::read(cache_path.parent().unwrap().join(".gitignore")).unwrap(),
            b"*\n"
        );

        // Conflicting and with an unknown card: rebuilt.
        std::fs::write(&cache_path, "T aaaaaaaa 1\nT bbbbbbbb 1\nT ghost000 2\n").unwrap();
        let n = load(dir.path(), &cards);
        assert_eq!(n.number_of(CardKind::Task, "bbbbbbbb"), Some(2));
        assert_eq!(
            std::fs::read_to_string(&cache_path).unwrap(),
            "T aaaaaaaa 1\nT bbbbbbbb 2\n"
        );

        // Unreadable (not UTF-8): rebuilt from scratch, no failure.
        std::fs::write(&cache_path, [0xFF, 0xFE, 0x00]).unwrap();
        let n = load(dir.path(), &cards);
        assert_eq!(n.number_of(CardKind::Task, "aaaaaaaa"), Some(1));
        assert_eq!(
            std::fs::read_to_string(&cache_path).unwrap(),
            "T aaaaaaaa 1\nT bbbbbbbb 2\n"
        );

        // CRLF cache with a BOM reads the same.
        std::fs::write(&cache_path, "\u{feff}T bbbbbbbb 1\r\nT aaaaaaaa 2\r\n").unwrap();
        let n = load(dir.path(), &cards);
        assert_eq!(n.number_of(CardKind::Task, "bbbbbbbb"), Some(1));
    }

    #[test]
    fn a_cache_that_cannot_be_written_does_not_fail_the_load() {
        let dir = tempfile::tempdir().unwrap();
        // `.trama/short-numbers` is a file: the folder cannot be created.
        std::fs::create_dir_all(trama_dir(dir.path())).unwrap();
        std::fs::write(trama_dir(dir.path()).join("short-numbers"), b"x").unwrap();
        let n = load(dir.path(), &[task("aaaaaaaa", "todo", None, "01")]);
        assert_eq!(n.number_of(CardKind::Task, "aaaaaaaa"), Some(1));
    }
}
