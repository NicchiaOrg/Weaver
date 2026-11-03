//! Which project windows are open, and what the quit path is doing: the pure part of the window
//! manager (no Tauri, no I/O), so the rules are tested without windows.

use std::collections::HashSet;
use std::path::PathBuf;

/// The label of the welcome window.
pub const WELCOME_LABEL: &str = "welcome";

/// `p-<project hash>`.
pub fn label_of(project_id: &str) -> String {
    format!("{}{project_id}", crate::pictures::WINDOW_LABEL_PREFIX)
}

/// An open project window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenProject {
    /// The project hash.
    pub id: String,
    /// The canonical root.
    pub root: PathBuf,
    /// The folder's name: the title of the window.
    pub name: String,
}

/// What [`Registry::begin_open`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenStart {
    /// The project is not open: the caller opens it and finishes with [`Registry::finish_open`]
    /// or [`Registry::abort_open`].
    Reserved,
    /// The project has its window.
    AlreadyOpen,
    /// Another call is opening it now: that one will show the window.
    InFlight,
}

/// Where the quit path is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuitState {
    #[default]
    Idle,
    /// Asking for confirmation or suspending the jobs.
    Running,
    /// Everything is done: the next exit request goes through.
    Ready,
}

/// What the app does with a `RunEvent::ExitRequested`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitDecision {
    /// Let the exit go on.
    Allow,
    /// Prevent it and stay: the last window closed on macOS, where the app stays in the Dock.
    Stay,
    /// Prevent it and run the quit path, which exits again with `code` when it is done.
    Quit { code: i32 },
    /// Prevent it: the quit path is already running.
    Ignore,
}

/// `RunEvent::ExitRequested` carries no code when the developer quits (`Cmd+Q`, File › Quit) and
/// when the last window closed; `windows_left` tells them apart. It counts every window that still
/// exists, those in the background included (they are hidden, not closed), so closing the welcome
/// window never ends Weaver while a project window is in the background. Closing the last project
/// window opens the welcome window first, so no window is left only after the welcome window closed.
pub fn decide_exit(
    quit: QuitState,
    code: Option<i32>,
    windows_left: usize,
    macos: bool,
) -> ExitDecision {
    match quit {
        QuitState::Ready => ExitDecision::Allow,
        QuitState::Running => ExitDecision::Ignore,
        QuitState::Idle if code.is_none() && windows_left == 0 => {
            if macos {
                ExitDecision::Stay
            } else {
                ExitDecision::Allow
            }
        }
        QuitState::Idle => ExitDecision::Quit {
            code: code.unwrap_or(0),
        },
    }
}

#[derive(Debug, Default)]
pub struct Registry {
    open: Vec<OpenProject>,
    opening: HashSet<String>,
    closing: HashSet<String>,
    /// The project hashes whose window is in the background: open, but not on the screen.
    background: HashSet<String>,
    /// Labels, the most recently focused last.
    focus: Vec<String>,
    quit: QuitState,
    restoring: bool,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// The open projects in opening order.
    pub fn projects(&self) -> &[OpenProject] {
        &self.open
    }

    pub fn project(&self, id: &str) -> Option<&OpenProject> {
        self.open.iter().find(|project| project.id == id)
    }

    pub fn begin_open(&mut self, id: &str) -> OpenStart {
        if self.project(id).is_some() {
            OpenStart::AlreadyOpen
        } else if self.opening.insert(id.to_owned()) {
            OpenStart::Reserved
        } else {
            OpenStart::InFlight
        }
    }

    pub fn finish_open(&mut self, project: OpenProject) {
        self.opening.remove(&project.id);
        self.focus.retain(|label| *label != label_of(&project.id));
        self.focus.push(label_of(&project.id));
        self.open.push(project);
    }

    pub fn abort_open(&mut self, id: &str) {
        self.opening.remove(id);
    }

    /// Starts closing a project window. `None` when it is not open or is already closing (a
    /// second close request while a confirmation is shown does nothing).
    pub fn begin_close(&mut self, id: &str) -> Option<OpenProject> {
        let project = self.project(id)?.clone();
        self.closing.insert(id.to_owned()).then_some(project)
    }

    /// The developer cancelled the confirmation: nothing changed.
    pub fn cancel_close(&mut self, id: &str) {
        self.closing.remove(id);
    }

    /// The window is gone.
    pub fn remove(&mut self, id: &str) -> Option<OpenProject> {
        self.closing.remove(id);
        self.background.remove(id);
        self.focus.retain(|label| *label != label_of(id));
        let at = self.open.iter().position(|project| project.id == id)?;
        Some(self.open.remove(at))
    }

