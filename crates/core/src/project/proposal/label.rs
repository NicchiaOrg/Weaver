//! The label of a Proposal: the short text on its card and on its Tasks, and its colour.

/// A label has 1 to this many characters.
pub const LABEL_MAX: usize = 30;

/// The colours of a label, in the order the hash picks them (pink, not red, so that no Task
/// border looks like a Drift's).
pub const TONES: [&str; 6] = ["blue", "green", "amber", "violet", "cyan", "pink"];

/// The label trimmed, or why it is refused: 1–30 characters on one line.
pub fn validate_label(label: &str) -> Result<String, String> {
    if label.contains(['\n', '\r']) {
        return Err("the label is one line".to_owned());
    }
    let label = label.trim();
    let length = label.chars().count();
    if length == 0 {
        return Err("the label is empty: it has 1 to 30 characters".to_owned());
    }
    if length > LABEL_MAX {
        return Err(format!(
            "the label has {length} characters: at most {LABEL_MAX}"
        ));
    }
    Ok(label.to_owned())
}

/// FNV-1a, 32 bits, of the UTF-8 bytes.
pub fn fnv1a(text: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in text.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// The colour of a label: `blue`, `green`, `amber`, `violet`, `cyan` or `pink`.
pub fn label_tone(label: &str) -> &'static str {
    TONES[(fnv1a(label) % TONES.len() as u32) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_has_one_to_thirty_characters_on_one_line() {
        assert_eq!(validate_label("  Checkout "), Ok("Checkout".to_owned()));
        assert!(validate_label("").is_err());
        assert!(validate_label("   ").is_err());
        assert!(validate_label("two\nlines").is_err());
        assert!(validate_label(&"a".repeat(30)).is_ok());
        assert!(validate_label(&"a".repeat(31)).is_err());
        // Characters, not bytes.
        assert!(validate_label(&"è".repeat(30)).is_ok());
    }

    #[test]
    fn the_colour_is_fnv1a_modulo_six() {
        // Reference values of FNV-1a 32.
        assert_eq!(fnv1a(""), 0x811c_9dc5);
        assert_eq!(fnv1a("a"), 0xe40c_292c);
        assert_eq!(fnv1a("foobar"), 0xbf9c_f968);
        assert_eq!(label_tone(""), TONES[(0x811c_9dc5u32 % 6) as usize]);
        assert_eq!(label_tone("a"), TONES[(0xe40c_292cu32 % 6) as usize]);
        assert_eq!(label_tone("Checkout"), label_tone("Checkout"));
        let labels: Vec<String> = (0..100).map(|n| format!("label {n}")).collect();
        let seen: std::collections::BTreeSet<&str> = labels.iter().map(|l| label_tone(l)).collect();
        assert_eq!(seen.len(), 6, "every colour is reachable");
    }
}
