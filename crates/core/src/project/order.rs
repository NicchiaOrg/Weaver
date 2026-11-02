//! Order of cards: oldest `created` first, ties by identity in byte order.

use std::cmp::Ordering;

use super::cards::Card;

/// Compares two cards by their `created` instant (UTC ISO 8601 with milliseconds, which sorts
/// as text) and then by identity in byte order.
pub fn compare(a_created: &str, a_id: &str, b_created: &str, b_id: &str) -> Ordering {
    a_created
        .as_bytes()
        .cmp(b_created.as_bytes())
        .then_with(|| a_id.as_bytes().cmp(b_id.as_bytes()))
}

/// Sorts `items` in creation order; `key` gives the `created` instant and the identity.
pub fn sort_by_creation<T>(items: &mut [T], key: impl Fn(&T) -> (&str, &str)) {
    items.sort_by(|a, b| {
        let ((a_created, a_id), (b_created, b_id)) = (key(a), key(b));
        compare(a_created, a_id, b_created, b_id)
    });
}

/// Sorts cards in creation order.
pub fn sort_cards(cards: &mut [Card]) {
    sort_by_creation(cards, |c| {
        (c.created.as_deref().unwrap_or(""), c.id.as_str())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_first_then_identity_by_byte() {
        let mut items = vec![
            ("2026-11-02T07:07:19.522Z", "bbbbbbbb"),
            ("2026-11-02T07:07:19.521Z", "zzzzzzzz"),
            ("2026-11-02T07:07:19.522Z", "aaaaaaaa"),
            ("2026-11-02T07:07:19.522Z", "1zzzzzzz"),
            ("2025-12-31T23:59:59.999Z", "mmmmmmmm"),
        ];
        sort_by_creation(&mut items, |i| (i.0, i.1));
        let ids: Vec<_> = items.iter().map(|i| i.1).collect();
        assert_eq!(
            ids,
            ["mmmmmmmm", "zzzzzzzz", "1zzzzzzz", "aaaaaaaa", "bbbbbbbb"]
        );
    }

    #[test]
    fn digits_sort_before_letters_in_byte_order() {
        assert_eq!(compare("t", "9aaaaaaa", "t", "aaaaaaaa"), Ordering::Less);
        assert_eq!(compare("t", "aaaaaaaa", "t", "aaaaaaaa"), Ordering::Equal);
    }
}