    /// Whether `id` is the only open project window. A window in the background is open: it counts.
    pub fn is_last_project(&self, id: &str) -> bool {
        self.open.len() == 1 && self.project(id).is_some()
    }

    /// `developer.SEND_TO_BACKGROUND`: the window of `id` is now in the background. `false` when the
    /// project is not open or its window already is in the background.
    pub fn send_to_background(&mut self, id: &str) -> bool {
        self.project(id).is_some() && self.background.insert(id.to_owned())
    }

    /// The window of `id` is back on the screen.
    pub fn shown(&mut self, id: &str) {
        self.background.remove(id);
    }

    pub fn is_background(&self, id: &str) -> bool {
        self.background.contains(id)
    }

    /// Whether any project window is in the background.
    pub fn has_background(&self) -> bool {
        !self.background.is_empty()
    }

    /// The projects whose window is in the background, the least recently used first (the order in
    /// which `developer.REOPEN_APP` shows them, so that the one used most recently ends on top).
    pub fn background_projects(&self) -> Vec<String> {
        self.focus
            .iter()
            .filter_map(|label| crate::pictures::project_of_label(label))
            .filter(|id| self.background.contains(*id))
            .map(str::to_owned)
            .collect()
    }

    /// A window got the focus.
    pub fn focused(&mut self, label: &str) {
        self.focus.retain(|known| known != label);
        self.focus.push(label.to_owned());
    }

    /// The window labels, the most recently focused first.
    pub fn focus_order(&self) -> impl Iterator<Item = &str> {
        self.focus.iter().rev().map(String::as_str)
    }

    pub fn forget_label(&mut self, label: &str) {
        self.focus.retain(|known| known != label);
    }

    pub fn quit_state(&self) -> QuitState {
        self.quit
    }

    /// `false` when the quit path is already running or done.
    pub fn begin_quit(&mut self) -> bool {
        if self.quit != QuitState::Idle {
            return false;
        }
        self.quit = QuitState::Running;
        true
    }

    pub fn cancel_quit(&mut self) {
        self.quit = QuitState::Idle;
    }

    pub fn quit_ready(&mut self) {
        self.quit = QuitState::Ready;
    }

    /// The projects of the last session are being reopened: a quit now must not overwrite the
    /// list with the few that are open so far.
    pub fn set_restoring(&mut self, restoring: bool) {
        self.restoring = restoring;
    }

