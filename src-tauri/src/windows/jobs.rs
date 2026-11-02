//! The hook between the windows and the jobs of the agent (task "Lavori dell'agente").
//!
//! The window manager knows nothing about jobs. It asks this trait how many jobs work (for the
//! confirmation of a close and of a quit), tells it that a window opened (`window.OPENED`: the
//! jobs of the project that were suspended may resume) and that it closed (`window.CLOSED`: the
//! jobs of the project are suspended, omp is ended as in `requirements/tech#job`). The jobs
//! implement [`WindowJobs`] over their registry and install it with
//! [`WindowManager::set_jobs`](super::WindowManager::set_jobs); until then [`NoJobs`] answers,
//! truthfully, that no job exists.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// Why a project window opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenReason {
    /// `developer.OPEN_PROJECT`: the developer opened the folder.
    Developer,
    /// `desktop.APP_STARTED`: the project was open when Weaver last quit. Its jobs that were
    /// working resume.
    AppStart,
}

/// Why a project window closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
    /// `developer.CLOSE_WINDOW`: the project does not reopen at the next start.
    WindowClosed,
    /// `developer.QUIT_APP`: Weaver quits; the project reopens at the next start.
    AppQuit,
}

/// `window.OPENED`: a project window opened and its picture is ready.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowOpened {
    /// The project hash.
    pub project_id: String,
    /// The canonical root.
    pub root: PathBuf,
    pub reason: OpenReason,
}

pub trait WindowJobs: Send + Sync {
    /// How many jobs of the project work (the number the confirmation shows). Cheap and
    /// synchronous: it is read from the jobs' registry, never from the disk.
    fn working_jobs(&self, project_id: &str) -> usize;

    /// `window.OPENED`. Must return quickly; resuming jobs goes on in the background.
    fn window_opened(&self, event: &WindowOpened);

    /// `window.CLOSED`: suspend the jobs of the project and end their omp processes. The window
    /// closes (or Weaver exits) when the future completes, so it completes when they are ended.
    fn window_closed(&self, project_id: &str, reason: CloseReason) -> BoxFuture<()>;
}

/// No job exists.
pub struct NoJobs;

impl WindowJobs for NoJobs {
    fn working_jobs(&self, _project_id: &str) -> usize {
        0
    }

    fn window_opened(&self, _event: &WindowOpened) {}

    fn window_closed(&self, _project_id: &str, _reason: CloseReason) -> BoxFuture<()> {
        Box::pin(async {})
    }
}
