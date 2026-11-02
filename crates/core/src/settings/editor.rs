//! The editor setting: the preset argument lists, the custom command template and its
//! splitting into arguments.
//!
//! A command never goes through a shell: the template is split with the quoting rules of POSIX
//! shell words ([`split_words`]), then `{file}` and `{line}` are replaced inside each argument
//! ([`fill`]). The program (first element) is looked up on the PATH by whoever starts it, with the
//! same rules as omp.

use serde::Serialize;
use ts_rs::TS;

/// Which editor opens files at a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum EditorPreset {
    Vscode,
    Cursor,
    Zed,
    Jetbrains,
    Sublime,
    Custom,
}

impl EditorPreset {
    pub const ALL: [EditorPreset; 6] = [
        EditorPreset::Vscode,
        EditorPreset::Cursor,
        EditorPreset::Zed,
        EditorPreset::Jetbrains,
        EditorPreset::Sublime,
        EditorPreset::Custom,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            EditorPreset::Vscode => "vscode",
            EditorPreset::Cursor => "cursor",
            EditorPreset::Zed => "zed",
            EditorPreset::Jetbrains => "jetbrains",
            EditorPreset::Sublime => "sublime",
            EditorPreset::Custom => "custom",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|preset| preset.as_str() == text)
    }

    /// What the developer reads in the preset list.
    pub fn label(self) -> &'static str {
        match self {
            EditorPreset::Vscode => "VS Code",
            EditorPreset::Cursor => "Cursor",
            EditorPreset::Zed => "Zed",
            EditorPreset::Jetbrains => "JetBrains",
            EditorPreset::Sublime => "Sublime Text",
            EditorPreset::Custom => "Custom command",
        }
    }

    /// The program and arguments of a preset, with `{file}` and `{line}` still to fill;
    /// `None` for `Custom`.
    pub fn template(self) -> Option<&'static [&'static str]> {
        match self {
            EditorPreset::Vscode => Some(&["code", "--goto", "{file}:{line}"]),
            EditorPreset::Cursor => Some(&["cursor", "--goto", "{file}:{line}"]),
            EditorPreset::Zed => Some(&["zed", "{file}:{line}"]),
            EditorPreset::Jetbrains => Some(&["idea", "--line", "{line}", "{file}"]),
            EditorPreset::Sublime => Some(&["subl", "{file}:{line}"]),
            EditorPreset::Custom => None,
        }
    }
}

/// Why a custom command cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    /// The template is empty, or has no `{file}`.
    #[error("The command must contain {{file}}.")]
    MissingFile,
    /// A quote or a trailing backslash is not closed.
    #[error("The command has an unclosed quote or a trailing backslash.")]
    Unsplittable,
}

/// Splits `template` into words with the quoting rules of POSIX shell words:
///
/// * blanks (space, tab, newline) separate words, except inside quotes;
/// * `\` outside quotes keeps the next character literally (backslash-newline disappears);
/// * `'...'` keeps everything literally;
/// * `"..."` keeps everything literally except that `\` before `"`, `\`, `$`, `` ` `` or a
///   newline escapes it (the newline disappears); any other `\` stays;
/// * `""` and `''` are an empty word.
///
/// No expansion of any kind happens (no `$`, no globbing, no comments).
pub fn split_words(template: &str) -> Result<Vec<String>, CommandError> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut chars = template.chars();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' | '\n' => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            '\\' => match chars.next() {
                None => return Err(CommandError::Unsplittable),
                Some('\n') => {}
                Some(escaped) => {
                    word.push(escaped);
                    in_word = true;
                }
            },
            '\'' => {
                in_word = true;
                loop {
                    match chars.next() {
                        None => return Err(CommandError::Unsplittable),
                        Some('\'') => break,
                        Some(inner) => word.push(inner),
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next() {
                        None => return Err(CommandError::Unsplittable),
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            None => return Err(CommandError::Unsplittable),
                            Some('\n') => {}
                            Some(escaped @ ('"' | '\\' | '$' | '`')) => word.push(escaped),
                            Some(other) => {
                                word.push('\\');
                                word.push(other);
                            }
                        },
                        Some(inner) => word.push(inner),
                    }
                }
            }
            other => {
                word.push(other);
                in_word = true;
            }
        }
    }
    if in_word {
        words.push(word);
    }
    Ok(words)
}

/// Splits a custom command and checks that it can be run: at least a program, and `{file}` in
/// some argument.
pub fn parse_custom(template: &str) -> Result<Vec<String>, CommandError> {
    let words = split_words(template)?;
    if words.iter().any(|word| word.contains("{file}")) {
        Ok(words)
    } else {
        Err(CommandError::MissingFile)
    }
}

