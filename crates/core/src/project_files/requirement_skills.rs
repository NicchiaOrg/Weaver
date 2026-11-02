//! The Requirement skills `trama-req-*`: generated from the Requirements of a project, in one of
//! two modes (`requirements/rules#project`). Everything here is a pure function of the Areas.
//!
//! * **Budget:** the sum, over every fragment with content, of the characters (Unicode scalar
//!   values) of its skill name `trama-req-<area>-<fragment>` plus its description. A fragment
//!   has content when at least one of its sections has non-blank text.
//! * **Mode:** up to [`BUDGET`] characters inclusive, *fragment mode*: one skill
//!   `trama-req-<area>-<fragment>` per fragment with content. Above, *index mode*: one skill
//!   `trama-req-<area>` per Area with prose or a fragment with content, and the content of each
//!   fragment in a file next to its `SKILL.md`. All the Areas switch together.

use std::collections::BTreeSet;

use serde::Serialize;
use ts_rs::TS;

use crate::agent_files::SKILLS_DIR;
use crate::project::refs::GENERAL_SECTION;
use crate::project::requirements::{Area, Fragment, Section};

/// The most characters the names and descriptions of the fragment skills may use for the project
/// to stay in fragment mode.
pub const BUDGET: usize = 4_000;

/// Every generated skill, and only generated skills, has a name starting with this.
pub const SKILL_PREFIX: &str = "trama-req-";

/// The file of a skill.
pub const SKILL_FILE: &str = "SKILL.md";

/// How the Requirement skills are generated, the same for every Area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename = "SkillMode")]
pub enum Mode {
    /// One skill per fragment with content.
    Fragment,
    /// One skill per Area, listing its fragments.
    Index,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Fragment => "fragment",
            Mode::Index => "index",
        }
    }
}

/// One generated file, by path relative to the project root (always with `/`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

/// What the Requirements of a project generate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generation {
    pub mode: Mode,
    /// The characters the names and descriptions of the fragment skills would use.
    pub budget: usize,
    /// Every file, sorted by path.
    pub files: Vec<GeneratedFile>,
    /// The Areas that have a generated skill, sorted.
    pub areas: Vec<String>,
}

/// `trama-req-<area>-<fragment>`.
pub fn fragment_skill_name(area: &str, fragment: &str) -> String {
    format!("{SKILL_PREFIX}{area}-{fragment}")
}

/// `trama-req-<area>`.
pub fn index_skill_name(area: &str) -> String {
    format!("{SKILL_PREFIX}{area}")
}

fn has_text(section: &Section) -> bool {
    !section.body.trim().is_empty()
}

/// Whether at least one section of the fragment has non-blank text.
pub fn has_content(fragment: &Fragment) -> bool {
    fragment.sections.iter().any(has_text)
}

/// The budget used by `areas`.
pub fn budget_of(areas: &[Area]) -> usize {
    areas
        .iter()
        .flat_map(|area| {
            area.fragments
                .iter()
                .filter(|f| has_content(f))
                .map(move |f| (area, f))
        })
        .map(|(area, fragment)| {
            fragment_skill_name(&area.id, &fragment.name)
                .chars()
                .count()
                + fragment.description.chars().count()
        })
        .sum()
}

/// The mode a budget gives.
pub fn mode_for(budget: usize) -> Mode {
    if budget <= BUDGET {
        Mode::Fragment
    } else {
        Mode::Index
    }
}

/// The mode the Requirements `areas` are generated in.
pub fn mode_of(areas: &[Area]) -> Mode {
    mode_for(budget_of(areas))
}

/// A description as a YAML scalar: a JSON string is a valid double-quoted YAML scalar.
fn yaml_text(text: &str) -> String {
    serde_json::to_string(text).expect("a string is serialisable")
}

fn front_matter(name: &str, description: &str) -> String {
    format!(
        "---\nname: {name}\ndescription: {}\n---\n",
        yaml_text(description)
    )
}

fn skill_path(name: &str, file: &str) -> String {
    format!("{SKILLS_DIR}/{name}/{file}")
}

/// `## <title>` and the text of each section with text, in order.
fn sections_text<'a>(sections: impl IntoIterator<Item = &'a Section>) -> String {
    let mut out = String::new();
    for section in sections.into_iter().filter(|s| has_text(s)) {
        out.push_str(&format!(
            "## {}\n\n{}\n\n",
            section.title,
            section.body.trim()
        ));
    }
    out
}

