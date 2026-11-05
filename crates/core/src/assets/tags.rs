//! The `@` tags of assets (`requirements/rules#assets`): which assets a text tags, the block
//! Weaver appends to a message for the agent, how that block is cut off again from a replayed
//! text, and the sentences a design or setup instruction gets.
//!
//! Everything here is pure. The names it is given are the names of the assets that exist at the
//! moment the developer sends; nothing is ever recognised against a later list.
//!
//! A name that cannot be written between backticks in a line for the agent (it holds a backtick or
//! a control character) is never recognised: the agent could not be told to read it without
//! breaking the line.

/// The line that opens the block appended to a message.
pub const BLOCK_HEAD: &str =
    "Assets tagged by the developer (added by Weaver, not written by the developer):";

/// What may follow the name of an asset in a tag, besides the end of the text and whitespace.
const AFTER_NAME: [char; 9] = ['.', ',', ';', ':', '!', '?', ')', ']', '}'];
const AFTER_NAME_QUOTES: [char; 2] = ['"', '\''];

const SENTENCE_HEAD: &str = "Read the file `.weaver/assets/";
const SENTENCE_TAIL: &str = "`.";

/// A tag found in a text: the byte range of `@<name>` and the asset it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// The byte offset of the `@`.
    pub start: usize,
    /// The byte offset just after the name.
    pub end: usize,
    pub name: String,
}

/// Whether the name of an asset can be told to the agent in a line, between backticks.
pub fn is_quotable(name: &str) -> bool {
    !name.is_empty() && !name.chars().any(|c| c == '`' || c.is_control())
}

fn may_follow(next: Option<char>) -> bool {
    match next {
        None => true,
        Some(c) => c.is_whitespace() || AFTER_NAME.contains(&c) || AFTER_NAME_QUOTES.contains(&c),
    }
}

/// Every tag of `text` against the assets `names`: `@` at the start of the text or after
/// whitespace, then the name of an asset compared exactly (case included), then the end of the
/// text, whitespace or one of `. , ; : ! ? ) ] } " '`. When several names fit the longest wins.
/// Anything else after `@` is plain text. Tags inside code count like any other.
pub fn find<S: AsRef<str>>(text: &str, names: &[S]) -> Vec<Tag> {
    let mut tags = Vec::new();
    let mut previous: Option<char> = None;
    let mut skip_to = 0;
    for (at, c) in text.char_indices() {
        let before = previous.replace(c);
        if at < skip_to || c != '@' || before.is_some_and(|b| !b.is_whitespace()) {
            continue;
        }
        let rest = &text[at + 1..];
        let longest = names
            .iter()
            .map(AsRef::as_ref)
            .filter(|name| is_quotable(name) && rest.starts_with(name))
            .filter(|name| may_follow(rest[name.len()..].chars().next()))
            .max_by_key(|name| name.len());
        if let Some(name) = longest {
            let end = at + 1 + name.len();
            tags.push(Tag {
                start: at,
                end,
                name: name.to_owned(),
            });
            skip_to = end;
        }
    }
    tags
}

/// The assets `text` tags, once each, in the order of their first tag.
pub fn tagged<S: AsRef<str>>(text: &str, names: &[S]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tag in find(text, names) {
        if !out.contains(&tag.name) {
            out.push(tag.name);
        }
    }
    out
}

/// ``Read the file `.weaver/assets/<name>`.``: the sentence that tells the agent to read an asset.
pub fn read_sentence(name: &str) -> String {
    format!("{SENTENCE_HEAD}{name}{SENTENCE_TAIL}")
}

/// The block appended to a message that tags `tagged`: the head line and one sentence per line;
/// `None` when nothing is tagged.
pub fn block(tagged: &[String]) -> Option<String> {
    if tagged.is_empty() {
        return None;
    }
    let sentences: Vec<String> = tagged.iter().map(|name| read_sentence(name)).collect();
    Some(format!("{BLOCK_HEAD}\n{}", sentences.join("\n")))
}

/// The text that goes to omp: the developer's `text`, then an empty line and the block. Without
/// tagged assets, the text as it is.
pub fn with_block(text: &str, tagged: &[String]) -> String {
    match block(tagged) {
        Some(block) => format!("{text}\n\n{block}"),
        None => text.to_owned(),
    }
}

