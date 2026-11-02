//! A small outline of a Markdown text: its headings and the top-level list items, outside
//! code fences, with their byte offsets. The Model, Requirement and card parsers read the
//! structure from here and the prose from the source text between the offsets.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
pub(crate) struct Heading {
    /// 1 to 6.
    pub level: u8,
    /// The heading text, without `#` marks.
    pub text: String,
    /// Byte range of the heading in the source, without the trailing newline.
    pub range: Range<usize>,
}

/// A list item that is a direct child of a top-level list.
#[derive(Debug, Clone)]
pub(crate) struct Item {
    /// Byte range of the item in the source, without the trailing newline.
    pub range: Range<usize>,
    /// When the item starts with a code span: its content and the end of the span.
    pub code: Option<(String, usize)>,
}

#[derive(Debug, Default)]
pub(crate) struct Outline {
    pub headings: Vec<Heading>,
    pub items: Vec<Item>,
}

pub(crate) fn outline(source: &str) -> Outline {
    let events: Vec<(Event, Range<usize>)> = Parser::new_ext(source, Options::empty())
        .into_offset_iter()
        .collect();
    let mut out = Outline::default();
    let mut list_depth = 0usize;
    let mut quote_depth = 0usize;
    let mut i = 0;
    while i < events.len() {
        let (event, range) = &events[i];
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let range = trim_end(source, range.clone());
                out.headings.push(Heading {
                    level: *level as u8,
                    text: heading_text(&source[range.clone()]),
                    range,
                });
            }
            Event::Start(Tag::BlockQuote(_)) => quote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => quote_depth = quote_depth.saturating_sub(1),
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth = list_depth.saturating_sub(1),
            Event::Start(Tag::Item) if list_depth == 1 && quote_depth == 0 => {
                let mut j = i + 1;
                while matches!(events.get(j), Some((Event::Start(Tag::Paragraph), _))) {
                    j += 1;
                }
                let code = match events.get(j) {
                    Some((Event::Code(code), span)) => Some((code.to_string(), span.end)),
                    _ => None,
                };
                out.items.push(Item {
                    range: trim_end(source, range.clone()),
                    code,
                });
            }
            _ => {}
        }
        i += 1;
    }
    out
}

fn trim_end(source: &str, range: Range<usize>) -> Range<usize> {
    let end = range.start + source[range.clone()].trim_end().len();
    range.start..end
}

/// The text of a heading from its source: ATX marks and closing marks removed (first line
/// only for a setext heading).
fn heading_text(source: &str) -> String {
    let first = source.lines().next().unwrap_or("");
    if first.starts_with('#') {
        let text = first.trim_start_matches('#').trim();
        let without_closing = text.trim_end_matches('#');
        // A closing sequence must be preceded by a space; `C#` keeps its `#`.
        if without_closing.len() < text.len() && without_closing.ends_with(' ') {
            return without_closing.trim().to_owned();
        }
        if without_closing.is_empty() {
            return String::new();
        }
        text.to_owned()
    } else {
        first.trim().to_owned()
    }
}

/// The source between `start` and `end`, minus the given ranges (which must be inside it,
/// in order), trimmed.
pub(crate) fn text_excluding(
    source: &str,
    start: usize,
    end: usize,
    excluded: &[Range<usize>],
) -> String {
    let mut out = String::new();
    let mut cursor = start;
    for range in excluded.iter().filter(|r| r.start >= start && r.end <= end) {
        out.push_str(&source[cursor..range.start]);
        cursor = range.end;
    }
    out.push_str(&source[cursor..end]);
    out.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_outside_code_fences_only() {
        let src = "# One\n\n```\n# not a heading\n```\n\n## Two ##\n\n### C#\n";
        let o = outline(src);
        let got: Vec<_> = o
            .headings
            .iter()
            .map(|h| (h.level, h.text.as_str()))
            .collect();
        assert_eq!(got, vec![(1, "One"), (2, "Two"), (3, "C#")]);
    }

    #[test]
    fn top_level_items_with_a_leading_code_span() {
        let src = "### S\n- `a.B` → T: prose\n  - `nested.C` → U\n- plain\n- `x.Y` and more\n";
        let o = outline(src);
        assert_eq!(o.items.len(), 3);
        assert_eq!(o.items[0].code.as_ref().unwrap().0, "a.B");
        assert!(o.items[1].code.is_none());
        assert_eq!(o.items[2].code.as_ref().unwrap().0, "x.Y");
        let first = &o.items[0];
        assert!(src[first.range.clone()].starts_with("- `a.B`"));
        assert!(src[first.range.clone()].contains("nested.C"));
        assert!(src[first.code.as_ref().unwrap().1..].starts_with(" →"));
    }

    #[test]
    fn loose_lists_and_quotes() {
        let src = "- `a.B` → T\n\n- `c.D` → U\n\n> - `q.R` → V\n";
        let o = outline(src);
        assert_eq!(o.items.len(), 2);
        assert!(o.items.iter().all(|i| i.code.is_some()));
    }

    #[test]
    fn text_between_offsets_without_the_excluded_ranges() {
        let src = "intro\n- keep\n- drop\nmore\n";
        let drop_start = src.find("- drop").unwrap();
        let text = text_excluding(
            src,
            0,
            src.len(),
            std::slice::from_ref(&(drop_start..drop_start + 6)),
        );
        assert_eq!(text, "intro\n- keep\n\nmore");
    }
}
