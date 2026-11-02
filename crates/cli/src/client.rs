//! Client of the app socket: finds the running app through `runtime.json`, connects, sends one
//! request and returns the reply.
//!
//! When no usable app answers, the caller gets [`CallError::Unavailable`] and acts as if no app
//! were running. A different protocol version counts the same way.

use serde_json::Value;
use weaver_core::ipc::{self, SendError};
use weaver_core::paths::AppDirs;
use weaver_core::protocol::{ProtocolError, Request, Response};
use weaver_core::runtime;

/// Why no app can be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unavailable {
    /// There is no `runtime.json`: the app never ran, or its folder is another one.
    RuntimeFileMissing,
    /// `runtime.json` cannot be read or parsed.
    RuntimeFileInvalid(String),
    /// `WEAVER_ENDPOINT` names another endpoint than `runtime.json`.
    EndpointMismatch,
    /// Nobody listens at the endpoint.
    ConnectionFailed(String),
    /// The app speaks another protocol version.
    ProtocolMismatch,
}

impl Unavailable {
    /// Short text for the `reason` of an `unavailable` outcome.
    pub fn reason(&self) -> String {
        match self {
            Self::RuntimeFileMissing => "runtime file missing".into(),
            Self::RuntimeFileInvalid(why) => format!("runtime file unreadable: {why}"),
            Self::EndpointMismatch => "runtime file names another endpoint".into(),
            Self::ConnectionFailed(why) => format!("cannot connect: {why}"),
            Self::ProtocolMismatch => "protocol version differs".into(),
        }
    }
}

#[derive(Debug)]
pub enum CallError {
    Unavailable(Unavailable),
    /// The app closed the connection without a reply (for example after a refusal).
    NoReply,
    /// The request cannot be sent, or the transport failed.
    Send(SendError),
}

/// What to send.
pub struct Call<'a> {
    /// Job identity (`WEAVER_JOB`), when the request comes from a job.
    pub job: Option<&'a str>,
    pub kind: &'a str,
    pub payload: Value,
    /// `WEAVER_ENDPOINT` when set: the call is unavailable if `runtime.json` names another one.
    pub expected_endpoint: Option<&'a str>,
}

/// The running app of `dirs`, from its runtime record.
fn locate(
    dirs: &AppDirs,
    expected_endpoint: Option<&str>,
) -> Result<runtime::RuntimeInfo, Unavailable> {
    let info = match runtime::read(dirs) {
        Ok(Some(info)) => info,
        Ok(None) => return Err(Unavailable::RuntimeFileMissing),
        Err(e) => return Err(Unavailable::RuntimeFileInvalid(e.to_string())),
    };
    if expected_endpoint.is_some_and(|expected| expected != info.endpoint) {
        return Err(Unavailable::EndpointMismatch);
    }
    Ok(info)
}

/// A connection to the running app, for a request that waits with no time limit: the caller sends
/// the request itself ([`ipc::exchange`]) and keeps the connection open until the reply.
pub struct Connection {
    pub stream: ipc::Stream,
    /// The secret of the app, for the request.
    pub secret: String,
}

/// Connects to the app whose folders are `dirs`.
pub fn open(dirs: &AppDirs, expected_endpoint: Option<&str>) -> Result<Connection, Unavailable> {
    let info = locate(dirs, expected_endpoint)?;
    let stream =
        ipc::connect(&info.endpoint).map_err(|e| Unavailable::ConnectionFailed(e.to_string()))?;
    Ok(Connection {
        stream,
        secret: info.secret,
    })
}

/// Sends `call` to the app whose folders are `dirs`.
pub fn call(dirs: &AppDirs, call: Call<'_>) -> Result<Response, CallError> {
    let info = locate(dirs, call.expected_endpoint).map_err(CallError::Unavailable)?;
    let mut request = Request::new(info.secret, call.kind, call.payload);
    request.job = call.job.map(str::to_owned);
    match ipc::send(&info.endpoint, &request) {
        Ok(response) => Ok(response),
        Err(SendError::Connect(e)) => Err(CallError::Unavailable(Unavailable::ConnectionFailed(
            e.to_string(),
        ))),
        Err(SendError::BadReply(ProtocolError::VersionMismatch { .. })) => {
            Err(CallError::Unavailable(Unavailable::ProtocolMismatch))
        }
        Err(SendError::NoReply) => Err(CallError::NoReply),
        Err(other) => Err(CallError::Send(other)),
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use serde_json::json;
    use weaver_core::endpoint::{new_endpoint, new_secret};
    use weaver_core::ipc::{serve_connection, Listener};
    use weaver_core::runtime::RuntimeInfo;

    use super::*;

    fn simple_call<'a>(expected_endpoint: Option<&'a str>) -> Call<'a> {
        Call {
            job: Some("job1"),
            kind: "ping",
            payload: json!({"n": 1}),
            expected_endpoint,
        }
    }

    #[test]
    fn app_closed_means_unavailable_because_there_is_no_runtime_file() {
        let base = tempfile::tempdir().unwrap();
        let dirs = AppDirs::from_bases(base.path(), base.path(), None);
        assert!(matches!(
            call(&dirs, simple_call(None)),
            Err(CallError::Unavailable(Unavailable::RuntimeFileMissing))
        ));
    }

    #[test]
    fn a_stale_runtime_file_without_listener_is_unavailable() {
        let base = tempfile::tempdir().unwrap();
        let dirs = AppDirs::from_bases(base.path(), base.path(), Some(base.path()));
        let info = RuntimeInfo {
            endpoint: new_endpoint(&dirs).unwrap(),
            secret: new_secret(),
            pid: 1,
            version: "0.1.0".into(),
        };
        runtime::write(&dirs, &info).unwrap();
        assert!(matches!(
            call(&dirs, simple_call(None)),
            Err(CallError::Unavailable(Unavailable::ConnectionFailed(_)))
        ));
    }

    #[test]
    fn reaches_the_app_with_the_secret_of_runtime_json_and_checks_the_endpoint() {
        let base = tempfile::tempdir().unwrap();
        let dirs = AppDirs::from_bases(base.path(), base.path(), Some(base.path()));
        let endpoint = new_endpoint(&dirs).unwrap();
        let secret = new_secret();
        runtime::write(
            &dirs,
            &RuntimeInfo {
                endpoint: endpoint.clone(),
                secret: secret.clone(),
                pid: 1,
                version: "0.1.0".into(),
            },
        )
        .unwrap();
        let listener = Listener::bind(&endpoint).unwrap();
        let server = thread::spawn(move || {
            let stream = listener.accept().unwrap();
            serve_connection(stream, &secret, |request| {
                assert_eq!(request.job.as_deref(), Some("job1"));
                Response::ok(json!({ "kind": request.kind }))
            })
            .unwrap();
        });

        assert!(matches!(
            call(&dirs, simple_call(Some("/another/endpoint"))),
            Err(CallError::Unavailable(Unavailable::EndpointMismatch))
        ));
        let reply = call(&dirs, simple_call(Some(&endpoint))).unwrap();
        assert_eq!(reply.payload, json!({ "kind": "ping" }));
        server.join().unwrap();
    }
}
