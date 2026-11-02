//! [`General`]: the General of every open project, over the Machine `general-chat`.
//!
//! All state lives here, behind one lock. Everything that changes the state of a General goes
//! through [`General::dispatch`]: the pure Machine decides ([`trama_core::jobs::general`]), the
//! conversation and the record are updated, and the effects are carried out: bookkeeping here, I/O
//! by the actor of the omp process ([`super::actor`]). The events to the UI follow each change.
//!
//! A General outlives its window in memory: closing the window ends omp and keeps the conversation
//! and the record, so the window that opens again finds both. Quitting Weaver keeps the record,
//! and the conversation comes back when omp reloads the session.

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use serde_json::Value;
use tokio::sync::{mpsc, watch};
use trama_core::jobs::general::{
    self as machine, Effect, GeneralMachine, GeneralRecord, GeneralStore, State,
};
use trama_core::jobs::machine::{MessageId, PermissionEnd, RequestId};
use trama_core::paths::project_hash_of_canonical;
use trama_core::project::clock::{now_instant, Clock, SystemClock};
use trama_core::project::ids;
use trama_core::project::random::OsRandom;

use super::actor::{self, Cmd, Msg};
use super::wire::{GeneralEntries, GeneralEvents, GeneralSnapshot, GeneralState};
use crate::jobs::authority::GeneralAuthority;
use crate::jobs::conversation::{Conversation, Delivery, PermissionState};
use crate::jobs::host::Host;
use crate::{log_info, log_warn};

/// The thread that writes the records, one after the other.
enum Write {
    Record(String, GeneralRecord),
    Delete(String),
    Flush(std_mpsc::Sender<()>),
}

/// The General of one project.
struct Slot {
    root: PathBuf,
    /// The record was read: the General takes messages.
    ready: bool,
    machine: GeneralMachine,
    conversation: Conversation,
    /// The conversation in memory is the one of the session: a session that was kept and not
    /// reloaded yet is not shown, and the replay of its history fills the conversation.
    loaded: bool,
    /// The kept session.
    record: Option<GeneralRecord>,
    /// The identity of the live omp (`TRAMA_GENERAL`).
    identity: Option<String>,
    /// The actor of the latest start: what an older actor reports is ignored.
    generation: u64,
    channel: Option<mpsc::UnboundedSender<Msg>>,
    /// `true` once an actor has ended; every start adds one.
    done: Vec<watch::Receiver<bool>>,
    /// The texts of the messages not delivered yet.
    texts: HashMap<MessageId, String>,
    next_message: MessageId,
    /// omp's last error lines after an exit.
    exit_details: Option<String>,
    /// The conversation was replaced: the UI fetches it again.
    reset: bool,
    last_state: Option<GeneralState>,
}

impl Slot {
    fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            ready: false,
            machine: GeneralMachine::new(),
            conversation: Conversation::general(),
            loaded: true,
            record: None,
            identity: None,
            generation: 0,
            channel: None,
            done: Vec::new(),
            texts: HashMap::new(),
            next_message: 1,
            exit_details: None,
            reset: false,
            last_state: None,
        }
    }

    fn state(&self, project_id: &str) -> GeneralState {
        GeneralState {
            project_id: project_id.to_owned(),
            working: self.machine.state() == State::Working,
            starting: self.machine.starting(),
            kept: self.record.is_some() && !self.loaded,
        }
    }

    fn send(&self, cmd: Cmd) {
        if let Some(channel) = &self.channel {
            let _ = channel.send(Msg::Cmd(cmd));
        }
    }
}

pub struct General {
    pub(crate) host: Arc<dyn Host>,
    events: Arc<dyn GeneralEvents>,
    store: GeneralStore,
    inner: Mutex<HashMap<String, Slot>>,
    writer: std_mpsc::Sender<Write>,
    clock: Arc<dyn Clock>,
    me: Weak<General>,
}

impl General {
    pub fn new(host: Arc<dyn Host>, events: Arc<dyn GeneralEvents>) -> Arc<General> {
        Self::with_clock(host, events, Arc::new(SystemClock))
    }

