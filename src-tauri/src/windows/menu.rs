//! The application menu: what makes `Quit` reachable by the keyboard.
//!
//! Quit is `Cmd+Q` on macOS, where Tauri's default menu has it in the application menu and the
//! system raises `RunEvent::ExitRequested` like any other quit; on Windows and Linux it is
//! `Ctrl+Q` and **File › Quit**, a menu item of Weaver's own (the predefined one ends the process
//! without asking), which starts the quit path ([`super::request_quit`]). The Edit entries are
//! needed for copy and paste to work in the webview.

use tauri::menu::Menu;
use tauri::{AppHandle, Wry};

/// Id of **File › Quit** (Windows and Linux).
pub const QUIT_ID: &str = "quit";

/// The menu of every window.
#[cfg(target_os = "macos")]
pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    Menu::default(app)
}

/// The menu of every window.
#[cfg(not(target_os = "macos"))]
pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    use tauri::menu::{MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};

    let quit = MenuItemBuilder::with_id(QUIT_ID, "Quit")
        .accelerator("Ctrl+Q")
        .build(app)?;
    let file = SubmenuBuilder::new(app, "File")
        .item(&PredefinedMenuItem::close_window(app, None)?)
        .item(&quit)
        .build()?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .build()?;
    Menu::with_items(app, &[&file, &edit, &window])
}

/// A menu item was chosen.
pub fn on_event(app: &AppHandle, id: &str) {
    if id == QUIT_ID {
        super::request_quit(app);
    }
}
