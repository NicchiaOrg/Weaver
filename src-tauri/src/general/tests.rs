//! General against the scripted fake ACP agent: the first message starts omp, the kept session is
//! reloaded after the window closed and after a restart of Weaver, an empty session starts when
//! the load fails, Stop, the exit of omp, Clear, permissions and forms, and `weaver job new` from
//! General. No omp, no network.

use std::path::Path;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use weaver_core::jobs::general::{GeneralRecord, GeneralStore};
use weaver_core::settings::Permissions;

use super::service::General;
use super::wire::{GeneralEntries, GeneralEvents, GeneralState};
use crate::jobs::conversation::{Delivery, Entry, PermissionState};
use crate::jobs::testing::{text_update, until, Project, TestHost, LIMIT, TASK};
use crate::jobs::Jobs;

#[derive(Default)]
struct Recorder {
    states: Mutex<Vec<GeneralState>>,
    entries: Mutex<Vec<GeneralEntries>>,
}

impl GeneralEvents for Recorder {
    fn state(&self, state: &GeneralState) {
        self.states.lock().unwrap().push(state.clone());
    }

    fn entries(&self, entries: &GeneralEntries) {
        self.entries.lock().unwrap().push(entries.clone());
    }
}

struct Fixture {
    host: Arc<TestHost>,
    general: Arc<General>,
    events: Arc<Recorder>,
    _dir: tempfile::TempDir,
    project: Project,
}

impl Fixture {
    fn id(&self) -> &str {
        &self.project.id
    }

    fn send(&self, text: &str) {
        self.general.send_message(self.id(), text).unwrap();
    }

    fn entries(&self) -> Vec<Entry> {
        self.general.entries(self.id())
    }

    async fn working(&self) {
        until("a turn to be in progress", LIMIT, || {
            self.general.working(self.id())
        })
        .await;
    }

    async fn received(&self, what: &str, wanted: impl Fn(&Value) -> bool) {
        until(what, LIMIT, || self.host.received().iter().any(&wanted)).await;
    }

    fn methods(&self) -> Vec<String> {
        self.host
            .received()
            .iter()
            .filter_map(|m| m["method"].as_str().map(str::to_owned))
            .collect()
    }

    fn record(&self) -> Option<GeneralRecord> {
        self.general.flush();
        GeneralStore::new(&self.host.dirs).read(self.id()).unwrap()
    }
}

async fn fixture() -> Fixture {
    let host = TestHost::new();
    let events = Arc::new(Recorder::default());
    let general = General::new(host.clone(), events.clone());
    let (dir, project) = host.project(&[]).await;
    general.open(&project.id, &project.root).await;
    Fixture {
        host,
        general,
        events,
        _dir: dir,
        project,
    }
}

fn agent(text: &str) -> Value {
    json!({ "update": text_update("agent_message_chunk", text) })
}

