//! The tray icon (Windows and Linux): where Weaver stays reachable while its windows are in the
//! background.
//!
//! Created once at start with Tauri 2's tray (`TrayIconBuilder`) and kept until Weaver exits, with
//! the icon of the app and the tooltip `Weaver`. The menu has `Show Weaver`
//! (`developer.REOPEN_APP`), a separator and `Quit` (`developer.QUIT_APP`, with the same
//! confirmations as `Ctrl+Q`). On Windows a left click (button released) is `Show Weaver` and the
//! right click opens the menu; on Linux there is only the menu, since libayatana-appindicator
//! reports no clicks. There is no tray icon on macOS.
//!
//! An icon that cannot be created is logged as a warning and changes nothing else: launching Weaver
//! again still brings the windows back.

use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::AppHandle;

/// Id of `Show Weaver`.
pub const SHOW_ID: &str = "tray-show";
/// Id of `Quit`.
pub const QUIT_ID: &str = "tray-quit";
/// The tooltip of the icon.
pub const TOOLTIP: &str = "Weaver";

/// What the developer asked of the tray icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    /// `developer.REOPEN_APP`.
    Show,
    /// `developer.QUIT_APP`.
    Quit,
}

/// The action of a menu item.
pub fn menu_action(id: &str) -> Option<TrayAction> {
    match id {
        SHOW_ID => Some(TrayAction::Show),
        QUIT_ID => Some(TrayAction::Quit),
        _ => None,
    }
}

/// The action of a click on the icon itself (Windows): the left button, when it is released. A
/// press, another button, a double click and the pointer moving are nothing.
pub fn click_action(event: &TrayIconEvent) -> Option<TrayAction> {
    match event {
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } => Some(TrayAction::Show),
        _ => None,
    }
}

/// Does what the developer asked.
#[cfg(not(target_os = "macos"))]
fn run(app: &AppHandle, action: TrayAction) {
    match action {
        TrayAction::Show => {
            if let Some(manager) = super::manager_of(app) {
                manager.reopen_app();
            }
        }
        TrayAction::Quit => super::request_quit(app),
    }
}

/// Creates the tray icon and keeps it until Weaver exits; a failure is a warning.
#[cfg(not(target_os = "macos"))]
pub fn install(app: &AppHandle) {
    match create(app) {
        Ok(tray) => {
            // Held by the app: the icon goes away when its last handle is dropped.
            if !tauri::Manager::manage(app, tray) {
                crate::log_warn!("the tray icon was created twice");
            }
        }
        Err(why) => crate::log_warn!("the tray icon could not be created: {why}"),
    }
}

/// There is no tray icon on macOS.
#[cfg(target_os = "macos")]
pub fn install(_app: &AppHandle) {}

#[cfg(not(target_os = "macos"))]
fn create(app: &AppHandle) -> Result<tauri::tray::TrayIcon, String> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("the app has no icon")?;
    let show = MenuItem::with_id(app, SHOW_ID, "Show Weaver", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let separator = PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Quit", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let menu =
        Menu::with_items(app, &[&show, &separator, &quit]).map_err(|error| error.to_string())?;

    let builder = TrayIconBuilder::with_id("weaver")
        .icon(icon)
        .tooltip(TOOLTIP)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if let Some(action) = menu_action(event.id().as_ref()) {
                run(app, action);
            }
        });
    // Windows reports the clicks; Linux reports none.
    #[cfg(windows)]
    let builder = builder.on_tray_icon_event(|tray, event| {
        if let Some(action) = click_action(&event) {
            run(tray.app_handle(), action);
        }
    });
    builder.build(app).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::tray::TrayIconId;
    use tauri::{PhysicalPosition, Rect};

    fn click(button: MouseButton, button_state: MouseButtonState) -> TrayIconEvent {
        TrayIconEvent::Click {
            id: TrayIconId::new("weaver"),
            position: PhysicalPosition::new(0.0, 0.0),
            rect: Rect::default(),
            button,
            button_state,
        }
    }

    #[test]
    fn the_menu_has_show_weaver_and_quit() {
        assert_eq!(menu_action(SHOW_ID), Some(TrayAction::Show));
        assert_eq!(menu_action(QUIT_ID), Some(TrayAction::Quit));
        assert_eq!(
            menu_action("quit"),
            None,
            "the Quit of the File menu is not the tray's"
        );
        assert_eq!(menu_action(""), None);
    }

    #[test]
    fn the_ids_do_not_clash_with_the_application_menu() {
        assert_ne!(SHOW_ID, super::super::menu::QUIT_ID);
        assert_ne!(QUIT_ID, super::super::menu::QUIT_ID);
        assert_ne!(QUIT_ID, super::super::menu::CLOSE_WINDOW_ID);
    }

    #[test]
    fn a_left_click_released_is_show_weaver_and_nothing_else_is() {
        assert_eq!(
            click_action(&click(MouseButton::Left, MouseButtonState::Up)),
            Some(TrayAction::Show)
        );
        assert_eq!(
            click_action(&click(MouseButton::Left, MouseButtonState::Down)),
            None
        );
        assert_eq!(
            click_action(&click(MouseButton::Right, MouseButtonState::Up)),
            None
        );
        assert_eq!(
            click_action(&click(MouseButton::Right, MouseButtonState::Down)),
            None
        );
    }

    #[test]
    fn the_tooltip_is_weaver() {
        assert_eq!(TOOLTIP, "Weaver");
    }
}
