---
name: weaver-req-tech-windows
description: "Use when building windows, their closing and quitting, or system notifications."
---

# Technology

Weaver is a Tauri 2 desktop app for macOS, Windows and Linux with a Rust core shared by the app and the `weaver` command line, and it drives omp over ACP.

## window

- **Single instance:** `tauri-plugin-single-instance` is registered first.
  - A second launch passes its arguments and cwd to the running instance and exits.
  - A folder argument is `developer.OPEN_PROJECT`. Otherwise the most recently focused window comes forward.
- **Project identity:** the canonical path, with symlinks resolved and the Windows `\\?\` prefix stripped (`dunce`), compared case-insensitively on Windows.
- **Window labels:** a project window is `p-<project hash>`; the welcome window is `welcome`.
- **Window state:** `tauri-plugin-window-state` saves size, position, maximised and fullscreen per label, never visibility.
  - A position on a missing monitor falls back to centred on the primary monitor.
  - Default 1280×800, minimum 960×600.
- **Close confirmation:**
  - `WindowEvent::CloseRequested` is prevented while jobs of that project work or its File view has unsaved changes, and `RunEvent::ExitRequested` while jobs of any project work or any File view has unsaved changes. The UI tells the backend, at every change, whether its File view has unsaved changes and in which files.
  - The confirmation is a native `tauri-plugin-dialog` dialog, so it works even with a hung webview. It says how many jobs work and lists the files with unsaved changes. With unsaved changes its buttons are `Save all`, `Discard all` and `Cancel`; with working jobs only, it asks to confirm or cancel.
  - On quit, every window with unsaved changes asks in turn, its dialog also counting the jobs of every project; Cancel in any of them stops the quit and nothing more closes.
  - `Save all`: the backend asks that window's UI for the texts of its tabs, as `requests-flush` does for the request editor, and waits at most the same time limit; then saves them as `developer.SAVE_FILE`. When the UI does not answer in time, or a file cannot be saved, the window stays open, the UI shows the tab of that file, and nothing is suspended.
  - After confirming and saving, the jobs are suspended (omp ended as in `requirements/tech#job`), then the window closes or the app exits.
- **Last window:**
  - Closing the last project window opens the welcome window.
  - Closing the welcome window quits on Windows and Linux. On macOS the app stays in the Dock with no window, and `RunEvent::Reopen` shows the welcome window.
  - Quit is `Cmd+Q` on macOS, `Ctrl+Q` and File › Quit elsewhere.
- **Restore list:** written only when Weaver quits. It lists the project windows open then.
- **Abnormal ends:** a shutdown that allows no confirmation suspends the jobs without asking. After a crash, the last-written resume registry brings them back as Suspended.

## notification

- **Sending:** `tauri-plugin-notification` ≥ 2.8, called from Rust only. Desktop clicks arrive through `Notification::on_action`. There is no Weaver setting; the OS permission decides.
- **Foreground test, at send time:** the project window is visible, not minimised and `is_focused()`, tracked from `WindowEvent::Focused`. Another focused Weaver window does not count.
- **Click routing:**
  - Each notification gets an `i32` id from an app-wide counter, mapped to the project hash and its target (job, round or permission).
  - A `tap` on a mapped id is `desktop.NOTIFICATION_CLICKED`: the project window is shown, unminimised and focused, and the UI is told what to show.
  - Clicks on unmapped ids, or for a closed project, are ignored.
  - **App notification:** the System Certificates notification is mapped to no project and skips the foreground test. Its `tap` focuses the project window used most recently (focus order of the window registry) and tells its UI to show Settings; with no project window, it focuses the welcome window. A system that does not show it while Weaver is in front changes nothing else: the banners still say it.
- **Withdrawing:**
  - Linux: `remove_active`.
  - macOS and Windows cannot remove a shown notification. Weaver drops its mapping instead, so a later click only brings the window forward.
  - All mappings are dropped when the window closes.
- **Windows:** a click is reported only while the toast is on screen. A click from the Action Center launches Weaver, and single-instance focuses the latest window.