fn developers(entries: &[Entry]) -> Vec<(&str, &Delivery)> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Developer { text, delivery, .. } => Some((text.as_str(), delivery)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn an_empty_general_keeps_nothing_and_the_first_message_starts_omp_with_the_general_command_line_and_environment(
) {
    let f = fixture().await;
    f.host.script(json!({
        "log": f.host.received_log(),
        "logEnv": ["WEAVER_GENERAL", "WEAVER_JOB", "WEAVER_ENDPOINT", "WEAVER_RUN", "WEAVER_TASK"],
        "prompts": [[agent("Hello "), agent("there")], [agent("Again")]],
    }));
    let snapshot = f.general.snapshot(f.id()).unwrap();
    assert!(snapshot.entries.is_empty());
    assert_eq!(
        (
            snapshot.state.working,
            snapshot.state.starting,
            snapshot.state.kept
        ),
        (false, false, false)
    );
    assert!(
        f.methods().is_empty(),
        "omp does not run before the first message"
    );

    f.send("  Hi omp  ");
    // The message shows at once, queued, while omp starts.
    assert!(
        matches!(&f.entries()[0], Entry::Developer { text, delivery: Delivery::Queued, .. } if text == "Hi omp")
    );
    let first_working = f
        .events
        .states
        .lock()
        .unwrap()
        .iter()
        .find(|s| s.working)
        .cloned()
        .unwrap();
    assert!(
        first_working.starting,
        "the state that shows the message says omp is starting"
    );

    until("the turn to end", LIMIT, || {
        let entries = f.entries();
        !f.general.working(f.id())
            && entries.contains(&Entry::Agent {
                text: "Hello there".into(),
                complete: true,
            })
    })
    .await;
    assert_eq!(
        developers(&f.entries()),
        vec![("Hi omp", &Delivery::Delivered)]
    );
    assert!(!f.events.states.lock().unwrap().last().unwrap().starting);

    let received = f.host.received();
    let start = &received[0];
    assert_eq!(
        start["env"]["WEAVER_GENERAL"],
        json!(f.general.identity(f.id()).unwrap())
    );
    assert_eq!(start["env"]["WEAVER_ENDPOINT"], "/tmp/weaver-test.sock");
    // No job variable reaches omp, not even the one Weaver's own environment held.
    for name in ["WEAVER_JOB", "WEAVER_RUN", "WEAVER_TASK"] {
        assert_eq!(start["env"][name], Value::Null, "{name}");
    }
    let args: Vec<&str> = start["args"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let file = weaver_core::jobs::general::instruction_file(&f.host.dirs);
    assert_eq!(
        args,
        [
            "--approval-mode",
            "yolo",
            "--append-system-prompt",
            file.to_str().unwrap(),
            "acp"
        ]
    );
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("weaver job new"),
        "the instruction is written to general.md"
    );
    let new = received
        .iter()
        .find(|m| m["method"] == "session/new")
        .unwrap();
    assert_eq!(
        new["params"]["cwd"],
        f.project.root.to_string_lossy().as_ref()
    );
    let prompt = received
        .iter()
        .find(|m| m["method"] == "session/prompt")
        .unwrap();
    assert_eq!(
        prompt["params"]["prompt"][0]["text"], "Hi omp",
        "no instruction goes in the conversation"
    );

    // The session is kept: the record holds the project, the session and the instant, and nothing of the conversation.
    let record = f.record().unwrap();
    assert_eq!(
        (record.project.as_str(), record.session_id.as_str()),
        (f.project.root.to_str().unwrap(), "fake-session")
    );
    let text = std::fs::read_to_string(GeneralStore::new(&f.host.dirs).file(f.id())).unwrap();
    assert!(!text.contains("Hi omp") && !text.contains("Hello"));

    // omp stays for the next message: the second one is a second prompt of the same process.
    f.send("And again");
    until("the second turn", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Again".into(),
            complete: true,
        })
    })
    .await;
    assert_eq!(
        f.methods().iter().filter(|m| *m == "initialize").count(),
        1,
        "omp was not started again"
    );
    assert_eq!(
        f.methods()
            .iter()
            .filter(|m| *m == "session/prompt")
            .count(),
        2
    );
}

#[tokio::test]
async fn nothing_of_a_general_reaches_the_board_the_jobs_or_a_notification() {
    let f = fixture().await;
    let jobs = Jobs::new(f.host.clone());
    f.host.script(json!({ "prompts": [[agent("Done")]] }));
    f.send("Hello");
    until("the turn to end", LIMIT, || {
        !f.general.working(f.id()) && !f.entries().is_empty() && f.entries().len() > 1
    })
    .await;
    assert_eq!(
        *f.host.board_changes.lock().unwrap(),
        0,
        "the Board is never touched"
    );
    assert!(
        f.host.snapshots.lock().unwrap().is_empty(),
        "no job list changes"
    );
    assert!(
        jobs.list_for(&f.project.root, true).is_empty(),
        "General is not a job"
    );
}

