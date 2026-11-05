//! The loop that reads the transcripts of the sub-agents of one owner, every 2 s while some must be
//! read, and stops when none must (`requirements/tech#job`, "Sub-agents").
//!
//! The owner (a job, or General) lends it access to its [`super::Agents`] through [`Access`]; the
//! reading itself is blocking file I/O on a blocking thread, outside every lock of the owner.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;

use super::reader::{self, Done};
use super::tree::Plan;

/// How often the transcripts are read while records may come.
pub const POLL_EVERY: Duration = if cfg!(test) {
    Duration::from_millis(30)
} else {
    Duration::from_secs(2)
};

/// What the poller needs from the owner of the sub-agents. Both calls take the owner's lock for
/// as long as they run, and nothing else.
pub trait Access: Send + Sync + 'static {
    /// What to read now; `None` when nothing must be read, or the owner is gone.
    fn plan(&self) -> Option<Plan>;
    /// Takes in what was read (and tells the UI what changed).
    fn apply(&self, generation: u64, done: Vec<Done>);
}

/// One loop per owner.
#[derive(Default)]
pub struct Poller {
    running: AtomicBool,
    notify: Notify,
}

impl Poller {
    /// Starts the loop unless it runs. `access` is only built when it starts.
    pub fn wake(self: &Arc<Self>, access: impl FnOnce() -> Arc<dyn Access>) {
        if !self.running.swap(true, Ordering::SeqCst) {
            tauri::async_runtime::spawn(self.clone().run(access()));
        }
    }

    /// Reads now, without waiting for the next turn of the loop: what is on screen changed.
    pub fn poke(self: &Arc<Self>, access: impl FnOnce() -> Arc<dyn Access>) {
        self.notify.notify_one();
        self.wake(access);
    }

    async fn run(self: Arc<Self>, access: Arc<dyn Access>) {
        loop {
            let Some(plan) = access.plan() else {
                self.running.store(false, Ordering::SeqCst);
                // A wake between the plan and the store found the loop running and started
                // nothing: look once more, and take the loop back if there is work.
                if access.plan().is_none() || self.running.swap(true, Ordering::SeqCst) {
                    return;
                }
                continue;
            };
            let generation = plan.generation;
            let done = tauri::async_runtime::spawn_blocking(move || {
                plan.jobs
                    .iter()
                    .map(|job| reader::read(&plan.location, plan.main.clone(), job))
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap_or_default();
            access.apply(generation, done);
            let _ = tokio::time::timeout(POLL_EVERY, self.notify.notified()).await;
        }
    }
}
