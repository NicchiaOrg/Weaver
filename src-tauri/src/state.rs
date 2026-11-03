//! App-wide state managed by Tauri. Domain state lives here, in the backend, never in the UI.

use std::sync::{Arc, Mutex};

use weaver_core::paths::AppDirs;

use crate::agent_work::AgentWorkSlot;
use crate::file_lock::FileLockService;
use crate::file_tabs::events::FileEvents;
use crate::file_tabs::watch::NotifyTabWatch;
use crate::file_tabs::FileTabs;
use crate::file_view::{FileViewService, TabsAndLocks};
use crate::mutations::MutationQueues;
use crate::pictures::Pictures;
use crate::proposals::ProposalJobsSlot;
use crate::server::Server;
use crate::settings::SettingsService;
use crate::windows::local_state::StateStore;

pub struct AppState {
    pub dirs: AppDirs,
    pub mutations: MutationQueues,
    /// The settings of the app, loaded on first use (see [`crate::settings`]).
    pub settings: SettingsService,
    /// `state.json`: the recent projects, the last session and what System Certificates
    /// remembers. The one owner of the file, shared by the windows and the Zscaler turn-on, so
    /// that no write overwrites another (see [`crate::windows::local_state`]).
    pub local_state: Arc<StateStore>,
    /// The in-memory picture of each open project (see [`crate::pictures`]).
    pub pictures: Arc<Pictures>,
    /// What the Board knows about the agent's work: ghosts and facts (see [`crate::agent_work`]).
    pub agent_work: AgentWorkSlot,
    /// What a decision on a Proposal starts: refinement and rework jobs, Run all after a setup
    /// (see [`crate::proposals`]).
    pub proposal_jobs: ProposalJobsSlot,
    /// The events of the File view's tabs and of the agent lock; attached to the app at setup
    /// (see [`crate::file_tabs::events`]).
    pub file_events: Arc<FileEvents>,
    /// Which files open in a tab an agent is changing, in memory only (see [`crate::file_lock`]).
    pub file_locks: Arc<FileLockService>,
    /// The files open in the tabs of the File view: what each read, its watch, its saving (see
    /// [`crate::file_tabs`]).
    pub file_tabs: Arc<FileTabs>,
    /// The tree of each window's File view: its listings and watches, in memory only (see
    /// [`crate::file_view`]).
    pub file_view: Arc<FileViewService>,
    /// The CLI socket server; `None` until it started, and after shutdown.
    pub server: Mutex<Option<Server>>,
}

impl AppState {
    pub fn new(dirs: AppDirs) -> Self {
        let file_events = Arc::new(FileEvents::default());
        let file_locks = FileLockService::new(file_events.clone());
        let file_tabs = FileTabs::new(
            file_locks.clone(),
            file_events.clone(),
            Arc::new(NotifyTabWatch),
        );
        let file_view = Arc::new(FileViewService::new(Arc::new(TabsAndLocks {
            tabs: file_tabs.clone(),
            locks: file_locks.clone(),
        })));
        Self {
            mutations: MutationQueues::new(dirs.clone()),
            settings: SettingsService::new(&dirs),
            local_state: Arc::new(StateStore::in_dir(dirs.local_data_dir())),
            pictures: Arc::new(Pictures::new()),
            agent_work: AgentWorkSlot::default(),
            proposal_jobs: ProposalJobsSlot::default(),
            file_events,
            file_locks,
            file_tabs,
            file_view,
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