#[tokio::test]
async fn the_conversation_is_reloaded_after_the_window_closed_without_repeating_what_the_window_kept(
) {
    let f = fixture().await;
    f.host
        .script(json!({ "log": f.host.received_log(), "prompts": [[agent("First answer")]] }));
    f.send("First question");
    until("the first turn", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "First answer".into(),
            complete: true,
        })
    })
    .await;

    // `window.CLOSED`: omp is ended in order, the conversation and the record stay.
    f.general.window_closed(f.id()).await;
    f.received("session/close", |m| m["method"] == "session/close")
        .await;
    assert!(
        f.general.identity(f.id()).is_none(),
        "the identity of the ended omp is no longer accepted"
    );
    assert!(f.record().is_some());
    assert_eq!(f.entries().len(), 2);

    f.host
        .script(json!({ "log": f.host.received_log(), "prompts": [[agent("Second answer")]] }));
    f.general.open(f.id(), &f.project.root).await;
    f.received("the load at window open", |m| m["method"] == "session/load")
        .await;
    assert_eq!(
        f.entries().len(),
        2,
        "the conversation is there when the window opens again"
    );
    let methods = f.methods();
    let load = methods.iter().position(|m| m == "session/load").unwrap();
    assert!(
        !methods[load..].contains(&"session/prompt".to_owned()),
        "the window opening sends no prompt"
    );
    f.send("Second question");
    until("the second turn", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Second answer".into(),
            complete: true,
        })
    })
    .await;
    let load = f
        .host
        .received()
        .into_iter()
        .find(|m| m["method"] == "session/load")
        .unwrap();
    assert_eq!(
        load["params"]["sessionId"], "fake-session",
        "the kept session is reloaded"
    );
    let texts: Vec<String> = f
        .entries()
        .iter()
        .filter_map(|entry| match entry {
            Entry::Developer { text, .. } | Entry::Agent { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        [
            "First question",
            "First answer",
            "Second question",
            "Second answer"
        ],
        "nothing is repeated"
    );
}

#[tokio::test]
async fn after_a_restart_of_weaver_the_history_comes_back_when_the_window_opens_without_a_message()
{
    let f = fixture().await;
    f.host.script(json!({ "prompts": [[agent("Answer")]] }));
    f.send("Question");
    until("the turn", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Answer".into(),
            complete: true,
        })
    })
    .await;
    f.general.window_closed(f.id()).await;
    f.record().unwrap();

    // A new run of Weaver: a new General over the same folders. The replay is what omp keeps.
    f.host.script(json!({
        "log": f.host.received_log(),
        "replay": [
            { "sessionUpdate": "user_message_chunk", "content": { "type": "text", "text": "Question" } },
            { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "Answer" } },
        ],
        "prompts": [[agent("Newer answer")]],
    }));
    let events = Arc::new(Recorder::default());
    let general = General::new(f.host.clone(), events.clone());
    general.open(f.id(), &f.project.root).await;
    let snapshot = general.snapshot(f.id()).unwrap();
    assert!(
        snapshot.state.loading && snapshot.state.starting && snapshot.state.kept,
        "the window opening starts the load: {:?}",
        snapshot.state
    );
    assert!(!snapshot.state.working);

    until("the history to come back", LIMIT, || {
        let state = general.snapshot(f.id()).unwrap().state;
        !state.loading && !general.entries(f.id()).is_empty()
    })
    .await;
    let entries = general.entries(f.id());
    assert!(
        matches!(&entries[0], Entry::Developer { text, delivery: Delivery::Delivered, .. } if text == "Question")
    );
    assert_eq!(
        entries[1],
        Entry::Agent {
            text: "Answer".into(),
            complete: true
        }
    );
    assert_eq!(entries.len(), 2);
    assert!(
        events.entries.lock().unwrap().iter().any(|e| e.reset),
        "the UI is told to fetch the rebuilt conversation"
    );
    let state = general.snapshot(f.id()).unwrap().state;
    assert!(!state.kept && !state.loading && !state.working && !state.starting);
    let methods = f.methods();
    let load = methods.iter().position(|m| m == "session/load").unwrap();
    assert!(
        !methods[load..].contains(&"session/prompt".to_owned()),
        "nothing is sent to the agent while the conversation comes back"
    );

    // The first message after the history is the first prompt of the reloaded session, and the
    // answer opens a new entry instead of continuing the replayed one.
    general.send_message(f.id(), "Newer question").unwrap();
    until("the reloaded turn", LIMIT, || {
        general.entries(f.id()).contains(&Entry::Agent {
            text: "Newer answer".into(),
            complete: true,
        })
    })
    .await;
    let entries = general.entries(f.id());
    assert!(
        !entries
            .iter()
            .any(|e| matches!(e, Entry::Instruction { .. })),
        "a General has no instruction, so the first user message is the developer's"
    );
    assert!(
        matches!(&entries[2], Entry::Developer { text, .. } if text == "Newer question"),
        "the message stays after the history"
    );
    assert_eq!(
        entries[1],
        Entry::Agent {
            text: "Answer".into(),
            complete: true
        }
    );
}

#[tokio::test]
async fn a_window_opening_with_no_kept_session_starts_no_omp() {
    let f = fixture().await;
    let state = f.general.snapshot(f.id()).unwrap().state;
    assert!(!state.loading && !state.starting && !state.kept && !state.working);
    assert!(
        f.general.identity(f.id()).is_none(),
        "omp starts only with the first message"
    );
    assert!(f.host.received().is_empty());
}

#[tokio::test]
async fn a_message_sent_while_the_conversation_loads_waits_and_goes_out_below_the_history() {
    let f = fixture().await;
    f.host.script(json!({ "prompts": [[agent("Answer")]] }));
    f.send("Question");
    until("the turn", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Answer".into(),
            complete: true,
        })
    })
    .await;
    f.general.window_closed(f.id()).await;
    f.record().unwrap();

    // The agent holds the load until the gate file exists: nothing else orders the test.
    let gate = f.host.dirs.local_data_dir().join("load.gate");
    f.host.script(json!({
        "log": f.host.received_log(),
        "loadGate": gate,
        "replay": [
            { "sessionUpdate": "user_message_chunk", "content": { "type": "text", "text": "Question" } },
            { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "Answer" } },
        ],
        "prompts": [[agent("Second answer")]],
    }));
    let events = Arc::new(Recorder::default());
    let general = General::new(f.host.clone(), events.clone());
    general.open(f.id(), &f.project.root).await;
    f.received("the load request", |m| m["method"] == "session/load")
        .await;
    general.send_message(f.id(), "Meanwhile").unwrap();
    let state = general.snapshot(f.id()).unwrap().state;
    assert!(
        state.loading && state.starting && !state.working,
        "the message waits in Loading, it is not Working: {state:?}"
    );
    assert!(
        matches!(&general.entries(f.id())[..], [Entry::Developer { text, delivery: Delivery::Queued, .. }] if text == "Meanwhile")
    );
    assert!(
        !f.methods()
            .iter()
            .skip_while(|m| *m != "session/load")
            .any(|m| m == "session/prompt" || m == "_session/steering"),
        "nothing is sent to the agent while the conversation loads"
    );

    std::fs::write(&gate, "open").unwrap();
    until("the message to be answered", LIMIT, || {
        general.entries(f.id()).contains(&Entry::Agent {
            text: "Second answer".into(),
            complete: true,
        })
    })
    .await;
    let entries = general.entries(f.id());
    assert!(matches!(&entries[0], Entry::Developer { text, .. } if text == "Question"));
    assert_eq!(
        entries[1],
        Entry::Agent {
            text: "Answer".into(),
            complete: true
        }
    );
    assert!(
        matches!(&entries[2], Entry::Developer { text, delivery: Delivery::Delivered, .. } if text == "Meanwhile"),
        "the history is above the message sent meanwhile: {entries:?}"
    );
    let methods = f.methods();
    let load = methods.iter().position(|m| m == "session/load").unwrap();
    assert!(
        methods[load..]
            .iter()
            .filter(|m| *m == "session/prompt")
            .count()
            == 1,
        "the waiting message is one prompt, after the load"
    );
}

