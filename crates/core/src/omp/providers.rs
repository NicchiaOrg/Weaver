//! The providers omp can log in to, in the order Weaver lists them (`requirements/tech#omp-setup`).
//!
//! Only an id of this list is ever put into a terminal command. `cursor` is offered through login
//! only: Weaver has no API key field and stores no secret.

use serde::Serialize;
use ts_rs::TS;

/// One provider of the fixed list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provider {
    /// The id omp uses (`omp login <id>`, `provider` in `omp models --json`).
    pub id: &'static str,
    /// The short name, shown in bold.
    pub name: &'static str,
    /// The plan or account the login is for, shown as secondary text.
    pub plan: &'static str,
}

/// The fixed list, in display order.
pub const PROVIDERS: [Provider; 7] = [
    Provider {
        id: "openai-codex",
        name: "OpenAI Codex",
        plan: "ChatGPT",
    },
    Provider {
        id: "anthropic",
        name: "Anthropic",
        plan: "Claude Pro or Max",
    },
    Provider {
        id: "github-copilot",
        name: "GitHub Copilot",
        plan: "Copilot subscription",
    },
    Provider {
        id: "google-gemini-cli",
        name: "Gemini CLI",
        plan: "Google account",
    },
    Provider {
        id: "google-antigravity",
        name: "Antigravity",
        plan: "Google account",
    },
    Provider {
        id: "cursor",
        name: "Cursor",
        plan: "Cursor account",
    },
    Provider {
        id: "xai-oauth",
        name: "xAI",
        plan: "Grok account",
    },
];

/// The provider with this id, when it is in the fixed list.
pub fn provider(id: &str) -> Option<&'static Provider> {
    PROVIDERS.iter().find(|p| p.id == id)
}

/// A row of `Providers` as the UI shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRow {
    pub id: String,
    pub name: String,
    pub plan: String,
    /// Whether a model of `omp models --json` has this provider; `None` while omp has not answered.
    pub connected: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_has_the_ids_in_the_required_order() {
        let ids: Vec<&str> = PROVIDERS.iter().map(|p| p.id).collect();
        assert_eq!(
            ids,
            [
                "openai-codex",
                "anthropic",
                "github-copilot",
                "google-gemini-cli",
                "google-antigravity",
                "cursor",
                "xai-oauth"
            ]
        );
    }

    #[test]
    fn only_ids_of_the_list_are_providers() {
        assert!(provider("cursor").is_some());
        assert!(provider("anthropic; rm -rf /").is_none());
        assert!(provider("").is_none());
    }

    #[test]
    fn ids_contain_nothing_a_shell_would_interpret() {
        for p in PROVIDERS {
            assert!(
                p.id.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "{}",
                p.id
            );
        }
    }
}
