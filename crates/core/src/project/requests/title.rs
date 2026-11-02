//! What a request's text says about it: whether it is empty and what its title is.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Empty: nothing is left after removing whitespace. A request made only of images is not empty.
pub fn is_empty(text: &str) -> bool {
    text.trim().is_empty()
}

/// The title of a request: the first H1 outside code fences; otherwise the first non-empty line
/// that is not only images; otherwise `id`.
pub fn title_of(text: &str, id: &str) -> String {
    first_h1(text)
        .or_else(|| first_text_line(text))
        .unwrap_or_else(|| id.to_owned())
}

fn first_h1(text: &str) -> Option<String> {
    let mut in_h1 = false;
    let mut title = String::new();
    for event in Parser::new_ext(text, Options::empty()) {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => {
                in_h1 = true;
                title.clear();
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) => {
                in_h1 = false;
                let found = title.trim();
                if !found.is_empty() {
                    return Some(found.to_owned());
                }
            }
            Event::Text(t) | Event::Code(t) if in_h1 => title.push_str(&t),
            Event::SoftBreak | Event::HardBreak if in_h1 => title.push(' '),
            _ => {}
        }
    }
    None
}

fn first_text_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !without_images(line).trim().is_empty())
        .map(str::to_owned)
}

/// `line` with every `![alt](target)` removed.
fn without_images(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find("![") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match image_end(after) {
            Some(end) => rest = &after[end..],
            None => {
                out.push_str("![");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Where the image syntax that starts after `![` ends (`alt](target)`), as an offset in `after`.
fn image_end(after: &str) -> Option<usize> {
    let alt_end = after.find("](")?;
    let target = &after[alt_end + 2..];
    let close = target.find(')')?;
    Some(alt_end + 2 + close + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "2026-11-02T10-15-30";

    #[test]
    fn empty_means_nothing_left_after_removing_whitespace() {
        assert!(is_empty(""));
        assert!(is_empty(" \n\t \r\n"));
        assert!(!is_empty("a"));
        // Only images is not empty.
        assert!(!is_empty("![](2026-11-02T10-15-30.images/image-1.png)"));
    }

    #[test]
    fn the_title_is_the_first_h1() {
        assert_eq!(
            title_of("Some intro\n\n# The real title\n\n# Another\n", ID),
            "The real title"
        );
        assert_eq!(
            title_of("# Add `export` to *orders*\n", ID),
            "Add export to orders"
        );
        assert_eq!(
            title_of("Setext title\n============\n\ntext", ID),
            "Setext title"
        );
    }

    #[test]
    fn a_heading_in_a_code_fence_is_not_the_title() {
        let text = "First line\n\n```sh\n# a comment\n```\n\n~~~\n# another\n~~~\n";
        assert_eq!(title_of(text, ID), "First line");
        assert_eq!(
            title_of("```\n# in code\n```\n\n# Outside\n", ID),
            "Outside"
        );
    }

    #[test]
    fn without_h1_the_title_is_the_first_line_that_is_not_only_images() {
        assert_eq!(
            title_of("\n\n  Make orders faster  \nmore", ID),
            "Make orders faster"
        );
        assert_eq!(title_of("## Not an H1\n", ID), "## Not an H1");
        let text = "![](2026.images/image-1.png)  ![shot](x.png)\n\nWhat the screen should do\n";
        assert_eq!(title_of(text, ID), "What the screen should do");
        // Text next to an image is a title.
        assert_eq!(
            title_of("![](a.png) the screen\n", ID),
            "![](a.png) the screen"
        );
    }

    #[test]
    fn without_text_the_title_is_the_id() {
        assert_eq!(title_of("", ID), ID);
        assert_eq!(title_of("  \n \n", ID), ID);
        assert_eq!(title_of("![](a.png)\n![](b.png)\n", ID), ID);
    }

    #[test]
    fn an_unclosed_image_marker_is_text() {
        assert_eq!(title_of("![ not closed\n", ID), "![ not closed");
    }
}
