//! Identities, slugs and card file names.
//!
//! A card identity (Proposal, Task, Drift, History entry) and a job identity match
//! `^[a-z0-9]{8}$`, are drawn from a [`RandomSource`] and drawn again while already used by
//! one of the same type. The identity of a card is read from its file or folder name
//! (`<id>-<slug>`), never from its content.

use unicode_normalization::UnicodeNormalization;

use super::random::RandomSource;

/// Number of characters of an identity.
pub const ID_LEN: usize = 8;

/// Longest slug, in characters.
pub const SLUG_MAX_LEN: usize = 40;

/// Slug of a title that leaves nothing.
pub const EMPTY_SLUG: &str = "untitled";

const ALPHABET: &[u8; 36] = b"abcdefghijklmnopqrstuvwxyz0123456789";

/// Bytes at or above this value are discarded so that every character is equally likely.
const ACCEPT_BELOW: u8 = 252; // the largest multiple of 36 that fits in a byte

/// Whether `text` matches `^[a-z0-9]{8}$`.
pub fn is_identity(text: &str) -> bool {
    text.len() == ID_LEN
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// Whether `text` is one letter followed by digits only, which the commands always read as a
/// short number, never as an identity.
pub fn looks_like_short_number(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && !text[1..].is_empty()
        && text[1..].bytes().all(|b| b.is_ascii_digit())
}

/// Draws a new identity: random, never in `taken`, and never one a command would read as a
/// short number (a letter followed by digits).
///
/// `taken` answers whether an identity is already used by a card of the same type.
pub fn draw_identity(rng: &mut dyn RandomSource, taken: &dyn Fn(&str) -> bool) -> String {
    loop {
        let candidate = draw_candidate(rng);
        if !taken(&candidate) && !looks_like_short_number(&candidate) {
            return candidate;
        }
    }
}

fn draw_candidate(rng: &mut dyn RandomSource) -> String {
    let mut id = String::with_capacity(ID_LEN);
    let mut byte = [0u8; 1];
    while id.len() < ID_LEN {
        rng.fill(&mut byte);
        if byte[0] < ACCEPT_BELOW {
            id.push(char::from(ALPHABET[usize::from(byte[0]) % ALPHABET.len()]));
        }
    }
    id
}

/// The slug of a title: lowercased, diacritics removed, every run of characters outside
/// `[a-z0-9]` turned into one `-`, no `-` at either end, at most [`SLUG_MAX_LEN`] characters
/// without a trailing `-`; [`EMPTY_SLUG`] when nothing is left.
pub fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for c in title
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
    {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(c);
        } else {
            pending_dash = true;
        }
    }
    slug.truncate(SLUG_MAX_LEN);
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        EMPTY_SLUG.to_owned()
    } else {
        slug
    }
}

fn is_combining_mark(c: char) -> bool {
    matches!(u32::from(c),
        0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F)
}

/// `<id>-<slug>`: the name of a card file (without `.md`) or folder.
pub fn card_stem(id: &str, slug: &str) -> String {
    format!("{id}-{slug}")
}

