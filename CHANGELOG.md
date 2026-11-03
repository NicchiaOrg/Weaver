# Changelog

All notable changes to Weaver that a user of the app can see are documented in this file.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.1.0] - 2026-11-03

### Added

- Ask more opens a new conversation with omp about the Proposal, Task or Drift in a details dialog, a History entry with the Tasks it cites, the question in view of a round or the Requirements as a whole, from a button in the toolbar of the Requirements view. The first message reaches the agent together with what Weaver knows about the chosen thing; for the Requirements, the agent loads by itself the Requirement skills that fit the question. The conversation is in the Jobs view, is not a job and ends with the window.
- System Certificates, off by default, makes every omp process Weaver starts trust the certificate authorities the system trusts, so omp can install itself and reach its provider behind a corporate proxy that inspects HTTPS traffic. It is in Settings → omp, in the omp box of Get Started and in the welcome window.
- Weaver turns System Certificates on by itself, once, when Zscaler is installed (macOS and Windows), and never turns it on again after it has been turned off.
- Where Weaver shows why omp failed, certificate errors, HTTP errors and a failed installer are explained in plain words, with the suggestion to turn on System Certificates when it is off and a corporate proxy is the likely cause.
- Usage, in the sidebar above Settings, opens a dialog with the usage limits of the providers omp is connected to and when each limit resets. Weaver asks omp for them only when Usage is opened, and never by calling a model.
- In Jobs, the sub-agents of the selected job are listed under its row, with the ones they started, and each opens its own read-only conversation, while the job works and after it ends. Each sub-agent line under a `task` row in a conversation opens it too, and a sub-agent that finished no longer shows as working.
- In Chat, each sub-agent the agent starts opens its own read-only conversation in place of the chat, with a `Back to Chat` button that returns to the chat where it was.
- Files, in the sidebar after Jobs, shows the project's files as a tree (what git ignores is left out) and opens them in tabs with an editor that colours the syntax. You can create, rename, move (by dragging or with `Move to…`) and delete files and folders from the tree; a deleted one goes to the system's trash. While the agent is changing a file, the tab and the tree show a lock and the file cannot be renamed, moved or deleted. `.weaver/` shows but stays read-only here, and closing the view with unsaved changes asks to save or discard them.
- Hide, in the sidebar before Usage and Settings, sends the window of a project out of sight, off the screen and, on Windows and Linux, off the taskbar, without suspending anything: jobs, Chat, Ask more conversations and question rounds keep working, and the window shows the same view as it was when it comes back. It comes back from the Dock (macOS), by launching Weaver again, or from the tray icon on Windows and Linux (`Show Weaver`; a left click on Windows), which also has `Quit`. On macOS a window in full screen leaves full screen before going out of sight, so no black screen is left behind, and goes back to full screen when it comes back.
- Closing a window with jobs working now offers `Keep working in background` next to `Suspend and Close` and `Cancel`: the jobs keep working and the window goes to the background instead of closing.

### Changed

- The omp installer now reports its own failure instead of leaving omp silently missing.
- Settings is now at the bottom of the sidebar, apart from the other entries, which stay at the top.
- The skills and sub-agents Weaver installs are now named `weaver-…` (the Requirement skills `weaver-req-…`). In projects already initialised, the old `trama-*` skills and sub-agents of Weaver are no longer used or touched; they can be deleted by hand.
- The sidebar, the toolbars at the top of each view and the content under them have more room from the top of the window, so nothing sits pressed against it.

### Fixed

- Chat, job, sub-agent and Ask more conversations keep scrolling to the latest text while the agent writes, and stop only when you scroll up, instead of stopping by themselves during fast answers.
