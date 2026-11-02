//! System notifications (Machine `notification`, feature `app`): the app side.
//!
//! The decisions are the core's ([`weaver_core::notification`]): whether the project window is in the
//! foreground, which text each event gets, how finished jobs group, where a click leads. This
//! module listens to the jobs ([`JobObserver`]: `job.ENDED`, and the waits of a round of questions
//! and of a permission), to the focus of the windows, and to the clicks of the system, and sends
//! through `tauri-plugin-notification`, called from Rust only.
//!
//! * **Foreground test**, at send time: the project's window exists, is visible, is not minimised
//!   and has the focus (`WindowEvent::Focused`, tracked here; `is_focused()` for a window whose
//!   events were not seen). Another focused window of Weaver does not count.
//! * **Clicks** arrive through `Notification::on_action` (`tap`): the id is looked up in the
//!   mapping of the core's [`Book`]; a mapped one brings the project's window forward showing the
//!   Jobs view (in the conversation of the job when there is one), an unmapped one or a project
//!   whose window is closed is ignored.
//! * **Withdrawing**: Linux removes the delivered notification (`remove_active`); macOS and Windows
//!   cannot, so only the mapping is withdrawn and a later click on it brings the window forward.
//!   Every mapping of a project goes when its window is destroyed.
//! * **Windows**: a click is reported only while the toast is on screen; one in the Action Center
//!   launches Weaver again and the single-instance plugin focuses the window
//!   ([`crate::windows::WindowManager::second_launch`]).
//! * There is no Weaver setting: the operating system's permission decides.
//!
//! Nothing here changes a project, a job or a round, and nothing the agent wrote reaches a notification.

use std::sync::{Arc, Mutex, MutexGuard};

use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_notification::{NotificationExt, PermissionState};
use weaver_core::notification::{
    decide_end, decide_wait, in_foreground, kind_of_end, Book, Click, Destination, Focus, Kind,
    Notice, WindowFacts,
};

use crate::jobs::wire::{JobInfo, JobObserver};
use crate::jobs::Jobs;
use crate::log_warn;
use crate::pictures::project_of_label;
use crate::windows::manager::{ShowRequest, ShowView};
use crate::windows::registry::label_of;

/// What the system says of a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemWindow {
    pub visible: bool,
    pub minimized: bool,
    /// `is_focused()`.
    pub focused: bool,
}

/// Everything the service asks of the system, so that the tests drive it with a fake.
pub trait Desktop: Send + Sync + 'static {
    /// The window `label`; `None` when it does not exist.
    fn window(&self, label: &str) -> Option<SystemWindow>;
    /// Sends the notification (replacing the one with the same id).
    fn show(&self, notice: &Notice);
    /// Whether the system can remove a delivered notification (Linux).
    fn can_remove(&self) -> bool;
    /// Removes delivered notifications; only called when [`Desktop::can_remove`].
    fn remove(&self, ids: &[i32]);
    /// `notification.CLICKED`: the window of the project comes forward, showing what the click leads to.
    fn bring_forward(&self, click: &Click);
}

struct State {
    book: Book,
    focus: Focus,
}

/// The notifications of the app.
pub struct Notifications {
    desktop: Arc<dyn Desktop>,
    state: Mutex<State>,
}

/// The name of the project in a notification: the folder name.
fn project_name(job: &JobInfo) -> String {
    job.root.file_name().map_or_else(
        || job.project_id.clone(),
        |name| name.to_string_lossy().into_owned(),
    )
}

