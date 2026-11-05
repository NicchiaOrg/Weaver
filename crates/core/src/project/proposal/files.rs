//! The text inside the folders of Proposals.
//!
//! The project image does not hold the copies a Proposal keeps of the files it changes (they sit
//! under `proposals/<id>-<slug>/` at the path they have under `.weaver/`), so the code that
//! decides reads them once with [`read_folders`] and passes them on as [`Folders`]: the rules
//! stay pure functions over text.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::project::cards::{Card, CardKind};
use crate::project::changes::ChangeSet;
use crate::project::image::ProjectImage;
use crate::project::layout::{self, PROPOSAL_FILE};
use crate::project::text;

use super::data::{self, Status, TASKS_FOLDER};

/// Folders nested deeper than this are not followed.
const MAX_DEPTH: usize = 8;

/// What one Proposal folder holds besides `proposal.md`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Folder {
    /// Path inside the folder (`/`-separated) → normalized text.
    pub files: BTreeMap<String, String>,
    /// Path inside the folder → why it could not be read.
    pub unreadable: BTreeMap<String, String>,
}

/// The folders of Proposals, by Proposal identity.
pub type Folders = BTreeMap<String, Folder>;

impl Folder {
    /// A folder from text, for the tests and for a Proposal that is being written in memory.
    pub fn with(files: &[(&str, &str)]) -> Folder {
        Folder {
            files: files
                .iter()
                .map(|(p, t)| ((*p).to_owned(), text::file_content(t)))
                .collect(),
            unreadable: BTreeMap::new(),
        }
    }

    /// The file `rel` the Proposal holds as a new version of a file under `.weaver/`: not a Task.
    pub fn held(&self) -> impl Iterator<Item = (&String, &String)> {
        self.files.iter().filter(|(path, _)| !is_task_path(path))
    }

    /// The text of the held file `rel`.
    pub fn text_of(&self, rel: &str) -> Option<&str> {
        self.files.get(rel).map(String::as_str)
    }
}

/// `tasks/…` inside a Proposal folder.
pub fn is_task_path(rel: &str) -> bool {
    rel.split('/').next() == Some(TASKS_FOLDER)
}

fn walk(dir: &Path, rel: &str, depth: usize, folder: &mut Folder) {
    let Ok(listing) = fs::read_dir(dir) else {
        return;
    };
    let mut names: Vec<(String, std::path::PathBuf, bool)> = Vec::new();
    for item in listing.flatten() {
        let name = item.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = item.path();
        let is_dir = fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false);
        names.push((name, path, is_dir));
    }
    names.sort();
    for (name, path, is_dir) in names {
        let child = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        if is_dir {
            if depth < MAX_DEPTH {
                walk(&path, &child, depth + 1, folder);
            }
        } else if !(rel.is_empty() && name == PROPOSAL_FILE) {
            match text::read_text(&path) {
                Ok(content) => {
                    folder.files.insert(child, content);
                }
                Err(e) => {
                    folder.unreadable.insert(child, e.to_string());
                }
            }
        }
    }
}

/// Reads the folder of `card` from disk.
pub fn read_folder(project_root: &Path, card: &Card) -> Folder {
    let mut folder = Folder::default();
    walk(
        &layout::absolute(project_root, &data::folder_of(card)),
        "",
        0,
        &mut folder,
    );
    folder
}

/// The folders of every Proposal that is not decided (Draft, Open, Refining).
pub fn read_folders(project_root: &Path, image: &ProjectImage) -> Folders {
    image
        .cards_of(CardKind::Proposal)
        .filter(|card| data::status_of(card).is_some_and(Status::is_live))
        .map(|card| (card.id.clone(), read_folder(project_root, card)))
        .collect()
}

/// The paths a Proposal changes or removes: the keys of `base`.
pub fn registered(card: &Card) -> Vec<String> {
    data::base_of(card).into_keys().collect()
}

