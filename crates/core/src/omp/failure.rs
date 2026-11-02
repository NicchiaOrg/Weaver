//! Why an omp command failed, in words for the developer (`requirements/tech#omp`, "Failure
//! reasons"): the last non-empty line of stderr, else `exit code N`, `could not start omp` or
//! `timed out`. A certificate or proxy marker in stderr adds a plain hint.

use std::time::Duration;

/// A command that did not complete normally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The program could not be started.
    NotStarted,
    /// The limit passed; the process was killed.
    TimedOut { limit: Duration },
    /// The process ended with a failure status (`code` is `None` when a signal ended it).
    Exit { code: Option<i32>, stderr: String },
}

/// Markers of an HTTPS-inspecting proxy or security tool.
const CERTIFICATE_MARKERS: [&str; 6] = [
    "UNABLE_TO_GET_ISSUER_CERT_LOCALLY",
    "SELF_SIGNED_CERT_IN_CHAIN",
    "CERT_HAS_EXPIRED",
    "certificate verify failed",
    "SSL certificate problem",
    "curl: (60)",
];

pub const CERTIFICATE_HINT: &str =
    "A proxy or security tool is likely inspecting HTTPS, and omp must trust its certificate.";
pub const PROXY_HINT: &str =
    "A proxy or the provider refused the request: check the proxy settings or try again later.";

/// The last non-empty line of `stderr`, trimmed.
pub fn last_line(stderr: &str) -> Option<&str> {
    stderr.lines().map(str::trim).rfind(|line| !line.is_empty())
}

/// The hint that goes with `stderr`, when it carries a certificate marker, else an HTTP `403`,
/// `407` or `5xx` (`HTTP <code>`, `status <code>`, `<code> <phrase>`).
pub fn hint(stderr: &str) -> Option<&'static str> {
    if CERTIFICATE_MARKERS
        .iter()
        .any(|marker| stderr.contains(marker))
    {
        return Some(CERTIFICATE_HINT);
    }
    has_refusing_status(stderr).then_some(PROXY_HINT)
}

fn refusing(code: &str) -> bool {
    matches!(code.parse::<u16>(), Ok(403 | 407 | 500..=599)) && code.len() == 3
}

fn has_refusing_status(stderr: &str) -> bool {
    let tokens: Vec<&str> = stderr
        .split(|c: char| {
            c.is_whitespace() || matches!(c, ',' | ';' | ':' | '(' | ')' | '[' | ']' | '"' | '\'')
        })
        .filter(|t| !t.is_empty())
        .collect();
    tokens.iter().enumerate().any(|(i, token)| {
        if !refusing(token) {
            return false;
        }
        let before = i.checked_sub(1).map(|j| tokens[j].to_ascii_lowercase());
        let after_is_phrase = tokens
            .get(i + 1)
            .is_some_and(|next| next.chars().next().is_some_and(char::is_uppercase));
        let named = before
            .as_deref()
            .is_some_and(|b| b == "http" || b == "status" || b.starts_with("http/"));
        named || after_is_phrase
    })
}

impl Failure {
    /// The reason in the words of the requirements: the last stderr line, else `exit code N`,
    /// `could not start omp` or `timed out`.
    pub fn reason(&self) -> String {
        match self {
            Failure::NotStarted => "could not start omp".to_owned(),
            Failure::TimedOut { .. } => "timed out".to_owned(),
            Failure::Exit { code, stderr } => match (last_line(stderr), code) {
                (Some(line), _) => line.to_owned(),
                (None, Some(code)) => format!("exit code {code}"),
                (None, None) => "omp was stopped".to_owned(),
            },
        }
    }

    /// [`Failure::reason`] followed by the hint for a certificate or proxy problem.
    pub fn reason_with_hint(&self) -> String {
        let reason = self.reason();
        match self.hint() {
            Some(hint) => format!("{} {hint}", reason.trim_end_matches('.').to_owned() + "."),
            None => reason,
        }
    }

    fn hint(&self) -> Option<&'static str> {
        match self {
            Failure::Exit { stderr, .. } => hint(stderr),
            _ => None,
        }
    }

    /// The reason in the words of the status box of `Missing`: `omp did not answer within 15 s`,
    /// `omp exited with code <N>`, else the last line of stderr.
    pub fn status_text(&self) -> String {
        match self {
            Failure::TimedOut { limit } => {
                format!("omp did not answer within {} s", limit.as_secs())
            }
            Failure::Exit { code, stderr } if last_line(stderr).is_none() => match code {
                Some(code) => format!("omp exited with code {code}"),
                None => "omp was stopped".to_owned(),
            },
            other => other.reason_with_hint(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exit(code: i32, stderr: &str) -> Failure {
        Failure::Exit {
            code: Some(code),
            stderr: stderr.to_owned(),
        }
    }

    #[test]
    fn the_reason_is_the_last_non_empty_line_of_stderr() {
        assert_eq!(
            exit(1, "first\nsecond problem  \n\n  \n").reason(),
            "second problem"
        );
    }

    #[test]
    fn without_stderr_the_reason_is_the_exit_code_or_the_way_it_failed() {
        assert_eq!(exit(3, "").reason(), "exit code 3");
        assert_eq!(exit(3, " \n").reason(), "exit code 3");
        assert_eq!(Failure::NotStarted.reason(), "could not start omp");
        assert_eq!(
            Failure::TimedOut {
                limit: Duration::from_secs(15)
            }
            .reason(),
            "timed out"
        );
    }

    #[test]
    fn the_status_box_texts_for_missing() {
        assert_eq!(
            Failure::TimedOut {
                limit: Duration::from_secs(15)
            }
            .status_text(),
            "omp did not answer within 15 s"
        );
        assert_eq!(exit(2, "").status_text(), "omp exited with code 2");
        assert_eq!(exit(2, "bad flag").status_text(), "bad flag");
    }

    #[test]
    fn a_certificate_marker_adds_the_proxy_hint() {
        for marker in CERTIFICATE_MARKERS {
            assert_eq!(
                hint(&format!("fetch failed: {marker}")),
                Some(CERTIFICATE_HINT),
                "{marker}"
            );
        }
        let text = exit(1, "error: self signed\nSELF_SIGNED_CERT_IN_CHAIN").reason_with_hint();
        assert_eq!(
            text,
            format!("SELF_SIGNED_CERT_IN_CHAIN. {CERTIFICATE_HINT}")
        );
    }

    #[test]
    fn an_http_refusal_adds_the_proxy_or_provider_hint() {
        assert_eq!(hint("request failed: HTTP 403"), Some(PROXY_HINT));
        assert_eq!(hint("got status 407"), Some(PROXY_HINT));
        assert_eq!(hint("502 Bad Gateway"), Some(PROXY_HINT));
        assert_eq!(hint("HTTP/1.1 503 Service Unavailable"), Some(PROXY_HINT));
        assert_eq!(hint("status: 500"), Some(PROXY_HINT));
    }

    #[test]
    fn other_numbers_and_codes_add_no_hint() {
        assert_eq!(hint("HTTP 404"), None);
        assert_eq!(hint("exit code 403"), None);
        assert_eq!(hint("read 5000 bytes"), None);
        assert_eq!(hint("status 200"), None);
        assert_eq!(hint("4030 files"), None);
        assert_eq!(hint(""), None);
    }

    #[test]
    fn a_certificate_marker_wins_over_an_http_code() {
        assert_eq!(
            hint("HTTP 502 curl: (60) SSL certificate problem"),
            Some(CERTIFICATE_HINT)
        );
    }
}
