//! `developer.INITIALISE_PROJECT` of the Machine `project`: the one function that turns a folder
//! with no `.weaver/` into a project, for the app (`Initialise`) and the command line (`weaver project new`).
//!
//! It creates `.weaver/` with an empty Model (the System named after the folder), the starting
//! Areas with `area.md` only, and every file installed for the agent. When agent files it would
//! install already exist with other content, nothing is created unless the overwrite is
//! confirmed (`force`); a failure leaves nothing behind. With a mode of Solo mode chosen, the
//! block of its exclusions is written first, before any other file, and removed again by a later
//! failure; a folder that cannot take it is refused with the reason and nothing is created.

use std::path::Path;

use crate::project::changes::{self, ChangeSet};
use crate::project::init::{initial_changes, STARTING_AREAS};
use crate::project::layout::{weaver_dir, SOLO_DIR};
use crate::solo_mode::{self, Git, Mode};

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
    /// Solo mode was chosen and cannot be turned on (no git, not in a repository, the exclusions
    /// cannot be written): the reason. Nothing was created.
    Solo(String),
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
    /// The `weaver-req-*` files removed because the new project generates none of them.
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
/// overwriting the agent files that differ. With a `solo` mode that is on, the block of Solo mode
/// is written first, before any agent file or `.weaver/` (`git` is called only then), and
/// `solo/solo.json` goes with the rest of `.weaver/`; a later failure removes the block again.
pub fn initialise(
    root: &Path,
    force: bool,
    solo: Mode,
    git: &Git,
) -> Result<Initialised, InitialiseError> {
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
    let block = if solo.is_on() {
        Some(
            solo_mode::place_block(git, root, solo)
                .map_err(|reason| InitialiseError::Solo(reason.to_string()))?,
        )
    } else {
        None
    };
    let remove_block = || {
        if let Some(block) = &block {
            let _ = block.remove_block();
        }
    };
    // The new project has no fragment, so it generates no Requirement skill: leftovers of an
    // earlier project go, the same as at every other generation.
    plan.remove_other_requirement_skills(&BTreeSet::new());
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut model: ChangeSet = initial_changes(&name);
    if solo.is_on() {
        model.write(&format!("{SOLO_DIR}/.gitignore"), "*".to_owned());
        model.write(
            &format!("{SOLO_DIR}/{}", solo_mode::STATE_FILE),
            solo_mode::state_text(solo),
        );
    }
    if let Err(error) = plan.apply() {
        remove_block();
        return Err(InitialiseError::Failed(error.to_string()));
    }
    if let Err(error) = changes::apply::<()>(root, &model, |_| Ok(())) {
        plan.roll_back();
        remove_block();
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

    /// Initialises with Solo mode off: git is not involved.
    fn init(root: &Path, force: bool) -> Result<Initialised, InitialiseError> {
        initialise(root, force, Mode::Off, &Git::system())
    }

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
        let done = init(&root, false).unwrap();
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
        // The assets folder is made when the first asset is copied in, never by initialising.
        assert!(!crate::assets::assets_dir(&root).exists());
        assert_eq!(init(&root, false), Err(InitialiseError::Exists));
    }

    #[test]
    fn differing_agent_files_are_all_listed_and_nothing_is_created_without_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        for file in [".agents/skills/weaver-design/SKILL.md", ".omp/WATCHDOG.yml"] {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "mine").unwrap();
        }
        let before = snapshot(dir.path());
        assert_eq!(
            init(dir.path(), false),
            Err(InitialiseError::Conflicts(vec![
                ".agents/skills/weaver-design/SKILL.md".into(),
                ".omp/WATCHDOG.yml".into()
            ]))
        );
        assert_eq!(
            conflicts(dir.path()),
            [".agents/skills/weaver-design/SKILL.md", ".omp/WATCHDOG.yml"]
        );
        assert_eq!(snapshot(dir.path()), before, "nothing was created");
        assert_eq!(
            fs::read_to_string(dir.path().join(".omp/WATCHDOG.yml")).unwrap(),
            "mine"
        );

        init(dir.path(), true).unwrap();
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
        let error = init(dir.path(), true).unwrap_err();
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
        let old = dir.path().join(".agents/skills/weaver-req-ui-old/SKILL.md");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, "old").unwrap();
        let done = init(dir.path(), false).unwrap();
        assert_eq!(done.removed, [".agents/skills/weaver-req-ui-old/SKILL.md"]);
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

        init(dir.path(), false).unwrap();

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
        init(dir.path(), false).unwrap();
        let old = dir.path().join(".trama/model/system.md");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, "not even a title\n").unwrap();
        assert!(crate::project_files::has_weaver(dir.path()));
        assert_eq!(init(dir.path(), false), Err(InitialiseError::Exists));
        let image = ProjectImage::load(dir.path());
        assert!(image.diagnostics().is_empty(), "{:?}", image.diagnostics());
        assert_eq!(fs::read_to_string(&old).unwrap(), "not even a title\n");
    }

    // --- Solo mode ---------------------------------------------------------------------------

    use crate::solo_mode::testing::{git, ignored, Sandbox};
    use crate::solo_mode::{locate, read_state};

    fn exclude(repo: &Path) -> std::path::PathBuf {
        repo.join(".git/info/exclude")
    }

    #[test]
    fn with_a_mode_the_block_and_the_state_come_with_the_project_for_each_mode() {
        for (mode, text) in [
            (Mode::Folders, "{\"mode\": \"folders\"}\n"),
            (Mode::Files, "{\"mode\": \"files\"}\n"),
        ] {
            let sandbox = Sandbox::new();
            let repo = sandbox.repo("r");
            let done = initialise(&repo, false, mode, &Git::system()).unwrap();
            assert!(done.files.contains(&"solo/solo.json".to_owned()));
            assert_eq!(
                fs::read_to_string(repo.join(".weaver/solo/solo.json")).unwrap(),
                text
            );
            assert_eq!(
                fs::read_to_string(repo.join(".weaver/solo/.gitignore")).unwrap(),
                "*\n"
            );
            assert_eq!(read_state(&repo).mode, mode);
            let location = locate(&Git::system(), &repo).unwrap();
            assert!(location.block_in_place(mode).unwrap());
            assert!(ignored(&repo, ".weaver/model/system.md"));
            assert!(ignored(&repo, ".omp/WATCHDOG.yml"));
            // The Model is intact: Solo mode's files are not part of it.
            let image = ProjectImage::load(&repo);
            assert!(image.diagnostics().is_empty(), "{:?}", image.diagnostics());
            // Git sees only the code: nothing of Weaver is untracked-and-shown.
            let status = git(&repo, &["status", "--porcelain"]);
            assert!(status.trim().is_empty(), "{status}");
        }
    }

    #[test]
    fn with_a_mode_chosen_a_folder_outside_git_is_refused_and_nothing_is_created() {
        let sandbox = Sandbox::new();
        let plain = sandbox.folder("plain");
        fs::write(plain.join("keep.txt"), "x").unwrap();
        let before = snapshot(&plain);
        let error = initialise(&plain, false, Mode::Folders, &Git::system()).unwrap_err();
        assert_eq!(
            error,
            InitialiseError::Solo("this folder is not in a git repository".to_owned())
        );
        assert_eq!(snapshot(&plain), before);

        // Without git.
        let repo = sandbox.repo("r");
        let before = snapshot(&repo);
        let exclude_before = fs::read(exclude(&repo)).unwrap();
        let no_git = Git::system().with_search_path("");
        assert_eq!(
            initialise(&repo, false, Mode::Files, &no_git),
            Err(InitialiseError::Solo("git was not found".to_owned()))
        );
        assert_eq!(snapshot(&repo), before);
        assert_eq!(fs::read(exclude(&repo)).unwrap(), exclude_before);
    }

    #[test]
    fn the_block_is_written_before_any_other_file_so_an_unwritable_one_creates_nothing() {
        let sandbox = Sandbox::new();
        let repo = sandbox.repo("r");
        fs::remove_file(exclude(&repo)).unwrap();
        fs::create_dir(exclude(&repo)).unwrap();
        let before = snapshot(&repo);
        let error = initialise(&repo, false, Mode::Folders, &Git::system()).unwrap_err();
        assert!(
            matches!(&error, InitialiseError::Solo(why) if why.starts_with("could not write ")),
            "{error:?}"
        );
        assert_eq!(snapshot(&repo), before);
        assert!(!repo.join(".weaver").exists());
        assert!(!repo.join(".agents").exists());
    }

    #[test]
    fn a_failure_after_the_block_removes_it_again() {
        let sandbox = Sandbox::new();
        let repo = sandbox.repo("r");
        fs::write(exclude(&repo), "mine\n").unwrap();
        let last = agent_files::installed_files().last().unwrap().path.clone();
        fs::create_dir_all(repo.join(&last)).unwrap();
        let error = initialise(&repo, true, Mode::Files, &Git::system()).unwrap_err();
        assert!(matches!(error, InitialiseError::Failed(_)), "{error:?}");
        assert_eq!(fs::read_to_string(exclude(&repo)).unwrap(), "mine\n");
        assert!(!repo.join(".weaver").exists());
    }

    #[test]
    fn with_off_git_is_not_called() {
        let sandbox = Sandbox::new();
        let plain = sandbox.folder("plain");
        let no_git = Git::system().with_search_path("");
        initialise(&plain, false, Mode::Off, &no_git).unwrap();
        assert!(plain.join(".weaver/model/system.md").is_file());
        assert!(!plain.join(".weaver/solo").exists());
        assert_eq!(read_state(&plain).mode, Mode::Off);
    }
}