#[tokio::test]
async fn clear_while_the_conversation_loads_ends_that_omp_and_starts_empty() {
    let f = fixture().await;
    f.host.script(json!({ "prompts": [[agent("Answer")]] }));
    f.send("Question");
    until("the turn", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Answer".into(),
            complete: true,
        })
    })
    .await;
    f.general.window_closed(f.id()).await;
    f.record().unwrap();

    let gate = f.host.dirs.local_data_dir().join("never.gate");
    f.host.script(json!({
        "log": f.host.received_log(),
        "loadGate": gate,
        "replay": [
            { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "Answer" } },
        ],
    }));
    let general = General::new(f.host.clone(), Arc::new(Recorder::default()));
    general.open(f.id(), &f.project.root).await;
    f.received("the load request", |m| m["method"] == "session/load")
        .await;
    general.send_message(f.id(), "Meanwhile").unwrap();

    general.clear(f.id()).unwrap();
    let state = general.snapshot(f.id()).unwrap().state;
    assert!(!state.loading && !state.starting && !state.kept && !state.working);
    assert!(
        general.entries(f.id()).is_empty(),
        "the conversation starts empty"
    );
    general.flush();
    assert!(
        GeneralStore::new(&f.host.dirs)
            .read(f.id())
            .unwrap()
            .is_none(),
        "the kept entry is deleted"
    );
    assert!(
        general.identity(f.id()).is_none(),
        "that omp is ended, its identity no longer accepted"
    );
    // The ended omp never answers the load: nothing comes back into the empty conversation.
    general.window_closed(f.id()).await;
    std::fs::write(&gate, "open").unwrap();
    assert!(general.entries(f.id()).is_empty());
    assert!(!general.snapshot(f.id()).unwrap().state.kept);
}

#[tokio::test]
async fn closing_the_window_while_the_conversation_loads_ends_omp_and_keeps_the_record() {
    let f = fixture().await;
    f.host.script(json!({ "prompts": [[agent("Answer")]] }));
    f.send("Question");
    until("the turn", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Answer".into(),
            complete: true,
        })
    })
    .await;
    f.general.window_closed(f.id()).await;
    f.record().unwrap();

    let gate = f.host.dirs.local_data_dir().join("never.gate");
    f.host
        .script(json!({ "log": f.host.received_log(), "loadGate": gate }));
    let general = General::new(f.host.clone(), Arc::new(Recorder::default()));
    general.open(f.id(), &f.project.root).await;
    f.received("the load request", |m| m["method"] == "session/load")
        .await;
    general.window_closed(f.id()).await;
    let state = general.snapshot(f.id()).unwrap().state;
    assert!(!state.loading && !state.starting && !state.working);
    assert!(
        state.kept,
        "the conversation is kept and loads again when the window opens"
    );
    general.flush();
    assert!(GeneralStore::new(&f.host.dirs)
        .read(f.id())
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn when_the_kept_session_cannot_be_loaded_an_empty_one_starts_with_a_notice_above_the_waiting_messages(
) {
    let f = fixture().await;
    f.host.script(json!({ "prompts": [[agent("Answer")]] }));
    f.send("Question");
    until("the turn", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Answer".into(),
            complete: true,
        })
    })
    .await;
    f.general.window_closed(f.id()).await;
    f.record().unwrap();

    f.host.script(json!({ "log": f.host.received_log(), "loadError": "session not found", "sessionId": "fresh", "prompts": [[agent("Fresh answer")]] }));
    let general = General::new(f.host.clone(), Arc::new(Recorder::default()));
    general.open(f.id(), &f.project.root).await;
    until("the empty session with the notice", LIMIT, || {
        general.entries(f.id()).iter().any(|e| matches!(e, Entry::Notice { text } if text.contains("could not be reloaded") && text.contains("session not found")))
    })
    .await;
    assert!(
        !general.snapshot(f.id()).unwrap().state.loading,
        "the loading ends with the empty session"
    );
    general.send_message(f.id(), "Hello again").unwrap();
    until("the turn", LIMIT, || {
        general.entries(f.id()).contains(&Entry::Agent {
            text: "Fresh answer".into(),
            complete: true,
        })
    })
    .await;
    let entries = general.entries(f.id());
    assert!(
        matches!(&entries[0], Entry::Notice { .. }),
        "the notice comes first: {entries:?}"
    );
    let methods: Vec<String> = f.methods();
    let load = methods.iter().position(|m| m == "session/load").unwrap();
    assert!(
        methods[load..].contains(&"session/new".to_owned()),
        "an empty session starts after the failed load"
    );
    general.flush();
    assert_eq!(
        GeneralStore::new(&f.host.dirs)
            .read(f.id())
            .unwrap()
            .unwrap()
            .session_id,
        "fresh",
        "the record names the new session"
    );
}

