//! App-wide state managed by Tauri. Domain state lives here, in the backend, never in the UI.

use std::sync::{Arc, Mutex};

use weaver_core::paths::AppDirs;

use crate::agent_work::AgentWorkSlot;
use crate::mutations::MutationQueues;
use crate::pictures::Pictures;
use crate::proposals::ProposalJobsSlot;
use crate::server::Server;
use crate::settings::SettingsService;

pub struct AppState {
    pub dirs: AppDirs,
    pub mutations: MutationQueues,
    /// The settings of the app, loaded on first use (see [`crate::settings`]).
    pub settings: SettingsService,
    /// The in-memory picture of each open project (see [`crate::pictures`]).
    pub pictures: Arc<Pictures>,
    /// What the Board knows about the agent's work: ghosts and facts (see [`crate::agent_work`]).
    pub agent_work: AgentWorkSlot,
    /// What a decision on a Proposal starts: refinement and rework jobs, Run all after a setup
    /// (see [`crate::proposals`]).
    pub proposal_jobs: ProposalJobsSlot,
    /// The CLI socket server; `None` until it started, and after shutdown.
    pub server: Mutex<Option<Server>>,
}

impl AppState {
    pub fn new(dirs: AppDirs) -> Self {
        Self {
            mutations: MutationQueues::new(dirs.clone()),
            settings: SettingsService::new(&dirs),
            pictures: Arc::new(Pictures::new()),
            agent_work: AgentWorkSlot::default(),
            proposal_jobs: ProposalJobsSlot::default(),
            dirs,
            server: Mutex::new(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use tauri::Manager;
    use weaver_core::paths::{AppDirs, APP_IDENTIFIER};

    /// The CLI resolves its folders with `AppDirs::resolve` with the app closed; they must be the
    /// folders Tauri gives the app for the identifier in `tauri.conf.json`.
    #[test]
    fn the_cli_finds_the_same_folders_as_the_app() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().identifier = config["identifier"].as_str().unwrap().to_owned();
        let app = tauri::test::mock_builder().build(context).unwrap();
        let resolver = app.path();
        let dirs = AppDirs::resolve().expect("folders");
        assert_eq!(resolver.app_config_dir().unwrap(), dirs.config_dir());
        assert_eq!(
            resolver.app_local_data_dir().unwrap(),
            dirs.local_data_dir()
        );
    }

    #[test]
    fn the_identifier_constant_is_the_one_in_the_tauri_config() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(config["identifier"], APP_IDENTIFIER);
    }
}
