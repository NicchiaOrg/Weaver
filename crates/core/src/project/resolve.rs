//! Naming a card on the command line: by identity, by a unique prefix of its identity, or by
//! short number (`P3`, `t3`).
//!
//! A letter followed by digits is always a short number: its letter must match the kind of
//! card the command expects, and a number no live card holds is refused. Resolution reads the
//! short numbers without writing the cache back, so a refusal writes nothing.

use super::cards::CardKind;
use super::ids::{looks_like_short_number, ID_LEN};
use super::image::ProjectImage;
use super::shortnum::{peek, ShortError};

/// Why a name is not a card.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResolveError {
    #[error(transparent)]
    Short(#[from] ShortError),
    #[error("`{given}` is not a card name: use an identity, a unique start of one, or a short number such as `{example}`")]
    NotAName { given: String, example: String },
    #[error("no {kind} has an identity starting with `{given}`")]
    NotFound { given: String, kind: &'static str },
    #[error("`{given}` starts the identities of several {kind}s ({found}): write more characters")]
    Ambiguous {
        given: String,
        kind: &'static str,
        found: String,
    },
}

fn kind_name(kind: CardKind) -> &'static str {
    match kind {
        CardKind::Proposal => "Proposal",
        CardKind::ProposalTask | CardKind::Task => "Task",
        CardKind::Drift => "Drift",
        CardKind::History => "History entry",
    }
}

/// The identity of the card of `kind` that `given` names.
pub fn resolve_card(
    image: &ProjectImage,
    kind: CardKind,
    given: &str,
) -> Result<String, ResolveError> {
    if kind.short_letter().is_some() && looks_like_short_number(given) {
        if let Some(id) = peek(image.root(), image.cards()).resolve(given, kind)? {
            return Ok(id);
        }
    }
    let wellformed = !given.is_empty()
        && given.len() <= ID_LEN
        && given
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    if !wellformed {
        let example = kind
            .short_letter()
            .map_or_else(|| "a1b2c3d4".to_owned(), |letter| format!("{letter}3"));
        return Err(ResolveError::NotAName {
            given: given.to_owned(),
            example,
        });
    }
    let found: Vec<&str> = image
        .cards_of(kind)
        .map(|card| card.id.as_str())
        .filter(|id| id.starts_with(given))
        .collect();
    match found.as_slice() {
        [] => Err(ResolveError::NotFound {
            given: given.to_owned(),
            kind: kind_name(kind),
        }),
        [only] => Ok((*only).to_owned()),
        several => Err(ResolveError::Ambiguous {
            given: given.to_owned(),
            kind: kind_name(kind),
            found: several.join(", "),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::project::changes::ChangeSet;

    fn image_with_tasks(ids: &[&str]) -> ProjectImage {
        let mut changes = ChangeSet::new();
        for (n, id) in ids.iter().enumerate() {
            changes.write(
                &format!("tasks/{id}-job.md"),
                format!("---\ncolumn: todo\ncreated: 2026-11-02T07:00:0{n}.000Z\nrefs: []\ntechnical: true\n---\n# Job {n}\n"),
            );
        }
        ProjectImage::empty(Path::new("/nonexistent-project")).with_changes(&changes)
    }

    #[test]
    fn an_identity_or_a_unique_prefix_names_a_card() {
        let image = image_with_tasks(&["abcd1234", "abef5678"]);
        assert_eq!(
            resolve_card(&image, CardKind::Task, "abcd1234").unwrap(),
            "abcd1234"
        );
        assert_eq!(
            resolve_card(&image, CardKind::Task, "abc").unwrap(),
            "abcd1234"
        );
        assert!(matches!(
            resolve_card(&image, CardKind::Task, "ab"),
            Err(ResolveError::Ambiguous { .. })
        ));
        assert!(matches!(
            resolve_card(&image, CardKind::Task, "zzzzzzzz"),
            Err(ResolveError::NotFound { .. })
        ));
        assert!(matches!(
            resolve_card(&image, CardKind::Task, "ABC"),
            Err(ResolveError::NotAName { .. })
        ));
        assert!(matches!(
            resolve_card(&image, CardKind::Task, ""),
            Err(ResolveError::NotAName { .. })
        ));
        assert!(matches!(
            resolve_card(&image, CardKind::Task, "abcd12345"),
            Err(ResolveError::NotAName { .. })
        ));
    }

    #[test]
    fn a_letter_with_digits_is_a_short_number_of_its_own_kind_only() {
        let image = image_with_tasks(&["abcd1234"]);
        assert_eq!(
            resolve_card(&image, CardKind::Task, "t1").unwrap(),
            "abcd1234"
        );
        assert_eq!(
            resolve_card(&image, CardKind::Task, "T1").unwrap(),
            "abcd1234"
        );
        assert!(matches!(
            resolve_card(&image, CardKind::Task, "t2"),
            Err(ResolveError::Short(ShortError::NotLive { .. }))
        ));
        assert!(matches!(
            resolve_card(&image, CardKind::Proposal, "t1"),
            Err(ResolveError::Short(ShortError::WrongLetter { .. }))
        ));
        assert!(matches!(
            resolve_card(&image, CardKind::Drift, "t1"),
            Err(ResolveError::Short(ShortError::WrongLetter { .. }))
        ));
    }

    #[test]
    fn a_history_entry_has_no_short_number() {
        let image = image_with_tasks(&[]);
        assert!(matches!(
            resolve_card(&image, CardKind::History, "h1"),
            Err(ResolveError::NotFound { .. })
        ));
    }
}
