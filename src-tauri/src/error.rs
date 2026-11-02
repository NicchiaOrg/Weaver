//! The typed error every Tauri command returns. The UI gets `{ code, message }` and decides
//! what to show from the code.

use serde::Serialize;
use ts_rs::TS;
use weaver_core::lock::LockError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    /// A bug or an unexpected system failure.
    Internal,
    /// The input of a command is not acceptable.
    InvalidInput,
    /// The project lock was not obtained within 10 s.
    ProjectBusy,
    /// A decision of the developer was refused when it was applied; nothing changed. The message
    /// is the notice the Board shows (`Could not delete T4: the agent that filed it is still working.`).
    Refused,
}

#[derive(Debug, Clone, Serialize, TS, thiserror::Error)]
#[error("{message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn internal(message: impl std::fmt::Display) -> Self {
        Self::new(ErrorCode::Internal, message.to_string())
    }
}

impl From<LockError> for AppError {
    fn from(error: LockError) -> Self {
        match error {
            LockError::Busy => Self::new(ErrorCode::ProjectBusy, "project busy"),
            LockError::Io(e) => Self::internal(e),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::internal(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialises_as_code_and_message() {
        let json =
            serde_json::to_value(AppError::new(ErrorCode::ProjectBusy, "project busy")).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "project-busy", "message": "project busy" })
        );
    }

    #[test]
    fn a_busy_lock_is_project_busy() {
        let error = AppError::from(LockError::Busy);
        assert_eq!(error.code, ErrorCode::ProjectBusy);
        assert_eq!(error.to_string(), "project busy");
    }
}