impl Notifications {
    pub fn new(desktop: Arc<dyn Desktop>) -> Arc<Self> {
        let state = State {
            book: Book::new(desktop.can_remove()),
            focus: Focus::default(),
        };
        Arc::new(Self {
            desktop,
            state: Mutex::new(state),
        })
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// `WindowEvent::Focused`.
    pub fn focus_changed(&self, label: &str, focused: bool) {
        self.state().focus.changed(label, focused);
    }

    /// The window was destroyed: its focus is forgotten and every mapping of its project is dropped.
    pub fn window_destroyed(&self, label: &str) {
        let mut state = self.state();
        state.focus.destroyed(label);
        if let Some(project_id) = project_of_label(label) {
            state.book.project_closed(project_id);
        }
    }

    /// The project's window exists and is not in the foreground now. A project without a window
    /// has nobody to notify.
    fn in_background(&self, project_id: &str) -> bool {
        let label = label_of(project_id);
        // Asked before the lock is taken: the system answers on another thread.
        let Some(system) = self.desktop.window(&label) else {
            return false;
        };
        let focused = self.state().focus.resolve(&label, system.focused);
        !in_foreground(WindowFacts {
            visible: system.visible,
            minimized: system.minimized,
            focused,
        })
    }

    /// Takes the notifications of `ids` out of the system's list, where it can.
    fn withdrawn(&self, ids: &[i32]) {
        if !ids.is_empty() && self.desktop.can_remove() {
            self.desktop.remove(ids);
        }
    }

    /// A job waits for the developer: shown when the project's window is in the background.
    fn waiting(&self, job: &JobInfo, make: impl FnOnce(&mut Book, &str) -> Notice) {
        if !decide_wait(!self.in_background(&job.project_id)) {
            return;
        }
        let notice = make(&mut self.state().book, &project_name(job));
        self.desktop.show(&notice);
    }

    /// `desktop.NOTIFICATION_CLICKED`: a mapped id brings its project's window forward.
    pub fn clicked(&self, id: i32) {
        let click = self.state().book.clicked(id);
        if let Some(click) = click {
            self.desktop.bring_forward(&click);
        }
    }
}

impl JobObserver for Notifications {
    fn job_ended(&self, job: &JobInfo) {
        // What the job announced about waiting is no longer true, whatever its end.
        let ids = self.state().book.job_ended(&job.project_id, &job.id);
        self.withdrawn(&ids);
        if kind_of_end(job.state).is_none() {
            return;
        }
        let Some(kind) = decide_end(job.state, !self.in_background(&job.project_id)) else {
            return;
        };
        let project = project_name(job);
        let notice = {
            let mut state = self.state();
            match kind {
                Kind::Finished => {
                    state
                        .book
                        .finished(&job.project_id, &project, &job.id, &job.name)
                }
                Kind::Failed => state
                    .book
                    .failed(&job.project_id, &project, &job.id, &job.name),
                Kind::Question | Kind::Permission => return,
            }
        };
        self.desktop.show(&notice);
    }

    fn round_opened(&self, job: &JobInfo) {
        self.waiting(job, |book, project| {
            book.question(&job.project_id, project, &job.id, &job.name)
        });
    }

    fn round_closed(&self, job: &JobInfo) {
        let ids = self.state().book.round_closed(&job.project_id, &job.id);
        self.withdrawn(&ids);
    }

    fn permission_asked(&self, job: &JobInfo, request: u64) {
        self.waiting(job, |book, project| {
            book.permission(&job.project_id, project, &job.id, &job.name, request)
        });
    }

    fn permission_answered(&self, job: &JobInfo, request: u64) {
        let ids = self
            .state()
            .book
            .permission_answered(&job.project_id, &job.id, request);
        self.withdrawn(&ids);
    }
}

// --- the real desktop ----------------------------------------------------------------------------

struct TauriDesktop {
    app: AppHandle,
}

impl Desktop for TauriDesktop {
    fn window(&self, label: &str) -> Option<SystemWindow> {
        let window = self.app.get_webview_window(label)?;
        Some(SystemWindow {
            visible: window.is_visible().unwrap_or(false),
            minimized: window.is_minimized().unwrap_or(false),
            focused: window.is_focused().unwrap_or(false),
        })
    }

