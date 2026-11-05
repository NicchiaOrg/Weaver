//! The app side of the request kinds `command` and `question.new`.
//!
//! `command` is a `weaver` command the CLI sent instead of running it itself. It runs in the queue
//! of its project, under the OS project lock, with the same function the CLI uses when no app
//! answers (`weaver_cli::exec::execute`), so the checks are the same in both places. The identities
//! it carries (`WEAVER_JOB`, `WEAVER_GENERAL`) are vouched for by the authority of the jobs.
//!
//! `question.new` is a round of questions ([`crate::rounds`]).

use std::path::Path;
use std::sync::Arc;

use tauri::{AppHandle, Manager};
use weaver_cli::exec::{self, Authority};
use weaver_cli::spec::Caller;
use weaver_core::command::{code, CommandRequest, KIND};
use weaver_core::paths::project_hash_of_canonical;
use weaver_core::protocol::{Request, Response};

use crate::error::ErrorCode;
use crate::mutations::MutationQueues;
use crate::server::Handler;
use crate::state::AppState;

/// The handler of the socket server. The identities a command carries are vouched for by the
/// authority of the jobs ([`crate::jobs::app_authority`]).
pub fn handler(app: AppHandle) -> Handler {
    let authority: Arc<dyn Authority + Send + Sync> = Arc::new(crate::jobs::app_authority(&app));
    Arc::new(move |request, peer| {
        // A round waits for the developer with no time limit: it never goes through the project's
        // queue, and the closing of its connection withdraws it.
        if request.kind == weaver_core::question::KIND {
            return app
                .state::<Arc<crate::rounds::Rounds>>()
                .handle(request, peer);
        }
        let state = app.state::<AppState>();
        let payload = request.payload.clone();
        let response = handle(&state.mutations, &authority, request);
        if response.error.is_none() {
            let command = serde_json::from_value::<CommandRequest>(payload);
            let reply = serde_json::from_value::<weaver_core::command::CommandReply>(
                response.payload.clone(),
            );
            if let (Ok(command), Ok(reply)) = (command, reply) {
                if let Some(notice) = crate::proposals::notice_of_command(&command, &reply) {
                    let _ = crate::events::emit_notice(&app, &notice);
                }
            }
        }
        response
    })
}

