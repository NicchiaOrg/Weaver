//! What General tells the UI: its state, its conversation and the changes of it.

use serde::Serialize;
use ts_rs::TS;

use crate::jobs::conversation::{Changed, Entry};

/// The state of the General of a project, as the view needs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GeneralState {
    /// The project hash.
    pub project_id: String,
    /// A turn is in progress (the Machine is in Working): `Stop` shows.
    pub working: bool,
    /// omp is starting or reloading its session (or is waited for): the message that waits says
    /// `Starting omp…`.
    pub starting: bool,
    /// The window just opened and the kept conversation is coming back from omp (the Machine is
    /// in Loading and its history is not in the conversation yet): the view says
    /// `Loading conversation…` above the messages sent meanwhile.
    pub loading: bool,
    /// A kept session has not come back yet, so `Clear` stays usable although the conversation
    /// shows nothing yet.
    pub kept: bool,
}

/// The conversation as it is now, with the count of change batches it holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSnapshot {
    pub state: GeneralState,
    #[ts(type = "number")]
    pub seq: u64,
    pub entries: Vec<Entry>,
}

/// A batch of changes of the conversation. `reset` says it was rebuilt (a session reloaded, or
/// cleared): the UI fetches the snapshot again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GeneralEntries {
    pub project_id: String,
    #[ts(type = "number")]
    pub seq: u64,
    pub reset: bool,
    pub changes: Vec<Changed>,
}

/// Where General's events go: the windows (the real one) or a test.
pub trait GeneralEvents: Send + Sync + 'static {
    fn state(&self, state: &GeneralState);
    fn entries(&self, entries: &GeneralEntries);
}
