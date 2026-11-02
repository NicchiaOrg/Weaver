//! `developer.INITIALISE_PROJECT` of the Machine `project`: the one function that turns a folder
//! with no `.weaver/` into a project, for the app (`Initialise`) and the command line (`weaver project new`).
//!
//! It creates `.weaver/` with an empty Model (the System named after the folder), the starting
//! Areas with `area.md` only, and every file installed for the agent. When agent files it would
//! install already exist with other content, nothing is created unless the overwrite is
//! confirmed (`force`); a failure leaves nothing behind.

use std::path::Path;

use crate::project::changes::{self, ChangeSet};
use crate::project::init::{initial_changes, STARTING_AREAS};
use crate::project::layout::weaver_dir;

use super::plan::{Differing, Plan};
use crate::agent_files;
use std::collections::BTreeSet;

/// Why a folder was not initialised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitialiseError {
    /// `.weaver/` exists: the folder is already a project.
    Exists,
    /// Agent files exist with other content and the overwrite was not confirmed: every one of
    /// them, relative to the project folder, sorted. Nothing was created.
    Conflicts(Vec<String>),
    /// Something could not be written; everything written before was undone.
    Failed(String),
}

/// What an initialisation created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Initialised {
    /// The files created in `.weaver/`, relative to it.
    pub files: Vec<String>,
    /// The ids of the starting Areas.
    pub areas: Vec<&'static str>,
    /// The agent files written, relative to the project folder.
    pub agent_files: Vec<String>,
    /// The `trama-req-*` files removed because the new project generates none of them.
    pub removed: Vec<String>,
}

/// The agent files a new project gets: the whole embedded list.
pub fn installed_paths() -> Vec<String> {
    agent_files::installed_files()
        .iter()
        .map(|f| f.path.clone())
        .collect()
}

/// The agent files that exist in `root` with content other than the one to install, sorted: what
/// initialising would ask to overwrite.
pub fn conflicts(root: &Path) -> Vec<String> {
    let mut plan = Plan::new(root);
    plan.write_missing_or_different(
        agent_files::installed_files()
            .iter()
            .map(|f| (f.path.as_str(), f.contents.as_str())),
        Differing::Conflict,
    )
}

