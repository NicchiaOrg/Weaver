//! The actor of a General: the one task that owns its omp process and its ACP session.
//!
//! It does the I/O the Machine `general-chat` asks for ([`super::General`] sends it [`Cmd`]s) and
//! turns what happens (omp's messages, the end of a turn, the exit of the process) into events of
//! the Machine. Everything arrives on one channel, so it is handled one at a time, in order. The
//! process, the JSON-RPC and the ACP calls are the ones of the jobs; what differs is what is
//! started (`--append-system-prompt` instead of an overlay), the identity (`WEAVER_GENERAL`), the
//! session that is reloaded when a record exists (an empty one starts when the load fails), and
//! the forms, declined with a text that asks the agent to ask in its reply.
//!
//! Nothing here ends omp by time: it ends on a fact omp or the developer produced.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use weaver_core::jobs::general::{self as machine, Event};
use weaver_core::jobs::launch;
use weaver_core::jobs::machine::{MessageId, RequestId, SteeringOutcome};
use weaver_core::jobs::subagents::{self, SubAgentState};
use weaver_core::omp::machine::ReadyOmp;

use super::service::General;
use crate::jobs::conversation::Conversation;
use crate::jobs::proc::{ExitInfo, OmpProcess, SpawnSpec};
use crate::jobs::rpc::{Incoming, RpcError};
use crate::jobs::session::{self, OpenError, TurnEnd, OPEN_LIMIT};

/// What a form or an ask of omp is declined with in General.
pub const DECLINE_TEXT: &str = "In General, ask the developer in your reply, in plain text, and wait for the answer. Do not use a form or a question tool.";

/// What the Machine asks the actor to do.
#[derive(Debug)]
pub(crate) enum Cmd {
    Steer {
        message: MessageId,
        text: String,
        follow_up: bool,
    },
    Prompt {
        text: String,
    },
    /// `session/cancel`: the turn in flight ends; omp stays.
    Cancel,
    /// Every permission request still pending is answered `cancelled`.
    ClosePermissions,
    AnswerPermission {
        request: RequestId,
        option: String,
    },
    /// End omp in order and finish.
    ShutDown {
        turn_running: bool,
    },
}

/// The session omp opened, and why the kept one could not be reloaded when it could not.
pub(crate) struct Opened {
    session: String,
    fell_back: Option<String>,
}

/// Everything the actor waits for, on one channel.
pub(crate) enum Msg {
    Cmd(Cmd),
    OmpReady(Arc<ReadyOmp>),
    Spawned(Result<(OmpProcess, UnboundedReceiver<Incoming>), String>),
    Opened(Result<Opened, OpenError>),
    /// `initialize` answered, and the kept session is to be loaded.
    Initialized(String),
    /// The limit of the opening passed; carries the id of the load it was set for.
    LoadTimeout(u64),
    Incoming(Incoming),
    Exited(ExitInfo),
    SteerDone(MessageId, SteeringOutcome),
    Tick,
    Background(Vec<(String, String, SubAgentState)>),
}

/// How often the background sub-agents are looked up in omp's session files.
const BACKGROUND_EVERY: Duration = Duration::from_secs(2);

pub(crate) async fn run(
    general: Arc<General>,
    project: String,
    generation: u64,
    tx: UnboundedSender<Msg>,
    mut rx: UnboundedReceiver<Msg>,
) {
    let mut actor = Actor {
        general,
        project,
        generation,
        tx,
        process: None,
        session: None,
        prompts: HashSet::new(),
        pending: HashMap::new(),
        next_request: 1,
        asks: false,
        reload: None,
        loading: false,
        load: None,
        replay: Conversation::general(),
        opened_at: Instant::now(),
        exit_reported: false,
        spawning: false,
        closing: false,
        turn_running: false,
        started: false,
        poll_scheduled: false,
        finished: false,
        root: PathBuf::new(),
    };
    actor.begin();
    while let Some(message) = rx.recv().await {
        actor.handle(message).await;
        if actor.finished {
            break;
        }
    }
    actor.cleanup();
}

