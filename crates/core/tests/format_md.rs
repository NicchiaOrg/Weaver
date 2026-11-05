//! The parser and the `FORMAT.md` reference of the `weaver-design` skill are tested together:
//! every example in the file Weaver installs (the one embedded in `weaver-core`) must parse.

use std::path::PathBuf;

use weaver_core::agent_files::installed_file;
use weaver_core::project::frontmatter;
use weaver_core::project::model::{parse_actor, parse_machine, parse_titled};
use weaver_core::project::requirements::{parse_area_doc, parse_fragment};
use weaver_core::project::specs::parse_specs;

const FORMAT_MD: &str = ".agents/skills/weaver-design/FORMAT.md";

fn format_md() -> (PathBuf, String) {
    let file = installed_file(FORMAT_MD).expect("FORMAT.md is embedded");
    (PathBuf::from(&file.path), file.contents.clone())
}

/// The fenced code blocks of a Markdown text: `(info string, content)`.
fn fenced_blocks(text: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in text.lines() {
        match (&mut current, line.strip_prefix("```")) {
            (None, Some(info)) => current = Some((info.trim().to_owned(), String::new())),
            (Some(_), Some(rest)) if rest.trim().is_empty() => {
                blocks.push(current.take().expect("open block"))
            }
            (Some((_, content)), _) => {
                content.push_str(line);
                content.push('\n');
            }
            (None, None) => {}
        }
    }
    blocks
}

#[derive(Debug, Default)]
struct Seen {
    actors: usize,
    machines: usize,
    areas: usize,
    fragments: usize,
    specs: usize,
}

#[test]
fn every_example_of_format_md_parses() {
    let (path, text) = format_md();
    let mut seen = Seen::default();
    let mut failures = Vec::new();
    for (index, (info, content)) in fenced_blocks(&text).into_iter().enumerate() {
        let label = format!("block {} ({info:?}) of {}", index + 1, path.display());
        let result: Result<(), Vec<String>> = match info.as_str() {
            "markdown" => {
                let body = frontmatter::parse(&content)
                    .map(|d| d.body)
                    .unwrap_or_else(|_| content.clone());
                if content.starts_with("---\n") {
                    if body.contains("\n## ") {
                        seen.fragments += 1;
                        parse_fragment("example", &content).map(|_| ())
                    } else {
                        seen.areas += 1;
                        parse_area_doc(&content).map(|_| ())
                    }
                } else if content.contains("## States") {
                    seen.machines += 1;
                    parse_machine("example", "example", &content).map(|_| ())
                } else if content.contains("## Events") {
                    seen.actors += 1;
                    parse_actor("example", &content).map(|_| ())
                } else {
                    parse_titled(&content).map(|_| ())
                }
            }
            "yaml" => {
                seen.specs += 1;
                parse_specs(&content).map(|_| ())
            }
            _ => Ok(()),
        };
        if let Err(errors) = result {
            failures.push(format!("{label}: {}", errors.join("; ")));
        }
    }
    assert!(
        failures.is_empty(),
        "examples that do not parse:\n{}",
        failures.join("\n")
    );
    // The file shows one of each; a rewrite that loses an example must update this test.
    assert!(seen.actors >= 1, "{seen:?}");
    assert!(seen.machines >= 1, "{seen:?}");
    assert!(seen.areas >= 1, "{seen:?}");
    assert!(seen.fragments >= 1, "{seen:?}");
    assert!(seen.specs >= 1, "{seen:?}");
}
