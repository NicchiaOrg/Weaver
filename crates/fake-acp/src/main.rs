//! A scripted fake ACP agent: the stand-in for omp in the tests of the jobs. It speaks the JSON-RPC
//! of ACP over its standard streams and does what a script says; no model, no network.
//!
//! The script is the JSON file named by `FAKE_ACP_SCRIPT`:
//!
//! ```json
//! {
//!   "log": "/tmp/received.jsonl",          // every line received is appended to it
//!   "loadSession": true, "steeringSupported": true,   // what `initialize` advertises
//!   "sessionId": "fake-session",
//!   "newError": "no provider",              // `session/new` fails
//!   "loadError": "gone",                    // `session/load` fails
//!   "replay": [ {"sessionUpdate": "..."} ], // sent as updates while `session/load` runs
//!   "steering": ["injected", "promptRequired", "failed:reason"],   // answers, in order
//!   "prompts": [ [ step, ... ], ... ],      // the steps of the 1st, 2nd... `session/prompt`
//!   "logEnv": ["TRAMA_JOB"],                // log these variables and the arguments at start
//!   "ignoreClose": true                     // `session/close` is never answered
//! }
//! ```
//!
//! Steps: `{"update": {...}}`, `{"sleep": ms}`, `{"stderr": "text"}`,
//! `{"permission": {"toolCall": {...}, "options": [...]}}` (waits for the answer, logs it),
//! `{"elicit": true}` (waits for the answer, logs it), `{"wait_cancel": true}` (until
//! `session/cancel`), `{"wait_steer": n}` (until n steering messages came), `{"exit": code}`,
//! `{"hold": true}` (the turn never ends), `{"end": "refusal"}` (stop reason, default `end_turn`),
//! `{"error": "text"}` (the prompt fails).

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

struct Shared {
    out: Mutex<std::io::Stdout>,
    log_lock: Mutex<()>,
    log: Option<String>,
    waiting: Mutex<HashMap<String, Sender<Value>>>,
    state: Mutex<State>,
    changed: Condvar,
}

#[derive(Default)]
struct State {
    cancelled: bool,
    steered: usize,
    prompts: usize,
    next_request: u64,
}

impl Shared {
    fn send(&self, value: &Value) {
        let mut out = self.out.lock().unwrap();
        let _ = writeln!(out, "{value}");
        let _ = out.flush();
    }

    fn log(&self, line: &str) {
        if let Some(path) = &self.log {
            // One write per line, one writer at a time: lines never interleave.
            let _guard = self.log_lock.lock().unwrap();
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = file.write_all(format!("{line}\n").as_bytes());
            }
        }
    }

    fn wait_until(&self, done: impl Fn(&State) -> bool) {
        let mut state = self.state.lock().unwrap();
        while !done(&state) {
            state = self
                .changed
                .wait_timeout(state, Duration::from_millis(50))
                .unwrap()
                .0;
        }
    }
}

