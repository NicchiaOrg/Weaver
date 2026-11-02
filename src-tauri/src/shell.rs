//! Commands of the app shell: what the base interface asks of the backend besides project data.
//!
//! * `startup_failures`: the failed startup steps so far (the UI also gets each one as the
//!   `startup-failed` event while it runs).
//! * `open_log`: reveals the log file in the system's file manager (**Open log**).
//! * `open_url`: opens an `http`/`https` link in the system browser; the webview never navigates.

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, ErrorCode};
use crate::logging;
use crate::startup::{StartupFailure, StartupReport};

/// Only `http` and `https` links leave Weaver, and only well-formed ones: no whitespace or
/// control characters, something after the scheme.
pub fn is_openable_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"));
    rest.is_some_and(|rest| {
        !rest.is_empty()
            && !rest.starts_with('/')
            && !url.chars().any(|c| c.is_whitespace() || c.is_control())
    })
}

#[tauri::command]
pub fn startup_failures(report: State<'_, StartupReport>) -> Vec<StartupFailure> {
    report.failures()
}

#[tauri::command]
pub async fn open_log(app: AppHandle) -> Result<(), AppError> {
    let logger =
        logging::logger().ok_or_else(|| AppError::internal("the log was not opened at start"))?;
    crate::blocking::disk_named("open_log", None, move || {
        let file = logger.current_file();
        // The file exists once the first line of the day was written; otherwise show the folder.
        let target = if file.exists() { file } else { logger.dir() };
        app.opener()
            .reveal_item_in_dir(&target)
            .map_err(AppError::internal)
    })
    .await?
}

#[tauri::command]
pub async fn open_url(app: AppHandle, url: String) -> Result<(), AppError> {
    if !is_openable_url(&url) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "only http and https links can be opened",
        ));
    }
    crate::blocking::disk_named("open_url", None, move || {
        app.opener()
            .open_url(url, None::<&str>)
            .map_err(AppError::internal)
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_http_and_https_links_open() {
        for ok in [
            "http://example.com",
            "https://example.com/a?b=c#d",
            "HTTPS://Example.com",
            "http://localhost:1420/x",
        ] {
            assert!(is_openable_url(ok), "{ok}");
        }
        for bad in [
            "",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "ftp://example.com",
            "mailto:a@b.c",
            "data:text/html,x",
            "//example.com",
            "https://",
            "https:///x",
            "https://exa mple.com",
            "https://example.com/\u{7}",
            "example.com",
            "./relative",
            "#anchor",
        ] {
            assert!(!is_openable_url(bad), "{bad:?}");
        }
    }
}