/// Initialises the project at `root` (the project's lock is held by the caller). `force` confirms
/// overwriting the agent files that differ.
pub fn initialise(root: &Path, force: bool) -> Result<Initialised, InitialiseError> {
    if weaver_dir(root).exists() {
        return Err(InitialiseError::Exists);
    }
    let mut plan = Plan::new(root);
    let wanted = agent_files::installed_files()
        .iter()
        .map(|f| (f.path.as_str(), f.contents.as_str()));
    let differing = if force {
        Differing::Rewrite
    } else {
        Differing::Conflict
    };
    let conflicts = plan.write_missing_or_different(wanted, differing);
    if !conflicts.is_empty() {
        return Err(InitialiseError::Conflicts(conflicts));
    }
    // The new project has no fragment, so it generates no Requirement skill: leftovers of an
    // earlier project go, the same as at every other generation.
    plan.remove_other_requirement_skills(&BTreeSet::new());
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let model: ChangeSet = initial_changes(&name);
    plan.apply()
        .map_err(|e| InitialiseError::Failed(e.to_string()))?;
    if let Err(error) = changes::apply::<()>(root, &model, |_| Ok(())) {
        plan.roll_back();
        return Err(InitialiseError::Failed(error.to_string()));
    }
    Ok(Initialised {
        files: model.iter().map(|(path, _)| path.clone()).collect(),
        areas: STARTING_AREAS.iter().map(|(id, _, _)| *id).collect(),
        agent_files: plan.written(),
        removed: plan.removed(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::project::image::ProjectImage;

    fn snapshot(root: &Path) -> Vec<String> {
        fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
            let Ok(listing) = fs::read_dir(dir) else {
                return;
            };
            for item in listing {
                let path = item.unwrap().path();
                out.push(
                    path.strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
                if path.is_dir() {
                    walk(&path, base, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(root, root, &mut out);
        out.sort();
        out
    }

    #[test]
    fn a_folder_becomes_a_project_with_an_empty_model_the_starting_areas_and_every_agent_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("My shop");
        fs::create_dir_all(&root).unwrap();
        let done = initialise(&root, false).unwrap();
        assert_eq!(done.areas, ["ui", "tech", "data", "rules", "quality"]);
        assert_eq!(done.agent_files.len(), agent_files::installed_files().len());
        assert!(done.files.contains(&"model/system.md".to_owned()));
        for file in agent_files::installed_files() {
            assert_eq!(
                fs::read_to_string(root.join(&file.path)).unwrap(),
                file.contents,
                "{}",
                file.path
            );
        }
        let image = ProjectImage::load(&root);
        assert!(image.diagnostics().is_empty(), "{:?}", image.diagnostics());
        assert!(image.model().is_empty());
        assert_eq!(image.model().system.as_ref().unwrap().title, "My shop");
        assert_eq!(image.areas().len(), 5);
        // Each Area holds `area.md` and nothing else.
        for (id, _, _) in STARTING_AREAS {
            let names: Vec<_> = fs::read_dir(root.join(".weaver/requirements").join(id))
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            assert_eq!(names, ["area.md"]);
        }
        assert_eq!(initialise(&root, false), Err(InitialiseError::Exists));
    }

    #[test]
    fn differing_agent_files_are_all_listed_and_nothing_is_created_without_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        for file in [".agents/skills/trama-design/SKILL.md", ".omp/WATCHDOG.yml"] {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "mine").unwrap();
        }
        let before = snapshot(dir.path());
        assert_eq!(
            initialise(dir.path(), false),
            Err(InitialiseError::Conflicts(vec![
                ".agents/skills/trama-design/SKILL.md".into(),
                ".omp/WATCHDOG.yml".into()
            ]))
        );
        assert_eq!(
            conflicts(dir.path()),
            [".agents/skills/trama-design/SKILL.md", ".omp/WATCHDOG.yml"]
        );
        assert_eq!(snapshot(dir.path()), before, "nothing was created");
        assert_eq!(
            fs::read_to_string(dir.path().join(".omp/WATCHDOG.yml")).unwrap(),
            "mine"
        );

        initialise(dir.path(), true).unwrap();
        assert!(dir.path().join(".weaver/model/system.md").is_file());
        assert_ne!(
            fs::read_to_string(dir.path().join(".omp/WATCHDOG.yml")).unwrap(),
            "mine"
        );
        assert!(conflicts(dir.path()).is_empty());
    }

    #[test]
    fn a_failure_leaves_nothing_behind() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("keep.txt"), "x").unwrap();
        // The last file of the list cannot be written: a folder is in its place.
        let last = agent_files::installed_files().last().unwrap().path.clone();
        fs::create_dir_all(dir.path().join(&last)).unwrap();
        let before = snapshot(dir.path());
        let error = initialise(dir.path(), true).unwrap_err();
        assert!(
            matches!(&error, InitialiseError::Failed(why) if why.contains(&last)),
            "{error:?}"
        );
        // The folder we put there stays (it is the developer's); every file written before is undone.
        let mut expected = before.clone();
        expected.retain(|p| !p.starts_with(".weaver"));
        assert_eq!(snapshot(dir.path()), expected);
        assert!(!dir.path().join(".weaver").exists());
    }

    #[test]
    fn requirement_skills_left_by_an_earlier_project_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join(".agents/skills/trama-req-ui-old/SKILL.md");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, "old").unwrap();
        let done = initialise(dir.path(), false).unwrap();
        assert_eq!(done.removed, [".agents/skills/trama-req-ui-old/SKILL.md"]);
        assert!(!old.exists());
    }

    #[test]
    fn an_old_trama_folder_means_nothing_initialising_creates_weaver_and_leaves_it_alone() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join(".trama/model/system.md");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, "# Old shop\n").unwrap();
        assert!(!crate::project_files::has_weaver(dir.path()));
        let before = snapshot(dir.path());

        initialise(dir.path(), false).unwrap();

        assert!(dir.path().join(".weaver/model/system.md").is_file());
        assert_eq!(fs::read_to_string(&old).unwrap(), "# Old shop\n");
        let kept: Vec<String> = snapshot(dir.path())
            .into_iter()
            .filter(|p| p.starts_with(".trama"))
            .collect();
        let expected: Vec<String> = before
            .into_iter()
            .filter(|p| p.starts_with(".trama"))
            .collect();
        assert_eq!(kept, expected);
        // The project is the one in `.weaver`, whatever `.trama` holds.
        let image = ProjectImage::load(dir.path());
        assert!(image.diagnostics().is_empty(), "{:?}", image.diagnostics());
        assert!(image.model().system.is_some());
        assert!(image.model().is_empty());
    }

    #[test]
    fn a_folder_with_both_is_the_project_in_weaver() {
        let dir = tempfile::tempdir().unwrap();
        initialise(dir.path(), false).unwrap();
        let old = dir.path().join(".trama/model/system.md");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, "not even a title\n").unwrap();
        assert!(crate::project_files::has_weaver(dir.path()));
        assert_eq!(initialise(dir.path(), false), Err(InitialiseError::Exists));
        let image = ProjectImage::load(dir.path());
        assert!(image.diagnostics().is_empty(), "{:?}", image.diagnostics());
        assert_eq!(fs::read_to_string(&old).unwrap(), "not even a title\n");
    }
}
