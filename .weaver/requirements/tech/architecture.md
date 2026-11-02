---
description: "Use when building the app structure, backend state, file watching, IPC with the CLI, windows or system notifications."
---
# Architecture

## General
- **Cargo workspace** at the repository root:
  - `crates/core`: library `weaver-core`, with parsing, validation, rules, mutations, skill generation, embedded installed files and the CLI↔app protocol types. It has no Tauri dependency, no async runtime and does sync `std` I/O only.
  - `crates/cli`: crate `weaver-cli`, building the binary `weaver-cli`, with arguments, output and the socket client. It never builds a binary named `weaver`, the name of the app's executable.
  - `src-tauri`: app crate `weaver`, with commands, events, watchers, the ACP client, the socket server, processes and notifications. It runs on Tokio. Disk work runs in `spawn_blocking`, never on the main or IPC thread.
  - `ui/`: Vite, TypeScript `strict`, pnpm.
- **Folders:** the Tauri identifier is `dev.nicchia.weaver`. `weaver-core` derives the app config, local data and runtime folders from it, so the CLI finds them with the app closed.
- **Bundled CLI:** `weaver-cli` is a Tauri `externalBin` sidecar, bundled under that name next to the app's executable `weaver`. At each start Weaver copies it into `<app local data>/bin/` as `weaver` (`weaver.exe` on Windows) when its content differs, and removes from that folder a `trama` (`trama.exe`) left there by earlier versions. That folder holds only `weaver` and goes first on the PATH of every omp Weaver starts. The copy keeps a stable path even under AppImage.
- **Tauri plugins:** `single-instance`, `window-state`, `notification`, `dialog`, `opener`, `updater`.
  - No `deep-link`, `shell`, `fs` or `http` plugin.
  - Plugins are called from Rust.
  - The webview capability grants only Weaver's commands and core event/window permissions.
  - The CSP allows no remote origin.
- **Backend ↔ UI:**
  - Actions are Tauri commands that return a result or a typed error. Updates are backend events.
  - The UI holds no domain state.
  - TypeScript types for commands, events and payloads are generated from the Rust types with `ts-rs`. CI fails on a stale file.
- **Project picture:**
  - Each open project has one in-memory picture built from its files.
  - It is rebuilt 150 ms after the last watcher event of a batch, and at most 1 s after the first event under continuous changes.
  - Parsing is incremental: each file is kept parsed with its content hash, and only changed or removed paths are parsed again. Derived data (blocking, design membership, short numbers, Board) is recomputed whole.
  - Each rebuild increments a revision. The event `project-changed` carries the project id, the revision and the changed slices.
  - The UI fetches a full snapshot when a window loads, or when it sees a gap in revisions.
- **Mutations:**
  - Every mutation of a project, from the UI or forwarded by the CLI, runs through one serial queue for that project.
  - Each mutation holds an OS advisory lock (`flock` / `LockFileEx`) on `<app local data>/locks/<project hash>.lock`, the same lock a CLI writing in-process takes.
  - `<project hash>` is the first 16 hex characters of the SHA-256 of the canonical project root.
  - Under the lock the mutation re-reads its files from disk, validates the result after the change, then writes. A refusal writes nothing.
  - If the lock is not obtained within 10 s, the mutation fails with "project busy".
- **CLI ↔ app channel:** via the `interprocess` crate. No TCP or HTTP port.
  - Endpoint:
    - Unix: a domain socket `weaver-<8 random>.sock` in a `0700` folder: `$XDG_RUNTIME_DIR/weaver/` when set, else `<app local data>/run/`, or `/tmp/weaver-<uid>/` when the path would exceed 100 bytes.
    - Windows: a named pipe `\\.\pipe\weaver-<16 random>`.
  - Runtime file: at each launch the app atomically writes `<app local data>/runtime.json` (mode `0600`) with `{endpoint, secret, pid, version}`.
  - Secret: 32 bytes from the OS CSPRNG, hex-encoded, new per launch. It is never in env vars, arguments, logs or `.weaver/`. It is compared in constant time; a wrong or missing secret closes the connection with no response.
  - Messages: newline-delimited JSON, at most 1 MiB, one request per connection, each with a protocol version. On a mismatch the CLI acts as if no app answered.
- **Processes:** every child is spawned without a shell, with a program path and an argument array. No user text is ever interpolated into a command line. On Windows children get `CREATE_NO_WINDOW`.

