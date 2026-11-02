//! The actor of a job: the one task that owns its omp process and its ACP session.
//!
//! It does the I/O the Machine `job` asks for ([`super::Jobs::dispatch`] sends it [`Cmd`]s) and
//! turns what happens (omp's messages, the end of a turn, the exit of the process) into events of
//! the Machine. Everything arrives on one channel, so it is handled one at a time, in order.
//! Nothing here ends a job by time: a job ends on a fact that omp or the developer produced.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use trama_core::jobs::machine::{Event, MessageId, RequestId, SteeringOutcome, RESUME_PROMPT};
use trama_core::jobs::subagents::{self, SubAgentState};
use trama_core::jobs::{launch, record};
use trama_core::omp::machine::ReadyOmp;

use super::conversation::Conversation;
use super::proc::{ExitInfo, OmpProcess, SpawnSpec};
use super::registry::Jobs;
use super::rpc::Incoming;
use super::session::{self, OpenError, TurnEnd, OPEN_LIMIT};

/// How the actor starts omp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// A new session with the instruction.
    Start,
    /// The session of a suspended job is reloaded.
    Resume,
}

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
    ResumePrompt,
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

/// Everything the actor waits for, on one channel.
pub(crate) enum Msg {
    Cmd(Cmd),
    OmpReady(Arc<ReadyOmp>),
    Spawned(Result<(OmpProcess, UnboundedReceiver<Incoming>, Option<String>), String>),
    Opened(Result<String, OpenError>),
    Incoming(Incoming),
    Exited(ExitInfo),
    SteerDone(MessageId, SteeringOutcome),
    Tick,
    Background(Vec<(String, String, SubAgentState)>),
}

/// How often the background sub-agents are looked up in omp's session files.
const BACKGROUND_EVERY: Duration = Duration::from_secs(2);

