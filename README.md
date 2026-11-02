# Weaver

Weaver is a desktop app for macOS, Windows and Linux that works on top of Weaver projects (the `.weaver/` folder of a repository). It shows a project's Model, Requirements and Tasks on a Board, and it drives [omp](https://omp.sh) over the Agent Client Protocol to design, scan and implement them. The same Rust core powers the app and the `weaver` command line.

## Install

Download the build for your system from the [Releases](https://github.com/nicchia-code/weaver/releases) page and open it. Weaver keeps itself up to date from the same releases.

> **Builds are not signed or notarised for now.** Code signing and notarisation on macOS and signing on Windows are deliberately left out and will come with a dedicated Proposal, so macOS and Windows warn that the app comes from an unidentified developer. The warning does not mean the file is damaged. Open an unsigned build like this:
>
> * **macOS** (`Weaver_<version>_universal.dmg`): drag Weaver to Applications, right-click it and choose **Open**, then **Open** again. If macOS still refuses, open System Settings → Privacy & Security and choose **Open Anyway** next to Weaver.
> * **Windows** (`Weaver_<version>_x64-setup.exe`): when SmartScreen shows "Windows protected your PC", choose **More info** and then **Run anyway**.
> * **Linux** (`Weaver_<version>_amd64.AppImage`, or `Weaver_<version>_amd64.deb`): make the AppImage executable with `chmod +x Weaver_<version>_amd64.AppImage` and run it; install the deb with `sudo apt install ./Weaver_<version>_amd64.deb`.

## Branches

* `dev`: where the work lands.
* `main`: what is released. It changes only from `dev`, through a pull request whose `ci` checks passed.

## Repository layout

| Path | What it is |
|---|---|
| `crates/core` | Library `weaver-core`: app folders, CLI↔app protocol, project lock, process spawning. No Tauri, no async runtime, synchronous `std` I/O only. |
| `crates/cli` | Binary `weaver-cli` (clap): one command registry (`registry`, `commands/`) drives parsing, validation, dispatch, help, JSON Schemas and the command reference of the skills. Library `weaver_cli` holds the code so tests and the app can reach it. |
| `src-tauri` | App crate `weaver` on Tauri 2 and Tokio: commands, events, socket server, mutation queues, sidecar install. |
| `ui` | Vite + React 19 + TypeScript (`strict`), pnpm, Tailwind CSS 4, shadcn/ui (zinc, blue primary). Holds no domain state. |
| `.weaver` | The Model, Requirements, Specs and Tasks of Weaver itself. |

### `weaver-core` modules

| Module | Responsibility |
|---|---|
| `paths` | Identifier `dev.nicchia.weaver`; config, local data and runtime folders; project hash (first 16 hex characters of the SHA-256 of the canonical root) |
| `atomic` | Atomic file writes (temp file + rename) with an optional Unix mode |
| `endpoint` | CSPRNG helpers; socket path (`weaver-<8>.sock`) or named pipe (`\\.\pipe\weaver-<16>`) naming |
| `runtime` | `<app local data>/runtime.json` with `{endpoint, secret, pid, version}`, mode `0600` |
| `protocol` | Newline-delimited JSON messages, at most 1 MiB, protocol version, constant-time secret check |
| `ipc` | Local-socket transport (`interprocess`), one request per connection, client and server sides |
| `lock` | OS advisory lock `<app local data>/locks/<project hash>.lock`; not obtained in 10 s is "project busy" |
| `process` | The single function every child process starts with: no shell, `CREATE_NO_WINDOW` on Windows |
| `agent_files` | The texts installed for the agent, embedded from `crates/core/assets/` with `include_dir`; edit the sources there, never the installed copies; `COMMANDS.md` is generated from the CLI command registry |
| `job_instruction` | The pure function that composes the one-line instruction of every job |
| `command` | The request kind `command`: a `weaver` command the app runs in the project's queue, its reply and the error codes |
| `project::{browse, init, resolve}` | Requirement sections that apply to an element and Model search; what `weaver project new` writes; naming a card by identity, prefix or short number |
| `project::requests` | The Machine `design-request` (pure) and the files of design requests: `.weaver/requests/<id>.md`, `<id>.images/`, `.state.json`, title and emptiness rules, images |
| `omp::*` | omp: the Machine `omp-setup` (pure, events and clock injected), the lookup of the executable, the login PATH, `omp --version` / `models --json` / `update` probes with their limits, failure reasons, the fixed provider list, the quoting of the system terminal commands and `MIN_OMP_VERSION` |

### `weaver` (app) modules

| Module | Responsibility |
|---|---|
| `commands` | Tauri commands the UI calls; each returns a result or a typed `AppError` |
| `events` | Backend → UI events such as `project-changed` (project id, revision, changed slices) |
| `bindings` | `ts-rs` generation of `ui/src/bindings/` and the check that fails when it is stale |
| `mutations` | One serial queue per project; each mutation holds the OS project lock |
| `server` | CLI↔app socket server; publishes `runtime.json` |
| `cli_handler` | The app side of `weaver` commands that write: runs them in the project's queue with the same function the CLI runs when no app answers |
| `sidecar` | Copies the bundled `weaver-cli` into `<app local data>/bin/` when its content differs |
| `blocking` | `spawn_blocking` helper: disk work never runs on the main or IPC thread |
| `state` | App-wide state managed by Tauri |
| `updates` | Updates of Weaver: the check at start and every 24 hours, **Install and restart** through the quit path, **Later**, the last check in `settings.json` |
| `omp` | The app side of `omp-setup`: runs the probes, the installer, the login and the log out off the UI thread, schedules the re-checks, emits `omp-changed`, and tells the jobs when omp is ready (`OmpSetup::wait_ready`) |
| `windows` | The Machine `window`: one window per project (`p-<hash>`) and the `welcome` window, `state.json` (recent projects, last session), restore at start, close and quit confirmations, the file watcher of each project, the menu, `Open in editor`; the jobs plug in through `WindowJobs` |
| `requests` | The design request: the editor's commands (save, Send, Discard, images), the setup or design job `Send` starts, consumption at the Proposal hand-over, return to writing when the job ends, and the flush before a window closes |

The bundled `weaver-cli` is a Tauri `externalBin` sidecar. `src-tauri/build.rs` builds it with the app's profile and target into `target/sidecar` and stores it as `src-tauri/binaries/weaver-cli-<target triple>`.

## Development

Requirements: the toolchains pinned in the repository (Rust in `rust-toolchain.toml`, with `rustfmt` and `clippy`; Node in `.nvmrc`; pnpm in `packageManager` of the root `package.json`). On Linux also `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev` and `patchelf`.

```sh
pnpm install                  # once (workspace root: ui + Tauri CLI)
pnpm --dir ui build            # type-check (tsc --noEmit) and build the UI
cargo build --workspace        # core, CLI and app
cargo test --workspace         # core, CLI and app tests
pnpm tauri dev                # run the app with the Vite dev server
```

### Generated TypeScript types

The types of commands, events and payloads in `ui/src/bindings/` are generated from the Rust types with `ts-rs` and committed. `cargo test -p weaver` fails when they are stale. After changing a Rust type that crosses the UI boundary, list it in `bindings::export_all` and run:

```sh
pnpm --dir ui gen:types        # cargo test -p weaver --lib regenerate_bindings -- --ignored
```

### The command reference of the skills

`crates/core/assets/skills/trama-design/COMMANDS.md` is generated from the command registry of `crates/cli` (`weaver_cli::registry::command_reference()`); never edit it by hand. `cargo test -p weaver-cli` fails when it differs from the registry. After changing a command, regenerate it and build again (the file is embedded in `weaver-core`):

```sh
cargo run -p weaver-cli --example write_command_reference
```

### Where Weaver keeps files

Only in the project's `.weaver/` folder and in the system folders of the identifier `dev.nicchia.weaver` (`<config>/dev.nicchia.weaver`, `<local data>/dev.nicchia.weaver`). No database; secrets would go to the system keychain (none are kept yet).

## Releases and updates

The workflow `.github/workflows/release.yml` (`release`) runs when `ci` completed with success on a push to `main`, and works on that run's commit; it never runs for `dev` or a pull request.

1. `prepare` reads `version` from `src-tauri/tauri.conf.json` and looks for the tag `v<version>` on `origin`. When the tag exists the workflow ends with a `::notice::` and builds nothing: the push only ran the tests. Otherwise it creates the draft release `Weaver v<version>` for that commit (a draft does not create the tag), with notes that name the file to download on each platform and how to open an unsigned build.
2. `build` runs `tauri-apps/tauri-action` on `macos-latest` (`universal-apple-darwin`: `.dmg`, `.app.tar.gz` + `.sig`), `windows-latest` (`x86_64-pc-windows-msvc`: NSIS `-setup.exe` + `.sig`, no MSI) and `ubuntu-22.04` (`x86_64-unknown-linux-gnu`: `.AppImage` + `.sig`, `.deb`) and uploads to the draft. `weaver-cli` is built for the same target as the app and bundled as its sidecar; on macOS the arm64 and x64 builds are joined with `lipo`.
3. `manifest` checks that the draft holds every file the notes name and attaches `latest.json` (platforms `darwin-aarch64`, `darwin-x86_64`, `windows-x86_64`, `linux-x86_64`), made by `scripts/release.mjs` from the `.sig` files.
4. `publish` publishes the draft, which creates the tag `v<version>` on that commit, only when every build job succeeded. When one failed or was cancelled, `discard` deletes the draft and no tag is created. The workflow never moves, deletes or recreates a tag and never edits a published release.

To release, raise the version in the five files `scripts/check-versions.mjs` checks and merge `dev` into `main`.

### Updater signing key

The updater files are signed with the Tauri updater key, which is needed even without code signing. Generate it once with `pnpm tauri signer generate -w <file>`, then:

* put the public key in `plugins.updater.pubkey` of `src-tauri/tauri.conf.json`;
* store the private key and its password as the GitHub Actions secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`;
* keep the private key somewhere outside GitHub too: losing it stops the updates of every installed copy.

`prepare` fails before building anything when a secret or the public key is missing.

### Updates inside the app

Weaver uses the Tauri updater plugin with the endpoint `https://github.com/nicchia-code/weaver/releases/latest/download/latest.json`. It checks at start and then at most once every 24 hours while it runs; the instant of the last check is stored in `settings.json` (`lastUpdateCheck`). A failed check is logged and retried at the next due time, never shown. An update found shows its version and notes; **Install and restart** downloads it, verifies its signature and quits Weaver through the usual quit path (with its confirmation when jobs are working), applying the update when the app exits; **Later** hides it until the next check. An update whose signature does not verify is refused and logged.

## Tests and CI

The workflow `.github/workflows/ci.yml` (`ci`) runs on every push to `dev` or `main` and every pull request to `dev` or `main`, on `macos-latest` (arm64), `windows-latest` (x64) and `ubuntu-22.04` (x64), with `fail-fast: false`. A change to `main` needs `ci` to pass. Per platform it installs from the lockfiles (`pnpm install --frozen-lockfile`, `cargo --locked`), builds the frontend, checks that the `ts-rs` types are up to date, runs `cargo test --workspace --locked` and Vitest, builds the app in release mode (with `weaver-cli` as its sidecar), smoke-tests the built `weaver-cli`, runs the CLI contract tests and, on Linux (`xvfb-run`) and Windows (`msedgedriver`), the end-to-end smoke test. A separate job fails when the version in `src-tauri/tauri.conf.json` differs from `src-tauri/Cargo.toml`, `crates/core/Cargo.toml`, `crates/cli/Cargo.toml` or `ui/package.json`.

| What | Where | Run it |
|---|---|---|
| Core, CLI, app | `cargo test` in the crates | `cargo test --workspace --locked` |
| Interface (Vitest, Testing Library) | `ui/src/**/*.test.tsx` | `pnpm --dir ui test` |
| Generated `ts-rs` types are current | `src-tauri/src/bindings.rs` | `cargo test -p weaver --lib generated_types_are_up_to_date` |
| Version alignment | `scripts/check-versions.mjs` (tests: `scripts/check-versions.test.mjs`) | `node scripts/check-versions.mjs` and `node --test scripts/` |
| Smoke of `weaver-cli` | `scripts/smoke-weaver-cli.mjs` | `node scripts/smoke-weaver-cli.mjs target/release/weaver-cli` |
| CLI contract tests (the built `weaver-cli`, every command of the registry) | `tests/cli-contract/` | `node --test tests/cli-contract/` (binary from `CLI_CONTRACT_BIN`, else `target/release` or `target/debug`) |
| End-to-end smoke (tauri-driver, WebdriverIO; release build) | `e2e/` | Linux and Windows only, in CI: `pnpm tauri build --no-bundle`, install `tauri-driver`, then `pnpm --dir e2e test` |

Tests never touch the developer's home, settings or keychain, never use the network or a real omp, and pass the same with or without `WEAVER_*` and `WEAVER_*` variables in the environment (the contract and smoke runs remove them from the child's environment). The end-to-end suite has no WebDriver on macOS, so it runs on GitHub Actions only.
