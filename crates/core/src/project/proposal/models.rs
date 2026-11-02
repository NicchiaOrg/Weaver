//! The model the developer chose for a Proposal.
//!
//! It is never written in the repository, so that a cloned project cannot choose models for the
//! developer: the app keeps it in its local data, `proposal-models/<project hash>.json`, a map
//! from Proposal identity to model value. An entry is dropped once no Task of that Proposal is in
//! To do or In progress (a Proposal still to be decided keeps its entry: its Tasks do not exist
//! yet). The model reaches omp only as one argument of a process started without a shell, and
//! choosing it never sends the Proposal back to Draft.

use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;

use crate::atomic;
use crate::paths::AppDirs;
use crate::project::cards::CardKind;
use crate::project::image::ProjectImage;
use crate::project::tasks::{column_of, Column};

use super::data::{self, Status};

/// The folder of the local data that holds one file per project.
pub const MODELS_DIR: &str = "proposal-models";

/// `<local data>/proposal-models/<project hash>.json`.
pub fn models_file(dirs: &AppDirs, project_hash: &str) -> PathBuf {
    dirs.local_data_dir()
        .join(MODELS_DIR)
        .join(format!("{project_hash}.json"))
}

/// The models chosen in one project.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProposalModels {
    models: BTreeMap<String, String>,
}

impl ProposalModels {
    /// Reads the file of a project; a missing or unreadable one is no choice at all.
    pub fn load(dirs: &AppDirs, project_hash: &str) -> Self {
        let models = std::fs::read(models_file(dirs, project_hash))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<BTreeMap<String, String>>(&bytes).ok())
            .unwrap_or_default();
        Self { models }
    }

    /// Writes the file atomically; with no choice left the file is removed.
    pub fn save(&self, dirs: &AppDirs, project_hash: &str) -> io::Result<()> {
        let path = models_file(dirs, project_hash);
        if self.models.is_empty() {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            };
        }
        let mut text = serde_json::to_string_pretty(&self.models).map_err(io::Error::other)?;
        text.push('\n');
        atomic::write(&path, text.as_bytes(), None)
    }

    /// The model chosen for a Proposal; `None` means the models of the Settings.
    pub fn get(&self, proposal_id: &str) -> Option<&str> {
        self.models.get(proposal_id).map(String::as_str)
    }

    /// Chooses a model, or goes back to the Settings models with `None` (or a blank value).
    pub fn set(&mut self, proposal_id: &str, model: Option<&str>) {
        match model.map(str::trim).filter(|m| !m.is_empty()) {
            Some(model) => {
                self.models.insert(proposal_id.to_owned(), model.to_owned());
            }
            None => {
                self.models.remove(proposal_id);
            }
        }
    }

    /// Drops the entries whose Proposal is decided and has no Task in To do or In progress (and
    /// those of a Proposal that is gone). Returns whether anything was dropped.
    pub fn prune(&mut self, image: &ProjectImage) -> bool {
        let before = self.models.len();
        self.models.retain(|id, _| {
            match image.card(CardKind::Proposal, id).and_then(data::status_of) {
                None | Some(Status::Rejected | Status::Discarded) => false,
                Some(Status::Approved) => image.cards_of(CardKind::Task).any(|t| {
                    t.str_key("proposal") == Some(id.as_str())
                        && matches!(column_of(t), Some(Column::Todo | Column::Doing))
                }),
                Some(_) => true,
            }
        });
        self.models.len() != before
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::proposal::fixtures::{proposal, put};

    fn dirs(base: &std::path::Path) -> AppDirs {
        AppDirs::from_bases(base, base, None)
    }

    #[test]
    fn a_choice_is_kept_in_the_local_data_and_never_under_trama() {
        let (base, project) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        proposal(
            project.path(),
            "pppppppp",
            "status: open\n",
            "# T\n",
            &[],
            &[],
        );
        let mut models = ProposalModels::default();
        models.set("pppppppp", Some(" anthropic/claude-opus "));
        assert_eq!(models.get("pppppppp"), Some("anthropic/claude-opus"));
        models.save(&dirs(base.path()), "0123456789abcdef").unwrap();

        let saved =
            std::fs::read_to_string(models_file(&dirs(base.path()), "0123456789abcdef")).unwrap();
        assert!(saved.contains("anthropic/claude-opus"));
        assert_eq!(
            ProposalModels::load(&dirs(base.path()), "0123456789abcdef"),
            models
        );
        // Nothing under `.trama/` names it.
        fn walk(dir: &std::path::Path, found: &mut bool) {
            for item in std::fs::read_dir(dir).unwrap() {
                let path = item.unwrap().path();
                if path.is_dir() {
                    walk(&path, found);
                } else if std::fs::read_to_string(&path).is_ok_and(|t| t.contains("claude-opus")) {
                    *found = true;
                }
            }
        }
        let mut found = false;
        walk(&project.path().join(".trama"), &mut found);
        assert!(!found);

        models.set("pppppppp", None);
        models.save(&dirs(base.path()), "0123456789abcdef").unwrap();
        assert!(!models_file(&dirs(base.path()), "0123456789abcdef").exists());
        assert!(ProposalModels::load(&dirs(base.path()), "0123456789abcdef").is_empty());
    }

    #[test]
    fn an_unreadable_file_is_no_choice() {
        let base = tempfile::tempdir().unwrap();
        let path = models_file(&dirs(base.path()), "0123456789abcdef");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not json").unwrap();
        assert!(ProposalModels::load(&dirs(base.path()), "0123456789abcdef").is_empty());
    }

    #[test]
    fn an_entry_goes_when_no_task_of_a_decided_proposal_is_in_to_do_or_in_progress() {
        let project = tempfile::tempdir().unwrap();
        let root = project.path();
        proposal(root, "aaaaaaaa", "status: open\n", "# Open\n", &[], &[]);
        proposal(
            root,
            "bbbbbbbb",
            "status: approved\n",
            "# Approved\n",
            &[],
            &[],
        );
        proposal(root, "cccccccc", "status: approved\n", "# Done\n", &[], &[]);
        proposal(
            root,
            "dddddddd",
            "status: rejected\n",
            "# Rejected\n",
            &[],
            &[],
        );
        put(root, "tasks/11111111-live.md", "---\ncolumn: doing\ntechnical: true\nrefs: []\nproposal: bbbbbbbb\ncreated: 2026-11-02T07:00:01.000Z\n---\n# Live\n");
        put(root, "tasks/22222222-done.md", "---\ncolumn: done\ntechnical: true\nrefs: []\nproposal: cccccccc\ncreated: 2026-11-02T07:00:02.000Z\nfinished: 2026-11-02T08:00:00.000Z\n---\n# Done\n");
        let image = ProjectImage::load(root);
        let mut models = ProposalModels::default();
        for id in ["aaaaaaaa", "bbbbbbbb", "cccccccc", "dddddddd", "eeeeeeee"] {
            models.set(id, Some("m"));
        }
        assert!(models.prune(&image));
        let kept: Vec<&str> = ["aaaaaaaa", "bbbbbbbb", "cccccccc", "dddddddd", "eeeeeeee"]
            .into_iter()
            .filter(|id| models.get(id).is_some())
            .collect();
        assert_eq!(kept, ["aaaaaaaa", "bbbbbbbb"]);
        assert!(!models.prune(&image));
    }
}