/// The changes the files of a Proposal make: each registered file it holds is written at its
/// path under `.weaver/`, each removed path is deleted. Files that are not registered, and Task
/// files, are not part of it.
pub fn changes_of(card: &Card, folder: &Folder) -> ChangeSet {
    let base = data::base_of(card);
    let deleted = data::deleted_of(card);
    let mut changes = ChangeSet::new();
    for (path, content) in folder.held() {
        if base.contains_key(path) && !deleted.contains(path) {
            changes.write(path, content.clone());
        }
    }
    for path in &deleted {
        changes.delete(path);
    }
    changes
}

/// How a Proposal's files compare with the Model as the Proposal started from it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileSummary {
    /// Paths that did not exist (`base` says `new`).
    pub added: Vec<String>,
    pub changed: Vec<String>,
    pub removed: Vec<String>,
}

impl FileSummary {
    pub fn total(&self) -> usize {
        self.added.len() + self.changed.len() + self.removed.len()
    }
}

/// The added, changed and removed paths of a Proposal.
pub fn summary_of(card: &Card) -> FileSummary {
    let base = data::base_of(card);
    let deleted = data::deleted_of(card);
    let mut summary = FileSummary::default();
    for (path, hash) in &base {
        if deleted.contains(path) {
            summary.removed.push(path.clone());
        } else if hash == data::NEW_FILE {
            summary.added.push(path.clone());
        } else {
            summary.changed.push(path.clone());
        }
    }
    for path in deleted {
        if !base.contains_key(&path) {
            summary.removed.push(path);
        }
    }
    summary.removed.sort();
    summary.removed.dedup();
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::proposal::fixtures::{proposal_card, put};

    #[test]
    fn a_folder_is_read_with_normalized_text_and_without_proposal_md_or_hidden_files() {
        let dir = tempfile::tempdir().unwrap();
        put(
            dir.path(),
            "proposals/pppppppp-t/proposal.md",
            "---\nstatus: draft\n---\n# T\n",
        );
        put(
            dir.path(),
            "proposals/pppppppp-t/model/system.md",
            "\u{feff}# Shop\r\n",
        );
        put(
            dir.path(),
            "proposals/pppppppp-t/tasks/tttttttt-a.md",
            "# A\n",
        );
        put(dir.path(), "proposals/pppppppp-t/.note", "hidden");
        let card =
            proposal_card("---\nstatus: draft\ncreated: 2026-11-02T07:00:00.000Z\n---\n# T\n");
        let folder = read_folder(dir.path(), &card);
        assert_eq!(
            folder.files.keys().map(String::as_str).collect::<Vec<_>>(),
            ["model/system.md", "tasks/tttttttt-a.md"]
        );
        assert_eq!(folder.text_of("model/system.md"), Some("# Shop\n"));
        assert_eq!(folder.held().count(), 1);
    }

    #[test]
    fn only_registered_files_make_changes_and_removed_paths_are_deleted() {
        let card = proposal_card(
            "---\nstatus: open\nbase:\n  model/system.md: new\n  specs.yaml: abc\n  model/old.md: def\ndelete:\n  - model/old.md\ncreated: 2026-11-02T07:00:00.000Z\n---\n# T\n",
        );
        let folder = Folder::with(&[
            ("model/system.md", "# S"),
            ("stray.md", "x"),
            ("tasks/tttttttt-a.md", "# A"),
        ]);
        let changes = changes_of(&card, &folder);
        assert_eq!(changes.len(), 2, "{:?}", changes.iter().collect::<Vec<_>>());
        assert!(changes.get("model/system.md").is_some());
        assert!(matches!(
            changes.get("model/old.md"),
            Some(crate::project::changes::Change::Delete)
        ));
        let summary = summary_of(&card);
        assert_eq!(summary.added, ["model/system.md"]);
        assert_eq!(summary.changed, ["specs.yaml"]);
        assert_eq!(summary.removed, ["model/old.md"]);
        assert_eq!(summary.total(), 3);
    }
}
