//! The sub-agents of a job, as omp reports them (`requirements/tech#job`, "Sub-agents").
//!
//! They are read from the `task` tool call: `rawOutput.details.progress[]`. A background
//! sub-agent (omp's `async.enabled`) ends at the `session_exit` record of
//! `<main session file minus .jsonl>/<id>.jsonl`, under omp's sessions folder. These are omp
//! internals, re-verified at each `MIN_OMP_VERSION` bump.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

/// How a sub-agent stands, in the words of the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum SubAgentState {
    Working,
    Succeeded,
    Failed,
    Interrupted,
}

impl SubAgentState {
    pub fn is_working(self) -> bool {
        self == SubAgentState::Working
    }

    /// `pending` and `running` work, `completed` succeeded, `failed` failed, `aborted` was
    /// interrupted; any other word counts as working until the session says otherwise.
    pub fn from_progress_status(status: &str) -> Self {
        match status {
            "completed" => SubAgentState::Succeeded,
            "failed" => SubAgentState::Failed,
            "aborted" => SubAgentState::Interrupted,
            _ => SubAgentState::Working,
        }
    }

    /// The `kind` of a `session_exit` record: `normal` succeeded, an error failed, anything else
    /// was interrupted.
    pub fn from_exit_kind(kind: &str) -> Self {
        match kind {
            "normal" => SubAgentState::Succeeded,
            "error" => SubAgentState::Failed,
            _ => SubAgentState::Interrupted,
        }
    }
}

/// One sub-agent of a `task` tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SubAgent {
    /// omp's identity for it; names its session file.
    pub id: String,
    /// omp's name for it.
    pub name: String,
    pub state: SubAgentState,
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// The sub-agents listed in `rawOutput.details.progress[]` of a `task` tool call, in omp's order;
/// `None` when the output carries no such list.
pub fn from_raw_output(raw_output: &Value) -> Option<Vec<SubAgent>> {
    let progress = raw_output.get("details")?.get("progress")?.as_array()?;
    Some(
        progress
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let id = text(item, "id")
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("#{}", index + 1));
                let name = ["name", "agent", "description", "id"]
                    .iter()
                    .find_map(|key| text(item, key))
                    .unwrap_or("agent")
                    .to_owned();
                let state = text(item, "status")
                    .map_or(SubAgentState::Working, SubAgentState::from_progress_status);
                SubAgent { id, name, state }
            })
            .collect(),
    )
}

/// Whether `id` may name a sub-agent session file: `^[A-Za-z0-9_.-]+$`, no leading `.`.
pub fn is_valid_agent_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('.')
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
}

/// The folder of the sub-agents of a main session file: its path minus `.jsonl`.
pub fn agent_folder(main_file: &Path) -> Option<PathBuf> {
    let name = main_file.file_name()?.to_str()?.strip_suffix(".jsonl")?;
    Some(main_file.with_file_name(name))
}

/// The main session file of `session_id` under `sessions_root` (`<root>/<folder>/<time>_<id>.jsonl`).
pub fn find_main_file(sessions_root: &Path, session_id: &str) -> Option<PathBuf> {
    if session_id.is_empty()
        || !session_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return None;
    }
    let suffix = format!("_{session_id}.jsonl");
    for folder in std::fs::read_dir(sessions_root)
        .ok()?
        .filter_map(Result::ok)
    {
        let Ok(files) = std::fs::read_dir(folder.path()) else {
            continue;
        };
        for file in files.filter_map(Result::ok) {
            if file.file_name().to_string_lossy().ends_with(&suffix) {
                return Some(file.path());
            }
        }
    }
    None
}

/// The session file of the background sub-agent `id` of the session `main_file`. `None` when `id`
/// is not a valid name, or when the real path of the file leaves the sub-agents' folder.
pub fn agent_file(main_file: &Path, id: &str) -> Option<PathBuf> {
    if !is_valid_agent_id(id) {
        return None;
    }
    let folder = std::fs::canonicalize(agent_folder(main_file)?).ok()?;
    let file = std::fs::canonicalize(folder.join(format!("{id}.jsonl"))).ok()?;
    file.starts_with(&folder).then_some(file)
}

