//! The app side of the CLI↔app channel.
//!
//! [`Server::start`] creates the endpoint and a fresh secret, atomically writes `runtime.json`
//! (mode `0600`) and accepts connections. Each connection is served on Tokio's blocking pool by
//! `weaver_core::ipc::serve_connection_watched`: a wrong or missing secret, a message over 1 MiB or
//! of another protocol version closes the connection with no response. The handler gets a [`Peer`]
//! that tells when the client closes the connection (a request that waits with no time limit, like
//! a round of questions, is withdrawn that way). The handler is [`crate::cli_handler::handler`];
//! other request kinds are added to it by the Tasks that own them.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use weaver_core::endpoint::{new_endpoint, new_secret};
use weaver_core::ipc::{self, Listener, Peer, Served};
use weaver_core::paths::AppDirs;
use weaver_core::protocol::{Request, Response};
use weaver_core::runtime::{self, RuntimeInfo};

/// Answers one authenticated request. It runs on the blocking pool, so it may block.
pub type Handler = Arc<dyn Fn(Request, Peer) -> Response + Send + Sync>;

pub struct Server {
    endpoint: String,
    stop: Arc<AtomicBool>,
    accept_loop: Option<JoinHandle<()>>,
}

impl Server {
    /// Starts serving and publishes `runtime.json`. Blocking (disk): call it from the blocking pool
    /// or from setup before windows open.
    pub fn start(dirs: &AppDirs, version: &str, handler: Handler) -> io::Result<Self> {
        let endpoint = new_endpoint(dirs)?;
        let secret = new_secret();
        let listener = Listener::bind(&endpoint)?;
        runtime::write(
            dirs,
            &RuntimeInfo {
                endpoint: endpoint.clone(),
                secret: secret.clone(),
                pid: std::process::id(),
                version: version.to_owned(),
            },
        )?;

        let stop = Arc::new(AtomicBool::new(false));
        let accept_loop = {
            let stop = stop.clone();
            std::thread::Builder::new()
                .name("weaver-socket".into())
                .spawn(move || {
                    accept_loop(listener, &secret, &handler, &stop);
                })?
        };
        Ok(Self {
            endpoint,
            stop,
            accept_loop: Some(accept_loop),
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Stops accepting and removes the socket. Connections being served finish on their own.
    pub fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the blocking accept; the loop sees the flag and drops the listener. The wake
        // connection stays open until the loop has ended: on Windows a client that connects and
        // closes before the accept starts is a dead connection that the listener discards, and the
        // accept would go on waiting for another client for ever.
        let wake = ipc::connect(&self.endpoint);
        if let Some(handle) = self.accept_loop.take() {
            let _ = handle.join();
        }
        drop(wake);
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn accept_loop(listener: Listener, secret: &str, handler: &Handler, stop: &AtomicBool) {
    let secret: Arc<str> = Arc::from(secret);
    while !stop.load(Ordering::SeqCst) {
        let stream = match listener.accept() {
            Ok(stream) => stream,
            Err(_) => continue,
        };
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let (secret, handler) = (secret.clone(), handler.clone());
        tauri::async_runtime::spawn_blocking(move || {
            // Rejections are silent by design; I/O errors end this connection only.
            let _: io::Result<Served> =
                ipc::serve_connection_watched(stream, &secret, |request, peer| {
                    handler(request, peer)
                });
        });
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use weaver_core::ipc::{send, SendError};

    use super::*;

    fn handler() -> Handler {
        Arc::new(|request, _peer| Response::ok(json!({ "kind": request.kind })))
    }

    fn setup() -> (tempfile::TempDir, AppDirs) {
        let base = tempfile::tempdir().unwrap();
        let dirs = AppDirs::from_bases(base.path(), base.path(), Some(base.path()));
        (base, dirs)
    }

    #[test]
    fn publishes_runtime_json_and_answers_the_cli_with_its_secret() {
        let (_base, dirs) = setup();
        let server = Server::start(&dirs, "9.9.9", handler()).unwrap();
        let info = runtime::read(&dirs).unwrap().expect("runtime.json written");
        assert_eq!(info.endpoint, server.endpoint());
        assert_eq!(info.pid, std::process::id());
        assert_eq!(info.version, "9.9.9");
        assert_eq!(info.secret.len(), 64);

        let reply = send(
            &info.endpoint,
            &Request::new(&info.secret, "ping", json!(null)),
        )
        .unwrap();
        assert_eq!(reply.payload, json!({ "kind": "ping" }));
    }

    #[test]
    fn a_client_with_the_wrong_secret_gets_no_response() {
        let (_base, dirs) = setup();
        let server = Server::start(&dirs, "1", handler()).unwrap();
        let outcome = send(
            server.endpoint(),
            &Request::new("0".repeat(64), "ping", json!(null)),
        );
        assert!(
            matches!(outcome, Err(SendError::NoReply)),
            "got {outcome:?}"
        );
    }

    #[test]
    fn every_launch_has_a_new_secret_and_endpoint() {
        let (_base, dirs) = setup();
        let first = {
            let _server = Server::start(&dirs, "1", handler()).unwrap();
            runtime::read(&dirs).unwrap().unwrap()
        };
        let _server = Server::start(&dirs, "1", handler()).unwrap();
        let second = runtime::read(&dirs).unwrap().unwrap();
        assert_ne!(first.secret, second.secret);
        assert_ne!(first.endpoint, second.endpoint);
    }

    #[test]
    fn shutdown_stops_the_server() {
        let (_base, dirs) = setup();
        let mut server = Server::start(&dirs, "1", handler()).unwrap();
        let endpoint = server.endpoint().to_owned();
        server.shutdown();
        let outcome = send(&endpoint, &Request::new("x", "ping", json!(null)));
        assert!(
            matches!(outcome, Err(SendError::Connect(_))),
            "got {outcome:?}"
        );
    }

    #[test]
    fn a_handler_that_waits_learns_that_its_client_went_away() {
        use std::sync::{mpsc, Mutex};
        use std::time::Duration;
        let (_base, dirs) = setup();
        let (waiting, is_waiting) = mpsc::channel();
        let (gone, is_gone) = mpsc::channel();
        let (finished, has_finished) = mpsc::channel();
        let (waiting, is_gone, finished) = (
            Mutex::new(waiting),
            Mutex::new(is_gone),
            Mutex::new(finished),
        );
        let handler: Handler = Arc::new(move |_, peer| {
            let gone = gone.clone();
            peer.on_close(move || {
                let _ = gone.send(());
            });
            let _ = waiting.lock().unwrap().send(());
            // No time limit: it returns only when its client has gone.
            let _ = is_gone.lock().unwrap().recv();
            let _ = finished.lock().unwrap().send(());
            Response::ok(json!(null))
        });
        let server = Server::start(&dirs, "1", handler).unwrap();
        let info = runtime::read(&dirs).unwrap().unwrap();
        let mut client = ipc::connect(server.endpoint()).unwrap();
        std::io::Write::write_all(
            &mut client,
            &weaver_core::protocol::encode(&Request::new(&info.secret, "wait", json!(null)))
                .unwrap(),
        )
        .unwrap();
        is_waiting
            .recv_timeout(Duration::from_secs(20))
            .expect("the handler did not start");
        assert!(
            has_finished.try_recv().is_err(),
            "the handler waits for as long as its client is there"
        );
        drop(client);
        has_finished
            .recv_timeout(Duration::from_secs(20))
            .expect("the handler was not told that its client went away");
    }
}