    fn show(&self, notice: &Notice) {
        let notification = self.app.notification();
        // The system's permission decides; the desktop plugins always answer `Granted`.
        if let Ok(state) = notification.permission_state() {
            if state != PermissionState::Granted {
                return;
            }
        }
        if let Err(error) = notification
            .builder()
            .id(notice.id)
            .title(&notice.title)
            .body(notice.message)
            .show()
        {
            log_warn!("a notification could not be shown: {error}");
        }
    }

    fn can_remove(&self) -> bool {
        cfg!(target_os = "linux")
    }

    fn remove(&self, ids: &[i32]) {
        if let Err(error) = self.app.notification().remove_active(ids.to_vec()) {
            log_warn!("a notification could not be removed: {error}");
        }
    }

    fn bring_forward(&self, click: &Click) {
        let Some(manager) = crate::windows::manager_of(&self.app) else {
            return;
        };
        let show = match &click.destination {
            Destination::WindowOnly => None,
            Destination::Jobs { job_id } => Some(ShowRequest {
                project_id: click.project_id.clone(),
                view: ShowView::Jobs,
                job_id: job_id.clone(),
            }),
        };
        // A project whose window is closed has nothing to show: the click is ignored.
        manager.focus_project(&click.project_id, show);
    }
}

/// Creates the service, listens to the clicks of the system and to the jobs. Call from the setup
/// of the app, after the jobs are installed.
pub fn install(app: &AppHandle, jobs: &Arc<Jobs>) -> Arc<Notifications> {
    let notifications = Notifications::new(Arc::new(TauriDesktop { app: app.clone() }));
    app.manage(notifications.clone());
    let clicks = notifications.clone();
    let registered = app.notification().on_action(move |action| {
        if action.action_id() != "tap" {
            return;
        }
        if let Some(notification) = action.notification() {
            clicks.clicked(notification.id());
        }
    });
    if let Err(error) = registered {
        log_warn!("clicks on notifications are not heard: {error}");
    }
    jobs.observe(notifications.clone());
    notifications
}

/// The focus and the destruction of the windows, from the run loop.
pub fn on_window_event(app: &AppHandle, label: &str, event: &WindowEvent) {
    let Some(notifications) = app.try_state::<Arc<Notifications>>() else {
        return;
    };
    match event {
        WindowEvent::Focused(focused) => notifications.focus_changed(label, *focused),
        WindowEvent::Destroyed => notifications.window_destroyed(label),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use weaver_core::jobs::{JobKind, JobState};

    use super::*;

    const ACME: &str = "aaaa0000aaaa0000";
    const OTHER: &str = "bbbb1111bbbb1111";

    #[derive(Default)]
    struct Fake {
        removable: bool,
        windows: Mutex<HashMap<String, SystemWindow>>,
        shown: Mutex<Vec<(i32, String, String)>>,
        removed: Mutex<Vec<i32>>,
        forward: Mutex<Vec<Click>>,
    }

    impl Fake {
        fn new(removable: bool) -> Arc<Self> {
            Arc::new(Self {
                removable,
                ..Default::default()
            })
        }

        fn set_window(&self, project: &str, visible: bool, minimized: bool, focused: bool) {
            self.windows.lock().unwrap().insert(
                label_of(project),
                SystemWindow {
                    visible,
                    minimized,
                    focused,
                },
            );
        }

        fn shown(&self) -> Vec<(i32, String, String)> {
            self.shown.lock().unwrap().clone()
        }
    }

    impl Desktop for Arc<Fake> {
        fn window(&self, label: &str) -> Option<SystemWindow> {
            self.windows.lock().unwrap().get(label).copied()
        }
        fn show(&self, notice: &Notice) {
            self.shown.lock().unwrap().push((
                notice.id,
                notice.title.clone(),
                notice.message.to_owned(),
            ));
        }
        fn can_remove(&self) -> bool {
            self.removable
        }
        fn remove(&self, ids: &[i32]) {
            self.removed.lock().unwrap().extend_from_slice(ids);
        }
        fn bring_forward(&self, click: &Click) {
            self.forward.lock().unwrap().push(click.clone());
        }
    }

    fn job(project: &str, id: &str, name: &str, state: JobState) -> JobInfo {
        JobInfo {
            id: id.to_owned(),
            project_id: project.to_owned(),
            root: PathBuf::from(format!(
                "/work/{}",
                if project == ACME { "acme" } else { "other" }
            )),
            kind: JobKind::Run,
            name: name.to_owned(),
            state,
            target: None,
            reason: None,
            announcements: Vec::new(),
            empty_model_at_start: false,
        }
    }

    fn service(removable: bool) -> (Arc<Notifications>, Arc<Fake>) {
        let fake = Fake::new(removable);
        fake.set_window(ACME, true, false, false);
        fake.set_window(OTHER, true, false, false);
        (Notifications::new(Arc::new(fake.clone())), fake)
    }

    #[test]
    fn with_the_window_in_the_foreground_nothing_is_shown() {
        let (service, fake) = service(true);
        fake.set_window(ACME, true, false, true);
        service.job_ended(&job(ACME, "j1", "Build", JobState::Done));
        service.job_ended(&job(ACME, "j2", "Scan", JobState::Failed));
        service.round_opened(&job(ACME, "j3", "Import", JobState::Waiting));
        service.permission_asked(&job(ACME, "j3", "Import", JobState::Waiting), 1);
        assert!(fake.shown().is_empty());
    }

    #[test]
    fn in_the_background_the_exact_texts_appear_without_anything_the_agent_said() {
        let (service, fake) = service(true);
        service.job_ended(&job(ACME, "j1", "Build", JobState::Done));
        service.job_ended(&job(ACME, "j2", "Scan", JobState::Failed));
        service.round_opened(&job(ACME, "j3", "Import", JobState::Waiting));
        service.permission_asked(&job(ACME, "j4", "Run T3", JobState::Waiting), 9);
        let texts: Vec<(String, String)> = fake
            .shown()
            .into_iter()
            .map(|(_, title, message)| (title, message))
            .collect();
        assert_eq!(
            texts,
            [
                ("acme – Build finished", "Weaver is waiting for you."),
                (
                    "acme – Scan stopped",
                    "The agent stopped before finishing its work."
                ),
                (
                    "acme – Import asks a question",
                    "The agent is waiting for your answers."
                ),
                (
                    "acme – Run T3 asks permission",
                    "The agent is waiting for your approval."
                ),
            ]
            .map(|(title, message)| (title.to_owned(), message.to_owned()))
        );
    }

    #[test]
    fn a_job_the_developer_stopped_never_notifies_and_another_window_having_the_focus_does_not_count(
    ) {
        let (service, fake) = service(true);
        service.job_ended(&job(ACME, "j1", "Build", JobState::Stopped));
        assert!(fake.shown().is_empty());
        // The window of the project lost the focus to the window of another project.
        fake.set_window(ACME, true, false, true);
        service.focus_changed(&label_of(ACME), true);
        service.focus_changed(&label_of(ACME), false);
        service.focus_changed(&label_of(OTHER), true);
        service.job_ended(&job(ACME, "j2", "Scan", JobState::Done));
        assert_eq!(
            fake.shown().len(),
            1,
            "the focus events say the project window lost the focus"
        );
    }

    #[test]
    fn a_minimised_or_hidden_window_is_in_the_background_and_a_closed_one_has_nobody_to_notify() {
        let (service, fake) = service(true);
        fake.set_window(ACME, true, true, true);
        service.job_ended(&job(ACME, "j1", "Build", JobState::Done));
        fake.set_window(ACME, false, false, true);
        service.job_ended(&job(ACME, "j2", "Scan", JobState::Done));
        assert_eq!(fake.shown().len(), 2);
        fake.windows.lock().unwrap().clear();
        service.job_ended(&job(ACME, "j3", "Run", JobState::Done));
        assert_eq!(fake.shown().len(), 2);
    }

    #[test]
    fn a_second_finished_job_replaces_the_notification_of_the_project_with_the_count() {
        let (service, fake) = service(true);
        service.job_ended(&job(ACME, "j1", "Build", JobState::Done));
        service.job_ended(&job(ACME, "j2", "Scan", JobState::Done));
        service.job_ended(&job(OTHER, "j3", "Build", JobState::Done));
        let shown = fake.shown();
        assert_eq!(shown[0].0, shown[1].0, "the same id replaces the shown one");
        assert_eq!(shown[1].1, "acme – 2 jobs finished");
        assert_ne!(shown[2].0, shown[1].0);
        assert_eq!(shown[2].1, "other – Build finished");
    }

    #[test]
    fn a_click_brings_the_window_forward_showing_the_job_and_a_second_click_does_nothing() {
        let (service, fake) = service(true);
        service.job_ended(&job(ACME, "j1", "Scan", JobState::Failed));
        let id = fake.shown()[0].0;
        service.clicked(id + 50);
        assert!(
            fake.forward.lock().unwrap().is_empty(),
            "an id that is not mapped is ignored"
        );
        service.clicked(id);
        service.clicked(id);
        let forward = fake.forward.lock().unwrap().clone();
        assert_eq!(
            forward,
            [Click {
                project_id: ACME.to_owned(),
                destination: Destination::Jobs {
                    job_id: Some("j1".to_owned())
                }
            }]
        );
    }

    #[test]
    fn a_closed_round_an_answered_permission_and_an_ended_job_withdraw_their_notifications() {
        let (service, fake) = service(true);
        let waiting = job(ACME, "j1", "Import", JobState::Waiting);
        service.round_opened(&waiting);
        service.permission_asked(&waiting, 4);
        service.permission_asked(&waiting, 5);
        let ids: Vec<i32> = fake.shown().iter().map(|(id, _, _)| *id).collect();
        service.round_closed(&job(ACME, "j1", "Import", JobState::Running));
        service.permission_answered(&job(ACME, "j1", "Import", JobState::Waiting), 4);
        assert_eq!(*fake.removed.lock().unwrap(), vec![ids[0], ids[1]]);
        service.job_ended(&job(ACME, "j1", "Import", JobState::Stopped));
        assert_eq!(
            *fake.removed.lock().unwrap(),
            vec![ids[0], ids[1], ids[2]],
            "the wait of an ended job is no longer true"
        );
        assert_eq!(
            fake.shown().len(),
            3,
            "a job the developer stopped adds nothing"
        );
    }

    #[test]
    fn where_the_system_cannot_remove_one_a_withdrawn_notification_only_brings_the_window_forward()
    {
        let (service, fake) = service(false);
        service.round_opened(&job(ACME, "j1", "Import", JobState::Waiting));
        let id = fake.shown()[0].0;
        service.round_closed(&job(ACME, "j1", "Import", JobState::Running));
        assert!(
            fake.removed.lock().unwrap().is_empty(),
            "nothing can be removed here"
        );
        service.clicked(id);
        assert_eq!(
            *fake.forward.lock().unwrap(),
            [Click {
                project_id: ACME.to_owned(),
                destination: Destination::WindowOnly
            }]
        );
    }

    #[test]
    fn closing_the_window_drops_its_mappings_and_a_later_click_is_ignored() {
        let (service, fake) = service(true);
        service.job_ended(&job(ACME, "j1", "Scan", JobState::Failed));
        service.job_ended(&job(OTHER, "j2", "Scan", JobState::Failed));
        let (acme, other) = (fake.shown()[0].0, fake.shown()[1].0);
        service.window_destroyed(&label_of(ACME));
        service.clicked(acme);
        assert!(fake.forward.lock().unwrap().is_empty());
        service.clicked(other);
        assert_eq!(fake.forward.lock().unwrap().len(), 1);
    }
}
