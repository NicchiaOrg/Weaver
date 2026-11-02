//! A set of files to write and remove in a project, applied whole or not at all.
//!
//! The plan keeps what each file held, so a failure half-way puts every file back and removes
//! the folders it created. Content is compared as text, with CRLF read as LF (`requirements/rules#project`).

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

use crate::atomic;
use crate::project::text;

use super::requirement_skills::SKILL_PREFIX;
use crate::agent_files::SKILLS_DIR;

/// What happened to one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename = "FileAction")]
pub enum Action {
    Created,
    Rewritten,
    Removed,
}

/// One file written or removed, by path relative to the project root (with `/`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "FileChange")]
pub struct Change {
    pub path: String,
    pub action: Action,
}

#[derive(Debug, Clone)]
enum Operation {
    Write(String),
    Remove,
}

#[derive(Debug, Clone)]
struct Step {
    rel: String,
    path: PathBuf,
    operation: Operation,
    /// What the file held; `None` when it did not exist.
    previous: Option<Vec<u8>>,
}

/// What to do with a file that exists with other content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Differing {
    /// Write it again.
    Rewrite,
    /// Refuse: it is a conflict.
    Conflict,
}

#[derive(Debug, Clone, Default)]
pub struct Plan {
    root: PathBuf,
    steps: Vec<Step>,
}

/// The content a file must hold once written.
fn wanted_text(content: &str) -> String {
    text::file_content(content)
}