    pub fn is_restoring(&self) -> bool {
        self.restoring
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(id: &str) -> OpenProject {
        OpenProject {
            id: id.to_owned(),
            root: PathBuf::from(format!("/work/{id}")),
            name: id.to_owned(),
        }
    }

    #[test]
    fn a_project_has_one_window_and_a_second_open_does_not_start_another() {
        let mut registry = Registry::new();
        assert_eq!(registry.begin_open("a"), OpenStart::Reserved);
        assert_eq!(registry.begin_open("a"), OpenStart::InFlight);
        registry.finish_open(project("a"));
        assert_eq!(registry.begin_open("a"), OpenStart::AlreadyOpen);
        assert_eq!(registry.projects().len(), 1);
    }

    #[test]
    fn an_aborted_open_can_be_tried_again() {
        let mut registry = Registry::new();
        assert_eq!(registry.begin_open("a"), OpenStart::Reserved);
        registry.abort_open("a");
        assert_eq!(registry.begin_open("a"), OpenStart::Reserved);
    }

    #[test]
    fn projects_stay_in_opening_order() {
        let mut registry = Registry::new();
        for id in ["b", "a", "c"] {
            registry.begin_open(id);
            registry.finish_open(project(id));
        }
        let ids: Vec<_> = registry.projects().iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["b", "a", "c"]);
        registry.remove("a");
        let ids: Vec<_> = registry.projects().iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["b", "c"]);
    }

    #[test]
    fn a_second_close_request_while_one_is_pending_does_nothing_and_a_cancel_clears_it() {
        let mut registry = Registry::new();
        registry.begin_open("a");
        registry.finish_open(project("a"));
        assert!(registry.begin_close("a").is_some());
        assert!(registry.begin_close("a").is_none());
        registry.cancel_close("a");
        assert!(registry.begin_close("a").is_some());
        assert!(registry.begin_close("missing").is_none());
    }

    #[test]
    fn the_last_project_window_is_known() {
        let mut registry = Registry::new();
        registry.begin_open("a");
        registry.finish_open(project("a"));
        assert!(registry.is_last_project("a"));
        registry.begin_open("b");
        registry.finish_open(project("b"));
        assert!(!registry.is_last_project("a"));
    }

    #[test]
    fn the_most_recently_focused_window_comes_first() {
        let mut registry = Registry::new();
        registry.begin_open("a");
        registry.finish_open(project("a"));
        registry.begin_open("b");
        registry.finish_open(project("b"));
        assert_eq!(registry.focus_order().collect::<Vec<_>>(), ["p-b", "p-a"]);
        registry.focused("p-a");
        registry.focused(WELCOME_LABEL);
        assert_eq!(
            registry.focus_order().collect::<Vec<_>>(),
            ["welcome", "p-a", "p-b"]
        );
        registry.remove("a");
        assert_eq!(
            registry.focus_order().collect::<Vec<_>>(),
            ["welcome", "p-b"]
        );
    }

    fn open_three() -> Registry {
        let mut registry = Registry::new();
        for id in ["a", "b", "c"] {
            registry.begin_open(id);
            registry.finish_open(project(id));
        }
        registry
    }

    #[test]
    fn a_window_in_the_background_stays_open_and_counts_for_the_last_window_rule() {
        let mut registry = open_three();
        assert!(registry.send_to_background("a"));
        assert!(registry.is_background("a"));
        assert!(registry.has_background());
        // Open, and kept in the session.
        assert_eq!(registry.projects().len(), 3);
        // Closing the two others never makes "c" the last project window.
        registry.remove("b");
        assert!(!registry.is_last_project("c"));
        registry.remove("c");
        assert!(registry.is_last_project("a"));
    }

    #[test]
    fn only_an_open_window_that_is_not_in_the_background_can_be_sent_there() {
        let mut registry = open_three();
        assert!(!registry.send_to_background("missing"));
        assert!(registry.send_to_background("a"));
        assert!(!registry.send_to_background("a"));
        registry.shown("a");
        assert!(!registry.is_background("a"));
        assert!(!registry.has_background());
        assert!(registry.send_to_background("a"));
    }

    #[test]
    fn the_background_windows_come_least_recently_used_first() {
        let mut registry = open_three();
        registry.focused("p-b");
        registry.focused("p-a");
        registry.send_to_background("a");
        registry.send_to_background("c");
        registry.send_to_background("b");
        // Focus order, most recent first: a, b, c.
        assert_eq!(registry.background_projects(), ["c", "b", "a"]);
        registry.shown("b");
        assert_eq!(registry.background_projects(), ["c", "a"]);
    }

    #[test]
    fn a_window_that_closes_leaves_the_background() {
        let mut registry = open_three();
        registry.send_to_background("a");
        registry.remove("a");
        assert!(!registry.has_background());
        assert!(registry.background_projects().is_empty());
    }

    #[test]
    fn the_quit_path_runs_once() {
        let mut registry = Registry::new();
        assert!(registry.begin_quit());
        assert!(!registry.begin_quit());
        registry.cancel_quit();
        assert!(registry.begin_quit());
        registry.quit_ready();
        assert!(!registry.begin_quit());
        assert_eq!(registry.quit_state(), QuitState::Ready);
    }

    #[test]
    fn exit_requests_are_decided_by_who_asked() {
        use ExitDecision::*;
        // The developer quits with windows open (Cmd+Q, Ctrl+Q, File › Quit): the quit path runs.
        assert_eq!(
            decide_exit(QuitState::Idle, None, 2, true),
            Quit { code: 0 }
        );
        assert_eq!(
            decide_exit(QuitState::Idle, None, 1, false),
            Quit { code: 0 }
        );
        // An exit asked with a code (an update to install) keeps its code.
        assert_eq!(
            decide_exit(QuitState::Idle, Some(7), 1, false),
            Quit { code: 7 }
        );
        assert_eq!(
            decide_exit(QuitState::Idle, Some(7), 0, true),
            Quit { code: 7 }
        );
        // The last window closed: Windows and Linux quit, macOS stays in the Dock.
        assert_eq!(decide_exit(QuitState::Idle, None, 0, false), Allow);
        assert_eq!(decide_exit(QuitState::Idle, None, 0, true), Stay);
        // While the quit path works another request waits; once done, the exit goes through.
        assert_eq!(decide_exit(QuitState::Running, None, 1, false), Ignore);
        assert_eq!(decide_exit(QuitState::Running, Some(3), 1, false), Ignore);
        assert_eq!(decide_exit(QuitState::Ready, Some(3), 1, false), Allow);
        assert_eq!(decide_exit(QuitState::Ready, None, 0, true), Allow);
    }
}
