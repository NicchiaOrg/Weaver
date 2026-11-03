//! Sending a fullscreen window to the background (`developer.SEND_TO_BACKGROUND`).
//!
//! On macOS a fullscreen window lives in its own Space: hiding it there leaves that Space on the
//! screen, empty and black. So a fullscreen window first leaves fullscreen and is hidden once
//! AppKit says the transition ended (`NSWindowDidExitFullScreenNotification`); when it comes back
//! it goes to fullscreen again. A window brought back before the transition ended is not hidden.
//! Elsewhere hiding a fullscreen window needs nothing more.

#[cfg(target_os = "macos")]
use std::collections::HashSet;
#[cfg(target_os = "macos")]
use std::sync::{Arc, Mutex};

use tauri::WebviewWindow;

/// The windows that left fullscreen to go to the background (macOS).
#[derive(Default)]
pub struct Fullscreen {
    /// Labels of the windows to put back in fullscreen when they come back. A label still here
    /// when the exit transition ends means the window is to be hidden.
    #[cfg(target_os = "macos")]
    away: Arc<Mutex<HashSet<String>>>,
}

impl Fullscreen {
    /// Takes `window` off the screen; a fullscreen window leaves fullscreen first (macOS).
    pub fn hide(&self, window: &WebviewWindow) -> tauri::Result<()> {
        #[cfg(target_os = "macos")]
        if window.is_fullscreen()? {
            self.away_set().insert(window.label().to_owned());
            let (away, target) = (self.away.clone(), window.clone());
            return window.run_on_main_thread(move || mac::leave_then_hide(target, away));
        }
        window.hide()
    }

    /// After `window` is shown again: back to fullscreen when it left it to go to the background.
    pub fn restore(&self, window: &WebviewWindow) {
        #[cfg(target_os = "macos")]
        if self.away_set().remove(window.label()) {
            if let Err(error) = window.set_fullscreen(true) {
                crate::log_warn!(
                    "the window {} did not go back to fullscreen: {error}",
                    window.label()
                );
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = window;
    }

    /// The window `label` is gone: a later window with the same label starts afresh.
    pub fn forget(&self, label: &str) {
        #[cfg(target_os = "macos")]
        self.away_set().remove(label);
        #[cfg(not(target_os = "macos"))]
        let _ = label;
    }

    #[cfg(target_os = "macos")]
    fn away_set(&self) -> std::sync::MutexGuard<'_, HashSet<String>> {
        self.away.lock().expect("fullscreen windows poisoned")
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::Cell;
    use std::collections::HashSet;
    use std::ptr::NonNull;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, ProtocolObject};
    use objc2_app_kit::NSWindowDidExitFullScreenNotification;
    use objc2_foundation::{
        NSNotification, NSNotificationCenter, NSObjectProtocol, NSOperationQueue,
    };
    use tauri::WebviewWindow;

    use crate::log_warn;

    type Observer = Retained<ProtocolObject<dyn NSObjectProtocol>>;

    /// On the main thread: leaves fullscreen, and hides the window when the transition ended.
    pub(super) fn leave_then_hide(window: WebviewWindow, away: Arc<Mutex<HashSet<String>>>) {
        let ns_window = match window.ns_window() {
            Ok(pointer) => pointer.cast::<AnyObject>(),
            Err(error) => {
                log_warn!(
                    "the window {} has no native window: {error}",
                    window.label()
                );
                return hide_if_away(&window, &away);
            }
        };
        let center = NSNotificationCenter::defaultCenter();
        // The observer removes itself the first time it is called; taking it out of the cell
        // also ends the cycle between it and its block.
        let slot: Rc<Cell<Option<Observer>>> = Rc::default();
        let block = {
            let (slot, window, away) = (slot.clone(), window.clone(), away.clone());
            RcBlock::new(move |_: NonNull<NSNotification>| {
                let Some(observer) = slot.take() else { return };
                remove(&observer);
                // Hidden on a later turn of the event loop, after AppKit and tao are done with
                // the transition.
                let (target, away) = (window.clone(), away.clone());
                if let Err(error) = window.run_on_main_thread(move || hide_if_away(&target, &away))
                {
                    log_warn!("the window {} was not hidden: {error}", window.label());
                }
            })
        };
        // SAFETY: `ns_window` is the live NSWindow of `window`, which outlives this call; the
        // notification is posted on the main thread and the block runs on the main queue.
        let observer = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWindowDidExitFullScreenNotification),
                Some(&*ns_window),
                Some(&NSOperationQueue::mainQueue()),
                &block,
            )
        };
        slot.set(Some(observer));
        if let Err(error) = window.set_fullscreen(false) {
            log_warn!(
                "the window {} did not leave fullscreen: {error}",
                window.label()
            );
            if let Some(observer) = slot.take() {
                remove(&observer);
            }
            hide_if_away(&window, &away);
        }
    }

    fn remove(observer: &Observer) {
        let observer: &AnyObject = observer.as_ref();
        // SAFETY: `observer` was returned by the default center's `addObserverForName…`.
        unsafe { NSNotificationCenter::defaultCenter().removeObserver(observer) };
    }

    /// Hides `window` unless it came back while it was leaving fullscreen.
    fn hide_if_away(window: &WebviewWindow, away: &Mutex<HashSet<String>>) {
        let still_away = away
            .lock()
            .expect("fullscreen windows poisoned")
            .contains(window.label());
        if still_away {
            if let Err(error) = window.hide() {
                log_warn!("the window {} was not hidden: {error}", window.label());
            }
        }
    }
}