fn read_previous(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

impl Plan {
    pub fn new(project_root: &Path) -> Self {
        Self {
            root: project_root.to_path_buf(),
            steps: Vec::new(),
        }
    }

    /// Adds the files of `wanted` (`(path relative to the root, content)`) that are missing or
    /// differ. Returns the paths that differ when `differing` is [`Differing::Conflict`]; a file
    /// that cannot be read counts as differing.
    pub fn write_missing_or_different<'a>(
        &mut self,
        wanted: impl IntoIterator<Item = (&'a str, &'a str)>,
        differing: Differing,
    ) -> Vec<String> {
        let mut conflicts = Vec::new();
        for (rel, content) in wanted {
            let path = self.root.join(rel);
            let content = wanted_text(content);
            let previous = match read_previous(&path) {
                Ok(previous) => previous,
                Err(_) => {
                    // Unreadable: it cannot be compared, so it differs; the write tells why it fails.
                    Some(Vec::new())
                }
            };
            if let Some(bytes) = &previous {
                if text::decode(bytes).is_ok_and(|current| current == content) {
                    continue;
                }
                if differing == Differing::Conflict {
                    conflicts.push(rel.to_owned());
                    continue;
                }
            }
            self.steps.push(Step {
                rel: rel.to_owned(),
                path,
                operation: Operation::Write(content),
                previous,
            });
        }
        conflicts.sort();
        conflicts
    }

    /// Adds the removal of every file under a `trama-req-*` skill folder whose path is not in
    /// `keep`. Nothing else is touched.
    pub fn remove_other_requirement_skills(&mut self, keep: &BTreeSet<String>) {
        let skills = self.root.join(SKILLS_DIR);
        let Ok(listing) = fs::read_dir(&skills) else {
            return;
        };
        let mut names: Vec<String> = listing
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.starts_with(SKILL_PREFIX))
            .collect();
        names.sort();
        for name in names {
            let mut files = Vec::new();
            collect_files(
                &skills.join(&name),
                &format!("{SKILLS_DIR}/{name}"),
                &mut files,
            );
            files.sort();
            for rel in files {
                if keep.contains(&rel) {
                    continue;
                }
                let path = self.root.join(&rel);
                let previous = read_previous(&path).ok().flatten().or(Some(Vec::new()));
                self.steps.push(Step {
                    rel,
                    path,
                    operation: Operation::Remove,
                    previous,
                });
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The files this plan writes (created or rewritten), by relative path.
    pub fn written(&self) -> Vec<String> {
        self.steps
            .iter()
            .filter(|s| matches!(s.operation, Operation::Write(_)))
            .map(|s| s.rel.clone())
            .collect()
    }

    /// The files this plan removes.
    pub fn removed(&self) -> Vec<String> {
        self.steps
            .iter()
            .filter(|s| matches!(s.operation, Operation::Remove))
            .map(|s| s.rel.clone())
            .collect()
    }

    /// What the plan does to each file, sorted by path.
    pub fn changes(&self) -> Vec<Change> {
        let mut changes: Vec<Change> = self
            .steps
            .iter()
            .map(|step| Change {
                path: step.rel.clone(),
                action: match (&step.operation, &step.previous) {
                    (Operation::Remove, _) => Action::Removed,
                    (Operation::Write(_), None) => Action::Created,
                    (Operation::Write(_), Some(_)) => Action::Rewritten,
                },
            })
            .collect();
        changes.sort_by(|a, b| a.path.cmp(&b.path));
        changes
    }

    /// Does every step, atomically each. When one fails, the steps already done are undone and the
    /// error says which file failed.
    pub fn apply(&self) -> io::Result<()> {
        for (done, step) in self.steps.iter().enumerate() {
            let result = match &step.operation {
                Operation::Write(content) => text::write_text(&step.path, content),
                Operation::Remove => self.remove(step),
            };
            if let Err(error) = result {
                self.undo(&self.steps[..=done]);
                return Err(io::Error::new(
                    error.kind(),
                    format!("cannot write {}: {error}", step.rel),
                ));
            }
        }
        Ok(())
    }

    /// Puts every file of the plan back as it was. Safe after a partial or a complete [`apply`](Self::apply).
    pub fn roll_back(&self) {
        self.undo(&self.steps);
    }

    fn undo(&self, steps: &[Step]) {
        for step in steps.iter().rev() {
            match &step.previous {
                Some(bytes) => {
                    let _ = atomic::write(&step.path, bytes, None);
                }
                None => {
                    let _ = fs::remove_file(&step.path);
                    self.remove_empty_folders_above(&step.path, &self.root);
                }
            }
        }
    }

    fn remove(&self, step: &Step) -> io::Result<()> {
        match fs::remove_file(&step.path) {
            Ok(()) => {}
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) => {}
            Err(e) => return Err(e),
        }
        self.remove_empty_folders_above(&step.path, &self.root.join(SKILLS_DIR));
        Ok(())
    }

    /// Removes the empty folders above `path`, stopping at `stop` (which stays) or at the first
    /// folder that is not empty.
    fn remove_empty_folders_above(&self, path: &Path, stop: &Path) {
        let mut dir = path.parent();
        while let Some(d) = dir {
            if d == stop
                || !d.starts_with(&self.root)
                || d == self.root
                || fs::remove_dir(d).is_err()
            {
                break;
            }
            dir = d.parent();
        }
    }
}

fn collect_files(dir: &Path, rel: &str, out: &mut Vec<String>) {
    let Ok(listing) = fs::read_dir(dir) else {
        return;
    };
    for entry in listing.filter_map(Result::ok) {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let child = format!("{rel}/{name}");
        if entry.path().is_dir() {
            collect_files(&entry.path(), &child, out);
        } else {
            out.push(child);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wanted<'a>(items: &'a [(&'a str, &'a str)]) -> impl Iterator<Item = (&'a str, &'a str)> {
        items.iter().copied()
    }

    #[test]
    fn only_missing_or_different_files_are_planned_and_crlf_counts_as_equal() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("same.md"), "a\r\nb\r\n").unwrap();
        fs::write(dir.path().join("other.md"), "mine").unwrap();
        let mut plan = Plan::new(dir.path());
        let conflicts = plan.write_missing_or_different(
            wanted(&[
                ("same.md", "a\nb\n"),
                ("other.md", "theirs"),
                ("new/dir/file.md", "x"),
            ]),
            Differing::Rewrite,
        );
        assert!(conflicts.is_empty());
        let changes = plan.changes();
        assert_eq!(
            changes,
            [
                Change {
                    path: "new/dir/file.md".into(),
                    action: Action::Created
                },
                Change {
                    path: "other.md".into(),
                    action: Action::Rewritten
                },
            ]
        );
        plan.apply().unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("other.md")).unwrap(),
            "theirs\n"
        );
        assert_eq!(
            fs::read(dir.path().join("same.md")).unwrap(),
            b"a\r\nb\r\n",
            "an equal file is left alone"
        );
    }

    #[test]
    fn a_differing_file_is_a_conflict_when_asked_and_nothing_is_planned_for_it() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("b.md"), "mine").unwrap();
        fs::write(dir.path().join("a.md"), "mine").unwrap();
        let mut plan = Plan::new(dir.path());
        let conflicts = plan.write_missing_or_different(
            wanted(&[("b.md", "x"), ("a.md", "x"), ("c.md", "x")]),
            Differing::Conflict,
        );
        assert_eq!(conflicts, ["a.md", "b.md"]);
        assert_eq!(plan.written(), ["c.md"]);
    }

    #[test]
    fn a_failure_half_way_puts_everything_back_and_removes_the_folders_it_made() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "old").unwrap();
        // `blocker` is a file: nothing can be written below it.
        fs::write(dir.path().join("zblocker"), "i am a file").unwrap();
        let mut plan = Plan::new(dir.path());
        plan.write_missing_or_different(
            wanted(&[
                ("a.md", "new"),
                ("made/inner/file.md", "x"),
                ("zblocker/x.md", "never"),
            ]),
            Differing::Rewrite,
        );
        let error = plan.apply().unwrap_err();
        assert!(error.to_string().contains("zblocker/x.md"), "{error}");
        assert_eq!(fs::read_to_string(dir.path().join("a.md")).unwrap(), "old");
        assert!(!dir.path().join("made").exists());
    }

    #[test]
    fn only_requirement_skills_outside_the_kept_set_are_removed_with_their_empty_folders() {
        let dir = tempfile::tempdir().unwrap();
        let put = |rel: &str| {
            let path = dir.path().join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "x").unwrap();
        };
        put(".agents/skills/trama-req-ui-main/SKILL.md");
        put(".agents/skills/trama-req-ui/SKILL.md");
        put(".agents/skills/trama-req-ui/old.md");
        put(".agents/skills/trama-design/SKILL.md");
        put(".agents/skills/my-own-skill/SKILL.md");
        let keep: BTreeSet<String> = [".agents/skills/trama-req-ui/SKILL.md".to_owned()].into();
        let mut plan = Plan::new(dir.path());
        plan.remove_other_requirement_skills(&keep);
        assert_eq!(
            plan.removed(),
            [
                ".agents/skills/trama-req-ui/old.md",
                ".agents/skills/trama-req-ui-main/SKILL.md"
            ]
        );
        plan.apply().unwrap();
        assert!(!dir.path().join(".agents/skills/trama-req-ui-main").exists());
        assert!(dir
            .path()
            .join(".agents/skills/trama-req-ui/SKILL.md")
            .exists());
        assert!(!dir
            .path()
            .join(".agents/skills/trama-req-ui/old.md")
            .exists());
        assert!(dir
            .path()
            .join(".agents/skills/trama-design/SKILL.md")
            .exists());
        assert!(dir
            .path()
            .join(".agents/skills/my-own-skill/SKILL.md")
            .exists());

        plan.roll_back();
        assert!(dir
            .path()
            .join(".agents/skills/trama-req-ui-main/SKILL.md")
            .exists());
        assert!(dir
            .path()
            .join(".agents/skills/trama-req-ui/old.md")
            .exists());
    }
}