fn fragment_skill(area: &Area, fragment: &Fragment) -> GeneratedFile {
    let name = fragment_skill_name(&area.id, &fragment.name);
    let mut content = front_matter(&name, &fragment.description);
    content.push_str(&format!("\n# {}\n\n", area.title));
    if !area.intro.trim().is_empty() {
        content.push_str(&format!("{}\n\n", area.intro.trim()));
    }
    content.push_str(&sections_text(&fragment.sections));
    GeneratedFile {
        path: skill_path(&name, SKILL_FILE),
        content: content.trim_end().to_owned() + "\n",
    }
}

/// The name of the file that holds a fragment inside an index skill. A fragment called `skill`
/// would share its name with `SKILL.md` on a case-insensitive disk, so it gets a suffix.
fn fragment_file_name(fragment: &str) -> String {
    if fragment.eq_ignore_ascii_case("skill") {
        format!("{fragment}-fragment.md")
    } else {
        format!("{fragment}.md")
    }
}

fn general_of(area: &Area) -> Option<&Section> {
    area.sections()
        .map(|(_, s)| s)
        .find(|s| s.title == GENERAL_SECTION && has_text(s))
}

fn index_skills(area: &Area) -> Vec<GeneratedFile> {
    let with_content: Vec<&Fragment> = area.fragments.iter().filter(|f| has_content(f)).collect();
    if with_content.is_empty() && area.intro.trim().is_empty() {
        return Vec::new();
    }
    let name = index_skill_name(&area.id);
    let mut body = String::new();
    if !area.intro.trim().is_empty() {
        body.push_str(&format!("{}\n\n", area.intro.trim()));
    }
    if let Some(general) = general_of(area) {
        body.push_str(&format!("## General\n\n{}\n\n", general.body.trim()));
    }
    let mut files = Vec::new();
    for fragment in &with_content {
        let file = fragment_file_name(&fragment.name);
        let references: Vec<String> = fragment
            .sections
            .iter()
            .filter(|s| has_text(s))
            .map(|s| format!("`{}`", s.title))
            .collect();
        body.push_str(&format!(
            "## {}\n\n{}\n\nSections: {}\n\nFile: `{file}`\n\n",
            fragment.title,
            fragment.description,
            references.join(", ")
        ));
        let content = format!(
            "# {}\n\n{}",
            fragment.title,
            sections_text(&fragment.sections)
        );
        files.push(GeneratedFile {
            path: skill_path(&name, &file),
            content: content.trim_end().to_owned() + "\n",
        });
    }
    let mut content = front_matter(&name, &area.description);
    if !body.trim().is_empty() {
        content.push('\n');
        content.push_str(&body);
    }
    files.push(GeneratedFile {
        path: skill_path(&name, SKILL_FILE),
        content: content.trim_end().to_owned() + "\n",
    });
    files
}

/// The skill names `areas` would give in `mode`.
pub fn skill_names(areas: &[Area], mode: Mode) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for area in areas {
        match mode {
            Mode::Fragment => {
                for fragment in area.fragments.iter().filter(|f| has_content(f)) {
                    names.insert(fragment_skill_name(&area.id, &fragment.name));
                }
            }
            Mode::Index => {
                if !index_skills(area).is_empty() {
                    names.insert(index_skill_name(&area.id));
                }
            }
        }
    }
    names
}