/// Replaces `{file}` and `{line}` inside `argument`, in one pass: a file name that itself
/// contains `{line}` is left as it is.
pub fn fill(argument: &str, file: &str, line: u32) -> String {
    let mut out = String::with_capacity(argument.len() + file.len());
    let mut rest = argument;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        if let Some(tail) = tail.strip_prefix("{file}") {
            out.push_str(file);
            rest = tail;
        } else if let Some(tail) = tail.strip_prefix("{line}") {
            out.push_str(&line.to_string());
            rest = tail;
        } else {
            out.push('{');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

/// `1` when the line is unknown, never `0`.
pub fn effective_line(line: Option<u32>) -> u32 {
    line.filter(|line| *line >= 1).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(text: &str) -> Vec<String> {
        split_words(text).unwrap()
    }

    #[test]
    fn blanks_separate_words() {
        assert_eq!(
            split("code --goto {file}:{line}"),
            ["code", "--goto", "{file}:{line}"]
        );
        assert_eq!(split("  a \t b\nc  "), ["a", "b", "c"]);
        assert!(split("").is_empty());
        assert!(split("   ").is_empty());
    }

    #[test]
    fn single_quotes_keep_everything_literally() {
        assert_eq!(
            split(r#"'my editor' --x '$HOME \n "q"'"#),
            ["my editor", "--x", r#"$HOME \n "q""#]
        );
    }

    #[test]
    fn double_quotes_escape_only_the_four_characters() {
        assert_eq!(
            split(r#""a b" "say \"hi\"" "c:\\dir" "\$x" "\`y" "keep \n and \q""#),
            [
                "a b",
                "say \"hi\"",
                "c:\\dir",
                "$x",
                "`y",
                "keep \\n and \\q",
            ]
        );
    }

    #[test]
    fn a_backslash_outside_quotes_keeps_the_next_character() {
        assert_eq!(
            split(r"my\ editor a\\b \'q\'"),
            ["my editor", r"a\b", "'q'"]
        );
        assert_eq!(split("a\\\nb"), ["ab"]);
    }

    #[test]
    fn quotes_glue_to_the_word_around_them() {
        assert_eq!(
            split(r#"pre'mid dle'"post fix"end"#),
            ["premid dlepost fixend"]
        );
    }

    #[test]
    fn an_empty_quoted_word_is_a_word() {
        assert_eq!(split(r#"a "" '' b"#), ["a", "", "", "b"]);
    }

    #[test]
    fn nothing_is_expanded() {
        assert_eq!(
            split("$HOME *.rs ~ #x `id`"),
            ["$HOME", "*.rs", "~", "#x", "`id`"]
        );
    }

    #[test]
    fn an_unclosed_quote_or_a_trailing_backslash_does_not_split() {
        for bad in ["'abc", "\"abc", "abc\\", "\"abc\\", "a 'b"] {
            assert_eq!(split_words(bad), Err(CommandError::Unsplittable), "{bad:?}");
        }
    }

    #[test]
    fn a_custom_command_needs_a_program_and_a_file_placeholder() {
        assert_eq!(
            parse_custom("myeditor +{line} {file}").unwrap(),
            ["myeditor", "+{line}", "{file}"]
        );
        assert_eq!(
            parse_custom("myeditor '{file}'").unwrap(),
            ["myeditor", "{file}"]
        );
        assert_eq!(parse_custom(""), Err(CommandError::MissingFile));
        assert_eq!(
            parse_custom("myeditor {line}"),
            Err(CommandError::MissingFile)
        );
        assert_eq!(
            parse_custom("myeditor {line} 'unclosed {file}"),
            Err(CommandError::Unsplittable)
        );
        assert_eq!(
            CommandError::MissingFile.to_string(),
            "The command must contain {file}."
        );
    }

    #[test]
    fn placeholders_are_replaced_inside_each_argument() {
        assert_eq!(fill("{file}:{line}", "/p/a b.rs", 42), "/p/a b.rs:42");
        assert_eq!(fill("+{line}", "/p/a.rs", 7), "+7");
        assert_eq!(fill("--no-placeholder", "/p/a.rs", 7), "--no-placeholder");
        assert_eq!(fill("{other}{", "/p/a.rs", 7), "{other}{");
        assert_eq!(fill("{file}{file}", "x", 1), "xx");
    }

    #[test]
    fn a_file_name_that_looks_like_a_placeholder_is_not_filled_again() {
        assert_eq!(fill("{file}:{line}", "/p/{line}.rs", 3), "/p/{line}.rs:3");
    }

    #[test]
    fn an_unknown_line_is_one() {
        assert_eq!(effective_line(None), 1);
        assert_eq!(effective_line(Some(0)), 1);
        assert_eq!(effective_line(Some(12)), 12);
    }

    #[test]
    fn the_presets_are_the_documented_argument_lists() {
        let expected: [(EditorPreset, &[&str]); 5] = [
            (EditorPreset::Vscode, &["code", "--goto", "{file}:{line}"]),
            (EditorPreset::Cursor, &["cursor", "--goto", "{file}:{line}"]),
            (EditorPreset::Zed, &["zed", "{file}:{line}"]),
            (
                EditorPreset::Jetbrains,
                &["idea", "--line", "{line}", "{file}"],
            ),
            (EditorPreset::Sublime, &["subl", "{file}:{line}"]),
        ];
        for (preset, template) in expected {
            assert_eq!(preset.template(), Some(template));
        }
        assert_eq!(EditorPreset::Custom.template(), None);
    }

    #[test]
    fn preset_names_round_trip_and_match_the_serialised_form() {
        for preset in EditorPreset::ALL {
            assert_eq!(EditorPreset::parse(preset.as_str()), Some(preset));
            assert_eq!(
                serde_json::to_value(preset).unwrap(),
                serde_json::json!(preset.as_str())
            );
        }
        assert_eq!(EditorPreset::parse("emacs"), None);
        assert_eq!(EditorPreset::Jetbrains.label(), "JetBrains");
    }
}
