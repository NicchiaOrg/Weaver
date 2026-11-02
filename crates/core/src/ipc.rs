//! Local-socket transport of the CLI↔app channel (`interprocess`; no TCP, no HTTP).
//!
//! One request per connection. The server side ([`serve_connection`]) closes the connection
//! with no response when the message is oversized, malformed, of another protocol version or
//! carries a wrong or missing secret. The client side ([`send`]) reports all of those, and a
//! reply of another version, as errors the CLI treats as "no app answered".

use std::io::{self, BufReader, Read, Write};
use std::sync::{Arc, Mutex};

use interprocess::local_socket::{prelude::*, GenericFilePath, ListenerOptions, Name};

pub use interprocess::local_socket::Stream;

use crate::protocol::{self, ProtocolError, Rejection, Request, Response};

fn name(endpoint: &str) -> io::Result<Name<'_>> {
    // On Windows a `\\.\pipe\...` path names the pipe; on Unix it is a socket file path.
    endpoint.to_fs_name::<GenericFilePath>()
}

/// Connects to the endpoint of a running app.
pub fn connect(endpoint: &str) -> io::Result<Stream> {
    Stream::connect(name(endpoint)?)
}

/// A bound endpoint. On Unix the socket file is removed when the listener is dropped.
pub struct Listener(interprocess::local_socket::Listener);

impl Listener {
    /// Binds the endpoint; a stale socket file of a dead launch is removed first.
    pub fn bind(endpoint: &str) -> io::Result<Self> {
        #[cfg(unix)]
        match std::fs::remove_file(endpoint) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        ListenerOptions::new()
            .name(name(endpoint)?)
            .create_sync()
            .map(Self)
    }

    /// Blocks until a client connects.
    pub fn accept(&self) -> io::Result<Stream> {
        self.0.accept()
    }
}

/// How a connection was served.
#[derive(Debug)]
pub enum Served {
    /// The request was valid and authenticated; the handler's response was written.
    Handled,
    /// The connection was closed with no response.
    Rejected(Rejection),
}

/// Serves one connection: reads one request, gates it ([`protocol::accept_request`]), runs the
/// handler and writes its response. The stream is closed when it is dropped by the caller.
pub fn serve_connection<S: Read + Write>(
    stream: S,
    expected_secret: &str,
    handler: impl FnOnce(Request) -> Response,
) -> io::Result<Served> {
    let mut reader = BufReader::new(stream);
    let line = match protocol::read_line(&mut reader) {
        Ok(line) => line,
        Err(ProtocolError::Io(e)) => return Err(e),
        Err(e) => return Ok(Served::Rejected(Rejection::Protocol(e))),
    };
    let request = match protocol::accept_request(&line, expected_secret) {
        Ok(request) => request,
        Err(rejection) => return Ok(Served::Rejected(rejection)),
    };
    let reply = protocol::encode(&handler(request)).map_err(io::Error::other)?;
    let stream = reader.get_mut();
    stream.write_all(&reply)?;
    stream.flush()?;
    Ok(Served::Handled)
}

/// The client of a connection being served: it tells when the client goes away.
///
/// A request that waits with no time limit (a round of questions) learns from it that its agent
/// stopped waiting: end-of-stream on the connection is how the CLI says so.
#[derive(Clone, Default)]
pub struct Peer(Arc<Mutex<PeerState>>);

#[derive(Default)]
struct PeerState {
    closed: bool,
    /// The handler returned: nobody waits for a callback any more.
    finished: bool,
    callbacks: Vec<Box<dyn FnOnce() + Send>>,
}

impl Peer {
    fn state(&self) -> std::sync::MutexGuard<'_, PeerState> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Whether the client closed the connection.
    pub fn is_closed(&self) -> bool {
        self.state().closed
    }

    /// Runs `callback` once, on another thread, when the client closes the connection; at once
    /// (on the calling thread) when it already has. Never runs after the handler returned.
    pub fn on_close(&self, callback: impl FnOnce() + Send + 'static) {
        let mut state = self.state();
        if state.finished {
            return;
        }
        if state.closed {
            drop(state);
            callback();
        } else {
            state.callbacks.push(Box::new(callback));
        }
    }

    fn close(&self) {
        let callbacks = {
            let mut state = self.state();
            state.closed = true;
            std::mem::take(&mut state.callbacks)
        };
        for callback in callbacks {
            callback();
        }
    }

