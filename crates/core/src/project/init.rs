//! What `weaver project new` writes in a folder without `.weaver/`: the System and the starting
//! Requirement Areas. The Model starts empty (no Feature, no Actor).
//!
//! The installation of the agent files and the whole initialisation are in
//! [`crate::project_files`].

use super::changes::ChangeSet;
use super::layout::{MODEL_DIR, REQUIREMENTS_DIR};
use super::requirements::AREA_FILE;

/// The starting Areas: id, title, description (`requirements/rules#project`).
pub const STARTING_AREAS: [(&str, &str, &str); 5] = [
    (
        "ui",
        "Interface",
        "Use when building or changing screens, layout, styling, components, navigation look or texts shown to users.",
    ),
    (
        "tech",
        "Technology and constraints",
        "Use when choosing or using languages, frameworks, libraries, platforms, project structure or build and deploy setup.",
    ),
    (
        "data",
        "Data",
        "Use when defining, storing, validating or showing the information the System keeps and what each piece contains.",
    ),
    (
        "rules",
        "Business rules",
        "Use when implementing calculations, limits, policies or other business rules behind the Model's conditions and consequences.",
    ),
    (
        "quality",
        "Quality",
        "Use when the work touches performance, security, privacy, accessibility, offline behaviour, localisation or reliability.",
    ),
];

/// The text of `model/system.md` of a new project: the folder name as display name and one line
/// that an agent replaces.
pub fn system_text(project_name: &str) -> String {
    let name: String = project_name
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let name = if name.is_empty() {
        "System"
    } else {
        name.as_str()
    };
    format!("# {name}\n\nDescribe what this System is and what it does.\n")
}

/// The text of the `area.md` of a starting Area.
pub fn area_text(title: &str, description: &str) -> String {
    // A JSON string is a valid double-quoted YAML scalar.
    let description = serde_json::to_string(description).expect("a string is serialisable");
    format!("---\ndescription: {description}\n---\n# {title}\n")
}

/// The files of a new project, by path relative to `.weaver/`.
pub fn initial_changes(project_name: &str) -> ChangeSet {
    let mut changes = ChangeSet::new();
    changes.write(&format!("{MODEL_DIR}/system.md"), system_text(project_name));
    for (id, title, description) in STARTING_AREAS {
        changes.write(
            &format!("{REQUIREMENTS_DIR}/{id}/{AREA_FILE}.md"),
            area_text(title, description),
        );
    }
    changes
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::project::image::ProjectImage;

    #[test]
    fn a_new_project_has_the_system_and_five_empty_areas_and_no_diagnostic() {
        let image = ProjectImage::empty(Path::new("/p")).with_changes(&initial_changes("My shop"));
        assert!(image.diagnostics().is_empty(), "{:?}", image.diagnostics());
        assert!(image.model().is_empty());
        assert_eq!(image.model().system.as_ref().unwrap().title, "My shop");
        let ids: Vec<_> = image.areas().iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["data", "quality", "rules", "tech", "ui"]);
        assert!(image.areas().iter().all(|a| a.fragments.is_empty()));
        let ui = image.areas().iter().find(|a| a.id == "ui").unwrap();
        assert_eq!(ui.title, "Interface");
        assert!(ui
            .description
            .starts_with("Use when building or changing screens"));
    }

    #[test]
    fn the_system_name_is_one_line_and_never_empty() {
        assert_eq!(
            system_text("  a\tb  "),
            "# a b\n\nDescribe what this System is and what it does.\n"
        );
        assert!(system_text("").starts_with("# System\n"));
    }
}
