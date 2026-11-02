//! The Machine `project` (feature `project-setup`) and the files Weaver installs for the agent.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`requirement_skills`] | The `trama-req-*` skills: budget, mode, generated text (pure) |
//! | [`plan`] | A set of files to write and remove, applied whole or not at all |
//! | [`sync`] | The one function that keeps the installed files and the Requirement skills right, and its structured report |
//! | [`initialise`] | `developer.INITIALISE_PROJECT`: `.trama/`, the starting Areas and every agent file |
//! | [`schedule`] | At most one check per project at a time; requests meanwhile coalesce |
//! | [`folder_tree`] | The folders of `Import from Code`: no dot names, nothing git ignores |
//!
//! The embedded list of installed files is [`crate::agent_files`]; the starting Areas are in
//! [`crate::project::init`]. The app and the `trama` command line call these same functions, so
//! both write identical bytes.

pub mod folder_tree;
pub mod initialise;
pub mod plan;
pub mod requirement_skills;
pub mod schedule;
pub mod sync;

use serde::Serialize;
use ts_rs::TS;

use crate::project::layout::trama_dir;
use crate::project::model::Model;

pub use initialise::{initialise, InitialiseError, Initialised};
pub use plan::{Action as FileAction, Change as FileChange};
pub use requirement_skills::{Mode as SkillMode, BUDGET as SKILL_BUDGET};
pub use sync::{regenerate_requirement_skills, sync, SyncError, SyncOptions, SyncReport};

/// The states of the Machine `project`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectState {
    /// The folder has no `.trama/`.
    NotInitialised,
    /// `.trama/` exists and its Model has no Feature and no Actor (open Drifts and Requirements do not count).
    EmptyModel,
    /// The Model has at least one Feature or one Actor.
    HasModel,
}

impl ProjectState {
    /// The state of a folder: whether `.trama/` exists, and its Model.
    pub fn of(trama_exists: bool, model: &Model) -> Self {
        match (trama_exists, model.is_empty()) {
            (false, _) => ProjectState::NotInitialised,
            (true, true) => ProjectState::EmptyModel,
            (true, false) => ProjectState::HasModel,
        }
    }

    /// Whether the folder has a `.trama/` (the project exists).
    pub fn is_initialised(self) -> bool {
        self != ProjectState::NotInitialised
    }
}

/// Whether `<root>/.trama` is a folder.
pub fn has_trama(root: &std::path::Path) -> bool {
    trama_dir(root).is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::{parse_actor, parse_titled, Model, ModelFiles};

    #[test]
    fn the_state_follows_the_folder_and_whether_the_model_has_a_feature_or_an_actor() {
        let empty = Model::default();
        assert_eq!(
            ProjectState::of(false, &empty),
            ProjectState::NotInitialised
        );
        assert_eq!(ProjectState::of(true, &empty), ProjectState::EmptyModel);
        let shop = parse_titled("# Shop\n").unwrap();
        let (with_feature, _) = Model::assemble(&ModelFiles {
            system: None,
            actors: vec![],
            features: vec![("model/shop/feature.md", "shop", &shop)],
            machines: vec![],
        });
        assert_eq!(
            ProjectState::of(true, &with_feature),
            ProjectState::HasModel
        );
        let user = parse_actor("user", "# User\n\nA person.\n").unwrap();
        let (with_actor, _) = Model::assemble(&ModelFiles {
            system: None,
            actors: vec![("model/actors/user.md", &user)],
            features: vec![],
            machines: vec![],
        });
        assert_eq!(ProjectState::of(true, &with_actor), ProjectState::HasModel);
        assert!(
            !ProjectState::NotInitialised.is_initialised()
                && ProjectState::EmptyModel.is_initialised()
        );
    }
}
