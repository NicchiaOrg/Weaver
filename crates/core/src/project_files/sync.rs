//! Keeping the files Weaver installs for the agent equal to the ones this version carries, and
//! the Requirement skills equal to the Requirements (`requirements/rules#project`).
//!
//! One function does it all, for every moment the rules name: [`sync`]. The caller holds the
//! project's write lock (the app's queue, or the command line's own lock).
//!
//! | Moment | Call |
//! |--------|------|
//! | approval of a Proposal that changed Requirements | [`regenerate_requirement_skills`] |
//! | 1 s after the last change under `.weaver/requirements/` | [`regenerate_requirement_skills`] |
//! | opening check, `Update skills` | [`sync`] with [`SyncOptions::everything`] / [`SyncOptions::update_skills`] |
//! | initialisation | [`super::initialise`] |

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

use crate::agent_files;
use crate::project::changes::{self, ChangeSet};
use crate::project::image::ProjectImage;
use crate::project::init;
use crate::project::layout::{weaver_dir, REQUIREMENTS_DIR, WEAVER_DIR};
use crate::project::requirements::{Area, AREA_FILE};

use super::plan::{Action, Change, Differing, Plan};
use super::requirement_skills::{self as skills, Mode, BUDGET};

/// The Requirement skills were generated in another mode than the one they were in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModeSwitch {
    pub from: Mode,
    pub to: Mode,
}

/// The generation of the Requirement skills, as the app and the command line report it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Generated {
    pub mode: Mode,
    /// The characters the names and descriptions of the fragment skills use.
    #[ts(type = "number")]
    pub budget: usize,
    /// The budget of fragment mode.
    #[ts(type = "number")]
    pub limit: usize,
    /// Set when the mode changed at this generation.
    pub switched: Option<ModeSwitch>,
}

/// The one structured result of a regeneration and of `Update skills`: the files written and
/// removed, the mode, the budget and the switch (`requirements/rules#project`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "SkillsReport")]
pub struct SyncReport {
    /// Every file created, rewritten or removed, by path relative to the project root, sorted.
    pub changes: Vec<Change>,
    /// The Areas that have a generated Requirement skill.
    pub areas: Vec<String>,
    pub generated: Generated,
}

impl SyncReport {
    /// Nothing changed: the app says nothing then.
    pub fn is_unchanged(&self) -> bool {
        self.changes.is_empty()
    }

    /// The paths of the files written or removed.
    pub fn changed_paths(&self) -> Vec<String> {
        self.changes.iter().map(|c| c.path.clone()).collect()
    }

    /// The line the reports print for the mode: `fragment mode, 1,234 of 4,000 characters`.
    pub fn mode_line(&self) -> String {
        format!(
            "{} mode, {} of {} characters",
            self.generated.mode.as_str(),
            thousands(self.generated.budget),
            thousands(self.generated.limit)
        )
    }
}

/// `1234` as `1,234`.
pub fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// What [`sync`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncOptions {
    /// Compare and write the files of the embedded list (skills, sub-agent definitions, advisor
    /// instructions). Without it only the Requirement skills are looked at.
    pub agent_files: bool,
    /// Create the starting Areas when the Requirements are missing (`Update skills`).
    pub starting_areas: bool,
}

impl SyncOptions {
    /// The opening check: every installed file.
    pub fn everything() -> Self {
        Self {
            agent_files: true,
            starting_areas: false,
        }
    }

    /// `Update skills`: every installed file, and the starting Areas when none exist.
    pub fn update_skills() -> Self {
        Self {
            agent_files: true,
            starting_areas: true,
        }
    }

    /// An approval or a change of the Requirements: the Requirement skills only.
    pub fn requirement_skills() -> Self {
        Self {
            agent_files: false,
            starting_areas: false,
        }
    }
}

/// Why files could not be written; the message names the file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct SyncError(pub String);

/// The Requirements are missing when `.weaver/requirements/` does not exist or holds nothing.
fn requirements_missing(root: &Path) -> bool {
    match fs::read_dir(weaver_dir(root).join(REQUIREMENTS_DIR)) {
        Ok(mut listing) => listing.next().is_none(),
        Err(_) => true,
    }
}

/// The names of the `trama-req-*` folders in the skills folder of the project.
fn existing_skill_names(root: &Path) -> Vec<String> {
    let Ok(listing) = fs::read_dir(root.join(agent_files::SKILLS_DIR)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = listing
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with(skills::SKILL_PREFIX))
        .collect();
    names.sort();
    names
}