/// Whether `line` is one of the sentences of the block.
fn is_sentence(line: &str) -> bool {
    line.strip_prefix(SENTENCE_HEAD)
        .and_then(|rest| rest.strip_suffix(SENTENCE_TAIL))
        .is_some_and(|name| !name.is_empty() && !name.contains('`'))
}

/// What the developer wrote of a text that went to omp: `text` without the block [`with_block`]
/// appended. The cut is at the last occurrence of the head line, and only when every line after it
/// is a sentence of the block; any other text is returned as it is.
pub fn without_block(text: &str) -> &str {
    let mut cut = None;
    for (at, _) in text.match_indices(BLOCK_HEAD) {
        if at == 0 || text.as_bytes()[at - 1] == b'\n' {
            cut = Some(at);
        }
    }
    let Some(at) = cut else { return text };
    let after = &text[at + BLOCK_HEAD.len()..];
    let Some(sentences) = after.strip_prefix('\n') else {
        return text;
    };
    if sentences.is_empty() || !sentences.lines().all(is_sentence) {
        return text;
    }
    let written = &text[..at];
    written.strip_suffix("\n\n").unwrap_or(written)
}

/// The sentences a design or setup instruction gets for the assets its request tags: one
/// ``Read the file `.weaver/assets/<name>`.`` each, every one preceded by a space, on the line of
/// the instruction. Empty without assets.
pub fn instruction_sentences(tagged: &[String]) -> String {
    tagged
        .iter()
        .map(|name| format!(" {}", read_sentence(name)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    fn found(text: &str, assets: &[&str]) -> Vec<String> {
        find(text, &names(assets))
            .into_iter()
            .map(|tag| text[tag.start..tag.end].to_owned())
            .collect()
    }

    #[test]
    fn a_tag_is_at_the_start_or_after_whitespace_and_names_an_existing_asset() {
        let assets = ["logo.png", "spec.pdf"];
        assert_eq!(found("@logo.png", &assets), ["@logo.png"]);
        assert_eq!(found("look at @logo.png now", &assets), ["@logo.png"]);
        assert_eq!(found("a\n@logo.png", &assets), ["@logo.png"]);
        assert_eq!(found("a\t@spec.pdf", &assets), ["@spec.pdf"]);
        // Not after a word, and not for a name that is no asset.
        assert_eq!(found("mail me@logo.png", &assets), Vec::<String>::new());
        assert_eq!(found("see @other.png", &assets), Vec::<String>::new());
        assert_eq!(found("@", &assets), Vec::<String>::new());
        assert_eq!(found("@ logo.png", &assets), Vec::<String>::new());
    }

    #[test]
    fn the_name_is_compared_exactly_with_its_case() {
        assert_eq!(found("@Logo.png", &["logo.png"]), Vec::<String>::new());
        assert_eq!(found("@logo.PNG", &["logo.png"]), Vec::<String>::new());
        assert_eq!(found("@logo.png", &["logo.png"]), ["@logo.png"]);
    }

    #[test]
    fn the_name_must_be_followed_by_the_end_whitespace_or_one_of_the_closing_characters() {
        let assets = ["a"];
        for ok in [
            "@a", "@a ", "@a\n", "@a.", "@a,", "@a;", "@a:", "@a!", "@a?", "@a)", "@a]", "@a}",
            "@a\"", "@a'",
        ] {
            assert_eq!(found(ok, &assets), ["@a"], "{ok:?}");
        }
        for bad in [
            "@ab", "@a-", "@a_b", "@a/", "@a(", "@a@", "@a`", "@a*", "@a>",
        ] {
            assert_eq!(found(bad, &assets), Vec::<String>::new(), "{bad:?}");
        }
    }

    #[test]
    fn the_longest_name_wins_and_names_with_spaces_need_no_quotes() {
        let assets = ["notes", "notes.txt", "my notes.txt", "my"];
        assert_eq!(found("@notes.txt", &assets), ["@notes.txt"]);
        assert_eq!(found("@notes.", &assets), ["@notes"]);
        assert_eq!(
            found("read @my notes.txt please", &assets),
            ["@my notes.txt"]
        );
        assert_eq!(found("read @my notes please", &assets), ["@my"]);
    }

    #[test]
    fn a_tag_counts_inside_code_and_next_to_another_tag() {
        let assets = ["a.png", "b.png"];
        // Code is no exception: the rule is only what comes before the `@` and after the name.
        assert_eq!(found("`@a.png`", &assets), Vec::<String>::new());
        assert_eq!(found("`x @a.png`", &assets), Vec::<String>::new());
        assert_eq!(found("`x @a.png `", &assets), ["@a.png"]);
        assert_eq!(found("```\n@a.png\n```", &assets), ["@a.png"]);
        assert_eq!(found("@a.png @b.png", &assets), ["@a.png", "@b.png"]);
        assert_eq!(found("(@a.png)", &assets), Vec::<String>::new());
        // The second `@` follows a comma, not whitespace.
        assert_eq!(found("@a.png,@b.png", &assets), ["@a.png"]);
    }

    #[test]
    fn the_offsets_are_byte_offsets_of_the_text() {
        let text = "è @a.png ☕ @b.png";
        let tags = find(text, &names(&["a.png", "b.png"]));
        assert_eq!(tags.len(), 2);
        assert_eq!(&text[tags[0].start..tags[0].end], "@a.png");
        assert_eq!(&text[tags[1].start..tags[1].end], "@b.png");
        assert_eq!(tags[1].name, "b.png");
    }

    #[test]
    fn an_asset_tagged_twice_counts_once_in_the_order_of_its_first_tag() {
        let assets = names(&["a.png", "b.png", "c.png"]);
        assert_eq!(
            tagged("@b.png then @a.png and @b.png again", &assets),
            ["b.png", "a.png"]
        );
        assert_eq!(tagged("nothing here @c", &assets), Vec::<String>::new());
    }

    #[test]
    fn a_name_that_cannot_be_quoted_is_never_a_tag() {
        let assets = ["a`b", "tab\tname", "x"];
        assert_eq!(found("@a`b @tab\tname @x", &assets), ["@x"]);
    }

    #[test]
    fn the_block_is_the_head_line_and_one_sentence_per_line_after_an_empty_line() {
        let tagged = names(&["a.png", "my notes.txt"]);
        assert_eq!(
            with_block("Look at these", &tagged),
            "Look at these\n\nAssets tagged by the developer (added by Weaver, not written by the developer):\nRead the file `.weaver/assets/a.png`.\nRead the file `.weaver/assets/my notes.txt`."
        );
        assert_eq!(with_block("Plain", &[]), "Plain");
        assert_eq!(block(&[]), None);
    }

    #[test]
    fn the_block_is_cut_off_again_and_only_when_it_is_the_whole_end_of_the_text() {
        let tagged = names(&["a.png", "my notes.txt"]);
        for written in [
            "Look",
            "Two\nlines\n\nwith a gap",
            "ends with an @a.png tag",
        ] {
            let sent = with_block(written, &tagged);
            assert_eq!(without_block(&sent), written, "{sent}");
        }
        assert_eq!(without_block("no block"), "no block");
        // The developer quoted the head line: the last one is the block's.
        let quoted = format!("{BLOCK_HEAD}\nbut I wrote this");
        assert_eq!(without_block(&quoted), quoted);
        let both = with_block(
            &format!("{BLOCK_HEAD}\nRead the file `.weaver/assets/x`."),
            &tagged,
        );
        assert_eq!(
            without_block(&both),
            format!("{BLOCK_HEAD}\nRead the file `.weaver/assets/x`.")
        );
        // Something other than a sentence after the head: not a block.
        let other = format!("a\n\n{BLOCK_HEAD}\nRead the file `.weaver/assets/a.png`.\nand more");
        assert_eq!(without_block(&other), other);
        let nothing = format!("a\n\n{BLOCK_HEAD}\n");
        assert_eq!(without_block(&nothing), nothing);
        let inline = format!("a {BLOCK_HEAD}\nRead the file `.weaver/assets/a.png`.");
        assert_eq!(without_block(&inline), inline);
    }

    #[test]
    fn the_instruction_gets_one_sentence_per_asset_each_after_a_space() {
        assert_eq!(instruction_sentences(&[]), "");
        assert_eq!(
            instruction_sentences(&names(&["a.png", "b c.txt"])),
            " Read the file `.weaver/assets/a.png`. Read the file `.weaver/assets/b c.txt`."
        );
    }
}