/// Generates the Requirement skills of `areas`.
pub fn generate(areas: &[Area]) -> Generation {
    let budget = budget_of(areas);
    let mode = mode_for(budget);
    let mut files: Vec<GeneratedFile> = Vec::new();
    let mut with_skill: BTreeSet<String> = BTreeSet::new();
    for area in areas {
        let made = match mode {
            Mode::Fragment => area
                .fragments
                .iter()
                .filter(|f| has_content(f))
                .map(|f| fragment_skill(area, f))
                .collect(),
            Mode::Index => index_skills(area),
        };
        if !made.is_empty() {
            with_skill.insert(area.id.clone());
        }
        files.extend(made);
    }
    // Two names that spell the same path (`ui-a` + `b` and `ui` + `a-b`): the later one wins, the
    // same way every time.
    let mut by_path: std::collections::BTreeMap<String, GeneratedFile> =
        std::collections::BTreeMap::new();
    for file in files {
        by_path.insert(file.path.clone(), file);
    }
    Generation {
        mode,
        budget,
        files: by_path.into_values().collect(),
        areas: with_skill.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::requirements::{assemble_areas, parse_area_doc, parse_fragment};

    fn area_doc(title: &str, intro: &str) -> String {
        format!("---\ndescription: Use for {title}.\n---\n# {title}\n\n{intro}\n")
    }

    fn fragment_text(description: &str, sections: &[(&str, &str)]) -> String {
        let mut text = format!("---\ndescription: {description}\n---\n# Fragment\n\n");
        for (title, body) in sections {
            text.push_str(&format!("## {title}\n{body}\n\n"));
        }
        text
    }

    /// `(section title, body)`.
    type SectionSpec<'a> = (&'a str, &'a str);
    /// `(fragment, description, sections)`.
    type FragmentSpec<'a> = (&'a str, &'a str, &'a [SectionSpec<'a>]);

    /// `(area, intro, [(fragment, description, [(section, body)])])`.
    fn areas(spec: &[(&str, &str, &[FragmentSpec])]) -> Vec<Area> {
        let docs: Vec<(String, String, crate::project::requirements::AreaDoc)> = spec
            .iter()
            .map(|(id, intro, _)| {
                (
                    (*id).to_owned(),
                    format!("requirements/{id}/area.md"),
                    parse_area_doc(&area_doc(id, intro)).unwrap(),
                )
            })
            .collect();
        let frags: Vec<(String, String, Fragment)> = spec
            .iter()
            .flat_map(|(id, _, fragments)| {
                fragments.iter().map(move |(name, description, sections)| {
                    (
                        format!("requirements/{id}/{name}.md"),
                        (*id).to_owned(),
                        parse_fragment(name, &fragment_text(description, sections)).unwrap(),
                    )
                })
            })
            .collect();
        let doc_refs: Vec<(&str, &str, &crate::project::requirements::AreaDoc)> = docs
            .iter()
            .map(|(id, path, doc)| (path.as_str(), id.as_str(), doc))
            .collect();
        let frag_refs: Vec<(&str, &str, &Fragment)> = frags
            .iter()
            .map(|(path, id, f)| (path.as_str(), id.as_str(), f))
            .collect();
        assemble_areas(&doc_refs, &frag_refs).0
    }

    /// One fragment whose skill name and description use exactly `chars` characters.
    fn sized(chars: usize) -> Vec<Area> {
        let name_len = "trama-req-ui-main".chars().count();
        let description = "d".repeat(chars - name_len);
        areas(&[(
            "ui",
            "",
            &[("main", description.as_str(), &[("General", "- x")])],
        )])
    }

    #[test]
    fn the_budget_is_names_plus_descriptions_of_the_fragments_with_content() {
        let areas = areas(&[
            (
                "ui",
                "",
                &[
                    ("main", "Use for screens.", &[("General", "- a")]),
                    ("empty", "Nothing here.", &[("General", "  ")]),
                ],
            ),
            ("tech", "Prose.", &[]),
        ]);
        assert_eq!(
            budget_of(&areas),
            "trama-req-ui-main".chars().count() + "Use for screens.".chars().count()
        );
    }

    #[test]
    fn characters_are_unicode_scalar_values_not_bytes() {
        let areas = areas(&[(
            "ui",
            "",
            &[("main", "Schermate è così", &[("General", "- a")])],
        )]);
        assert_eq!(
            budget_of(&areas),
            "trama-req-ui-main".chars().count() + "Schermate è così".chars().count()
        );
    }

    #[test]
    fn up_to_4000_characters_is_fragment_mode_and_4001_is_index_mode() {
        let at = sized(4_000);
        assert_eq!((budget_of(&at), mode_of(&at)), (4_000, Mode::Fragment));
        let over = sized(4_001);
        assert_eq!((budget_of(&over), mode_of(&over)), (4_001, Mode::Index));
        assert_eq!(
            generate(&at).files[0].path,
            ".agents/skills/trama-req-ui-main/SKILL.md"
        );
        assert_eq!(
            generate(&over)
                .files
                .iter()
                .map(|f| f.path.as_str())
                .collect::<Vec<_>>(),
            [
                ".agents/skills/trama-req-ui/SKILL.md",
                ".agents/skills/trama-req-ui/main.md",
            ]
        );
    }

    #[test]
    fn fragment_mode_writes_one_skill_per_fragment_with_the_area_title_prose_and_non_empty_sections(
    ) {
        let areas = areas(&[(
            "ui",
            "Applies to the whole Area.",
            &[(
                "main",
                "Use when building screens.",
                &[
                    ("General", "- Material 3."),
                    ("order/Paid", "   "),
                    ("checkout", "- Total at the bottom."),
                ],
            )],
        )]);
        let made = generate(&areas);
        assert_eq!(made.mode, Mode::Fragment);
        assert_eq!(made.areas, ["ui"]);
        assert_eq!(made.files.len(), 1);
        assert_eq!(
            made.files[0].content,
            "---\nname: trama-req-ui-main\ndescription: \"Use when building screens.\"\n---\n\n# ui\n\nApplies to the whole Area.\n\n## General\n\n- Material 3.\n\n## checkout\n\n- Total at the bottom.\n"
        );
    }

    #[test]
    fn index_mode_lists_each_fragment_and_keeps_its_content_in_a_file() {
        let long = "Use when working on this part of the project and its many screens and flows.";
        let many: Vec<(String, &str)> = (0..60).map(|n| (format!("frag-{n:02}"), long)).collect();
        let fragments: Vec<FragmentSpec> = many
            .iter()
            .map(|(n, d)| (n.as_str(), *d, &[("General", "- g")][..]))
            .collect();
        let mut with_section = fragments.clone();
        with_section[0] = (
            "frag-00",
            long,
            &[("General", "- general text"), ("checkout", "- c")][..],
        );
        with_section[1] = ("frag-01", long, &[("order/Paid", "- p")][..]);
        let areas = areas(&[
            ("ui", "Prose.", &with_section[..2]),
            ("tech", "", &fragments[2..]),
        ]);
        // `General` appears once per Area: the generated spec above repeats it in `tech`, which is a
        // different Area, so it is accepted.
        assert!(budget_of(&areas) > BUDGET);
        let made = generate(&areas);
        assert_eq!(made.mode, Mode::Index);
        let skill = made
            .files
            .iter()
            .find(|f| f.path.ends_with("trama-req-ui/SKILL.md"))
            .unwrap();
        assert!(skill.content.starts_with("---\nname: trama-req-ui\ndescription: \"Use for ui.\"\n---\n\nProse.\n\n## General\n\n- general text\n\n## Fragment\n"), "{}", skill.content);
        assert!(skill
            .content
            .contains("Sections: `General`, `checkout`\n\nFile: `frag-00.md`"));
        assert!(skill
            .content
            .contains("Sections: `order/Paid`\n\nFile: `frag-01.md`"));
        let file = made
            .files
            .iter()
            .find(|f| f.path.ends_with("trama-req-ui/frag-00.md"))
            .unwrap();
        assert_eq!(
            file.content,
            "# Fragment\n\n## General\n\n- general text\n\n## checkout\n\n- c\n"
        );
        assert_eq!(made.areas, ["tech", "ui"]);
    }

    #[test]
    fn an_area_without_prose_or_content_has_no_index_skill_and_nothing_else_is_generated() {
        let none = areas(&[("ui", "", &[("main", "d", &[("General", "")])])]);
        let made = generate(&none);
        assert!(made.files.is_empty() && made.areas.is_empty());
        assert_eq!(made.budget, 0);
    }

    #[test]
    fn a_fragment_called_skill_never_overwrites_the_skill_file() {
        assert_eq!(fragment_file_name("skill"), "skill-fragment.md");
        assert_eq!(fragment_file_name("main"), "main.md");
    }

    #[test]
    fn skill_names_follow_the_mode() {
        let areas = areas(&[
            ("ui", "Prose.", &[("main", "d", &[("General", "- x")])]),
            ("tech", "", &[]),
        ]);
        assert_eq!(
            skill_names(&areas, Mode::Fragment)
                .into_iter()
                .collect::<Vec<_>>(),
            ["trama-req-ui-main"]
        );
        assert_eq!(
            skill_names(&areas, Mode::Index)
                .into_iter()
                .collect::<Vec<_>>(),
            ["trama-req-ui"]
        );
    }
}