    pub fn with_clock(
        host: Arc<dyn Host>,
        events: Arc<dyn GeneralEvents>,
        clock: Arc<dyn Clock>,
    ) -> Arc<General> {
        let store = GeneralStore::new(host.dirs());
        let (writer, queue) = std_mpsc::channel::<Write>();
        let thread_store = store.clone();
        std::thread::Builder::new()
            .name("weaver-general-record".into())
            .spawn(move || {
                for write in queue {
                    match write {
                        Write::Record(project, record) => {
                            if let Err(error) = thread_store.write(&project, &record) {
                                log_warn!("the record of the General of {project} could not be written: {error}");
                            }
                        }
                        Write::Delete(project) => {
                            if let Err(error) = thread_store.delete(&project) {
                                log_warn!("the record of the General of {project} could not be deleted: {error}");
                            }
                        }
                        Write::Flush(done) => {
                            let _ = done.send(());
                        }
                    }
                }
            })
            .expect("the thread of the General records starts");
        Arc::new_cyclic(|me| General {
            host,
            events,
            store,
            inner: Mutex::new(HashMap::new()),
            writer,
            clock,
            me: me.clone(),
        })
    }

    pub(crate) fn arc(&self) -> Arc<General> {
        self.me
            .upgrade()
            .expect("the General lives as long as the app")
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, Slot>> {
        self.inner.lock().expect("general poisoned")
    }

    pub fn host(&self) -> &Arc<dyn Host> {
        &self.host
    }

    /// Waits until every record written so far is on the disk.
    pub fn flush(&self) {
        let (done, wait) = std_mpsc::channel();
        if self.writer.send(Write::Flush(done)).is_ok() {
            let _ = wait.recv();
        }
    }

    // -----------------------------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------------------------

    /// The conversation and the state of the project's General.
    pub fn snapshot(&self, project_id: &str) -> Result<GeneralSnapshot, String> {
        let inner = self.lock();
        let slot = inner
            .get(project_id)
            .ok_or_else(|| "General is not open for this project".to_owned())?;
        Ok(GeneralSnapshot {
            state: slot.state(project_id),
            seq: slot.conversation.seq(),
            entries: slot.conversation.entries().to_vec(),
        })
    }

    /// The kept session of the project, if any.
    pub fn record_of(&self, project_id: &str) -> Option<GeneralRecord> {
        self.lock()
            .get(project_id)
            .and_then(|slot| slot.record.clone())
    }

    // -----------------------------------------------------------------------------------------
    // The window
    // -----------------------------------------------------------------------------------------

    /// `window.OPENED`: reads the record of the project (the first time in this run). A kept
    /// session is not reloaded now: the first message does it, as the Machine says.
    pub async fn open(&self, project_id: &str, root: &Path) {
        if self.lock().get(project_id).is_some_and(|slot| slot.ready) {
            self.publish_state(project_id);
            return;
        }
        let (store, hash) = (self.store.clone(), project_id.to_owned());
        let record = match tauri::async_runtime::spawn_blocking(move || store.read(&hash)).await {
            Ok(Ok(record)) => record,
            Ok(Err(error)) => {
                log_warn!("the record of the General of {project_id} is ignored: {error}");
                None
            }
            Err(_) => None,
        };
        {
            let mut inner = self.lock();
            let slot = inner
                .entry(project_id.to_owned())
                .or_insert_with(|| Slot::new(root));
            slot.root = root.to_path_buf();
            slot.loaded = record.is_none();
            slot.record = record;
            slot.ready = true;
            self.emit(project_id, slot);
        }
        log_info!("the General of a project is open");
    }

    /// `window.CLOSED`: the turn is cancelled and omp ends in order; the conversation and the
    /// record stay. Completes when omp is ended.
    pub fn window_closed(&self, project_id: &str) -> impl Future<Output = ()> + Send + 'static {
        let waits = {
            let mut inner = self.lock();
            match inner.get_mut(project_id) {
                Some(slot) => {
                    self.apply(project_id, slot, machine::Event::WindowClosed);
                    slot.done.clone()
                }
                None => Vec::new(),
            }
        };
        async move {
            for mut wait in waits {
                let _ = wait.wait_for(|done| *done).await;
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // The developer
    // -----------------------------------------------------------------------------------------

    /// `developer.SEND_MESSAGE`.
    pub fn send_message(&self, project_id: &str, text: &str) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("a message cannot be empty".to_owned());
        }
        let mut inner = self.lock();
        let slot = inner
            .get_mut(project_id)
            .filter(|slot| slot.ready)
            .ok_or_else(|| "General is not open for this project".to_owned())?;
        let message = slot.next_message;
        slot.next_message += 1;
        slot.texts.insert(message, text.to_owned());
        slot.conversation
            .developer_message(message, text, Delivery::Queued);
        self.apply(project_id, slot, machine::Event::SendMessage(message));
        Ok(())
    }

    /// `developer.STOP_AGENT`.
    pub fn stop(&self, project_id: &str) -> Result<(), String> {
        self.with_slot(project_id, |this, slot| {
            this.apply(project_id, slot, machine::Event::StopAgent)
        })
    }

    /// `developer.CLEAR_GENERAL`: also during a turn, which it stops.
    pub fn clear(&self, project_id: &str) -> Result<(), String> {
        self.with_slot(project_id, |this, slot| {
            this.apply(project_id, slot, machine::Event::Clear)
        })
    }

    /// `developer.ANSWER_PERMISSION`: `option` is one of the options of the request.
    pub fn answer_permission(
        &self,
        project_id: &str,
        request: RequestId,
        option: &str,
    ) -> Result<(), String> {
        self.with_slot(project_id, |this, slot| {
            let name = slot
                .conversation
                .option_name(request, option)
                .ok_or_else(|| "this permission has no such option".to_owned())?;
            if !slot.conversation.asking().contains(&request) {
                return Err("This permission is no longer asked.".to_owned());
            }
            slot.conversation
                .close_permission(request, PermissionState::Chosen { name });
            slot.send(Cmd::AnswerPermission {
                request,
                option: option.to_owned(),
            });
            this.apply(
                project_id,
                slot,
                machine::Event::PermissionAnswered(request),
            );
            Ok(())
        })?
    }

    fn with_slot<T>(
        &self,
        project_id: &str,
        f: impl FnOnce(&General, &mut Slot) -> T,
    ) -> Result<T, String> {
        let mut inner = self.lock();
        let slot = inner
            .get_mut(project_id)
            .ok_or_else(|| "General is not open for this project".to_owned())?;
        Ok(f(self, slot))
    }

    // -----------------------------------------------------------------------------------------
    // The Machine
    // -----------------------------------------------------------------------------------------

    /// Applies an event to the Machine and carries out what it asks.
    fn apply(&self, project_id: &str, slot: &mut Slot, event: machine::Event) {
        let effects = slot.machine.handle(event);
        self.carry_out(project_id, slot, effects);
        self.emit(project_id, slot);
    }

    /// What an actor reports. An actor that is not the latest one is ignored.
    pub(crate) fn dispatch(&self, project_id: &str, generation: u64, event: machine::Event) {
        let mut inner = self.lock();
        let Some(slot) = inner
            .get_mut(project_id)
            .filter(|slot| slot.generation == generation)
        else {
            return;
        };
        self.apply(project_id, slot, event);
    }

    fn carry_out(&self, project_id: &str, slot: &mut Slot, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::StartOmp => self.start_actor(project_id, slot),
                Effect::Steer(message) | Effect::FollowUp(message) => {
                    let follow_up = matches!(effect, Effect::FollowUp(_));
                    let text = slot.texts.get(&message).cloned().unwrap_or_default();
                    slot.send(Cmd::Steer {
                        message,
                        text,
                        follow_up,
                    });
                }
                Effect::Prompt(messages) => {
                    let text = messages
                        .iter()
                        .filter_map(|m| slot.texts.get(m))
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    slot.send(Cmd::Prompt { text });
                }
                Effect::Delivered(message) => {
                    slot.conversation.set_delivery(message, Delivery::Delivered);
                    slot.texts.remove(&message);
                }
                Effect::NotDelivered(message, reason) => {
                    slot.conversation
                        .set_delivery(message, Delivery::NotDelivered { reason });
                    slot.texts.remove(&message);
                }
                Effect::ClosePermissions(how) => {
                    let to = match how {
                        PermissionEnd::Cancelled => PermissionState::Cancelled,
                        PermissionEnd::Void => PermissionState::Void,
                    };
                    for request in slot.conversation.asking() {
                        slot.conversation.close_permission(request, to.clone());
                    }
                    slot.send(Cmd::ClosePermissions);
                }
                Effect::Cancel => slot.send(Cmd::Cancel),
                Effect::ShutDown { turn_running } => {
                    slot.identity = None;
                    slot.send(Cmd::ShutDown { turn_running });
                }
                Effect::Clear { turn_running } => {
                    slot.identity = None;
                    slot.send(Cmd::ShutDown { turn_running });
                    slot.conversation = Conversation::general();
                    slot.loaded = true;
                    slot.record = None;
                    slot.texts.clear();
                    slot.exit_details = None;
                    slot.reset = true;
                    let _ = self.writer.send(Write::Delete(project_id.to_owned()));
                }
                Effect::TurnFailed(reason) => slot.conversation.turn_failed(&reason),
                Effect::Stopped(reason) => {
                    slot.identity = None;
                    let details = slot.exit_details.take();
                    slot.conversation.omp_stopped(&reason, details);
                }
            }
        }
    }

    /// Starts the actor of a new omp.
    fn start_actor(&self, project_id: &str, slot: &mut Slot) {
        slot.generation += 1;
        let generation = slot.generation;
        let (tx, rx) = mpsc::unbounded_channel();
        let (done_tx, done_rx) = watch::channel(false);
        slot.channel = Some(tx.clone());
        slot.done.retain(|done| !*done.borrow());
        slot.done.push(done_rx);
        slot.identity = None;
        slot.exit_details = None;
        let general = self.arc();
        let project = project_id.to_owned();
        tauri::async_runtime::spawn(async move {
            actor::run(general.clone(), project.clone(), generation, tx, rx).await;
            general.actor_ended(&project, generation);
            let _ = done_tx.send(true);
        });
    }

    fn actor_ended(&self, project_id: &str, generation: u64) {
        let mut inner = self.lock();
        if let Some(slot) = inner
            .get_mut(project_id)
            .filter(|slot| slot.generation == generation)
        {
            slot.channel = None;
            slot.identity = None;
        }
    }

    /// Tells the UI what changed: the entries first, then the state.
    fn emit(&self, project_id: &str, slot: &mut Slot) {
        let changes = slot.conversation.take_changes();
        let reset = std::mem::take(&mut slot.reset);
        if !changes.is_empty() || reset {
            self.events.entries(&GeneralEntries {
                project_id: project_id.to_owned(),
                seq: slot.conversation.seq(),
                reset,
                changes,
            });
        }
        let state = slot.state(project_id);
        if slot.last_state.as_ref() != Some(&state) {
            slot.last_state = Some(state.clone());
            self.events.state(&state);
        }
    }

    fn publish_state(&self, project_id: &str) {
        let mut inner = self.lock();
        if let Some(slot) = inner.get_mut(project_id) {
            slot.last_state = None;
            self.emit(project_id, slot);
        }
    }

    // -----------------------------------------------------------------------------------------
    // What the actor needs
    // -----------------------------------------------------------------------------------------

    /// The identity of the omp that starts now, the root, and the session to reload. `None` when
    /// the actor is not the latest one.
    pub(crate) fn launch_facts(
        &self,
        project_id: &str,
        generation: u64,
    ) -> Option<(String, PathBuf, Option<String>)> {
        let mut inner = self.lock();
        let taken: Vec<String> = inner
            .values()
            .filter_map(|slot| slot.identity.clone())
            .collect();
        let slot = inner
            .get_mut(project_id)
            .filter(|slot| slot.generation == generation)?;
        let identity = ids::draw_identity(&mut OsRandom, &|candidate| {
            taken.iter().any(|t| t == candidate)
        });
        slot.identity = Some(identity.clone());
        Some((
            identity,
            slot.root.clone(),
            slot.record.as_ref().map(|record| record.session_id.clone()),
        ))
    }

    /// Runs `f` on the conversation and tells the UI what it changed.
    pub(crate) fn with_conversation<T>(
        &self,
        project_id: &str,
        generation: u64,
        f: impl FnOnce(&mut Conversation) -> T,
    ) -> Option<T> {
        let mut inner = self.lock();
        let slot = inner
            .get_mut(project_id)
            .filter(|slot| slot.generation == generation)?;
        let value = f(&mut slot.conversation);
        self.emit(project_id, slot);
        Some(value)
    }

    pub(crate) fn set_exit_details(
        &self,
        project_id: &str,
        generation: u64,
        details: Option<String>,
    ) {
        if let Some(slot) = self
            .lock()
            .get_mut(project_id)
            .filter(|slot| slot.generation == generation)
        {
            slot.exit_details = details;
        }
    }

    /// `omp.PERMISSION_REQUESTED`: the card is shown and the Machine stays in Working. `false` when
    /// no turn is in progress: the actor cancels the request.
    pub(crate) fn permission_requested(
        &self,
        project_id: &str,
        generation: u64,
        request: RequestId,
        tool_call: &Value,
        options: &Value,
    ) -> bool {
        let mut inner = self.lock();
        let Some(slot) = inner
            .get_mut(project_id)
            .filter(|slot| slot.generation == generation)
        else {
            return false;
        };
        if !slot.machine.accepts_agent_calls() {
            return false;
        }
        slot.conversation.permission(request, tool_call, options);
        self.apply(
            project_id,
            slot,
            machine::Event::PermissionRequested(request),
        );
        true
    }

    /// omp opened its session: the history it replayed (when the kept session was reloaded)
    /// becomes the conversation under the messages already sent, the record is written, and the
    /// Machine goes on. `fallback` is why the kept session could not be reloaded and an empty one
    /// started.
    pub(crate) fn session_opened(
        &self,
        project_id: &str,
        generation: u64,
        session: &str,
        replay: Option<Conversation>,
        fallback: Option<&str>,
    ) {
        let updated = now_instant(self.clock.as_ref())
            .unwrap_or_else(|_| trama_core::project::clock::format_instant(1));
        let mut inner = self.lock();
        let Some(slot) = inner
            .get_mut(project_id)
            .filter(|slot| slot.generation == generation)
        else {
            return;
        };
        if let Some(mut replayed) = replay {
            if !slot.loaded {
                for entry in slot.conversation.entries() {
                    replayed.append(entry);
                }
                // The merged conversation is new as a whole: the UI fetches it.
                replayed.mark_sent();
                slot.conversation = replayed;
                slot.reset = true;
            }
        }
        slot.loaded = true;
        if let Some(reason) = fallback {
            slot.conversation.notice(&format!(
                "The conversation could not be reloaded ({reason}). A new one started."
            ));
        }
        let record = GeneralRecord {
            project: slot.root.to_string_lossy().into_owned(),
            session_id: session.to_owned(),
            updated,
        };
        slot.record = Some(record.clone());
        let _ = self
            .writer
            .send(Write::Record(project_id.to_owned(), record));
        self.apply(project_id, slot, machine::Event::SessionOpened);
    }

    // -----------------------------------------------------------------------------------------
    // The agent
    // -----------------------------------------------------------------------------------------

    /// `agent.START_JOB`: the jobs the agent started on the developer's request, or the refusal.
    fn started_jobs(
        &self,
        root: &Path,
        token: &str,
        kind: &str,
        outcome: Result<&[(String, String)], &str>,
    ) {
        let project_id = project_hash_of_canonical(root);
        let mut inner = self.lock();
        let Some(slot) = inner
            .get_mut(&project_id)
            .filter(|slot| slot.identity.as_deref() == Some(token))
        else {
            return;
        };
        match outcome {
            Ok(started) => {
                for (id, name) in started {
                    slot.conversation.job_started(id, name);
                }
            }
            Err(reason) => slot.conversation.job_refused(kind, reason),
        }
        self.apply(&project_id, slot, machine::Event::StartJob);
    }
}

impl GeneralAuthority for General {
    fn accepts(&self, root: &Path, token: &str) -> bool {
        let project_id = project_hash_of_canonical(root);
        self.lock().get(&project_id).is_some_and(|slot| {
            slot.identity.as_deref() == Some(token) && slot.machine.accepts_agent_calls()
        })
    }

    fn job_requested(
        &self,
        root: &Path,
        token: &str,
        kind: &str,
        outcome: Result<&[(String, String)], &str>,
    ) {
        self.started_jobs(root, token, kind, outcome);
    }
}

/// The entries of a conversation, for the tests.
#[cfg(test)]
impl General {
    pub(crate) fn entries(&self, project_id: &str) -> Vec<crate::jobs::conversation::Entry> {
        self.lock()
            .get(project_id)
            .map(|slot| slot.conversation.entries().to_vec())
            .unwrap_or_default()
    }

    pub(crate) fn working(&self, project_id: &str) -> bool {
        self.lock()
            .get(project_id)
            .is_some_and(|slot| slot.machine.state() == State::Working)
    }

    pub(crate) fn identity(&self, project_id: &str) -> Option<String> {
        self.lock()
            .get(project_id)
            .and_then(|slot| slot.identity.clone())
    }
}