struct Actor {
    general: Arc<General>,
    project: String,
    generation: u64,
    tx: UnboundedSender<Msg>,
    process: Option<Arc<OmpProcess>>,
    session: Option<String>,
    /// The prompts sent and not answered yet.
    prompts: HashSet<u64>,
    /// The JSON-RPC ids of the permission requests that wait for the developer.
    pending: HashMap<RequestId, Value>,
    next_request: RequestId,
    /// `--approval-mode always-ask`: permissions go to the UI.
    asks: bool,
    /// The kept session this omp reloads.
    reload: Option<String>,
    /// The session is being reloaded: what omp sends is the replayed history.
    loading: bool,
    /// The request id and the kept session of the `session/load` sent and not answered yet.
    load: Option<(u64, String)>,
    replay: Conversation,
    opened_at: Instant,
    exit_reported: bool,
    /// The process is being spawned: `Msg::Spawned` is on its way.
    spawning: bool,
    /// `Cmd::ShutDown` came while it was spawning: it ends when it arrives.
    closing: bool,
    turn_running: bool,
    /// The session opened.
    started: bool,
    poll_scheduled: bool,
    finished: bool,
    root: PathBuf,
}

impl Actor {
    fn begin(&mut self) {
        let host = self.general.host.clone();
        let tx = self.tx.clone();
        tauri::async_runtime::spawn(async move {
            let ready = host.omp_ready().await;
            let _ = tx.send(Msg::OmpReady(ready));
        });
    }

    async fn handle(&mut self, message: Msg) {
        match message {
            Msg::Cmd(cmd) => self.command(cmd).await,
            Msg::OmpReady(ready) => {
                // The end of Waiting for omp: the kept conversation reloads without a message.
                self.dispatch(Event::OmpReady);
                // The CLI socket starts with the app: wait for it.
                let mut endpoint = self.general.host.endpoint();
                for _ in 0..600 {
                    if endpoint.is_some() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    endpoint = self.general.host.endpoint();
                }
                match endpoint {
                    Some(endpoint) => self.launch(&ready, &endpoint),
                    None => self.fail_opening("the CLI socket of Weaver did not start", None),
                }
            }
            Msg::Spawned(result) => self.spawned(result),
            Msg::Opened(result) => self.opened(result),
            Msg::Initialized(previous) => self.initialized(previous),
            Msg::LoadTimeout(id) => self.load_timeout(id),
            Msg::Incoming(incoming) => self.incoming(incoming),
            Msg::Exited(info) => self.exited(info).await,
            Msg::SteerDone(message, outcome) => {
                self.dispatch(Event::SteeringReplied(message, outcome))
            }
            Msg::Tick => self.tick(),
            Msg::Background(results) => self.background(results),
        }
    }

    fn dispatch(&self, event: Event) {
        self.general.dispatch(&self.project, self.generation, event);
    }

    fn conversation<T>(&self, f: impl FnOnce(&mut Conversation) -> T) -> Option<T> {
        self.general
            .with_conversation(&self.project, self.generation, f)
    }

    // -----------------------------------------------------------------------------------------
    // Starting
    // -----------------------------------------------------------------------------------------

    /// omp is ready: write the instruction, spawn the process.
    fn launch(&mut self, ready: &ReadyOmp, endpoint: &str) {
        if self.finished || self.process.is_some() {
            return;
        }
        let Some((identity, root, reload)) =
            self.general.launch_facts(&self.project, self.generation)
        else {
            self.finished = true;
            return;
        };
        self.opened_at = Instant::now();
        self.root = root.clone();
        self.reload = reload;
        let host = &self.general.host;
        let settings = host.settings();
        let language =
            Some(settings.language.trim().to_owned()).filter(|language| !language.is_empty());
        let instruction = machine::instruction(language.as_deref());
        let instruction_file = machine::instruction_file(host.dirs());
        let args = machine::args(&settings.models, settings.permissions, &instruction_file);
        self.asks = args
            .windows(2)
            .any(|pair| pair[0] == "--approval-mode" && pair[1] == "always-ask");
        let env = machine::env(&identity, endpoint);
        let path = host.child_path(&ready.omp);
        let spec = SpawnSpec {
            program: ready.omp.clone(),
            args,
            env: launch::child_env(host.inherited_env(), &env, &path),
            cwd: root,
            name: format!("general-{}", self.project),
        };
        self.spawning = true;
        let tx = self.tx.clone();
        tauri::async_runtime::spawn(async move {
            let spawned = tauri::async_runtime::spawn_blocking(move || {
                weaver_core::atomic::write(&instruction_file, instruction.as_bytes(), None)
                    .map_err(|error| format!("cannot write the instruction of Chat: {error}"))?;
                OmpProcess::spawn(spec).map_err(|error| format!("could not start omp: {error}"))
            })
            .await
            .unwrap_or_else(|_| Err("could not start omp".to_owned()));
            let _ = tx.send(Msg::Spawned(spawned));
        });
    }