#[tokio::test]
async fn stop_cancels_the_turn_without_a_dialog_and_omp_stays_for_the_next_message() {
    let f = fixture().await;
    f.host.script(json!({
        "log": f.host.received_log(),
        "prompts": [[agent("Working"), { "wait_cancel": true }], [agent("Next answer")]],
    }));
    f.send("Do a long thing");
    until("the agent to answer", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Working".into(),
            complete: false,
        })
    })
    .await;
    f.general.stop(f.id()).unwrap();
    assert!(!f.general.working(f.id()), "the Machine is Idle at once");
    f.received("session/cancel", |m| m["method"] == "session/cancel")
        .await;
    until("the cancelled answer to be complete", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Working".into(),
            complete: true,
        })
    })
    .await;
    assert!(
        !f.methods().contains(&"session/close".to_owned()),
        "omp is kept"
    );

    f.send("Another thing");
    until("the next answer", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Next answer".into(),
            complete: true,
        })
    })
    .await;
    assert_eq!(f.methods().iter().filter(|m| *m == "initialize").count(), 1);
}

#[tokio::test]
async fn the_exit_of_omp_shows_the_reason_and_the_next_message_starts_omp_again() {
    let f = fixture().await;
    f.host.script(json!({ "log": f.host.received_log(), "prompts": [[agent("Starting"), { "stderr": "error: no provider" }, { "exit": 3 }]] }));
    f.send("Hello");
    until("omp to stop", LIMIT, || {
        f.entries()
            .iter()
            .any(|e| matches!(e, Entry::OmpStopped { .. }))
    })
    .await;
    let stopped = f.entries().into_iter().find_map(|e| match e {
        Entry::OmpStopped { reason, details } => Some((reason, details)),
        _ => None,
    });
    let (reason, details) = stopped.unwrap();
    assert_eq!(reason, "error: no provider");
    assert_eq!(details.as_deref(), Some("error: no provider"));
    assert!(!f.general.working(f.id()));
    assert!(f.general.identity(f.id()).is_none());

    f.send("Are you there?");
    until("omp to start again", LIMIT, || {
        f.methods().iter().filter(|m| *m == "initialize").count() == 2
    })
    .await;
    until("the second stop", LIMIT, || {
        f.entries()
            .iter()
            .filter(|e| matches!(e, Entry::OmpStopped { .. }))
            .count()
            == 2
    })
    .await;
    assert!(
        f.methods().contains(&"session/load".to_owned()),
        "the second start reloads the session the first one opened"
    );
}

#[tokio::test]
async fn omp_that_cannot_open_its_session_is_a_stop_with_the_reason_and_a_missing_omp_is_reported()
{
    let f = fixture().await;
    f.host
        .script(json!({ "newError": "no provider is logged in" }));
    f.send("Hello");
    until("omp to stop", LIMIT, || f.entries().iter().any(|e| matches!(e, Entry::OmpStopped { reason, .. } if reason == "no provider is logged in"))).await;
    assert!(!f.general.working(f.id()));
    assert!(
        matches!(
            &f.entries()[0],
            Entry::Developer {
                delivery: Delivery::NotDelivered { .. },
                ..
            }
        ),
        "the message did not reach the agent"
    );
    assert!(
        f.record().is_none(),
        "no session was opened, so none is kept"
    );
}

#[tokio::test]
async fn a_turn_that_fails_shows_its_reason_and_keeps_omp() {
    let f = fixture().await;
    f.host
        .script(json!({ "prompts": [[{ "end": "refusal" }]] }));
    f.send("Hello");
    until("the failure", LIMIT, || {
        f.entries()
            .iter()
            .any(|e| matches!(e, Entry::TurnFailed { reason } if reason == "the agent refused"))
    })
    .await;
    assert!(!f.general.working(f.id()));
    assert!(f.general.identity(f.id()).is_some(), "omp stays");
}

