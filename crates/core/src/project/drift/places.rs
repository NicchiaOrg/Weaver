//! The files a Drift names under `## Where in the code`.
//!
//! The section holds files and code names as Markdown. A file is a code span that reads as a path
//! (`src/ship.rs`) with its line when it has one (`src/ship.rs:42`, `src/ship.rs:42-60`,
//! `src/ship.rs#L42`). The details of a Drift show one row for each file, with `Open in editor`.

/// A file named in the section, with the first line given for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// As written: relative to the project, or absolute.
    pub file: String,
    pub line: Option<u32>,
}

/// Extensions that make a bare name a file (a name with a `/` is one already).
const EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "go", "java", "kt", "kts", "swift", "c",
    "h", "cc", "cpp", "hpp", "cs", "rb", "php", "scala", "md", "mdx", "json", "yaml", "yml",
    "toml", "css", "scss", "html", "sql", "sh", "ps1", "vue", "svelte", "lua", "dart", "ex", "exs",
];

/// The code spans of a line, in order.
fn spans(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('`') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('`') else { break };
        out.push(&after[..end]);
        rest = &after[end + 1..];
    }
    out
}

/// A span as a file and a line: `path`, `path:42`, `path:42-60`, `path:42:7`, `path#L42`.
fn place_of(span: &str) -> Option<Place> {
    let span = span.trim();
    if span.is_empty()
        || span.contains(char::is_whitespace)
        || span.contains("://")
        || span.contains(['(', ')', '{', '}', '<', '>', '*', '"', '\''])
    {
        return None;
    }
    let (file, line) = match span.split_once("#L") {
        Some((file, rest)) => (
            file,
            rest.split(['-', ':'])
                .next()
                .and_then(|n| n.parse::<u32>().ok()),
        ),
        None => match span.split_once(':') {
            Some((file, rest)) => (
                file,
                Some(
                    rest.split(['-', ':'])
                        .next()
                        .and_then(|n| n.parse::<u32>().ok())?,
                ),
            ),
            None => (span, None),
        },
    };
    let name = file.rsplit('/').next().unwrap_or(file);
    let extension = name.rsplit_once('.').map(|(_, extension)| extension);
    let known = extension.is_some_and(|e| EXTENSIONS.contains(&e));
    let named = !name.is_empty()
        && (known
            || (file.contains('/')
                && extension.is_some_and(|e| {
                    !e.is_empty() && e.bytes().all(|b| b.is_ascii_alphanumeric())
                })));
    (named && !file.ends_with('/')).then(|| Place {
        file: file.to_owned(),
        line: line.filter(|n| *n >= 1),
    })
}

/// The files named in `text`, one for each file in the order they first appear; the line is the
/// first one given for the file.
pub fn places(text: &str) -> Vec<Place> {
    let mut found: Vec<Place> = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        for span in spans(line) {
            let Some(place) = place_of(span) else {
                continue;
            };
            match found
                .iter_mut()
                .find(|existing| existing.file == place.file)
            {
                Some(existing) => {
                    if existing.line.is_none() {
                        existing.line = place.line;
                    }
                }
                None => found.push(place),
            }
        }
    }
    found
}