/// How the sub-agent whose session file holds `jsonl` ended: the last `session_exit` record, or
/// `None` when there is none yet.
pub fn exit_state(jsonl: &str) -> Option<SubAgentState> {
    jsonl
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .rfind(|record| record.get("customType").and_then(Value::as_str) == Some("session_exit"))
        .map(|record| {
            let kind = record
                .get("data")
                .and_then(|data| data.get("kind"))
                .or_else(|| record.get("kind"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            SubAgentState::from_exit_kind(kind)
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn progress_is_read_in_omps_order_with_the_status_mapping() {
        let output = json!({ "details": { "progress": [
            { "id": "0-Reader", "name": "Reader", "status": "running" },
            { "id": "1-Writer", "agent": "trama-implementer", "status": "pending" },
            { "id": "2", "description": "Compare", "status": "completed" },
            { "id": "3", "status": "failed" },
            { "status": "aborted" },
            { "id": "5", "name": "  ", "agent": "x", "status": "weird" },
        ] } });
        let agents = from_raw_output(&output).unwrap();
        let view: Vec<(&str, &str, SubAgentState)> = agents
            .iter()
            .map(|a| (a.id.as_str(), a.name.as_str(), a.state))
            .collect();
        assert_eq!(
            view,
            [
                ("0-Reader", "Reader", SubAgentState::Working),
                ("1-Writer", "trama-implementer", SubAgentState::Working),
                ("2", "Compare", SubAgentState::Succeeded),
                ("3", "3", SubAgentState::Failed),
                ("#5", "agent", SubAgentState::Interrupted),
                ("5", "x", SubAgentState::Working),
            ]
        );
        assert_eq!(from_raw_output(&json!({ "details": {} })), None);
        assert_eq!(from_raw_output(&json!("text")), None);
    }

    #[test]
    fn exit_kinds_map_to_succeeded_failed_or_interrupted() {
        assert_eq!(
            SubAgentState::from_exit_kind("normal"),
            SubAgentState::Succeeded
        );
        assert_eq!(
            SubAgentState::from_exit_kind("error"),
            SubAgentState::Failed
        );
        assert_eq!(
            SubAgentState::from_exit_kind("signal"),
            SubAgentState::Interrupted
        );
        let file = "{\"type\":\"message\"}\n{\"type\":\"custom\",\"customType\":\"session_exit\",\"data\":{\"kind\":\"normal\"}}\nnot json\n";
        assert_eq!(exit_state(file), Some(SubAgentState::Succeeded));
        assert_eq!(exit_state("{\"type\":\"message\"}\n"), None);
        assert_eq!(
            exit_state("{\"customType\":\"session_exit\",\"kind\":\"error\"}\n"),
            Some(SubAgentState::Failed)
        );
    }

    #[test]
    fn only_plain_names_may_name_a_session_file() {
        for good in ["0-Reader", "a_b.c", "x"] {
            assert!(is_valid_agent_id(good), "{good}");
        }
        for bad in ["", ".hidden", "../x", "a/b", "a b", "é", "a\\b"] {
            assert!(!is_valid_agent_id(bad), "{bad}");
        }
    }

    #[test]
    fn the_files_of_the_background_sub_agents_stay_in_their_folder() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("--proj--");
        std::fs::create_dir_all(&folder).unwrap();
        let main = folder.join("2026-11-02T07-00-00-000Z_sess-1.jsonl");
        std::fs::write(&main, "{}\n").unwrap();
        assert_eq!(find_main_file(root.path(), "sess-1"), Some(main.clone()));
        assert_eq!(find_main_file(root.path(), "sess-2"), None);
        assert_eq!(find_main_file(root.path(), "../x"), None);

        let agents = agent_folder(&main).unwrap();
        assert_eq!(
            agents.file_name().unwrap(),
            "2026-11-02T07-00-00-000Z_sess-1"
        );
        std::fs::create_dir_all(&agents).unwrap();
        std::fs::write(agents.join("0-Reader.jsonl"), "{}\n").unwrap();
        let found = agent_file(&main, "0-Reader").unwrap();
        assert!(found.ends_with("0-Reader.jsonl"));
        assert_eq!(agent_file(&main, "1-Missing"), None);
        assert_eq!(agent_file(&main, "../escape"), None);

        #[cfg(unix)]
        {
            let outside = root.path().join("outside.jsonl");
            std::fs::write(&outside, "{}\n").unwrap();
            std::os::unix::fs::symlink(&outside, agents.join("2-Link.jsonl")).unwrap();
            assert_eq!(
                agent_file(&main, "2-Link"),
                None,
                "a link out of the folder is refused"
            );
        }
    }
}