pub(crate) async fn run(
    jobs: Arc<Jobs>,
    id: String,
    mode: Mode,
    tx: UnboundedSender<Msg>,
    mut rx: UnboundedReceiver<Msg>,
) {
    let mut actor = Actor {
        jobs,
        id,
        mode,
        tx,
        process: None,
        session: None,
        prompts: HashSet::new(),
        pending: HashMap::new(),
        next_request: 1,
        asks: false,
        loading: false,
        replay: Conversation::new(),
        opened_at: Instant::now(),
        exit_reported: false,
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
    actor.cleanup().await;
}

struct Actor {
    jobs: Arc<Jobs>,
    id: String,
    mode: Mode,
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
    /// A session is being reloaded: what omp sends is the replayed history.
    loading: bool,
    replay: Conversation,
    opened_at: Instant,
    exit_reported: bool,
    turn_running: bool,
    /// The session started (or was reloaded).
    started: bool,
    poll_scheduled: bool,
    finished: bool,
    root: PathBuf,
}

impl Actor {
    fn begin(&mut self) {
        let host = self.jobs.host.clone();
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
                // The CLI socket starts with the app; a job that resumes at start waits for it.
                let mut endpoint = self.jobs.host.endpoint();
                for _ in 0..600 {
                    if endpoint.is_some() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    endpoint = self.jobs.host.endpoint();
                }
                match endpoint {
                    Some(endpoint) => self.launch(ready, endpoint),
                    None => self.fail_opening("the CLI socket of Weaver did not start", None),
                }
            }
            Msg::Spawned(result) => self.spawned(result),
            Msg::Opened(result) => self.opened(result),
            Msg::Incoming(incoming) => self.incoming(incoming),
            Msg::Exited(info) => self.exited(info).await,
            Msg::SteerDone(message, outcome) => self
                .jobs
                .dispatch(&self.id, Event::SteeringReplied(message, outcome)),
            Msg::Tick => self.tick(),
            Msg::Background(results) => self.background(results),
        }
    }

    // -----------------------------------------------------------------------------------------
    // Starting
    // -----------------------------------------------------------------------------------------

    /// omp is ready: write the overlay, spawn the process.
    fn launch(&mut self, ready: Arc<ReadyOmp>, endpoint: String) {
        if self.finished || self.process.is_some() {
            return;
        }
        self.opened_at = Instant::now();
        let prepared = self.prepare(&ready, &endpoint);
        let tx = self.tx.clone();
        tauri::async_runtime::spawn(async move {
            let spawned = tauri::async_runtime::spawn_blocking(move || {
                let (spec, overlay, overlay_file) = prepared?;
                trama_core::atomic::write(&overlay_file, overlay.as_bytes(), Some(0o600))
                    .map_err(|error| format!("cannot write the overlay of omp: {error}"))?;
                let (process, incoming) = OmpProcess::spawn(spec)
                    .map_err(|error| format!("could not start omp: {error}"))?;
                let started = trama_core::process_group::start_time(process.pid);
                Ok((process, incoming, started))
            })
            .await
            .unwrap_or_else(|_| Err("could not start omp".to_owned()));
            let _ = tx.send(Msg::Spawned(spawned));
        });
    }

    /// The command line, the environment and the overlay of omp, copied from the settings now (a
    /// resumed job keeps the ones it started with).
    #[allow(clippy::type_complexity)]
    fn prepare(
        &mut self,
        ready: &ReadyOmp,
        endpoint: &str,
    ) -> Result<(SpawnSpec, String, PathBuf), String> {
        let host = &self.jobs.host;
        let record = self.jobs.record(&self.id).ok_or("the job is gone")?;
        self.root = PathBuf::from(&record.project);
        let overlay_file = record::overlay_file(host.dirs(), &self.id);
        let (args, mut env, overlay) = match self.mode {
            Mode::Start => {
                let settings = host.settings();
                let proposal_model = self.jobs.proposal_model(&self.id);
                let plan = launch::LaunchPlan {
                    kind: record.kind,
                    models: &settings.models,
                    proposal_model: proposal_model.as_deref(),
                    permissions: settings.permissions,
                    overlay: &overlay_file,
                };
                let env = launch::weaver_env(&self.id, endpoint, record.kind, record.run_task());
                (
                    launch::args(&plan),
                    env,
                    launch::overlay_text(record.kind, &settings.models.scan),
                )
            }
            Mode::Resume => (
                record.args.clone(),
                record.env.clone(),
                record.overlay.clone(),
            ),
        };
        // The socket is new at each launch of Weaver.
        env.insert("TRAMA_ENDPOINT".to_owned(), endpoint.to_owned());
        self.asks = args
            .windows(2)
            .any(|pair| pair[0] == "--approval-mode" && pair[1] == "always-ask");
        if self.mode == Mode::Start {
            let (omp, args_kept, env_kept, overlay_kept) = (
                ready.omp.to_string_lossy().into_owned(),
                args.clone(),
                env.clone(),
                overlay.clone(),
            );
            self.jobs.with_record(&self.id, move |r| {
                r.omp = omp;
                r.args = args_kept;
                r.env = env_kept;
                r.overlay = overlay_kept;
            });
        }
        let path = host.child_path(&ready.omp);
        let spec = SpawnSpec {
            program: ready.omp.clone(),
            args,
            env: launch::child_env(host.inherited_env(), &env, &path),
            cwd: self.root.clone(),
            name: format!("job-{}", self.id),
        };
        Ok((spec, overlay, overlay_file))
    }

    fn spawned(
        &mut self,
        result: Result<(OmpProcess, UnboundedReceiver<Incoming>, Option<String>), String>,
    ) {
        let (process, mut incoming, started) = match result {
            Ok(parts) => parts,
            Err(reason) => {
                if self.mode == Mode::Start {
                    self.jobs.host.report_omp_missing(Some(reason.clone()));
                }
                self.fail_opening(&trama_core::omp::Failure::NotStarted.reason(), Some(reason));
                return;
            }
        };
        if self.finished {
            process.kill_now();
            return;
        }
        let process = Arc::new(process);
        let (pid, pid_started) = (process.pid, started);
        self.jobs.with_record(&self.id, move |r| {
            r.pid = Some(pid);
            r.pid_started = pid_started;
        });
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
        let reload = (self.mode == Mode::Resume)
            .then(|| self.jobs.record(&self.id).and_then(|r| r.session_id))
            .flatten();
        self.loading = reload.is_some();
        let remaining = OPEN_LIMIT.saturating_sub(self.opened_at.elapsed());
        let tx = self.tx.clone();
        self.process = Some(process);
        tauri::async_runtime::spawn(async move {
            let opening = async {
                session::initialize(&rpc).await?;
                match reload {
                    None => session::new_session(&rpc, &cwd).await,
                    Some(session) => {
                        session::load_session(&rpc, &session, &cwd).await?;
                        Ok(session)
                    }
                }
            };
            let result = tokio::time::timeout(remaining, opening)
                .await
                .unwrap_or(Err(OpenError::Timeout));
            let _ = tx.send(Msg::Opened(result));
        });
    }

    fn opened(&mut self, result: Result<String, OpenError>) {
        if self.finished {
            return;
        }
        let session = match result {
            Ok(session) => session,
            Err(error) => {
                let reason = error.reason();
                self.fail_opening(&reason, None);
                return;
            }
        };
        self.session = Some(session.clone());
        self.started = true;
        match self.mode {
            Mode::Start => {
                self.jobs
                    .with_record(&self.id, |r| r.session_id = Some(session.clone()));
                let instruction = self.jobs.instruction(&self.id).unwrap_or_default();
                self.jobs
                    .with_conversation(&self.id, |c| c.instruction(&instruction));
                self.start_turn(instruction);
                self.jobs.dispatch(&self.id, Event::SessionStarted);
            }
            Mode::Resume => {
                self.loading = false;
                let replayed = std::mem::take(&mut self.replay);
                self.jobs.adopt_replay(&self.id, replayed);
                self.jobs.dispatch(&self.id, Event::SessionResumed);
            }
        }
    }

    /// The session could not be opened: the job fails (a start) or cannot be resumed.
    fn fail_opening(&mut self, reason: &str, details: Option<String>) {
        self.jobs.set_exit_details(
            &self.id,
            details.or_else(|| {
                self.process
                    .as_ref()
                    .map(|p| p.stderr_text())
                    .filter(|t| !t.is_empty())
            }),
        );
        match self.mode {
            Mode::Start => self
                .jobs
                .dispatch(&self.id, Event::StartFailed(reason.to_owned())),
            Mode::Resume => self.jobs.dispatch(
                &self.id,
                Event::SessionNotResumed("the session could not be resumed".to_owned()),
            ),
        }
        // The Machine asks for the shutdown; with no process there is nothing to wait for.
        if self.process.is_none() {
            self.finished = true;
        }
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
        self.jobs.with_conversation(&self.id, |c| c.end_turn());
        match end {
            TurnEnd::Ended => self.jobs.dispatch(&self.id, Event::TurnEnded),
            TurnEnd::Failed(reason) => self.jobs.dispatch(&self.id, Event::TurnFailed(reason)),
            // The process is gone: its exit (reported on its own) says why the job ended.
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
            Cmd::ResumePrompt => self.start_turn(RESUME_PROMPT.to_owned()),
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
                            self.jobs
                                .with_conversation(&self.id, |c| c.apply_update(update));
                            self.schedule_background();
                        }
                    }
                }
            }
            Incoming::Request { id, method, params } => self.request(id, &method, &params),
            Incoming::Response { id, result } => {
                if self.prompts.remove(&id) {
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
            self.pending.insert(request, id);
            let tool_call = params.get("toolCall").cloned().unwrap_or(Value::Null);
            self.jobs
                .with_conversation(&self.id, |c| c.permission(request, &tool_call, &options));
            self.jobs
                .dispatch(&self.id, Event::PermissionRequested(request));
            self.jobs.tell(&self.id, true, |observer, job| {
                observer.permission_asked(job, request)
            });
        } else if session::is_elicitation(method) {
            process.rpc.respond(&id, session::decline());
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
        self.jobs.set_exit_details(&self.id, details);
        let event = if self.started {
            Event::Exited(reason)
        } else if self.mode == Mode::Start {
            Event::StartFailed(reason)
        } else {
            Event::SessionNotResumed("the session could not be resumed".to_owned())
        };
        self.jobs.dispatch(&self.id, event);
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
            .jobs
            .with_conversation(&self.id, |c| !c.background_agents().is_empty())
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
            .jobs
            .with_conversation(&self.id, |c| c.background_agents())
            .unwrap_or_default();
        let (Some(session), Some(root)) = (self.session.clone(), self.jobs.host.sessions_root())
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
        self.jobs.with_conversation(&self.id, |c| {
            for (tool, agent, state) in &results {
                c.set_sub_agent(tool, agent, *state);
            }
        });
        self.schedule_background();
    }

    async fn cleanup(&mut self) {
        if let Some(process) = &self.process {
            if !process.has_exited() {
                process.kill_now();
            }
        }
        let file = record::overlay_file(self.jobs.host.dirs(), &self.id);
        let _ = tauri::async_runtime::spawn_blocking(move || std::fs::remove_file(file)).await;
    }
}
