//! A drag of assets out of Weaver, to another app (`developer.DRAG_ASSETS_OUT`).
//!
//! It uses CrabNebula's `drag` crate from the backend, never `tauri-plugin-drag`: that plugin lets
//! the webview name any absolute path, while here the UI names assets and the backend resolves
//! them ([`weaver_core::assets::ops::drag_paths`]). The drag starts on the main thread for the
//! window of the project (its GTK window on Linux) with `DragItem::Files`, so the app that
//! receives the drop gets files, and `DragMode::Copy`, so it gets copies whatever it asks: the
//! assets are never moved or changed. The preview image is bundled with the app.

use std::path::PathBuf;

use tauri::{Manager, WebviewWindow};

use crate::log_debug;

/// The image the system draws under the pointer while dragging.
const PREVIEW: &[u8] = include_bytes!("../../icons/32x32.png");

/// Starts the drag of `paths` (absolute, canonical) out of `window`. It answers once the system
/// took the drag over (on Windows, whose drag loop does not return before, when it ended); the
/// result of the drop, dropped or cancelled, only reaches the log.
pub async fn start(window: &WebviewWindow, paths: Vec<PathBuf>) -> Result<(), String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let target = window.clone();
    window
        .app_handle()
        .run_on_main_thread(move || {
            let _ = sender.send(start_on_main_thread(&target, paths));
        })
        .map_err(|error| error.to_string())?;
    receiver
        .await
        .map_err(|_| "the window closed before the drag started".to_owned())?
}

/// The drag, which the system wants started from the thread of the window.
fn start_on_main_thread(window: &WebviewWindow, paths: Vec<PathBuf>) -> Result<(), String> {
    let item = drag::DragItem::Files(paths);
    let image = drag::Image::Raw(PREVIEW.to_vec());
    let options = drag::Options {
        mode: drag::DragMode::Copy,
        ..Default::default()
    };
    let ended = |result: drag::DragResult, _: drag::CursorPosition| {
        log_debug!("a drag of assets out of Weaver ended: {result:?}");
    };
    #[cfg(target_os = "linux")]
    {
        let gtk_window = window.gtk_window().map_err(|error| error.to_string())?;
        drag::start_drag(&gtk_window, item, image, ended, options)
            .map_err(|error| error.to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        drag::start_drag(window, item, image, ended, options).map_err(|error| error.to_string())
    }
}