/// The mode the existing `trama-req-*` skills are in, when their names say one: a skill only one
/// of the two modes would name for these Areas.
fn previous_mode(existing: &[String], areas: &[Area]) -> Option<Mode> {
    let fragment_names = skills::skill_names(areas, Mode::Fragment);
    let index_names = skills::skill_names(areas, Mode::Index);
    let fragment = existing
        .iter()
        .any(|n| fragment_names.contains(n) && !index_names.contains(n));
    let index = existing
        .iter()
        .any(|n| index_names.contains(n) && !fragment_names.contains(n));
    match (fragment, index) {
        (true, false) => Some(Mode::Fragment),
        (false, true) => Some(Mode::Index),
        _ => None,
    }
}

/// What a mode change the Requirements would cause says, for `weaver proposal move <id> open`:
/// `None` when both are in the same mode.
pub fn mode_switch_warning(before: &[Area], after: &[Area]) -> Option<String> {
    let (was, will_be) = (skills::mode_of(before), skills::mode_of(after));
    if was == will_be {
        return None;
    }
    Some(format!(
        "Approving it switches the Requirement skills from {} mode to {} mode: the names and descriptions of the fragments would use {} of {} characters.",
        was.as_str(),
        will_be.as_str(),
        thousands(skills::budget_of(after)),
        thousands(BUDGET)
    ))
}

/// Compares the installed files and the Requirement skills of the project at `root` with what
/// they must be, and writes (atomically, all or nothing) what is missing or differs. Removes the
/// `trama-req-*` skills the current mode does not produce. Nothing else is written or removed.
pub fn sync(root: &Path, options: SyncOptions) -> Result<SyncReport, SyncError> {
    let mut image = ProjectImage::load(root);
    let mut starting: Vec<Change> = Vec::new();
    if options.starting_areas && requirements_missing(root) {
        let mut changes = ChangeSet::new();
        for (id, title, description) in init::STARTING_AREAS {
            changes.write(
                &format!("{REQUIREMENTS_DIR}/{id}/{AREA_FILE}.md"),
                init::area_text(title, description),
            );
        }
        image = changes::apply::<()>(root, &changes, |_| Ok(()))
            .map_err(|e| SyncError(format!("cannot create the starting Areas: {e}")))?;
        starting = changes
            .iter()
            .map(|(rel, _)| Change {
                path: format!("{WEAVER_DIR}/{rel}"),
                action: Action::Created,
            })
            .collect();
    }
    let mut report = sync_files(root, image.areas(), options)?;
    report.changes.extend(starting);
    report.changes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(report)
}

/// [`sync`] for the Requirement skills only, from the Requirements the project holds.
pub fn regenerate_requirement_skills(root: &Path) -> Result<SyncReport, SyncError> {
    sync(root, SyncOptions::requirement_skills())
}

/// [`sync`] from a picture the caller already has (the one a decision just produced).
pub fn sync_image(
    root: &Path,
    image: &ProjectImage,
    options: SyncOptions,
) -> Result<SyncReport, SyncError> {
    sync_files(root, image.areas(), options)
}

