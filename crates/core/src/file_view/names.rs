//! The names the File view accepts for a new file or folder (`requirements/rules#file-view`).

use super::Refusal;

/// The characters Windows refuses in a name (besides `/`, `\` and NUL, refused everywhere).
const WINDOWS_FORBIDDEN: [char; 7] = ['<', '>', ':', '"', '|', '?', '*'];

/// The device names Windows reserves, with or without an extension.
const RESERVED: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];

/// Whether `stem` is a reserved device name: `CON`, `PRN`, `AUX`, `NUL`, `COM0`–`COM9`,
/// `LPT0`–`LPT9`, in any case.
fn is_reserved(stem: &str) -> bool {
    let upper = stem.trim_end_matches(' ').to_ascii_uppercase();
    if RESERVED.contains(&upper.as_str()) {
        return true;
    }
    let numbered = |prefix: &str| {
        upper
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.len() == 1 && rest.chars().all(|c| c.is_ascii_digit()))
    };
    numbered("COM") || numbered("LPT")
}

/// `Ok` when `name` is a valid name for an entry. The Windows rules apply when `windows` is true
/// (always on Windows; the argument lets the tests check them on any system).
pub fn validate(name: &str, windows: bool) -> Result<(), Refusal> {
    if name.is_empty() {
        return Err(Refusal::new("The name cannot be empty."));
    }
    if name == "." || name == ".." {
        return Err(Refusal::new("The name cannot be \".\" or \"..\"."));
    }
    if name.contains(['/', '\\', '\0']) {
        return Err(Refusal::new(
            "The name cannot contain \"/\", \"\\\" or a NUL character.",
        ));
    }
    if windows {
        if name.contains(WINDOWS_FORBIDDEN) {
            return Err(Refusal::new(
                "The name cannot contain any of < > : \" | ? *.",
            ));
        }
        if name.ends_with([' ', '.']) {
            return Err(Refusal::new("The name cannot end with a space or a dot."));
        }
        let stem = name.split('.').next().unwrap_or(name);
        if is_reserved(stem) {
            return Err(Refusal::new(format!(
                "\"{stem}\" is a reserved name and cannot be used."
            )));
        }
    }
    Ok(())
}

/// [`validate`] with the rules of the system Weaver runs on.
pub fn check(name: &str) -> Result<(), Refusal> {
    validate(name, cfg!(windows))
}

/// The refusal for a name already in the folder.
pub fn taken(name: &str) -> Refusal {
    Refusal::new(format!(
        "A file or folder named \"{name}\" already exists here."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(name: &str, windows: bool) -> bool {
        validate(name, windows).is_err()
    }

    #[test]
    fn empty_dot_and_dot_dot_are_refused_everywhere() {
        for windows in [false, true] {
            assert!(refused("", windows));
            assert!(refused(".", windows));
            assert!(refused("..", windows));
            assert!(!refused("...", false));
        }
    }

    #[test]
    fn a_separator_or_a_nul_is_refused_everywhere() {
        for windows in [false, true] {
            assert!(refused("a/b", windows));
            assert!(refused("a\\b", windows));
            assert!(refused("a\0b", windows));
            assert!(!refused("a b.txt", windows));
            assert!(!refused(".gitignore", windows));
        }
    }

    #[test]
    fn windows_forbids_its_characters_trailing_space_or_dot_and_device_names() {
        for name in [
            "a<b",
            "a>b",
            "a:b",
            "a\"b",
            "a|b",
            "a?b",
            "a*b",
            "a ",
            "a.",
            "CON",
            "nul",
            "Com1",
            "lpt9.txt",
            "aux.tar.gz",
            "COM0",
        ] {
            assert!(refused(name, true), "{name} on Windows");
            if !name.ends_with([' ', '.']) && !name.contains(['<', '>', ':', '"', '|', '?', '*']) {
                assert!(!refused(name, false), "{name} elsewhere");
            }
        }
        for name in ["a<b", "a:b", "a ", "a."] {
            assert!(!refused(name, false), "{name} elsewhere");
        }
        for name in ["console", "COM10", "COMA", "LPT", "nullable"] {
            assert!(!refused(name, true), "{name} is a fine name");
        }
    }

    #[test]
    fn the_reason_names_what_is_wrong() {
        assert_eq!(
            validate("CON", true).unwrap_err().to_string(),
            "\"CON\" is a reserved name and cannot be used."
        );
        assert_eq!(
            taken("a.txt").to_string(),
            "A file or folder named \"a.txt\" already exists here."
        );
    }
}