#[tokio::test]
async fn clear_deletes_the_record_and_the_conversation_also_during_a_turn_and_a_new_one_starts_empty(
) {
    let f = fixture().await;
    f.host.script(json!({ "log": f.host.received_log(), "prompts": [[agent("One")], [agent("Two"), { "hold": true }]] }));
    f.send("First");
    until("the turn", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "One".into(),
            complete: true,
        })
    })
    .await;
    assert!(f.record().is_some());

    // Clear while idle.
    f.general.clear(f.id()).unwrap();
    assert!(f.entries().is_empty());
    assert!(f.record().is_none(), "the record is deleted");
    f.received("session/close", |m| m["method"] == "session/close")
        .await;
    assert!(f.events.entries.lock().unwrap().last().unwrap().reset);
    assert!(!f.general.snapshot(f.id()).unwrap().state.kept);

    // The next message starts an empty session, not a reload.
    f.host.script(
        json!({ "log": f.host.received_log(), "prompts": [[agent("Fresh"), { "hold": true }]] }),
    );
    f.send("Second");
    until("the fresh answer", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Fresh".into(),
            complete: false,
        })
    })
    .await;
    assert!(!f.methods().contains(&"session/load".to_owned()));
    assert_eq!(
        f.methods().iter().filter(|m| *m == "session/new").count(),
        2
    );

    // Clear during a turn stops it.
    f.general.clear(f.id()).unwrap();
    assert!(f.entries().is_empty() && !f.general.working(f.id()));
    until("the second shutdown", LIMIT, || {
        f.methods().iter().filter(|m| *m == "session/close").count() == 2
    })
    .await;
    assert!(
        f.methods().contains(&"session/cancel".to_owned()),
        "the turn was cancelled"
    );
    assert!(f.record().is_none());
}

#[tokio::test]
async fn closing_the_window_during_a_turn_cancels_it_and_keeps_the_conversation() {
    let f = fixture().await;
    f.host.script(
        json!({ "log": f.host.received_log(), "prompts": [[agent("Working"), { "hold": true }]] }),
    );
    f.send("Long work");
    until("the agent to answer", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Working".into(),
            complete: false,
        })
    })
    .await;
    f.general.window_closed(f.id()).await;
    assert!(!f.general.working(f.id()));
    let methods = f.methods();
    assert!(
        methods.contains(&"session/cancel".to_owned())
            && methods.contains(&"session/close".to_owned()),
        "{methods:?}"
    );
    assert!(
        f.entries()
            .iter()
            .any(|e| matches!(e, Entry::Agent { text, .. } if text == "Working")),
        "the conversation is kept"
    );
    assert!(f.record().is_some());
}

#[tokio::test]
async fn a_permission_waits_for_the_developer_in_working_and_a_form_is_declined_with_a_text_that_asks_in_the_reply(
) {
    let f = fixture().await;
    f.host.permissions(Permissions::Ask);
    f.host.script(json!({
        "log": f.host.received_log(),
        "prompts": [[
            { "elicit": true },
            { "permission": { "toolCall": { "toolCallId": "t1", "title": "$ ls", "kind": "execute", "rawInput": { "command": "ls" } }, "options": [
                { "optionId": "allow", "name": "Allow once", "kind": "allow_once" },
                { "optionId": "no", "name": "Reject", "kind": "reject_once" },
            ] } },
            agent("Listed"),
        ]],
    }));
    f.send("List the files");
    until("the permission card", LIMIT, || {
        f.entries().iter().any(|e| {
            matches!(
                e,
                Entry::Permission {
                    state: PermissionState::Asking,
                    ..
                }
            )
        })
    })
    .await;
    assert!(f.general.working(f.id()), "the permission waits in Working");
    let declined = f
        .host
        .received()
        .into_iter()
        .find(|m| m["answerTo"] == "elicitation/create")
        .unwrap();
    assert_eq!(declined["result"]["action"], "decline");
    assert!(declined["result"]["_meta"]["message"]
        .as_str()
        .unwrap()
        .contains("ask the developer in your reply"));

    let request = f.entries().iter().find_map(|e| match e {
        Entry::Permission { request, .. } => Some(*request),
        _ => None,
    });
    assert!(f
        .general
        .answer_permission(f.id(), request.unwrap(), "missing")
        .is_err());
    f.general
        .answer_permission(f.id(), request.unwrap(), "allow")
        .unwrap();
    assert!(
        f.general
            .answer_permission(f.id(), request.unwrap(), "allow")
            .is_err(),
        "answered once"
    );
    until("the turn to end", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Listed".into(),
            complete: true,
        })
    })
    .await;
    let answer = f
        .host
        .received()
        .into_iter()
        .find(|m| m["answerTo"] == "session/request_permission")
        .unwrap();
    assert_eq!(
        answer["result"]["outcome"],
        json!({ "outcome": "selected", "optionId": "allow" })
    );
    assert!(f.entries().iter().any(|e| matches!(e, Entry::Permission { state: PermissionState::Chosen { name }, .. } if name == "Allow once")));
}

