//! The sub-agents of an owner (a job, or General), shared by both: who they are, how they stand,
//! and the read-only conversation of each, built from omp's transcript files
//! (`requirements/tech#job`, "Sub-agents").
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`tree`] | [`Agents`]: the sub-agents of one owner, kept next to its conversation under its lock |
//! | [`reader`] | Incremental, blocking reads of the transcripts |
//! | [`records`] | Transcript records → entries of a conversation |
//! | [`poller`] | The 2 s loop that reads what must be read, through [`poller::Access`] |
//! | [`wire`] | [`wire::Owner`], paths, the list, the conversation and its change events |
//!
//! # How an owner uses it
//!
//! 1. Keep an [`Agents`] and a [`poller::Poller`] next to the conversation.
//! 2. When the session is open, [`Agents::locate`]; after a replayed history, [`Agents::reset`].
//! 3. After each change of the conversation, [`Agents::collect`] and send what it returns.
//! 4. Implement [`poller::Access`] (`plan` and `apply` under the owner's lock) and call
//!    [`poller::Poller::wake`] when the conversation lists sub-agents, [`poller::Poller::poke`]
//!    when [`Agents::interest`] changed.
//! 5. When the owner ends, [`Agents::interrupt`] after the conversation's own `finish`.

pub mod poller;
pub mod reader;
pub mod records;
pub mod tree;
pub mod wire;

pub use reader::Location;
pub use tree::Agents;