fn sync_files(root: &Path, areas: &[Area], options: SyncOptions) -> Result<SyncReport, SyncError> {
    let generation = skills::generate(areas);
    let existing = existing_skill_names(root);
    let previous = previous_mode(&existing, areas);

    let mut plan = Plan::new(root);
    let mut kept: BTreeSet<String> = BTreeSet::new();
    let mut wanted: Vec<(&str, &str)> = Vec::new();
    if options.agent_files {
        wanted.extend(
            agent_files::installed_files()
                .iter()
                .map(|f| (f.path.as_str(), f.contents.as_str())),
        );
    }
    for file in &generation.files {
        kept.insert(file.path.clone());
        wanted.push((file.path.as_str(), file.content.as_str()));
    }
    plan.write_missing_or_different(wanted, Differing::Rewrite);
    plan.remove_other_requirement_skills(&kept);
    plan.apply().map_err(|e| SyncError(e.to_string()))?;

    let switched = previous
        .filter(|was| *was != generation.mode)
        .map(|was| ModeSwitch {
            from: was,
            to: generation.mode,
        });
    Ok(SyncReport {
        changes: plan.changes(),
        areas: generation.areas,
        generated: Generated {
            mode: generation.mode,
            budget: generation.budget,
            limit: BUDGET,
            switched,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::init::initial_changes;

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        changes::apply::<()>(dir.path(), &initial_changes("shop"), |_| Ok(())).unwrap();
        dir
    }

    fn put(root: &Path, rel: &str, text: &str) {
        let path = root.join(".weaver").join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn fragment(description: &str) -> String {
        format!("---\ndescription: {description}\n---\n# Main\n\n## General\n- A requirement.\n")
    }

    #[test]
    fn the_first_sync_creates_every_installed_file_and_the_next_one_changes_nothing() {
        let dir = project();
        let first = sync(dir.path(), SyncOptions::everything()).unwrap();
        assert_eq!(first.changes.len(), agent_files::installed_files().len());
        assert!(first.changes.iter().all(|c| c.action == Action::Created));
        assert_eq!(first.generated.mode, Mode::Fragment);
        assert_eq!(first.generated.switched, None);
        assert!(first.areas.is_empty());
        let second = sync(dir.path(), SyncOptions::everything()).unwrap();
        assert!(second.is_unchanged(), "{:?}", second.changes);
    }

    #[test]
    fn a_file_changed_by_hand_is_written_again_and_a_crlf_copy_is_left_alone() {
        let dir = project();
        sync(dir.path(), SyncOptions::everything()).unwrap();
        let watchdog = dir.path().join(".omp/WATCHDOG.yml");
        let setup = dir.path().join(".agents/skills/trama-setup/SKILL.md");
        fs::write(&watchdog, "edited by hand").unwrap();
        let crlf = fs::read_to_string(&setup).unwrap().replace('\n', "\r\n");
        fs::write(&setup, &crlf).unwrap();
        let report = sync(dir.path(), SyncOptions::everything()).unwrap();
        assert_eq!(
            report.changes,
            [Change {
                path: ".omp/WATCHDOG.yml".into(),
                action: Action::Rewritten
            }]
        );
        assert_eq!(
            fs::read_to_string(&watchdog).unwrap(),
            agent_files::installed_file(".omp/WATCHDOG.yml")
                .unwrap()
                .contents
        );
        assert_eq!(fs::read_to_string(&setup).unwrap(), crlf);
    }

    #[test]
    fn a_file_that_is_not_in_the_list_is_never_touched() {
        let dir = project();
        sync(dir.path(), SyncOptions::everything()).unwrap();
        let mine = dir.path().join(".agents/skills/my-skill/SKILL.md");
        fs::create_dir_all(mine.parent().unwrap()).unwrap();
        fs::write(&mine, "mine").unwrap();
        let extra = dir.path().join(".omp/agents/other.md");
        fs::write(&extra, "mine").unwrap();
        assert!(sync(dir.path(), SyncOptions::everything())
            .unwrap()
            .is_unchanged());
        assert_eq!(fs::read_to_string(mine).unwrap(), "mine");
        assert_eq!(fs::read_to_string(extra).unwrap(), "mine");
    }

    #[test]
    fn the_requirement_skills_follow_the_requirements_and_the_removed_ones_go() {
        let dir = project();
        put(
            dir.path(),
            "requirements/ui/main.md",
            &fragment("Use when building screens."),
        );
        let made = regenerate_requirement_skills(dir.path()).unwrap();
        assert_eq!(
            made.changes,
            [Change {
                path: ".agents/skills/trama-req-ui-main/SKILL.md".into(),
                action: Action::Created
            }]
        );
        assert_eq!(made.areas, ["ui"]);
        assert!(regenerate_requirement_skills(dir.path())
            .unwrap()
            .is_unchanged());
        // A regeneration looks at the Requirement skills only.
        assert!(!dir.path().join(".agents/skills/trama-setup").exists());

        put(
            dir.path(),
            "requirements/ui/main.md",
            &fragment("Use when changing screens."),
        );
        let rewritten = regenerate_requirement_skills(dir.path()).unwrap();
        assert_eq!(rewritten.changes[0].action, Action::Rewritten);

        fs::remove_file(dir.path().join(".weaver/requirements/ui/main.md")).unwrap();
        let removed = regenerate_requirement_skills(dir.path()).unwrap();
        assert_eq!(
            removed.changes,
            [Change {
                path: ".agents/skills/trama-req-ui-main/SKILL.md".into(),
                action: Action::Removed
            }]
        );
        assert!(!dir.path().join(".agents/skills/trama-req-ui-main").exists());
        assert!(removed.areas.is_empty());
    }

    #[test]
    fn a_change_of_mode_removes_the_other_modes_skills_and_reports_the_switch() {
        let dir = project();
        put(
            dir.path(),
            "requirements/ui/main.md",
            &fragment("Use when building screens."),
        );
        regenerate_requirement_skills(dir.path()).unwrap();

        let long = "Use when working on screens, flows, dialogs, menus, toolbars, lists, forms and every other thing a user sees.";
        for n in 0..40 {
            put(
                dir.path(),
                &format!("requirements/tech/frag-{n:02}.md"),
                &format!("---\ndescription: {long}\n---\n# F\n\n## order{n}\n- x\n"),
            );
        }
        // The sections name no element: that is for `model check`, not for the skills.
        let to_index = regenerate_requirement_skills(dir.path()).unwrap();
        assert_eq!(to_index.generated.mode, Mode::Index);
        assert!(to_index.generated.budget > BUDGET);
        assert_eq!(
            to_index.generated.switched,
            Some(ModeSwitch {
                from: Mode::Fragment,
                to: Mode::Index
            })
        );
        assert!(to_index
            .changes
            .iter()
            .any(|c| c.path == ".agents/skills/trama-req-ui-main/SKILL.md"
                && c.action == Action::Removed));
        assert!(dir
            .path()
            .join(".agents/skills/trama-req-ui/SKILL.md")
            .is_file());
        assert!(dir
            .path()
            .join(".agents/skills/trama-req-tech/frag-07.md")
            .is_file());
        assert!(!dir.path().join(".agents/skills/trama-req-ui-main").exists());
        let again = regenerate_requirement_skills(dir.path()).unwrap();
        assert!(again.is_unchanged() && again.generated.switched.is_none());

        for n in 3..40 {
            fs::remove_file(
                dir.path()
                    .join(format!(".weaver/requirements/tech/frag-{n:02}.md")),
            )
            .unwrap();
        }
        let back = regenerate_requirement_skills(dir.path()).unwrap();
        assert_eq!(back.generated.mode, Mode::Fragment);
        assert_eq!(
            back.generated.switched,
            Some(ModeSwitch {
                from: Mode::Index,
                to: Mode::Fragment
            })
        );
        assert!(!dir.path().join(".agents/skills/trama-req-ui").exists());
        assert!(dir
            .path()
            .join(".agents/skills/trama-req-ui-main/SKILL.md")
            .is_file());
        assert!(dir
            .path()
            .join(".agents/skills/trama-req-tech-frag-02/SKILL.md")
            .is_file());
    }

    #[test]
    fn update_skills_creates_the_starting_areas_only_when_the_requirements_are_missing() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".weaver/model")).unwrap();
        fs::write(
            dir.path().join(".weaver/model/system.md"),
            "# Shop\n\nA shop.\n",
        )
        .unwrap();
        let report = sync(dir.path(), SyncOptions::update_skills()).unwrap();
        let created: Vec<&str> = report
            .changes
            .iter()
            .filter(|c| c.path.starts_with(".weaver/"))
            .map(|c| c.path.as_str())
            .collect();
        assert_eq!(created.len(), 5);
        assert!(created.contains(&".weaver/requirements/ui/area.md"));
        assert!(dir
            .path()
            .join(".weaver/requirements/quality/area.md")
            .is_file());
        // Present now: nothing is created again, even if the developer deletes one Area.
        fs::remove_dir_all(dir.path().join(".weaver/requirements/ui")).unwrap();
        let again = sync(dir.path(), SyncOptions::update_skills()).unwrap();
        assert!(again.is_unchanged());
        assert!(!dir.path().join(".weaver/requirements/ui").exists());
    }

    #[test]
    fn a_failure_writes_nothing_and_says_which_file() {
        let dir = project();
        // A folder where one installed file must be: writing it fails after the skills were written.
        fs::create_dir_all(dir.path().join(".omp/WATCHDOG.yml")).unwrap();
        let error = sync(dir.path(), SyncOptions::everything()).unwrap_err();
        assert!(
            error.0.contains("cannot write .omp/WATCHDOG.yml"),
            "{error}"
        );
        assert!(
            !dir.path().join(".agents").exists(),
            "what was written before the failure is undone"
        );
        assert!(dir.path().join(".omp/WATCHDOG.yml").is_dir());
    }

    #[test]
    fn the_mode_switch_warning_names_both_modes_and_the_budget() {
        let dir = project();
        let before = ProjectImage::load(dir.path());
        assert_eq!(mode_switch_warning(before.areas(), before.areas()), None);
        let long = "Use when working on screens, flows, dialogs, menus, toolbars, lists, forms and every other thing a user sees.";
        let mut changes = ChangeSet::new();
        for n in 0..40 {
            changes.write(
                &format!("requirements/tech/frag-{n:02}.md"),
                format!("---\ndescription: {long}\n---\n# F\n\n## order{n}\n- x\n"),
            );
        }
        let after = before.with_changes(&changes);
        let warning = mode_switch_warning(before.areas(), after.areas()).unwrap();
        assert!(
            warning.starts_with(
                "Approving it switches the Requirement skills from fragment mode to index mode"
            ),
            "{warning}"
        );
        assert!(warning.contains("of 4,000 characters"));
        assert!(mode_switch_warning(after.areas(), before.areas())
            .unwrap()
            .contains("from index mode to fragment mode"));
        assert_eq!(thousands(4001), "4,001");
        assert_eq!(thousands(40), "40");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }
}