## project
- **Watcher:** one `notify` recommended watcher per open project. Every debounce, rebuild and skill regeneration is keyed by project hash.
- **Watched paths:**
  - the project root, non-recursively, only to see `.weaver` appear or vanish; a `.trama` folder appearing, changing or vanishing is ignored;
  - `.weaver/`, recursively, while it exists;
  - no code, no skills folder, no `.omp/`.
- **Ignored events:** temp files of atomic writes, and the local folders of `.weaver/` except `requests/`, where a deletion is `workspace.REQUEST_DELETED`.
- **`.weaver` vanishes:** the recursive watch is dropped, then `PROJECT_FOLDER_REMOVED`.
- **`.weaver` appears:** the watch is added and the picture fully rebuilt, then `PROJECT_FOLDER_APPEARED`.
- A rescan or overflow event triggers a full rebuild.
- On Linux, if inotify returns `ENOSPC`, the project falls back to the poll watcher every 2 s, with a one-time warning naming `fs.inotify.max_user_watches`.
- **Self-writes:** the app records the hash of each file it writes. An event whose content matches the recorded hash updates the picture but is not `workspace.FILES_CHANGED`. Writes by the CLI and the agent are `FILES_CHANGED`.
- **Requirement skills:** when a batch touches `.weaver/requirements/`, they are regenerated in the same rebuild through the project queue, by one core function.
- **Embedded files:** the installed files are embedded in `weaver-core` with `include_dir`, so the CLI and the app write identical bytes:
  - skills `trama-setup`, `trama-design` (+ format reference), `trama-import`, `trama-scan`, `trama-compare`, `trama-implement`, `trama-resolve-drifts`;
  - `.omp/agents/trama-analysis.md` and `.omp/agents/trama-implementer.md`;
  - `.omp/WATCHDOG.yml`.
- **Comparing installed files:**
  - Pure content: read the file, normalise CRLF to LF, compare byte for byte with the embedded or generated content.
  - No version stamp, no timestamp, no network.
  - Only differing or missing files are written, atomically.
  - At most one check per project at a time; requests made meanwhile coalesce into one more run.
  - The check never delays the window.

## question-round
- **Transport:**
  - `weaver question new` validates the request with `weaver-core`, then connects to `WEAVER_ENDPOINT` and sends `{secret, job, request}`.
  - It blocks reading the reply. Neither side has a timeout.
- **Routing:**
  - Only by the job identity in `WEAVER_JOB`, never by cwd, focus or path. Rounds from sub-agents and git worktrees reach the job's project.
  - The app accepts a round only for a job it launched that is Running or Waiting for the developer.
  - Each project has one round queue, created when it opens and never recreated while it is open.
- **Outcomes:** exactly one JSON document on stdout:
  - `answered`: exit 0.
  - `unavailable` with a reason: exit 0, returned at once when:
    - `WEAVER_JOB` or `WEAVER_ENDPOINT` is missing;
    - the runtime file is missing or names another endpoint;
    - the connection fails;
    - the protocol version differs;
    - the job is unknown or ended.
  - `interrupted`: exit 1, with a code and message and no answers. Returned when the round is interrupted, or when the connection drops after submission, detected by end-of-stream at once.
  - `invalid-input`: exit 1, naming the question and field.

  These outcomes are never conflated.
- **Abandoned:** the CLI closes the connection on SIGINT, SIGTERM or a closed stdout (Windows: Ctrl+C or pipe closure). End-of-stream on a queued or presented round is `agent.STOP_WAITING`.
- Round state, drafts included, lives only in backend memory. Free-text drafts reach it debounced at 300 ms. Nothing of a round is written to disk or logged.

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
  - `WindowEvent::CloseRequested` is prevented while jobs of that project work, and `RunEvent::ExitRequested` while jobs of any project work.
  - The confirmation is a native `tauri-plugin-dialog` dialog that says how many jobs work, so it works even with a hung webview.
  - After confirming, the jobs are suspended (omp ended as in `requirements/tech#job`), then the window closes or the app exits.
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
- **Withdrawing:**
  - Linux: `remove_active`.
  - macOS and Windows cannot remove a shown notification. Weaver drops its mapping instead, so a later click only brings the window forward.
  - All mappings are dropped when the window closes.
- **Windows:** a click is reported only while the toast is on screen. A click from the Action Center launches Weaver, and single-instance focuses the latest window.
