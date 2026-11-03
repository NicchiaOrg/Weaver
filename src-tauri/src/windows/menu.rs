//! The application menu: what makes `Quit` and `Close Window` reachable by the keyboard.
//!
//! Quit is `Cmd+Q` on macOS, where Tauri's default menu has it in the application menu and the
//! system raises `RunEvent::ExitRequested` like any other quit; on Windows and Linux it is
//! `Ctrl+Q` and **File › Quit**, a menu item of Weaver's own (the predefined one ends the process
//! without asking), which starts the quit path ([`super::request_quit`]). The Edit entries are
//! needed for copy and paste to work in the webview.
//!
//! **Close Window** (`Cmd+W` on macOS, `Ctrl+W` elsewhere) is also an item of Weaver's own, since a
//! menu accelerator is handled before the webview sees the key: while the File view of the window
//! has a tab ([`super::WindowManager::close_key_claimed`]) it closes that tab (`files-close-tab`),
//! otherwise it closes the window, through the same close path as its title bar button.

use tauri::menu::{Menu, MenuItemBuilder};
use tauri::{AppHandle, Manager, Wry};

/// Id of **File › Quit** (Windows and Linux).
pub const QUIT_ID: &str = "quit";
/// Id of **Close Window**.
pub const CLOSE_WINDOW_ID: &str = "close-window";

fn close_window_item(app: &AppHandle) -> tauri::Result<tauri::menu::MenuItem<Wry>> {
    MenuItemBuilder::with_id(CLOSE_WINDOW_ID, "Close Window")
        .accelerator(if cfg!(target_os = "macos") {
            "Cmd+W"
        } else {
            "Ctrl+W"
        })
        .build(app)
}

/// The menu of every window.
#[cfg(target_os = "macos")]
pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    use tauri::menu::MenuItemKind;

    let menu = Menu::default(app)?;
    // The default menu closes the window itself, on `Cmd+W`: its first **Close Window** becomes ours.
    for kind in menu.items()? {
        let MenuItemKind::Submenu(submenu) = kind else {
            continue;
        };
        for (at, item) in submenu.items()?.into_iter().enumerate() {
            let MenuItemKind::Predefined(predefined) = &item else {
                continue;
            };
            if predefined.text()? != "Close Window" {
                continue;
            }
            submenu.remove(&item)?;
            submenu.insert(&close_window_item(app)?, at)?;
            return Ok(menu);
        }
    }
    Ok(menu)
}

/// The menu of every window.
#[cfg(not(target_os = "macos"))]
pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    use tauri::menu::SubmenuBuilder;

    let quit = MenuItemBuilder::with_id(QUIT_ID, "Quit")
        .accelerator("Ctrl+Q")
        .build(app)?;
    let file = SubmenuBuilder::new(app, "File")
        .item(&close_window_item(app)?)
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
    match id {
        QUIT_ID => super::request_quit(app),
        CLOSE_WINDOW_ID => close_window(app),
        _ => {}
    }
}

/// **Close Window** in the window that has the focus: a tab of its File view, or the window.
fn close_window(app: &AppHandle) {
    let Some(window) = app
        .webview_windows()
        .into_values()
        .find(|window| window.is_focused().unwrap_or(false))
    else {
        return;
    };
    if let (Some(project_id), Some(manager)) = (
        crate::pictures::project_of_label(window.label()),
        super::manager_of(app),
    ) {
        if manager.close_key_claimed(project_id) {
            super::unsaved::emit_close_tab(app, project_id);
            return;
        }
    }
    if let Err(error) = window.close() {
        crate::log_warn!("the window {} could not be closed: {error}", window.label());
    }
}