    fn spawned(&mut self, result: Result<(OmpProcess, UnboundedReceiver<Incoming>), String>) {
        self.spawning = false;
        if self.closing || self.finished {
            // omp was asked to end while it was starting: whatever started ends at once.
            if let Ok((process, _)) = result {
                process.kill_now();
            }
            self.finished = true;
            return;
        }
        let (process, mut incoming) = match result {
            Ok(parts) => parts,
            Err(reason) => {
                self.general.host.report_omp_missing(Some(reason.clone()));
                self.fail_opening(
                    &weaver_core::omp::Failure::NotStarted.reason(),
                    Some(reason),
                );
                return;
            }
        };
        let process = Arc::new(process);
        let tx = self.tx.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(message) = incoming.recv().await {
                if tx.send(Msg::Incoming(message)).is_err() {
                    break;
                }
            }
        });
        let (tx, watched) = (self.tx.clone(), process.clone());
        tauri::async_runtime::spawn(async move {
            let info = watched.wait_exit().await;
            let _ = tx.send(Msg::Exited(info));
        });

        let rpc = process.rpc.clone();
        let cwd = self.root.clone();
        let reload = self.reload.clone();
        self.loading = reload.is_some();
        let remaining = OPEN_LIMIT.saturating_sub(self.opened_at.elapsed());
        let tx = self.tx.clone();
        self.process = Some(process);
        tauri::async_runtime::spawn(async move {
            let message = match reload {
                None => {
                    let opening = async {
                        session::initialize(&rpc).await?;
                        session::new_session(&rpc, &cwd)
                            .await
                            .map(|session| Opened {
                                session,
                                fell_back: None,
                            })
                    };
                    Msg::Opened(
                        tokio::time::timeout(remaining, opening)
                            .await
                            .unwrap_or(Err(OpenError::Timeout)),
                    )
                }
                // The load itself is sent by the actor, ordered with the replayed updates.
                Some(previous) => {
                    match tokio::time::timeout(remaining, session::initialize(&rpc))
                        .await
                        .unwrap_or(Err(OpenError::Timeout))
                    {
                        Ok(()) => Msg::Initialized(previous),
                        Err(error) => Msg::Opened(Err(error)),
                    }
                }
            };
            let _ = tx.send(message);
        });
    }

    /// `initialize` answered: the kept session is loaded, sent ordered with the notifications omp
    /// sends meanwhile, so that its answer is handled after every replayed update. The 30 s limit
    /// of the opening goes on running.
    fn initialized(&mut self, previous: String) {
        let Some(process) = self.process.clone().filter(|_| !self.finished) else {
            return;
        };
        let id = session::load_session(&process.rpc, &previous, &self.root);
        self.load = Some((id, previous));
        let remaining = OPEN_LIMIT.saturating_sub(self.opened_at.elapsed());
        let tx = self.tx.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(remaining).await;
            let _ = tx.send(Msg::LoadTimeout(id));
        });
    }

    /// The limit passed while the load had no answer yet.
    fn load_timeout(&mut self, id: u64) {
        if self.load.as_ref().is_some_and(|(load, _)| *load == id) {
            self.load = None;
            self.opened(Err(OpenError::Timeout));
        }
    }

    /// The answer of `session/load`. When omp refused the kept session, an empty one starts and a
    /// notice says so.
    fn load_answered(&mut self, previous: String, result: Result<Value, RpcError>) {
        match result {
            Ok(_) => self.opened(Ok(Opened {
                session: previous,
                fell_back: None,
            })),
            Err(error) => {
                let reason = error.text();
                let (rpc, cwd) = match &self.process {
                    Some(process) if !self.finished => (process.rpc.clone(), self.root.clone()),
                    _ => return,
                };
                let remaining = OPEN_LIMIT.saturating_sub(self.opened_at.elapsed());
                let tx = self.tx.clone();
                tauri::async_runtime::spawn(async move {
                    let result = tokio::time::timeout(remaining, session::new_session(&rpc, &cwd))
                        .await
                        .unwrap_or(Err(OpenError::Timeout))
                        .map(|session| Opened {
                            session,
                            fell_back: Some(reason),
                        });
                    let _ = tx.send(Msg::Opened(result));
                });
            }
        }
    }

    fn opened(&mut self, result: Result<Opened, OpenError>) {
        if self.finished {
            return;
        }
        let opened = match result {
            Ok(opened) => opened,
            Err(error) => {
                let reason = error.reason();
                self.fail_opening(&reason, None);
                return;
            }
        };
        self.session = Some(opened.session.clone());
        self.started = true;
        let reloaded = self.loading && opened.fell_back.is_none();
        self.loading = false;
        let replay = reloaded.then(|| std::mem::replace(&mut self.replay, Conversation::general()));
        self.general.session_opened(
            &self.project,
            self.generation,
            &opened.session,
            replay,
            opened.fell_back.as_deref(),
        );
    }

    /// The session could not be opened: omp stopped, and the next message starts it again.
    fn fail_opening(&mut self, reason: &str, details: Option<String>) {
        let details = details.or_else(|| {
            self.process
                .as_ref()
                .map(|process| process.stderr_text())
                .filter(|text| !text.is_empty())
        });
        self.general
            .set_exit_details(&self.project, self.generation, details);
        self.dispatch(Event::Exited(reason.to_owned()));
        self.finished = true;
    }

    // -----------------------------------------------------------------------------------------
    // Turns and messages
    // -----------------------------------------------------------------------------------------

    fn start_turn(&mut self, text: String) {
        let (Some(process), Some(session)) = (self.process.clone(), self.session.clone()) else {
            return;
        };
        self.turn_running = true;
        let id = session::prompt(&process.rpc, &session, &text);
        self.prompts.insert(id);
    }

    fn turn_done(&mut self, end: TurnEnd) {
        self.turn_running = false;
        self.conversation(|c| c.end_turn());
        match end {
            TurnEnd::Ended => self.dispatch(Event::TurnEnded),
            TurnEnd::Failed(reason) => self.dispatch(Event::TurnFailed(reason)),
            // The process is gone: its exit (reported on its own) says why.
            TurnEnd::Closed => {}
        }
    }

    async fn command(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Steer {
                message,
                text,
                follow_up,
            } => {
                let (Some(process), Some(session)) = (self.process.clone(), self.session.clone())
                else {
                    let _ = self.tx.send(Msg::SteerDone(
                        message,
                        SteeringOutcome::Failed("omp is not running".to_owned()),
                    ));
                    return;
                };
                let tx = self.tx.clone();
                tauri::async_runtime::spawn(async move {
                    let outcome = session::steer(&process.rpc, &session, &text, follow_up).await;
                    let _ = tx.send(Msg::SteerDone(message, outcome));
                });
            }
            Cmd::Prompt { text } => self.start_turn(text),
            Cmd::Cancel => {
                if let (Some(process), Some(session)) = (&self.process, &self.session) {
                    process
                        .rpc
                        .notify("session/cancel", json!({ "sessionId": session }));
                }
            }
            Cmd::ClosePermissions => {
                if let Some(process) = &self.process {
                    for (_, id) in self.pending.drain() {
                        process.rpc.respond(&id, session::cancelled());
                    }
                }
                self.pending.clear();
            }
            Cmd::AnswerPermission { request, option } => {
                if let (Some(process), Some(id)) = (&self.process, self.pending.remove(&request)) {
                    process.rpc.respond(&id, session::selected(&option));
                }
            }
            Cmd::ShutDown { turn_running } => {
                if let Some(process) = self.process.clone() {
                    process
                        .shutdown(self.session.as_deref(), turn_running || self.turn_running)
                        .await;
                } else if self.spawning {
                    // The process is being spawned: it is ended when it arrives, so the actor
                    // stays until then.
                    self.closing = true;
                    return;
                }
                self.finished = true;
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // What omp sends
    // -----------------------------------------------------------------------------------------

    fn incoming(&mut self, message: Incoming) {
        match message {
            Incoming::Notification { method, params } => {
                if method == "session/update" {
                    if let Some(update) = params.get("update") {
                        if self.loading {
                            self.replay.apply_update(update);
                        } else {
                            self.conversation(|c| c.apply_update(update));
                            self.schedule_background();
                        }
                    }
                }
            }
            Incoming::Request { id, method, params } => self.request(id, &method, &params),
            Incoming::Response { id, result } => {
                if self.load.as_ref().is_some_and(|(load, _)| *load == id) {
                    // Every replayed update read before this answer was handled before it.
                    let (_, previous) = self.load.take().expect("the load was just matched");
                    self.load_answered(previous, result);
                } else if self.prompts.remove(&id) {
                    self.turn_done(session::turn_end(result));
                }
            }
            Incoming::Closed => {
                // The output ended: the process is exiting, or is gone. Give it a moment, then count it as exited.
                let Some(process) = self.process.clone() else {
                    return;
                };
                if process.is_shutting_down() || self.exit_reported {
                    return;
                }
                let tx = self.tx.clone();
                tauri::async_runtime::spawn(async move {
                    let info = tokio::time::timeout(Duration::from_secs(2), process.wait_exit())
                        .await
                        .unwrap_or(ExitInfo { code: None });
                    let _ = tx.send(Msg::Exited(info));
                });
            }
        }
    }

    /// A request of omp to the client.
    fn request(&mut self, id: Value, method: &str, params: &Value) {
        let Some(process) = self.process.clone() else {
            return;
        };
        if method == "session/request_permission" {
            let options = params.get("options").cloned().unwrap_or(Value::Null);
            if !self.asks {
                // `yolo`: a request that still arrives is answered at once.
                match session::auto_option(&options) {
                    Some(option) => process.rpc.respond(&id, session::selected(&option)),
                    None => process.rpc.respond(&id, session::cancelled()),
                }
                return;
            }
            let request = self.next_request;
            self.next_request += 1;
            let tool_call = params.get("toolCall").cloned().unwrap_or(Value::Null);
            if self.general.permission_requested(
                &self.project,
                self.generation,
                request,
                &tool_call,
                &options,
            ) {
                self.pending.insert(request, id);
            } else {
                // No turn is in progress: nobody can answer.
                process.rpc.respond(&id, session::cancelled());
            }
        } else if session::is_elicitation(method) {
            process.rpc.respond(
                &id,
                json!({ "action": "decline", "_meta": { "message": DECLINE_TEXT } }),
            );
        } else {
            process
                .rpc
                .respond_error(&id, -32601, "Weaver does not offer this method");
        }
    }

    async fn exited(&mut self, info: ExitInfo) {
        let Some(process) = self.process.clone() else {
            return;
        };
        if self.exit_reported || process.is_shutting_down() {
            return;
        }
        self.exit_reported = true;
        process.kill_now();
        process.settle_stderr().await;
        let (reason, details) = process.exit_reason(&info);
        self.general
            .set_exit_details(&self.project, self.generation, details);
        self.dispatch(Event::Exited(reason));
        self.finished = true;
    }

    // -----------------------------------------------------------------------------------------
    // Sub-agents in the background
    // -----------------------------------------------------------------------------------------

    fn schedule_background(&mut self) {
        if self.poll_scheduled {
            return;
        }
        let waiting = self
            .conversation(|c| !c.background_agents().is_empty())
            .unwrap_or(false);
        if !waiting {
            return;
        }
        self.poll_scheduled = true;
        let tx = self.tx.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(BACKGROUND_EVERY).await;
            let _ = tx.send(Msg::Tick);
        });
    }

    fn tick(&mut self) {
        self.poll_scheduled = false;
        let agents = self
            .conversation(|c| c.background_agents())
            .unwrap_or_default();
        let (Some(session), Some(root)) = (self.session.clone(), self.general.host.sessions_root())
        else {
            return;
        };
        if agents.is_empty() {
            return;
        }
        let tx = self.tx.clone();
        tauri::async_runtime::spawn(async move {
            let found = tauri::async_runtime::spawn_blocking(move || {
                let Some(main) = subagents::find_main_file(&root, &session) else {
                    return Vec::new();
                };
                agents
                    .into_iter()
                    .filter_map(|(tool, agent)| {
                        let file = subagents::agent_file(&main, &agent)?;
                        let state = subagents::exit_state(&std::fs::read_to_string(file).ok()?)?;
                        Some((tool, agent, state))
                    })
                    .collect()
            })
            .await
            .unwrap_or_default();
            let _ = tx.send(Msg::Background(found));
        });
    }

    fn background(&mut self, results: Vec<(String, String, SubAgentState)>) {
        self.conversation(|c| {
            for (tool, agent, state) in &results {
                c.set_sub_agent(tool, agent, *state);
            }
        });
        self.schedule_background();
    }

    fn cleanup(&mut self) {
        if let Some(process) = &self.process {
            if !process.has_exited() {
                process.kill_now();
            }
        }
    }
}
