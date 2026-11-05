//! The notes a job declares with `weaver job edit` (`requirements/cli#job`).
//!
//! Line breaks become spaces and the ends are trimmed; an empty note is invalid input; a note over
//! 120 characters is cut to 119 plus `…`. The last 6 notes are kept with their instants, and a
//! repeat of the last one only renews its instant.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The longest note, in characters.
pub const MAX_NOTE_CHARS: usize = 120;

/// How many notes a job keeps.
pub const KEPT_NOTES: usize = 6;

/// A note with the instant it was declared (UTC ISO 8601 with milliseconds).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub text: String,
    pub at: String,
}

/// A note as it is kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Normalised {
    pub text: String,
    /// Whether it was over [`MAX_NOTE_CHARS`] and cut.
    pub cut: bool,
}

/// The note as kept, or `None` when nothing is left of `text`.
pub fn normalise(text: &str) -> Option<Normalised> {
    let one_line = text.replace("\r\n", " ").replace(['\r', '\n'], " ");
    let trimmed = one_line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().count() <= MAX_NOTE_CHARS {
        return Some(Normalised {
            text: trimmed.to_owned(),
            cut: false,
        });
    }
    let mut cut: String = trimmed.chars().take(MAX_NOTE_CHARS - 1).collect();
    cut.push('…');
    Some(Normalised {
        text: cut,
        cut: true,
    })
}

/// Records `text` (already normalised) at `at`: the same text as the last note only renews its
/// instant; otherwise it is added and only the last [`KEPT_NOTES`] stay.
pub fn record(notes: &mut Vec<Note>, text: &str, at: &str) {
    match notes.last_mut() {
        Some(last) if last.text == text => last.at = at.to_owned(),
        _ => {
            notes.push(Note {
                text: text.to_owned(),
                at: at.to_owned(),
            });
            if notes.len() > KEPT_NOTES {
                let excess = notes.len() - KEPT_NOTES;
                notes.drain(..excess);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_breaks_become_spaces_and_the_ends_are_trimmed() {
        let note = normalise("  Reading\nthe order\r\nMachine \n").unwrap();
        assert_eq!(
            note,
            Normalised {
                text: "Reading the order Machine".to_owned(),
                cut: false
            }
        );
        assert_eq!(normalise(" \n\r ").map(|n| n.text), None);
        assert_eq!(normalise(""), None);
    }

    #[test]
    fn a_long_note_is_cut_to_119_characters_and_an_ellipsis() {
        let exact = "é".repeat(MAX_NOTE_CHARS);
        assert_eq!(
            normalise(&exact),
            Some(Normalised {
                text: exact.clone(),
                cut: false
            })
        );
        let long = format!("{exact}x");
        let cut = normalise(&long).unwrap();
        assert!(cut.cut);
        assert_eq!(cut.text.chars().count(), MAX_NOTE_CHARS);
        assert_eq!(cut.text, format!("{}…", "é".repeat(MAX_NOTE_CHARS - 1)));
    }

    #[test]
    fn the_last_six_are_kept_and_a_repeat_renews_the_instant() {
        let mut notes = Vec::new();
        for n in 0..8 {
            record(&mut notes, &format!("note {n}"), &format!("t{n}"));
        }
        assert_eq!(notes.len(), KEPT_NOTES);
        assert_eq!(notes[0].text, "note 2");
        record(&mut notes, "note 7", "later");
        assert_eq!(notes.len(), KEPT_NOTES);
        assert_eq!(
            notes.last().unwrap(),
            &Note {
                text: "note 7".into(),
                at: "later".into()
            }
        );
        // Only the last one counts as a repeat.
        record(&mut notes, "note 6", "again");
        assert_eq!(notes.len(), KEPT_NOTES);
        assert_eq!(notes.last().unwrap().text, "note 6");
    }
}