/// Answers one request: runs the command in its project's queue.
pub fn handle(
    queues: &MutationQueues,
    authority: &Arc<dyn Authority + Send + Sync>,
    request: Request,
) -> Response {
    if request.kind != KIND {
        return Response::failure(
            "unsupported",
            format!("unknown request kind `{}`", request.kind),
        );
    }
    let command: CommandRequest = match serde_json::from_value(request.payload) {
        Ok(command) => command,
        Err(e) => {
            return Response::failure(
                code::INVALID_INPUT,
                format!("the command cannot be read: {e}"),
            )
        }
    };
    let caller = Caller {
        job: request.job,
        general: command.general.clone(),
        run: None,
        task: None,
        ..Caller::default()
    };
    let hash = project_hash_of_canonical(Path::new(&command.root));
    let authority = authority.clone();
    // This runs on the blocking pool, never on the main thread or the thread that serves Tauri IPC.
    let outcome = tauri::async_runtime::block_on(queues.run(&hash, move || {
        Ok(exec::execute(&command, &caller, authority.as_ref()))
    }));
    match outcome {
        Ok(Ok(reply)) => match serde_json::to_value(&reply) {
            Ok(payload) => Response::ok(payload),
            Err(e) => Response::failure(code::FAILED, format!("the answer cannot be encoded: {e}")),
        },
        Ok(Err(failure)) => Response::failure(failure.code, failure.message),
        Err(error) if error.code == ErrorCode::ProjectBusy => Response::failure(
            code::PROJECT_BUSY,
            "the project is busy: another Weaver or weaver command holds it",
        ),
        Err(error) => Response::failure(code::FAILED, error.message),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Mutex;
    use std::time::Duration;

    use serde_json::{json, Map, Value};
    use weaver_core::command::CommandReply;
    use weaver_core::lock;
    use weaver_core::paths::{canonical_root, AppDirs};

    use super::*;

    /// Accepts the job `jobjobjo` and nothing else, and remembers how often it was asked.
    struct OneJob(Mutex<usize>);

    impl Authority for OneJob {
        fn accepts_job(&self, _root: &Path, job: &str) -> bool {
            *self.0.lock().unwrap() += 1;
            job == "jobjobjo"
        }

        fn accepts_general(&self, _root: &Path, _token: &str) -> bool {
            false
        }
    }

    fn request(
        root: &Path,
        job: Option<&str>,
        object: &str,
        verb: &str,
        card: Option<&str>,
        state: Option<&str>,
        input: Value,
    ) -> Request {
        let command = CommandRequest {
            object: object.into(),
            verb: verb.into(),
            root: root.to_string_lossy().into_owned(),
            card: card.map(Into::into),
            state: state.map(Into::into),
            input: input.as_object().cloned().unwrap_or_else(Map::new),
            general: None,
            ask: None,
        };
        let mut request = Request::new("secret", KIND, serde_json::to_value(command).unwrap());
        request.job = job.map(Into::into);
        request
    }

    struct Fixture {
        _base: tempfile::TempDir,
        root: std::path::PathBuf,
        dirs: AppDirs,
    }

    fn fixture() -> Fixture {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("project");
        fs::create_dir_all(&root).unwrap();
        let dirs =
            AppDirs::from_bases(&base.path().join("config"), &base.path().join("data"), None);
        Fixture {
            root: canonical_root(&root).unwrap(),
            dirs,
            _base: base,
        }
    }

    fn nobody() -> Arc<dyn Authority + Send + Sync> {
        Arc::new(exec::NoJobs)
    }

    #[test]
    fn a_command_runs_in_the_queue_of_its_project_and_answers_with_its_reply() {
        let f = fixture();
        let queues = MutationQueues::new(f.dirs.clone());
        let response = handle(
            &queues,
            &nobody(),
            request(&f.root, None, "project", "new", None, None, json!({})),
        );
        assert!(response.error.is_none(), "{:?}", response.error);
        let reply: CommandReply = serde_json::from_value(response.payload).unwrap();
        assert!(reply.markdown.starts_with("# Project initialised"));
        assert!(f.root.join(".weaver/model/system.md").is_file());

        let check = handle(
            &queues,
            &nobody(),
            request(&f.root, None, "model", "check", None, None, json!({})),
        );
        let reply: CommandReply = serde_json::from_value(check.payload).unwrap();
        assert_eq!(reply.json["ok"], true);
    }

    #[test]
    fn a_refusal_is_a_failure_with_its_code_and_writes_nothing() {
        let f = fixture();
        let queues = MutationQueues::new(f.dirs.clone());
        let missing = handle(
            &queues,
            &nobody(),
            request(&f.root, None, "model", "check", None, None, json!({})),
        );
        assert_eq!(missing.error.unwrap().code, code::NOT_INITIALISED);
        assert!(!f.root.join(".weaver").exists());

        let invalid = handle(
            &queues,
            &nobody(),
            request(
                &f.root,
                None,
                "project",
                "new",
                None,
                None,
                json!({ "nope": 1 }),
            ),
        );
        let error = invalid.error.unwrap();
        assert_eq!(error.code, code::INVALID_INPUT);
        assert!(error.message.contains("nope"));
        assert!(!f.root.join(".weaver").exists());
    }

    #[test]
    fn a_job_is_accepted_only_when_the_authority_vouches_for_it() {
        let f = fixture();
        let queues = MutationQueues::new(f.dirs.clone());
        handle(
            &queues,
            &nobody(),
            request(&f.root, None, "project", "new", None, None, json!({})),
        );
        let asked = |job: Option<&str>, authority: &Arc<dyn Authority + Send + Sync>| {
            handle(
                &queues,
                authority,
                request(
                    &f.root,
                    job,
                    "task",
                    "move",
                    Some("aaaaaaaa"),
                    Some("doing"),
                    json!({}),
                ),
            )
        };
        // No job, an unknown job, and the app as it is today (no registry): forbidden.
        for (job, authority) in [(None, nobody()), (Some("jobjobjo"), nobody())] {
            assert_eq!(asked(job, &authority).error.unwrap().code, code::FORBIDDEN);
        }
        let one: Arc<dyn Authority + Send + Sync> = Arc::new(OneJob(Mutex::new(0)));
        assert_eq!(
            asked(Some("otherjob"), &one).error.unwrap().code,
            code::FORBIDDEN
        );
        // An accepted job gets past the gate and meets the card: no such Task.
        let error = asked(Some("jobjobjo"), &one).error.unwrap();
        assert_eq!(error.code, code::REFUSED, "{}", error.message);
    }

    #[test]
    fn another_request_kind_is_unsupported_and_a_garbled_command_is_invalid() {
        let f = fixture();
        let queues = MutationQueues::new(f.dirs.clone());
        let other = Request::new("secret", "ping", json!({}));
        assert_eq!(
            handle(&queues, &nobody(), other).error.unwrap().code,
            "unsupported"
        );
        let garbled = Request::new("secret", KIND, json!({ "object": 3 }));
        assert_eq!(
            handle(&queues, &nobody(), garbled).error.unwrap().code,
            code::INVALID_INPUT
        );
    }

    #[test]
    fn a_project_whose_lock_is_held_elsewhere_is_busy_and_nothing_is_written() {
        let f = fixture();
        let queues = MutationQueues::with_lock_timeout(f.dirs.clone(), Duration::from_millis(100));
        let hash = project_hash_of_canonical(&f.root);
        let (held_tx, held_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let dirs = f.dirs.clone();
        let holder = std::thread::spawn(move || {
            lock::with_project_lock(&dirs, &hash, || {
                held_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            })
            .unwrap();
        });
        held_rx.recv().unwrap();
        let response = handle(
            &queues,
            &nobody(),
            request(&f.root, None, "project", "new", None, None, json!({})),
        );
        assert_eq!(response.error.unwrap().code, code::PROJECT_BUSY);
        assert!(!f.root.join(".weaver").exists());
        release_tx.send(()).unwrap();
        holder.join().unwrap();
    }
}
