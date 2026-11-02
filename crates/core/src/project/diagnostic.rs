//! Diagnostics: a file that cannot be read or interpreted, or a rule the project breaks.
//!
//! A diagnostic carries the path (relative to `.trama/`, with `/` separators) and the reason.
//! It is shown in the project window and in the output of the CLI command that met it; the
//! file is never dropped silently nor rewritten, and the other files still load.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Fails validation and the hand-over.
    Error,
    /// Reported, never fails anything.
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Path relative to `.trama/`, `/`-separated.
    pub path: String,
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    pub fn error(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            severity: Severity::Error,
            message: message.into(),
        }
    }

    pub fn warning(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            severity: Severity::Warning,
            message: message.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{}: {level}: {}", self.path, self.message)
    }
}

/// Whether any diagnostic of `list` is an error.
pub fn has_errors(list: &[Diagnostic]) -> bool {
    list.iter().any(Diagnostic::is_error)
}
