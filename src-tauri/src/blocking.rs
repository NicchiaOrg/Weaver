//! Disk work never runs on the main thread or the IPC thread: it goes through [`disk`] or
//! [`disk_named`], which use Tokio's blocking pool (`spawn_blocking`) under Tauri's async runtime.
//!
//! A named operation that takes 200 ms or more is logged at `info` with its name, project and
//! duration; a panic inside it is logged (with its backtrace, by the panic hook) and comes back
//! as an [`AppError`] of that operation, never as a crash.

use std::time::Instant;

use crate::error::AppError;
use crate::{log_error, logging};

/// Runs blocking `f` (file I/O, locks, parsing) on the blocking pool and awaits its result.
pub async fn disk<T, F>(f: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    disk_named("disk", None, f).await
}

/// Like [`disk`], for an operation worth naming in the log (`project` is the folder name).
pub async fn disk_named<T, F>(
    name: &'static str,
    project: Option<String>,
    f: F,
) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let started = Instant::now();
    let joined = tauri::async_runtime::spawn_blocking(f).await;
    logging::report_if_slow(name, project.as_deref(), started.elapsed());
    joined.map_err(|error| {
        log_error!(ctx: logging::Context { project: project.as_deref(), job: None }, "operation `{name}` did not complete: {error}");
        AppError::internal(format!("operation `{name}` failed unexpectedly"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_panic_in_the_work_is_a_failure_of_that_operation_only() {
        // The default panic hook prints to stderr here; the app's hook is installed by `logging::init`.
        let failed = disk_named("explode", None, || -> u8 { panic!("boom") }).await;
        let error = failed.unwrap_err();
        assert_eq!(error.message, "operation `explode` failed unexpectedly");
        assert_eq!(disk(|| 7).await.unwrap(), 7);
    }
}