// -----------------------------------------------------------------------------------------------
// `weaver job new` from General
// -----------------------------------------------------------------------------------------------

mod job_new {
    use weaver_cli::exec::{execute, Authority};
    use weaver_cli::spec::Caller;
    use weaver_core::command::{code, CommandRequest};

    use super::*;
    use crate::jobs::authority::AppAuthority;

    fn request(
        object: &str,
        verb: &str,
        root: &Path,
        input: Value,
        token: Option<&str>,
    ) -> CommandRequest {
        CommandRequest {
            object: object.into(),
            verb: verb.into(),
            card: None,
            state: None,
            root: root.to_string_lossy().into_owned(),
            input: input.as_object().unwrap().clone(),
            general: token.map(str::to_owned),
        }
    }

    struct Setup {
        f: Fixture,
        jobs: Arc<Jobs>,
        authority: AppAuthority,
    }

    async fn setup() -> Setup {
        let host = TestHost::new();
        let events = Arc::new(Recorder::default());
        let general = General::new(host.clone(), events.clone());
        let jobs = Jobs::new(host.clone());
        jobs.set_general(general.clone());
        let (dir, project) = host
            .project(&[("tasks/aaaaaaaa-first-task.md", TASK)])
            .await;
        general.open(&project.id, &project.root).await;
        let authority = AppAuthority { jobs: jobs.clone() };
        Setup {
            f: Fixture {
                host,
                general,
                events,
                _dir: dir,
                project,
            },
            jobs,
            authority,
        }
    }

    fn run(
        s: &Setup,
        input: Value,
        token: Option<&str>,
    ) -> Result<weaver_core::command::CommandReply, weaver_cli::spec::Failure> {
        let caller = Caller {
            general: token.map(str::to_owned),
            ..Caller::default()
        };
        execute(
            &request("job", "new", &s.f.project.root, input, token),
            &caller,
            &s.authority,
        )
    }

    #[tokio::test]
    async fn the_working_general_starts_a_job_with_its_own_conversation_and_the_developers_refusals_are_shown(
    ) {
        let s = setup().await;
        s.f.host
            .script(json!({ "prompts": [[agent("Starting the scan"), { "hold": true }]] }));
        s.f.send("Please scan the System");
        s.f.working().await;
        until("omp to open", LIMIT, || {
            s.f.general.identity(s.f.id()).is_some()
        })
        .await;
        until("the agent to answer", LIMIT, || {
            s.f.entries()
                .iter()
                .any(|e| matches!(e, Entry::Agent { .. }))
        })
        .await;
        let token = s.f.general.identity(s.f.id()).unwrap();

        let started = run(&s, json!({ "kind": "run", "task": "t1" }), Some(&token)).unwrap();
        let job = started.json["id"].as_str().unwrap().to_owned();
        assert_eq!(started.json["kind"], "run");
        // The job is a normal job with a conversation of its own.
        assert_eq!(s.jobs.record(&job).unwrap().name, "Run T1");
        assert!(s.jobs.conversation(s.f.id(), &job).is_ok());
        assert!(
            s.f.entries().contains(&Entry::JobStarted {
                job: job.clone(),
                name: "Run T1".into()
            }),
            "{:?}",
            s.f.entries()
        );
        assert!(s.f.general.working(s.f.id()), "START_JOB stays in Working");

        // A second Run is refused as it is for the developer, and General says so.
        let refused = run(&s, json!({ "kind": "run" }), Some(&token)).unwrap_err();
        assert_eq!(refused.code, code::REFUSED);
        let shown = s.f.entries().into_iter().find_map(|e| match e {
            Entry::JobRefused { job_kind, reason } => Some((job_kind, reason)),
            _ => None,
        });
        let (kind, reason) = shown.unwrap();
        assert_eq!(kind, "run");
        assert!(
            reason.contains("A run is in progress")
                && !reason.ends_with('.')
                && !reason.starts_with("Could not start"),
            "{reason}"
        );
        // A scan always starts, and an unknown card is refused with the card's own words.
        assert!(run(&s, json!({ "kind": "scan" }), Some(&token)).is_ok());
        assert!(run(
            &s,
            json!({ "kind": "resolve", "drift": "d9" }),
            Some(&token)
        )
        .is_err());
    }

