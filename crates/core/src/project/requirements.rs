//! Requirements: Areas (`requirements/<area>/`) made of `area.md` and fragments.
//!
//! `area.md` holds the description, title and intro of the whole Area and no sections. Every
//! other `.md` of the folder is a fragment holding some of the Area's sections: `## General`
//! or `## <one element reference>`. A section appears once in the whole Area.

use std::collections::BTreeMap;

use serde::Serialize;

use super::diagnostic::Diagnostic;
use super::frontmatter;
use super::md;
use super::model::is_element_id;
use super::refs::{parse_element_ref, GENERAL_SECTION};

/// A fragment larger than this many bytes (front matter included) earns a warning.
pub const FRAGMENT_WARN_BYTES: usize = 12_288;

/// The file name (without `.md`) reserved for the Area's own file.
pub const AREA_FILE: &str = "area";

/// `area.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AreaDoc {
    pub description: String,
    pub title: String,
    pub intro: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    /// `General`, or one element reference.
    pub title: String,
    /// What follows the heading, as written.
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fragment {
    pub name: String,
    pub description: String,
    pub title: String,
    pub sections: Vec<Section>,
    /// Bytes of the normalized file, front matter included.
    pub size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Area {
    pub id: String,
    pub description: String,
    pub title: String,
    pub intro: String,
    pub fragments: Vec<Fragment>,
}

impl Area {
    pub fn has_section(&self, title: &str) -> bool {
        self.fragments
            .iter()
            .flat_map(|f| &f.sections)
            .any(|s| s.title == title)
    }

    /// Every section with the fragment that holds it.
    pub fn sections(&self) -> impl Iterator<Item = (&Fragment, &Section)> {
        self.fragments
            .iter()
            .flat_map(|f| f.sections.iter().map(move |s| (f, s)))
    }
}

fn required_description(front: &serde_yaml::Mapping, errors: &mut Vec<String>) -> String {
    match frontmatter::get_str(front, "description") {
        Some(d) if !d.trim().is_empty() => d.trim().to_owned(),
        Some(_) => {
            errors.push("front matter `description` is empty".to_owned());
            String::new()
        }
        None => {
            errors.push(
                "front matter needs a `description` saying when an agent needs it".to_owned(),
            );
            String::new()
        }
    }
}

/// Parses `area.md`: `description`, one H1, optional prose and no `##` section.
pub fn parse_area_doc(text: &str) -> Result<AreaDoc, Vec<String>> {
    let mut errors = Vec::new();
    let doc = match frontmatter::parse(text) {
        Ok(doc) => doc,
        Err(e) => return Err(vec![e]),
    };
    let description = required_description(&doc.front, &mut errors);
    let outline = md::outline(&doc.body);
    let h1s: Vec<_> = outline.headings.iter().filter(|h| h.level == 1).collect();
    if h1s.len() != 1 {
        errors.push(format!(
            "area.md needs exactly one H1 with the Area's title (found {})",
            h1s.len()
        ));
    }
    for h in outline.headings.iter().filter(|h| h.level == 2) {
        errors.push(format!(
            "area.md holds no sections: `## {}` belongs in a fragment",
            h.text
        ));
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let h1 = h1s[0];
    Ok(AreaDoc {
        description,
        title: h1.text.clone(),
        intro: doc.body[h1.range.end..].trim().to_owned(),
    })
}

/// Parses a fragment called `name` (the file name without `.md`).
pub fn parse_fragment(name: &str, text: &str) -> Result<Fragment, Vec<String>> {
    let mut errors = Vec::new();
    if name == AREA_FILE {
        errors.push("`area` is reserved for the Area's own file".to_owned());
    } else if !is_element_id(name) {
        errors.push(format!("fragment name `{name}` must be kebab-case"));
    }
    let doc = match frontmatter::parse(text) {
        Ok(doc) => doc,
        Err(e) => return Err(vec![e]),
    };
    let description = required_description(&doc.front, &mut errors);
    let body = &doc.body;
    let outline = md::outline(body);
    let h1s: Vec<_> = outline.headings.iter().filter(|h| h.level == 1).collect();
    if h1s.len() != 1 {
        errors.push(format!(
            "a fragment needs exactly one H1 with its title (found {})",
            h1s.len()
        ));
    }
    let h2s: Vec<usize> = outline
        .headings
        .iter()
        .enumerate()
        .filter(|(_, h)| h.level == 2)
        .map(|(i, _)| i)
        .collect();

    // Nothing but the H1 may come before the first section.
    let first_section_start = h2s
        .first()
        .map_or(body.len(), |i| outline.headings[*i].range.start);
    let h1_range = h1s.first().map(|h| h.range.clone());
    let excluded: Vec<_> = h1_range.into_iter().collect();
    if !md::text_excluding(body, 0, first_section_start, &excluded).is_empty() {
        errors.push(
            "there is prose before the first section: a fragment holds only sections".to_owned(),
        );
    }

    let mut sections: Vec<Section> = Vec::new();
    for (k, &i) in h2s.iter().enumerate() {
        let h = &outline.headings[i];
        if h.text != GENERAL_SECTION {
            if let Err(e) = parse_element_ref(&h.text) {
                errors.push(format!(
                    "section `## {}`: title must be `General` or one element reference ({e})",
                    h.text
                ));
            }
        }
        if sections.iter().any(|s| s.title == h.text) {
            errors.push(format!(
                "section `{}` appears twice in this fragment",
                h.text
            ));
        }
        let end = h2s
            .get(k + 1)
            .map_or(body.len(), |j| outline.headings[*j].range.start);
        sections.push(Section {
            title: h.text.clone(),
            body: body[h.range.end..end].trim().to_owned(),
        });
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Fragment {
        name: name.to_owned(),
        description,
        title: h1s[0].text.clone(),
        sections,
        size: text.len(),
    })
}

/// Joins the parsed `area.md` and fragment files into Areas, sorted by id (fragments by name).
///
/// Reports a folder with fragments and no `area.md`, and a section that appears more than
/// once in one Area (in the same or another fragment).
pub fn assemble_areas(
    docs: &[(&str, &str, &AreaDoc)],
    fragments: &[(&str, &str, &Fragment)],
) -> (Vec<Area>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut areas: BTreeMap<String, Area> = docs
        .iter()
        .map(|(_, id, doc)| {
            (
                (*id).to_owned(),
                Area {
                    id: (*id).to_owned(),
                    description: doc.description.clone(),
                    title: doc.title.clone(),
                    intro: doc.intro.clone(),
                    fragments: Vec::new(),
                },
            )
        })
        .collect();
    let mut seen_missing: BTreeMap<&str, ()> = BTreeMap::new();
    let mut section_homes: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut sorted: Vec<_> = fragments.to_vec();
    sorted.sort_by(|a, b| a.2.name.cmp(&b.2.name).then(a.0.cmp(b.0)));
    for (path, area_id, fragment) in sorted {
        let Some(area) = areas.get_mut(area_id) else {
            if seen_missing.insert(area_id, ()).is_none() {
                diagnostics.push(Diagnostic::error(
                    format!("requirements/{area_id}"),
                    format!("Area `{area_id}` has fragments but no `area.md`"),
                ));
            }
            continue;
        };
        for section in &fragment.sections {
            let key = (area_id.to_owned(), section.title.clone());
            if let Some(first) = section_homes.get(&key) {
                diagnostics.push(Diagnostic::error(
                    path,
                    format!("section `{}` is already in `{first}`: a section appears once in the whole Area", section.title),
                ));
            } else {
                section_homes.insert(key, path.to_owned());
            }
        }
        if fragment.size > FRAGMENT_WARN_BYTES {
            diagnostics.push(Diagnostic::warning(
                path,
                format!(
                    "the fragment is {} bytes, over {FRAGMENT_WARN_BYTES}: split it by Feature or group of Machines",
                    fragment.size
                ),
            ));
        }
        area.fragments.push(fragment.clone());
    }
    (areas.into_values().collect(), diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: &str = "---\ndescription: Use when building or changing screens, layout, styling or texts.\n---\n# Interface\n\nProse here applies to the whole Area (optional).\n";
    const FRAGMENT: &str = "---\ndescription: Use when building the checkout and order screens.\n---\n# Checkout and orders\n\n## General\n- Material 3, light and dark theme following the system.\n\n## checkout\n- Every checkout screen shows the cart total at the bottom.\n\n## order/Paid\n- A green \"Paid\" badge next to the order number.\n";

    fn fragment_errors(text: &str) -> String {
        parse_fragment("main", text)
            .expect_err("must fail")
            .join(" | ")
    }

    fn fragment_with(body: &str) -> String {
        format!("---\ndescription: d\n---\n# T\n\n{body}")
    }

    #[test]
    fn the_format_example_area_parses() {
        let doc = parse_area_doc(AREA).unwrap();
        assert_eq!(doc.title, "Interface");
        assert_eq!(
            doc.description,
            "Use when building or changing screens, layout, styling or texts."
        );
        assert_eq!(
            doc.intro,
            "Prose here applies to the whole Area (optional)."
        );
    }

    #[test]
    fn area_md_needs_a_description_one_h1_and_no_sections() {
        assert!(parse_area_doc("# Interface\n")
            .unwrap_err()
            .join("")
            .contains("`description`"));
        assert!(parse_area_doc("---\ndescription: d\n---\nprose\n")
            .unwrap_err()
            .join("")
            .contains("exactly one H1"));
        assert!(parse_area_doc("---\ndescription: d\n---\n# A\n# B\n")
            .unwrap_err()
            .join("")
            .contains("exactly one H1"));
        let e = parse_area_doc("---\ndescription: d\n---\n# A\n\n## General\n- x\n")
            .unwrap_err()
            .join("");
        assert!(e.contains("belongs in a fragment"), "{e}");
        assert!(parse_area_doc("---\ndescription: ''\n---\n# A\n")
            .unwrap_err()
            .join("")
            .contains("empty"));
        assert!(parse_area_doc("---\ndescription: d\n---\n# A\n")
            .unwrap()
            .intro
            .is_empty());
    }

    #[test]
    fn the_format_example_fragment_parses() {
        let f = parse_fragment("checkout", FRAGMENT).unwrap();
        assert_eq!(f.title, "Checkout and orders");
        let titles: Vec<_> = f.sections.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(titles, ["General", "checkout", "order/Paid"]);
        assert_eq!(
            f.sections[2].body,
            "- A green \"Paid\" badge next to the order number."
        );
        assert_eq!(f.size, FRAGMENT.len());
    }

    #[test]
    fn a_fragment_needs_a_description_and_one_h1() {
        assert!(fragment_errors("# T\n\n## General\n- x\n").contains("`description`"));
        assert!(
            fragment_errors("---\ndescription: d\n---\n\n## General\n- x\n")
                .contains("exactly one H1")
        );
        assert!(fragment_errors("---\ndescription: d\n---\n# A\n# B\n").contains("exactly one H1"));
    }

    #[test]
    fn a_fragment_name_is_kebab_case_and_area_is_reserved() {
        assert!(parse_fragment("area", FRAGMENT)
            .unwrap_err()
            .join("")
            .contains("reserved"));
        assert!(parse_fragment("Main", FRAGMENT)
            .unwrap_err()
            .join("")
            .contains("kebab-case"));
        assert!(parse_fragment("my_frag", FRAGMENT).is_err());
        assert!(parse_fragment("my-frag", FRAGMENT).is_ok());
    }

    #[test]
    fn no_prose_before_the_first_section() {
        let e = fragment_errors(&fragment_with("Some prose first.\n\n## General\n- x\n"));
        assert!(e.contains("prose before the first section"), "{e}");
        let e = fragment_errors(&fragment_with("### Stray\n\n## General\n- x\n"));
        assert!(e.contains("prose before the first section"), "{e}");
        let e = fragment_errors("---\ndescription: d\n---\nIntro\n# T\n\n## General\n- x\n");
        assert!(e.contains("prose before the first section"), "{e}");
        assert!(parse_fragment("main", &fragment_with("")).is_ok());
    }

    #[test]
    fn section_titles_are_general_or_exactly_one_element_reference() {
        for bad in ["Notes", "two refs: a, b", "order/", "Order"] {
            let e = fragment_errors(&fragment_with(&format!("## {bad}\n- x\n")));
            assert!(
                e.contains("title must be `General` or one element reference"),
                "{bad}: {e}"
            );
        }
        for good in [
            "General",
            "shop",
            "order",
            "user",
            "order/Paid",
            "order/Paid/user.CANCEL",
            "user.CANCEL",
        ] {
            assert!(
                parse_fragment("main", &fragment_with(&format!("## {good}\n- x\n"))).is_ok(),
                "{good}"
            );
        }
    }

    #[test]
    fn deeper_headings_and_code_inside_a_section_are_free_text() {
        let f = parse_fragment(
            "main",
            &fragment_with(
                "## order\n### Details\n- x\n\n```\n## not-a-section\n```\n\n## general-ish\n",
            ),
        );
        // `general-ish` is a valid element id, so it is a section too.
        let f = f.unwrap();
        assert_eq!(f.sections.len(), 2);
        assert!(f.sections[0].body.contains("### Details"));
        assert!(f.sections[0].body.contains("## not-a-section"));
    }

    #[test]
    fn a_section_appears_once_per_fragment() {
        let e = fragment_errors(&fragment_with("## order\n- x\n\n## order\n- y\n"));
        assert!(e.contains("appears twice"), "{e}");
    }

    #[test]
    fn a_section_appears_once_in_the_whole_area() {
        let area = parse_area_doc(AREA).unwrap();
        let a = parse_fragment("a-frag", &fragment_with("## order\n- x\n")).unwrap();
        let b = parse_fragment(
            "b-frag",
            &fragment_with("## order\n- y\n\n## checkout\n- z\n"),
        )
        .unwrap();
        let (areas, diags) = assemble_areas(
            &[("requirements/ui/area.md", "ui", &area)],
            &[
                ("requirements/ui/b-frag.md", "ui", &b),
                ("requirements/ui/a-frag.md", "ui", &a),
            ],
        );
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].path, "requirements/ui/b-frag.md");
        assert!(diags[0].message.contains("requirements/ui/a-frag.md"));
        assert_eq!(
            areas[0]
                .fragments
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            ["a-frag", "b-frag"]
        );
    }

    #[test]
    fn the_same_section_in_two_areas_is_fine() {
        let area = parse_area_doc(AREA).unwrap();
        let f = parse_fragment("main", &fragment_with("## order\n- x\n")).unwrap();
        let (_, diags) = assemble_areas(
            &[
                ("requirements/ui/area.md", "ui", &area),
                ("requirements/api/area.md", "api", &area),
            ],
            &[
                ("requirements/ui/main.md", "ui", &f),
                ("requirements/api/main.md", "api", &f),
            ],
        );
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn fragments_without_area_md_are_reported_once() {
        let f = parse_fragment("main", &fragment_with("## order\n- x\n")).unwrap();
        let g = parse_fragment("other", &fragment_with("## checkout\n- x\n")).unwrap();
        let (areas, diags) = assemble_areas(
            &[],
            &[
                ("requirements/ui/main.md", "ui", &f),
                ("requirements/ui/other.md", "ui", &g),
            ],
        );
        assert!(areas.is_empty());
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("no `area.md`"));
    }

    #[test]
    fn a_fragment_over_12_kib_earns_a_warning_not_an_error() {
        let area = parse_area_doc(AREA).unwrap();
        let big_body = format!(
            "## order\n{}\n",
            "- a long line of requirement text\n".repeat(400)
        );
        let text = fragment_with(&big_body);
        assert!(text.len() > FRAGMENT_WARN_BYTES);
        let f = parse_fragment("big", &text).unwrap();
        let (areas, diags) = assemble_areas(
            &[("requirements/ui/area.md", "ui", &area)],
            &[("requirements/ui/big.md", "ui", &f)],
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].severity,
            crate::project::diagnostic::Severity::Warning
        );
        assert_eq!(areas[0].fragments.len(), 1);
        let exact = fragment_with("");
        let pad = "x".repeat(FRAGMENT_WARN_BYTES - exact.len() - "## order\n".len() - 1);
        let at_limit = fragment_with(&format!("## order\n{pad}\n"));
        assert_eq!(at_limit.len(), FRAGMENT_WARN_BYTES);
        let f = parse_fragment("edge", &at_limit).unwrap();
        let (_, diags) = assemble_areas(
            &[("requirements/ui/area.md", "ui", &area)],
            &[("requirements/ui/edge.md", "ui", &f)],
        );
        assert!(diags.is_empty());
    }
}
