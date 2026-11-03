//! Reading the tool calls of a `session/update` for the lock.
//!
//! Only `tool_call` and `tool_call_update` whose `kind` is `edit`, `delete` or `move` count. The
//! fields read here are omp's own internals (`kind`, `locations`, and for a move
//! `rawOutput.details.sourcePath`): check them again at every rise of `MIN_OMP_VERSION`, like the
//! sub-agent fields of [`weaver_core::jobs::subagents`].

use serde_json::Value;
use weaver_core::file_lock::{is_file_kind, FileLocks, LockChange, Moved};

use crate::jobs::conversation::looks_like_task_call;

/// What the lock needs of a tool call or of its update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolUpdate {
    pub call: String,
    /// `kind`, when the update has one.
    pub kind: Option<String>,
    /// `status`: `pending`, `in_progress`, `completed` or `failed`.
    pub status: Option<String>,
    /// `locations[].path`, absolute.
    pub locations: Vec<String>,
    /// Where omp says it moved a file: the `sourcePath` of the result, and the destination among
    /// the final `locations`.
    pub moved: Option<Moved>,
}

impl ToolUpdate {
    fn ended(&self) -> bool {
        matches!(self.status.as_deref(), Some("completed" | "failed"))
    }
}

/// The tool call of an `update`, or `None` when it is something else, or the `task` tool (the
/// sub-agents' tool never counts).
pub fn read_update(update: &Value) -> Option<ToolUpdate> {
    if !matches!(
        update.get("sessionUpdate").and_then(Value::as_str),
        Some("tool_call" | "tool_call_update")
    ) || looks_like_task_call(update)
    {
        return None;
    }
    let call = update.get("toolCallId").and_then(Value::as_str)?.to_owned();
    let text = |name: &str| update.get(name).and_then(Value::as_str).map(str::to_owned);
    let locations: Vec<String> = update
        .get("locations")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|location| location.get("path").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let source = update
        .get("rawOutput")
        .and_then(|output| output.get("details"))
        .and_then(|details| details.get("sourcePath"))
        .and_then(Value::as_str);
    let moved = source.and_then(|source| {
        // The final locations of a completed edit hold the destination.
        locations
            .iter()
            .rev()
            .find(|location| location.as_str() != source)
            .map(|destination| Moved {
                source: source.to_owned(),
                destination: destination.clone(),
            })
    });
    Some(ToolUpdate {
        call,
        kind: text("kind"),
        status: text("status"),
        locations,
        moved,
    })
}

/// Applies a tool update to the lock state of a project.
pub fn apply(locks: &mut FileLocks, conversation: &str, update: &ToolUpdate) -> Vec<LockChange> {
    let running = locks.is_running(conversation, &update.call);
    // An update without a `kind` is about the tool that already counts, if any.
    let counts = match update.kind.as_deref() {
        Some(kind) => is_file_kind(kind),
        None => running,
    };
    if !counts {
        return Vec::new();
    }
    match (running, update.ended()) {
        // Never counted and already over: there is nothing to hold.
        (false, true) => Vec::new(),
        (false, false) => locks.tool_started(conversation, &update.call, &update.locations),
        (true, true) => locks.tool_ended(conversation, &update.call, update.moved.as_ref()),
        (true, false) if update.locations.is_empty() => Vec::new(),
        (true, false) => locks.tool_located(conversation, &update.call, &update.locations),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn only_tool_calls_are_read() {
        assert_eq!(
            read_update(&json!({ "sessionUpdate": "agent_message_chunk", "content": {} })),
            None
        );
        let call = read_update(&json!({
            "sessionUpdate": "tool_call", "toolCallId": "t1", "kind": "edit", "status": "pending",
            "locations": [{ "path": "/p/a.rs", "line": 3 }, { "path": "/p/b.rs" }]
        }))
        .unwrap();
        assert_eq!(call.call, "t1");
        assert_eq!(call.kind.as_deref(), Some("edit"));
        assert_eq!(call.locations, vec!["/p/a.rs", "/p/b.rs"]);
        assert_eq!(call.moved, None);
    }

    #[test]
    fn the_task_tool_is_never_read() {
        assert_eq!(
            read_update(&json!({
                "sessionUpdate": "tool_call", "toolCallId": "t1", "kind": "edit", "title": "Task: fix",
                "locations": [{ "path": "/p/a.rs" }]
            })),
            None
        );
    }

    #[test]
    fn a_move_is_the_source_of_the_result_and_the_last_other_location() {
        let update = read_update(&json!({
            "sessionUpdate": "tool_call_update", "toolCallId": "t1", "status": "completed",
            "locations": [{ "path": "/p/a.rs" }, { "path": "/p/moved/a.rs" }],
            "rawOutput": { "details": { "sourcePath": "/p/a.rs" } }
        }))
        .unwrap();
        assert_eq!(
            update.moved,
            Some(Moved {
                source: "/p/a.rs".to_owned(),
                destination: "/p/moved/a.rs".to_owned()
            })
        );
    }

    #[test]
    fn a_source_with_no_other_location_is_not_a_move() {
        let update = read_update(&json!({
            "sessionUpdate": "tool_call_update", "toolCallId": "t1", "status": "completed",
            "locations": [{ "path": "/p/a.rs" }],
            "rawOutput": { "details": { "sourcePath": "/p/a.rs" } }
        }))
        .unwrap();
        assert_eq!(update.moved, None);
    }
}
