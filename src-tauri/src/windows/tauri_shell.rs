//! [`Shell`] over Tauri: the real windows, the native confirmation (`tauri-plugin-dialog`) and the
//! exit.
//!
//! * A project window is `p-<project hash>`, titled with the project's folder name, 1280 × 800 by
//!   default and never smaller than 960 × 600. It is created hidden and shown once built: the
//!   window-state plugin restores its size, position, maximised and fullscreen state while it is
//!   built (a saved position on a monitor that is gone is ignored, and the window stays centred on
//!   the primary monitor), and it never touches visibility.
//! * The welcome window is `welcome`, 640 × 480 and not resizable; its size never needs restoring,
//!   so the window-state plugin skips it (`with_denylist`).
//! * The confirmation is a native dialog, not a page of the webview, so it works with a hung webview.

use std::path::Path;

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_window_state::AppHandleExt;

use super::jobs::BoxFuture;
use super::manager::{ConfirmPrompt, Shell, ShowRequest};
use super::registry::WELCOME_LABEL;
use super::STATE_FLAGS;
use crate::events::{self, EventName, ProjectChanged};
use crate::log_warn;
use crate::pictures::Rebuild;

/// Size of a project window without a saved state.
pub const DEFAULT_SIZE: (f64, f64) = (1280.0, 800.0);
/// Smallest a project window gets.
pub const MIN_SIZE: (f64, f64) = (960.0, 600.0);
/// The welcome window.
pub const WELCOME_SIZE: (f64, f64) = (640.0, 480.0);

pub struct TauriShell {
    app: AppHandle,
}

impl TauriShell {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

/// Restores, shows and focuses a window.
fn bring_forward(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

impl Shell for TauriShell {
    fn create_project_window(&self, label: &str, title: &str) -> Result<(), String> {
        let window =
            WebviewWindowBuilder::new(&self.app, label, WebviewUrl::App("index.html".into()))
                .title(title)
                .inner_size(DEFAULT_SIZE.0, DEFAULT_SIZE.1)
                .min_inner_size(MIN_SIZE.0, MIN_SIZE.1)
                .center()
                .visible(false)
                .build()
                .map_err(|error| error.to_string())?;
        bring_forward(&window);
        Ok(())
    }

    fn show_welcome(&self) -> Result<(), String> {
        if let Some(window) = self.app.get_webview_window(WELCOME_LABEL) {
            bring_forward(&window);
            return Ok(());
        }
        let window = WebviewWindowBuilder::new(
            &self.app,
            WELCOME_LABEL,
            WebviewUrl::App("index.html".into()),
        )
        .title("Weaver")
        .inner_size(WELCOME_SIZE.0, WELCOME_SIZE.1)
        .resizable(false)
        .maximizable(false)
        .center()
        .visible(false)
        .build()
        .map_err(|error| error.to_string())?;
        bring_forward(&window);
        Ok(())
    }

    fn close_welcome(&self) {
        if let Some(window) = self.app.get_webview_window(WELCOME_LABEL) {
            let _ = window.destroy();
        }
    }

    fn focus(&self, label: &str) -> bool {
        match self.app.get_webview_window(label) {
            Some(window) => {
                bring_forward(&window);
                true
            }
            None => false,
        }
    }

    fn focus_any(&self) -> bool {
        match self.app.webview_windows().into_values().next() {
            Some(window) => {
                bring_forward(&window);
                true
            }
            None => false,
        }
    }

    fn destroy(&self, label: &str) {
        if let Some(window) = self.app.get_webview_window(label) {
            if let Err(error) = window.destroy() {
                log_warn!("the window {label} could not be closed: {error}");
            }
        }
    }

    fn window_count(&self) -> usize {
        self.app.webview_windows().len()
    }

    fn confirm(&self, prompt: ConfirmPrompt) -> BoxFuture<bool> {
        let (answer, receiver) = tokio::sync::oneshot::channel();
        self.app
            .dialog()
            .message(prompt.message)
            .title(prompt.title)
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                prompt.accept,
                "Cancel".to_owned(),
            ))
            .show(move |accepted| {
                let _ = answer.send(accepted);
            });
        // A dialog that never answered (the app is going away) counts as a cancel.
        Box::pin(async move { receiver.await.unwrap_or(false) })
    }

    fn exit(&self, code: i32) {
        self.app.exit(code);
    }

    fn emit_show(&self, label: &str, request: &ShowRequest) {
        if let Err(error) = self
            .app
            .emit_to(label, EventName::ShowRequested.as_str(), request)
        {
            log_warn!("the window {label} was not told what to show: {error}");
        }
    }

    fn files_changed(&self, project_id: &str, root: &Path, batch: &Rebuild) {
        crate::requests::files_changed(&self.app, project_id, root, batch);
    }

    fn flush_before_close(&self, project_id: &str) -> BoxFuture<()> {
        let app = self.app.clone();
        let project_id = project_id.to_owned();
        Box::pin(async move { crate::requests::flush_window(&app, &project_id).await })
    }

    fn publish(&self, change: ProjectChanged) {
        if let Err(error) = events::emit_project_changed(&self.app, &change) {
            log_warn!("the UI was not told about a change of a project: {error}");
        }
    }

    fn save_window_state(&self) {
        if let Err(error) = self.app.save_window_state(STATE_FLAGS) {
            log_warn!("the size and position of the windows were not saved: {error}");
        }
    }
}