    fn finish(&self) {
        let mut state = self.state();
        state.finished = true;
        state.callbacks.clear();
    }
}

/// A connection shared by the thread that watches it and the one that writes the reply: the
/// stream behind an `Arc`, which `interprocess` offers in place of splitting it.
struct Shared(Arc<Stream>);

impl Read for Shared {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&*self.0).read(buf)
    }
}

/// Reads until the client closes the connection (or it fails), then tells `peer`.
fn watch_peer(mut receive: Shared, peer: Peer) {
    std::thread::spawn(move || {
        let mut sink = [0u8; 256];
        loop {
            match receive.read(&mut sink) {
                Ok(0) => break,
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        peer.close();
    });
}

/// Like [`serve_connection`], for a handler that waits for a long time: the handler gets a
/// [`Peer`] that tells when the client closes the connection. The request is read and gated first,
/// then a thread reads the connection until end-of-stream while the handler runs.
pub fn serve_connection_watched(
    stream: Stream,
    expected_secret: &str,
    handler: impl FnOnce(Request, Peer) -> Response,
) -> io::Result<Served> {
    let stream = Arc::new(stream);
    let mut reader = BufReader::new(Shared(stream.clone()));
    let line = match protocol::read_line(&mut reader) {
        Ok(line) => line,
        Err(ProtocolError::Io(e)) => return Err(e),
        Err(e) => return Ok(Served::Rejected(Rejection::Protocol(e))),
    };
    let request = match protocol::accept_request(&line, expected_secret) {
        Ok(request) => request,
        Err(rejection) => return Ok(Served::Rejected(rejection)),
    };
    let peer = Peer::default();
    watch_peer(reader.into_inner(), peer.clone());
    let response = handler(request, peer.clone());
    peer.finish();
    let reply = protocol::encode(&response).map_err(io::Error::other)?;
    let mut send = &*stream;
    send.write_all(&reply)?;
    send.flush()?;
    Ok(Served::Handled)
}

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    /// Nobody listens, or the connection failed.
    #[error("cannot connect: {0}")]
    Connect(io::Error),
    /// The server closed the connection without a (complete) reply.
    #[error("the app closed the connection without a reply")]
    NoReply,
    /// The reply is oversized, malformed or of another protocol version.
    #[error("unusable reply: {0}")]
    BadReply(ProtocolError),
    /// The request itself cannot be sent (for example it exceeds 1 MiB).
    #[error("cannot send the request: {0}")]
    BadRequest(ProtocolError),
    #[error(transparent)]
    Io(io::Error),
}

/// Sends one request over an open connection and waits for the reply.
pub fn exchange<S: Read + Write>(stream: S, request: &Request) -> Result<Response, SendError> {
    let bytes = protocol::encode(request).map_err(SendError::BadRequest)?;
    exchange_encoded(stream, &bytes)
}

fn exchange_encoded<S: Read + Write>(stream: S, bytes: &[u8]) -> Result<Response, SendError> {
    let mut reader = BufReader::new(stream);
    let io_err = |e: io::Error| match e.kind() {
        io::ErrorKind::BrokenPipe
        | io::ErrorKind::ConnectionReset
        | io::ErrorKind::UnexpectedEof => SendError::NoReply,
        _ => SendError::Io(e),
    };
    reader.get_mut().write_all(bytes).map_err(io_err)?;
    reader.get_mut().flush().map_err(io_err)?;
    match protocol::read_line(&mut reader) {
        Ok(line) => protocol::decode(&line).map_err(SendError::BadReply),
        Err(ProtocolError::Closed) => Err(SendError::NoReply),
        Err(ProtocolError::Io(e)) => Err(io_err(e)),
        Err(e) => Err(SendError::BadReply(e)),
    }
}

/// Connects to `endpoint`, sends `request` and returns the reply. A request that cannot be
/// sent (over 1 MiB) fails before connecting.
pub fn send(endpoint: &str, request: &Request) -> Result<Response, SendError> {
    let bytes = protocol::encode(request).map_err(SendError::BadRequest)?;
    let stream = connect(endpoint).map_err(SendError::Connect)?;
    exchange_encoded(stream, &bytes)
}

#[cfg(test)]
mod tests {
    use std::thread;

    use serde_json::json;

    use super::*;
    use crate::endpoint::new_endpoint;
    use crate::paths::AppDirs;
    use crate::protocol::MAX_MESSAGE_BYTES;

    const SECRET: &str = "right-secret";

    /// Starts a server for one connection; returns the endpoint and the join handle.
    fn serve_once(base: &std::path::Path) -> (String, thread::JoinHandle<Served>) {
        let dirs = AppDirs::from_bases(base, base, Some(base));
        let endpoint = new_endpoint(&dirs).unwrap();
        let listener = Listener::bind(&endpoint).unwrap();
        let handle = thread::spawn(move || {
            let stream = listener.accept().unwrap();
            serve_connection(stream, SECRET, |request| {
                Response::ok(json!({ "echo": request.kind }))
            })
            .unwrap()
        });
        (endpoint, handle)
    }

    #[test]
    fn right_secret_gets_the_reply() {
        let base = tempfile::tempdir().unwrap();
        let (endpoint, server) = serve_once(base.path());
        let reply = send(&endpoint, &Request::new(SECRET, "ping", json!(null))).unwrap();
        assert_eq!(reply.payload, json!({ "echo": "ping" }));
        assert!(matches!(server.join().unwrap(), Served::Handled));
    }

    #[test]
    fn wrong_secret_receives_no_response() {
        let base = tempfile::tempdir().unwrap();
        let (endpoint, server) = serve_once(base.path());
        let outcome = send(
            &endpoint,
            &Request::new("wrong-secret", "ping", json!(null)),
        );
        assert!(
            matches!(outcome, Err(SendError::NoReply)),
            "got {outcome:?}"
        );
        assert!(matches!(
            server.join().unwrap(),
            Served::Rejected(Rejection::BadSecret)
        ));
    }

    #[test]
    fn missing_secret_receives_no_response() {
        let base = tempfile::tempdir().unwrap();
        let (endpoint, server) = serve_once(base.path());
        let stream = connect(&endpoint).unwrap();
        let mut reader = BufReader::new(stream);
        reader
            .get_mut()
            .write_all(b"{\"version\":1,\"kind\":\"ping\"}\n")
            .unwrap();
        let mut rest = Vec::new();
        reader.read_to_end(&mut rest).unwrap();
        assert!(rest.is_empty(), "server answered: {rest:?}");
        assert!(matches!(server.join().unwrap(), Served::Rejected(_)));
    }

    #[test]
    fn a_request_over_one_mib_is_refused_by_the_client_and_by_the_server() {
        let big = Request::new(SECRET, "ping", json!("x".repeat(MAX_MESSAGE_BYTES)));
        assert!(matches!(
            send("/nonexistent", &big),
            Err(SendError::BadRequest(ProtocolError::TooLarge))
        ));

        let base = tempfile::tempdir().unwrap();
        let (endpoint, server) = serve_once(base.path());
        let stream = connect(&endpoint).unwrap();
        let mut reader = BufReader::new(stream);
        // A peer that ignores the limit: the server rejects it and replies nothing.
        let mut line = vec![b'a'; MAX_MESSAGE_BYTES + 1];
        line.push(b'\n');
        // The server may close before reading everything; a reset while writing is fine.
        let _ = reader.get_mut().write_all(&line);
        let mut rest = Vec::new();
        let _ = reader.read_to_end(&mut rest);
        assert!(rest.is_empty());
        assert!(matches!(
            server.join().unwrap(),
            Served::Rejected(Rejection::Protocol(ProtocolError::TooLarge))
        ));
    }

    #[test]
    fn a_request_of_another_version_is_refused_with_no_response() {
        let base = tempfile::tempdir().unwrap();
        let (endpoint, server) = serve_once(base.path());
        let mut request = Request::new(SECRET, "ping", json!(null));
        request.version = protocol::PROTOCOL_VERSION + 1;
        assert!(matches!(send(&endpoint, &request), Err(SendError::NoReply)));
        assert!(matches!(
            server.join().unwrap(),
            Served::Rejected(Rejection::Protocol(ProtocolError::VersionMismatch { .. }))
        ));
    }

    #[test]
    fn a_reply_of_another_version_is_unusable() {
        let reply = br#"{"version":99,"payload":null}"#;
        let mut wire = reply.to_vec();
        wire.push(b'\n');
        struct Fake(io::Cursor<Vec<u8>>, Vec<u8>);
        impl Read for Fake {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                self.0.read(buf)
            }
        }
        impl Write for Fake {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.1.extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let outcome = exchange(
            Fake(io::Cursor::new(wire), Vec::new()),
            &Request::new(SECRET, "ping", json!(null)),
        );
        assert!(matches!(
            outcome,
            Err(SendError::BadReply(ProtocolError::VersionMismatch {
                got: 99
            }))
        ));
    }

    #[test]
    fn no_listener_is_a_connect_error() {
        let base = tempfile::tempdir().unwrap();
        let missing = base.path().join("none.sock");
        let outcome = send(
            missing.to_str().unwrap(),
            &Request::new(SECRET, "ping", json!(null)),
        );
        assert!(matches!(outcome, Err(SendError::Connect(_))));
    }

    /// Serves one connection with the watched handler; the handler's `Peer` goes through `peers`.
    fn serve_watched_once(
        base: &std::path::Path,
        handler: impl FnOnce(Request, Peer) -> Response + Send + 'static,
    ) -> (String, thread::JoinHandle<io::Result<Served>>) {
        let dirs = AppDirs::from_bases(base, base, Some(base));
        let endpoint = new_endpoint(&dirs).unwrap();
        let listener = Listener::bind(&endpoint).unwrap();
        let handle = thread::spawn(move || {
            serve_connection_watched(listener.accept().unwrap(), SECRET, handler)
        });
        (endpoint, handle)
    }

    #[test]
    fn a_watched_connection_replies_like_a_plain_one() {
        let base = tempfile::tempdir().unwrap();
        let (endpoint, server) = serve_watched_once(base.path(), |request, peer| {
            assert!(!peer.is_closed());
            Response::ok(json!({ "echo": request.kind }))
        });
        let reply = send(&endpoint, &Request::new(SECRET, "ping", json!(null))).unwrap();
        assert_eq!(reply.payload, json!({ "echo": "ping" }));
        assert!(matches!(server.join().unwrap().unwrap(), Served::Handled));
    }

    #[test]
    fn a_client_that_goes_away_while_the_handler_waits_is_told_to_the_handler() {
        use std::sync::mpsc;
        let base = tempfile::tempdir().unwrap();
        let (waiting, is_waiting) = mpsc::channel();
        let (closed, is_closed) = mpsc::channel();
        let (endpoint, server) = serve_watched_once(base.path(), move |_, peer| {
            peer.on_close(move || closed.send(()).unwrap());
            waiting.send(()).unwrap();
            // No time limit: the handler waits until its client has gone.
            is_closed.recv().unwrap();
            assert!(peer.is_closed());
            Response::ok(json!(null))
        });
        let mut stream = connect(&endpoint).unwrap();
        stream
            .write_all(&protocol::encode(&Request::new(SECRET, "wait", json!(null))).unwrap())
            .unwrap();
        is_waiting.recv().unwrap();
        drop(stream);
        // The reply cannot be written to a client that left; the handler still ran to the end.
        let _ = server.join().unwrap();
    }

    #[test]
    fn on_close_fires_at_once_when_the_client_already_left_and_never_after_the_handler_returned() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::mpsc;
        let base = tempfile::tempdir().unwrap();
        let (left, has_left) = mpsc::channel::<()>();
        let (seen, saw) = mpsc::channel();
        let (endpoint, server) = serve_watched_once(base.path(), move |_, peer| {
            has_left.recv().unwrap();
            // Wait until the watching thread has seen the end of the stream.
            while !peer.is_closed() {
                thread::yield_now();
            }
            let fired = Arc::new(AtomicBool::new(false));
            let flag = fired.clone();
            peer.on_close(move || flag.store(true, Ordering::SeqCst));
            assert!(
                fired.load(Ordering::SeqCst),
                "fires on the calling thread when already closed"
            );
            seen.send(peer).unwrap();
            Response::ok(json!(null))
        });
        let mut stream = connect(&endpoint).unwrap();
        stream
            .write_all(&protocol::encode(&Request::new(SECRET, "wait", json!(null))).unwrap())
            .unwrap();
        drop(stream);
        left.send(()).unwrap();
        let _ = server.join().unwrap();
        let peer = saw.recv().unwrap();
        let late = Arc::new(AtomicBool::new(false));
        let flag = late.clone();
        peer.on_close(move || flag.store(true, Ordering::SeqCst));
        assert!(
            !late.load(Ordering::SeqCst),
            "a handler that returned waits for nothing"
        );
    }
}
