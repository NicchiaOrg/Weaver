//! The request kind `command`: a `trama` command that the app runs in the project's queue.
//!
//! The CLI validates the whole input first, then sends one [`CommandRequest`] (the job identity
//! `TRAMA_JOB` travels in the envelope, [`crate::protocol::Request::job`]). The app answers with
//! a [`CommandReply`] in `Response::payload`, or with a failure whose `code` is one of [`code`];
//! [`exit_code`] maps it to the exit code of the command line.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// `Request::kind` of a command.
pub const KIND: &str = "command";

/// A command for the app to run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandRequest {
    pub object: String,
    pub verb: String,
    /// The canonical project root (the folder that holds `.trama/`).
    pub root: String,
    /// The card named on the command line, as typed: an identity, a prefix or a short number.
    /// The app turns it into an identity under the project lock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<String>,
    /// The state argument of `move`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// The validated input object.
    #[serde(default)]
    pub input: Map<String, Value>,
    /// `TRAMA_GENERAL`, the token of the project's working General, when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub general: Option<String>,
}

/// What a command printed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandReply {
    /// The output without `--json`.
    pub markdown: String,
    /// The output with `--json`: one object where every card has `id` and `number`.
    pub json: Value,
    /// Exit code of the command: `0`, or `1` for a command that refuses after printing a report
    /// (`model check` with errors).
    #[serde(default)]
    pub exit: u8,
    /// Text for stderr, when the command has something to say next to its output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Codes of the `{error: {code, message}}` of a refused or failed command.
pub mod code {
    /// Wrong usage of the command line (arguments, options).
    pub const USAGE: &str = "usage";
    /// The input is not acceptable; the message lists every problem with its JSON path.
    pub const INVALID_INPUT: &str = "invalid-input";
    /// The project has no `.trama/`.
    pub const NOT_INITIALISED: &str = "not-initialised";
    /// The command is refused: nothing was written.
    pub const REFUSED: &str = "refused";
    /// The caller may not run the command (no job, not General).
    pub const FORBIDDEN: &str = "forbidden";
    /// The command needs the app and no app answered.
    pub const APP_NOT_RUNNING: &str = "app-not-running";
    /// The project lock was not obtained in time.
    pub const PROJECT_BUSY: &str = "project-busy";
    /// The command failed for another reason (a file cannot be read or written).
    pub const FAILED: &str = "failed";
}

/// The exit code of a failure with this `code`: `2` for wrong usage or input, else `1`.
pub fn exit_code(code: &str) -> u8 {
    match code {
        code::USAGE | code::INVALID_INPUT => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_request_round_trips_without_its_empty_parts() {
        let request = CommandRequest {
            object: "task".into(),
            verb: "move".into(),
            root: "/p".into(),
            card: Some("t3".into()),
            state: Some("doing".into()),
            input: Map::new(),
            general: None,
        };
        let text = serde_json::to_string(&request).unwrap();
        assert!(!text.contains("general"));
        assert_eq!(
            serde_json::from_str::<CommandRequest>(&text).unwrap(),
            request
        );
    }

    #[test]
    fn a_reply_without_exit_means_done() {
        let reply: CommandReply =
            serde_json::from_value(json!({ "markdown": "ok", "json": {} })).unwrap();
        assert_eq!(reply.exit, 0);
    }

    #[test]
    fn wrong_usage_and_invalid_input_are_exit_two_everything_else_one() {
        assert_eq!(exit_code(code::USAGE), 2);
        assert_eq!(exit_code(code::INVALID_INPUT), 2);
        for other in [
            code::REFUSED,
            code::FORBIDDEN,
            code::APP_NOT_RUNNING,
            code::PROJECT_BUSY,
            code::FAILED,
            code::NOT_INITIALISED,
        ] {
            assert_eq!(exit_code(other), 1);
        }
    }
}
