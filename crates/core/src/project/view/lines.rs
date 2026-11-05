//! The file and the line of an element, found in the text of the file it comes from.
//!
//! The parsed Model does not keep where each thing was written, so the line is found by reading
//! the outline of the file again (headings and top-level list items, with their offsets). Texts
//! are asked for once per file and kept for the rest of the build.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::project::frontmatter;
use crate::project::layout::{REQUIREMENTS_DIR, SPECS_FILE, WEAVER_DIR};
use crate::project::md::{self, Outline};
use crate::project::requirements::AREA_FILE;

use super::Files;

/// A file of the project and the line in it, as `Open in editor` takes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// Relative to the project folder: `.weaver/...`.
    pub file: String,
    pub line: Option<u32>,
}

/// One file's text and outline.
pub struct Doc {
    /// Relative to `.weaver/`.
    path: String,
    text: String,
    /// Bytes of front matter before the body the outline was taken from.
    prefix: usize,
    outline: Outline,
}

impl Doc {
    fn new(path: String, text: String) -> Doc {
        let (prefix, outline) = match frontmatter::parse(&text) {
            Ok(doc) => (text.len() - doc.body.len(), md::outline(&doc.body)),
            Err(_) => (0, md::outline(&text)),
        };
        Doc {
            path,
            text,
            prefix,
            outline,
        }
    }

    fn line_at(&self, body_offset: usize) -> u32 {
        let at = (self.prefix + body_offset).min(self.text.len());
        self.text.as_bytes()[..at]
            .iter()
            .filter(|b| **b == b'\n')
            .count() as u32
            + 1
    }

    fn heading_line(&self, matches: impl Fn(u8, &str) -> bool) -> Option<u32> {
        self.outline
            .headings
            .iter()
            .find(|h| matches(h.level, &h.text))
            .map(|h| self.line_at(h.range.start))
    }

    /// The first heading of level 1: the title.
    pub fn title(&self) -> Option<u32> {
        self.heading_line(|level, _| level == 1)
    }

    /// The heading of the State `name` (its text may end with `(initial)` or `(final)`).
    pub fn state(&self, name: &str) -> Option<u32> {
        self.heading_line(|level, text| level >= 3 && strip_marker(text) == name)
    }

    /// The `### NAME` under `## Events`.
    pub fn event(&self, name: &str) -> Option<u32> {
        let events = self
            .outline
            .headings
            .iter()
            .position(|h| h.level == 2 && h.text == "Events")?;
        self.outline.headings[events + 1..]
            .iter()
            .take_while(|h| h.level > 2)
            .find(|h| h.text == name)
            .map(|h| self.line_at(h.range.start))
    }

    /// The `## <title>` heading of a Requirements section.
    pub fn section(&self, title: &str) -> Option<u32> {
        self.heading_line(|level, text| level == 2 && text == title)
    }

    /// The line of `<field>:` in `specs.yaml`.
    pub fn specs_field(&self, field: &str) -> Option<u32> {
        let key = format!("{field}:");
        self.text
            .lines()
            .position(|l| l.starts_with(&key))
            .map(|i| i as u32 + 1)
    }
}

/// `Name (initial)` → `Name`.
fn strip_marker(text: &str) -> &str {
    text.strip_suffix(" (initial)")
        .or_else(|| text.strip_suffix(" (final)"))
        .unwrap_or(text)
}

type DocKey = (bool, String);

/// Finds places, reading each file's text once.
pub struct Locator<'a> {
    files: &'a Files<'a>,
    cache: RefCell<HashMap<DocKey, Option<Rc<Doc>>>>,
}

impl<'a> Locator<'a> {
    pub fn new(files: &'a Files<'a>) -> Self {
        Self {
            files,
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// The document of `rel`: in the shown image, or (for what only the current image has) on disk.
    pub fn doc(&self, rel: &str, removed: bool) -> Option<Rc<Doc>> {
        let key = (removed, rel.to_owned());
        if let Some(found) = self.cache.borrow().get(&key) {
            return found.clone();
        }
        let made = if removed {
            (self.files.current)(rel).map(|text| Doc::new(rel.to_owned(), text))
        } else {
            (self.files.shown)(rel).map(|(path, text)| Doc::new(path, text))
        }
        .map(Rc::new);
        self.cache.borrow_mut().insert(key, made.clone());
        made
    }

    /// The place `find` locates in `rel`; `None` when the file is not known.
    pub fn place(
        &self,
        rel: &str,
        removed: bool,
        find: impl FnOnce(&Doc) -> Option<u32>,
    ) -> Option<Place> {
        let doc = self.doc(rel, removed)?;
        Some(Place {
            file: format!("{WEAVER_DIR}/{}", doc.path),
            line: find(&doc),
        })
    }

    pub fn fragment_path(area: &str, fragment: &str) -> String {
        format!("{REQUIREMENTS_DIR}/{area}/{fragment}.md")
    }

    pub fn area_path(area: &str) -> String {
        format!("{REQUIREMENTS_DIR}/{area}/{AREA_FILE}.md")
    }

    pub fn specs_path() -> String {
        SPECS_FILE.to_owned()
    }

    /// The line an error message names: the first backticked name in it that is a heading of the
    /// file (a State, a section, an Event) or a field of `specs.yaml`.
    pub fn guess_line(&self, rel: &str, message: &str) -> Option<u32> {
        let doc = self.doc(rel, false).or_else(|| self.doc(rel, true))?;
        let mut names = message.split('`').skip(1).step_by(2);
        names.find_map(|name| {
            if rel == SPECS_FILE {
                doc.specs_field(name)
            } else {
                doc.state(name)
                    .or_else(|| doc.section(name))
                    .or_else(|| doc.event(name))
            }
        })
    }
}