fn main() {
    let script: Value = std::env::var("FAKE_ACP_SCRIPT")
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| json!({}));
    let shared = Arc::new(Shared {
        out: Mutex::new(std::io::stdout()),
        log_lock: Mutex::new(()),
        log: script.get("log").and_then(Value::as_str).map(str::to_owned),
        waiting: Mutex::new(HashMap::new()),
        state: Mutex::new(State::default()),
        changed: Condvar::new(),
    });
    let script = Arc::new(script);
    if let Some(names) = script.get("logEnv").and_then(Value::as_array) {
        let env: serde_json::Map<String, Value> = names
            .iter()
            .filter_map(Value::as_str)
            .map(|name| {
                (
                    name.to_owned(),
                    std::env::var(name).map_or(Value::Null, Value::from),
                )
            })
            .collect();
        shared.log(
            &json!({ "env": env, "args": std::env::args().skip(1).collect::<Vec<_>>() })
                .to_string(),
        );
    }
    let session_id = script
        .get("sessionId")
        .and_then(Value::as_str)
        .unwrap_or("fake-session")
        .to_owned();

    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        shared.log(&line);
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let id = message.get("id").cloned();
        match (method.as_deref(), id) {
            (Some("initialize"), Some(id)) => {
                let load = script.get("loadSession").and_then(Value::as_bool).unwrap_or(true);
                let steering = script.get("steeringSupported").and_then(Value::as_bool).unwrap_or(true);
                shared.send(&json!({ "jsonrpc": "2.0", "id": id, "result": {
                    "protocolVersion": 1,
                    "agentInfo": { "name": "fake", "version": "0" },
                    "agentCapabilities": { "loadSession": load },
                    "_meta": { "steering": { "supported": steering } },
                } }));
            }
            (Some("session/new"), Some(id)) => match script.get("newError").and_then(Value::as_str) {
                Some(error) => shared.send(&json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": error } })),
                None => shared.send(&json!({ "jsonrpc": "2.0", "id": id, "result": { "sessionId": session_id } })),
            },
            (Some("session/load"), Some(id)) => {
                for update in script.get("replay").and_then(Value::as_array).into_iter().flatten() {
                    shared.send(&json!({ "jsonrpc": "2.0", "method": "session/update", "params": { "sessionId": session_id, "update": update } }));
                }
                match script.get("loadError").and_then(Value::as_str) {
                    Some(error) => shared.send(&json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": error } })),
                    None => shared.send(&json!({ "jsonrpc": "2.0", "id": id, "result": {} })),
                }
            }
            (Some("session/prompt"), Some(id)) => {
                let index = {
                    let mut state = shared.state.lock().unwrap();
                    state.prompts += 1;
                    state.cancelled = false;
                    state.prompts - 1
                };
                let steps = script.get("prompts").and_then(|p| p.get(index)).and_then(Value::as_array).cloned().unwrap_or_default();
                let (shared, session) = (shared.clone(), session_id.clone());
                std::thread::spawn(move || run_turn(&shared, &session, id, &steps));
            }
            (Some("_session/steering"), Some(id)) => {
                let index = {
                    let mut state = shared.state.lock().unwrap();
                    state.steered += 1;
                    state.steered - 1
                };
                shared.changed.notify_all();
                let answer = script.get("steering").and_then(|s| s.get(index)).and_then(Value::as_str).unwrap_or("injected");
                let result = match answer.strip_prefix("failed:") {
                    Some(reason) => json!({ "outcome": "failed", "reason": reason }),
                    None => json!({ "outcome": answer }),
                };
                shared.send(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
            }
            (Some("session/cancel"), None) => {
                shared.state.lock().unwrap().cancelled = true;
                shared.changed.notify_all();
            }
            (Some("session/close"), Some(id)) => {
                if script.get("ignoreClose").and_then(Value::as_bool) != Some(true) {
                    shared.send(&json!({ "jsonrpc": "2.0", "id": id, "result": {} }));
                }
            }
            (Some(_), Some(id)) => shared.send(&json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": "unknown method" } })),
            (None, Some(id)) => {
                // The answer to a request of ours.
                let key = id.to_string();
                if let Some(waiting) = shared.waiting.lock().unwrap().remove(&key) {
                    let _ = waiting.send(message.get("result").cloned().unwrap_or(Value::Null));
                }
            }
            _ => {}
        }
    }
}

/// A request of the agent to the client; waits for the answer and logs it.
fn ask(shared: &Shared, method: &str, params: Value) -> Value {
    let id = {
        let mut state = shared.state.lock().unwrap();
        state.next_request += 1;
        format!("req-{}", state.next_request)
    };
    let (tx, rx) = channel();
    shared
        .waiting
        .lock()
        .unwrap()
        .insert(json!(id).to_string(), tx);
    shared.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
    let answer = rx.recv().unwrap_or(Value::Null);
    shared.log(&json!({ "answerTo": method, "result": answer }).to_string());
    answer
}

fn run_turn(shared: &Shared, session: &str, id: Value, steps: &[Value]) {
    let mut stop = "end_turn".to_owned();
    for step in steps {
        if let Some(update) = step.get("update") {
            shared.send(&json!({ "jsonrpc": "2.0", "method": "session/update", "params": { "sessionId": session, "update": update } }));
        } else if let Some(ms) = step.get("sleep").and_then(Value::as_u64) {
            std::thread::sleep(Duration::from_millis(ms));
        } else if let Some(text) = step.get("stderr").and_then(Value::as_str) {
            eprintln!("{text}");
        } else if let Some(permission) = step.get("permission") {
            let mut params = permission.clone();
            params["sessionId"] = json!(session);
            ask(shared, "session/request_permission", params);
        } else if step.get("elicit").is_some() {
            ask(
                shared,
                "elicitation/create",
                json!({ "sessionId": session, "mode": "form", "message": "Which folder?", "requestedSchema": { "type": "object", "properties": {} } }),
            );
        } else if step.get("wait_cancel").is_some() {
            shared.wait_until(|state| state.cancelled);
            stop = "cancelled".to_owned();
            break;
        } else if let Some(n) = step.get("wait_steer").and_then(Value::as_u64) {
            shared.wait_until(|state| state.steered as u64 >= n);
        } else if let Some(code) = step.get("exit").and_then(Value::as_i64) {
            std::process::exit(i32::try_from(code).unwrap_or(1));
        } else if step.get("hold").is_some() {
            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        } else if let Some(reason) = step.get("end").and_then(Value::as_str) {
            stop = reason.to_owned();
        } else if let Some(error) = step.get("error").and_then(Value::as_str) {
            shared.send(&json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": error } }));
            return;
        }
    }
    shared.send(&json!({ "jsonrpc": "2.0", "id": id, "result": { "stopReason": stop } }));
}