/// Splits `<id>-<slug>` into its identity and slug; `None` when the name does not have that
/// shape.
pub fn split_card_stem(stem: &str) -> Option<(&str, &str)> {
    let id = stem.get(..ID_LEN)?;
    let slug = stem.get(ID_LEN..)?.strip_prefix('-')?;
    (is_identity(id) && !slug.is_empty()).then_some((id, slug))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::random::ScriptedRandom;

    #[test]
    fn identity_shape() {
        assert!(is_identity("vxdhoylk"));
        assert!(is_identity("12345678"));
        for bad in [
            "",
            "vxdhoyl",
            "vxdhoylkk",
            "VXDHOYLK",
            "vxdhoyl-",
            "vxdhoylé",
        ] {
            assert!(!is_identity(bad), "{bad}");
        }
    }

    #[test]
    fn identities_come_from_the_random_bytes() {
        // byte % 36 indexes "a..z0..9": 0 -> a, 25 -> z, 26 -> 0, 35 -> 9.
        let mut rng = ScriptedRandom::new([0, 1, 2, 25, 26, 35, 36, 37]);
        let id = draw_identity(&mut rng, &|_| false);
        assert_eq!(id, "abcz09ab");
    }

    #[test]
    fn biased_bytes_are_discarded() {
        // 252..=255 would favour the first characters; they are skipped.
        let mut rng = ScriptedRandom::new([255, 252, 253, 254, 0]);
        assert_eq!(draw_identity(&mut rng, &|_| false), "aaaaaaaa");
    }

    #[test]
    fn a_collision_with_an_identity_of_the_same_type_draws_again() {
        let mut rng = ScriptedRandom::new([0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1]);
        let taken = |id: &str| id == "aaaaaaaa";
        assert_eq!(draw_identity(&mut rng, &taken), "bbbbbbbb");
    }

    #[test]
    fn several_collisions_in_a_row_are_all_skipped() {
        let mut rng = ScriptedRandom::new([
            0, 0, 0, 0, 0, 0, 0, 0, // aaaaaaaa
            1, 1, 1, 1, 1, 1, 1, 1, // bbbbbbbb
            2, 2, 2, 2, 2, 2, 2, 2, // cccccccc
        ]);
        let taken = |id: &str| id == "aaaaaaaa" || id == "bbbbbbbb";
        assert_eq!(draw_identity(&mut rng, &taken), "cccccccc");
    }

    #[test]
    fn an_identity_that_reads_as_a_short_number_is_drawn_again() {
        assert!(looks_like_short_number("t1234567"));
        assert!(looks_like_short_number("T3"));
        assert!(!looks_like_short_number("t"));
        assert!(!looks_like_short_number("12345678"));
        assert!(!looks_like_short_number("t12345a7"));
        // 19 = t, then 7 times 27 = '1'.
        let mut rng = ScriptedRandom::new([19, 27, 27, 27, 27, 27, 27, 27, 1, 1, 1, 1, 1, 1, 1, 1]);
        assert_eq!(draw_identity(&mut rng, &|_| false), "bbbbbbbb");
    }

    #[test]
    fn slugs() {
        assert_eq!(
            slugify("Core dei file del progetto"),
            "core-dei-file-del-progetto"
        );
        assert_eq!(slugify("  Già  fatto!  "), "gia-fatto");
        assert_eq!(slugify("Crème brûlée — ÀÉÎÕÜ"), "creme-brulee-aeiou");
        assert_eq!(slugify("a---b__c  d"), "a-b-c-d");
        assert_eq!(slugify("--x--"), "x");
        assert_eq!(slugify("Task 42"), "task-42");
    }

    #[test]
    fn empty_slugs_are_untitled() {
        for title in ["", "   ", "---", "日本語", "!!!"] {
            assert_eq!(slugify(title), "untitled", "{title}");
        }
    }

    #[test]
    fn slugs_are_cut_at_40_characters_without_a_trailing_dash() {
        let long = "abcdefghij ".repeat(10);
        let slug = slugify(&long);
        assert!(slug.len() <= SLUG_MAX_LEN);
        assert_eq!(slug, "abcdefghij-abcdefghij-abcdefghij-abcdefg");
        // A cut that lands on a dash drops it.
        let title = format!("{} tail", "x".repeat(39));
        assert_eq!(slugify(&title), "x".repeat(39));
        let exact = format!("{}-{}", "y".repeat(40), "z");
        assert_eq!(slugify(&exact), "y".repeat(40));
    }

    #[test]
    fn card_names_round_trip() {
        let stem = card_stem("vxdhoylk", "core");
        assert_eq!(stem, "vxdhoylk-core");
        assert_eq!(split_card_stem(&stem), Some(("vxdhoylk", "core")));
        for bad in [
            "vxdhoylk",
            "vxdhoylk-",
            "vxdhoy-core",
            "VXDHOYLK-core",
            "vxdhoylkcore",
            "é",
        ] {
            assert_eq!(split_card_stem(bad), None, "{bad}");
        }
    }
}