    #[tokio::test]
    async fn job_new_is_refused_to_every_other_caller_and_in_every_other_state() {
        let s = setup().await;
        s.f.host
            .script(json!({ "log": s.f.host.received_log(), "prompts": [[{ "hold": true }]] }));
        // No General has started omp yet.
        assert_eq!(
            run(&s, json!({ "kind": "scan" }), Some("gggggggg"))
                .unwrap_err()
                .code,
            code::FORBIDDEN
        );
        s.f.send("Hello");
        until("omp to open", LIMIT, || {
            s.f.general.identity(s.f.id()).is_some() && s.f.general.working(s.f.id())
        })
        .await;
        let token = s.f.general.identity(s.f.id()).unwrap();
        until("the turn to start", LIMIT, || {
            s.f.methods().contains(&"session/prompt".to_owned())
        })
        .await;

        assert_eq!(
            run(&s, json!({ "kind": "scan" }), None).unwrap_err().code,
            code::FORBIDDEN,
            "no identity"
        );
        assert_eq!(
            run(&s, json!({ "kind": "scan" }), Some("another1"))
                .unwrap_err()
                .code,
            code::FORBIDDEN,
            "not this General"
        );
        let a_job = Caller {
            job: Some(token.clone()),
            ..Caller::default()
        };
        let as_job = execute(
            &request(
                "job",
                "new",
                &s.f.project.root,
                json!({ "kind": "scan" }),
                None,
            ),
            &a_job,
            &s.authority,
        );
        assert_eq!(
            as_job.unwrap_err().code,
            code::FORBIDDEN,
            "a job's identity is not General's"
        );
        assert!(
            s.jobs.list_for(&s.f.project.root, true).is_empty(),
            "a refusal starts nothing"
        );
        assert!(!s
            .f
            .entries()
            .iter()
            .any(|e| matches!(e, Entry::JobStarted { .. } | Entry::JobRefused { .. })));

        // The commands tied to a job are refused in General, with the same identity.
        for (object, verb, input) in [
            ("job", "edit", json!({ "note": "x" })),
            (
                "announcement",
                "new",
                json!({ "kind": "task", "title": "t" }),
            ),
        ] {
            let caller = Caller {
                general: Some(token.clone()),
                ..Caller::default()
            };
            let error = execute(
                &request(object, verb, &s.f.project.root, input, Some(&token)),
                &caller,
                &s.authority,
            )
            .unwrap_err();
            assert_eq!(error.code, code::FORBIDDEN, "{object} {verb}");
        }
        let caller = Caller {
            general: Some(token.clone()),
            ..Caller::default()
        };
        let mut moved = request("task", "move", &s.f.project.root, json!({}), Some(&token));
        moved.card = Some("aaaaaaaa".into());
        moved.state = Some("doing".into());
        assert_eq!(
            execute(&moved, &caller, &s.authority).unwrap_err().code,
            code::FORBIDDEN
        );

        // Once the turn ends, General is Idle and its identity is not accepted any more.
        assert!(s.authority.accepts_general(&s.f.project.root, &token));
        s.f.general.stop(s.f.id()).unwrap();
        assert!(
            !s.authority.accepts_general(&s.f.project.root, &token),
            "only the working General"
        );
        assert_eq!(
            run(&s, json!({ "kind": "scan" }), Some(&token))
                .unwrap_err()
                .code,
            code::FORBIDDEN
        );
    }
}

/// Whether the process is alive (a zombie already reaped counts as gone).
#[cfg(unix)]
fn alive(pid: &str) -> bool {
    std::process::Command::new("kill")
        .args(["-0", pid])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(unix)]
#[tokio::test]
async fn clear_right_after_the_window_opened_leaves_no_omp_alive_however_far_it_had_started() {
    let f = fixture().await;
    f.host.script(json!({ "prompts": [[agent("Answer")]] }));
    f.send("Question");
    until("the turn", LIMIT, || {
        f.entries().contains(&Entry::Agent {
            text: "Answer".into(),
            complete: true,
        })
    })
    .await;
    f.general.window_closed(f.id()).await;
    f.record().unwrap();

    for round in 0..5 {
        let pid_file = f
            .host
            .dirs
            .local_data_dir()
            .join(format!("omp-{round}.pid"));
        let gate = f.host.dirs.local_data_dir().join("never.gate");
        f.host
            .script(json!({ "loadGate": gate, "pidFile": pid_file }));
        let general = General::new(f.host.clone(), Arc::new(Recorder::default()));
        general.open(f.id(), &f.project.root).await;
        // Clear while omp may be waited for, being spawned, or loading.
        general.clear(f.id()).unwrap();
        // Every omp of that actor is ended when it is done.
        general.window_closed(f.id()).await;
        if let Ok(pid) = std::fs::read_to_string(&pid_file) {
            until("the omp to be gone", LIMIT, || !alive(pid.trim())).await;
        }
        // Clear deleted the record: put one back for the next round.
        f.general.flush();
        general.flush();
        weaver_core::jobs::general::GeneralStore::new(&f.host.dirs)
            .write(
                f.id(),
                &GeneralRecord {
                    project: f.project.root.to_string_lossy().into_owned(),
                    session_id: "fake-session".into(),
                    updated: "2026-11-02T07:00:01.000Z".into(),
                },
            )
            .unwrap();
    }
}
