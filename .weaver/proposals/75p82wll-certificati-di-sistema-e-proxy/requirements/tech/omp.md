---
description: "Use when finding, checking, installing, updating or logging in omp, or opening the system terminal for its installer and logins."
---
# omp

## omp
- **Lookup:** the first executable found wins (`omp.exe` on Windows):
  1. the `ompPath` setting, trimmed. When set, it is used alone, with no fallback;
  2. PATH;
  3. the installer folders:
     - macOS and Linux: `$PI_INSTALL_DIR`, `~/.local/bin`, then `$BUN_INSTALL/bin` or `~/.bun/bin`;
     - Windows: `%PI_INSTALL_DIR%`, `%LOCALAPPDATA%\omp`, `%USERPROFILE%\.bun\bin`.
- **PATH of a GUI app:**
  - macOS and Linux: at start Weaver reads the login PATH once by running `$SHELL -l -c 'printf %s "$PATH"'`, with a 5 s limit. This is the only shell Weaver starts on its own. On failure it keeps the inherited PATH.
  - Windows: every lookup reads the current user and machine `Path` from the registry, so a fresh install is seen without restarting Weaver.
- **Child PATH:** every omp process gets `<app local data>/bin`, omp's folder and bun's bin folder, then the PATH above.
- **Certificate variables:** every omp process, of the table below, of jobs and of General, also gets the System Certificates variables of `requirements/tech#system-certificates`, read at each start.
- **Minimum version:** `MIN_OMP_VERSION = 18.15.0`, one tested constant in `weaver-core`. It is compared as semver against `omp/<version>`. An unreadable version counts as outdated. Weaver has no fallbacks for older omp.
- **Commands:** no shell, fixed arguments. Stdout is parsed and stderr kept for reasons. On timeout the process is killed and the run fails.

| Command | Limit | Use |
|---|---|---|
| `omp --version` | 15 s | version |
| `omp models --json` | 15 s | `selector`, `provider`; no models means `NO_PROVIDER` |
| `omp update --check` | 30 s | `UPDATE_AVAILABLE` |
| `omp update` | 5 min | `UPDATE_ENDED` / `UPDATE_FAILED` |
| `omp auth-broker logout <provider>` | 30 s | log out |

- **Readiness:** one app-wide state, with each result tagged by the omp path it was computed for.
- **Failure reasons:** the last non-empty stderr line, else `exit code N`, `could not start omp` or `timed out`. The same rule applies to ACP errors (opening, turn and message errors) and job stderr tails.
  - A reason that is a certificate or HTTP error (`requirements/rules#system-certificates`) is replaced by the explanation of `requirements/ui#system-certificates`, written when the failure happens with the setting as it is then. omp's own lines stay in the details where the surface has them.

## omp-setup
- **System terminal launch:** the command runs where the developer sees it, and the terminal stays open at the end.
  - macOS: `/usr/bin/osascript` with fixed `-e` lines: Terminal runs `do script "<command>"` and comes forward. Terminal starts the developer's own shell, which does not inherit Weaver's environment: with System Certificates on, `<command>` starts with `/usr/bin/env <NAME>=<bundle path> … ` and runs the rest under `/bin/sh -c`.
  - Windows: Windows PowerShell 5.1, `powershell.exe -NoExit -NoProfile -ExecutionPolicy Bypass -Command <command>`, with `CREATE_NEW_CONSOLE`. The certificate variables go in its process environment.
  - Linux: the first of `x-terminal-emulator -e`, `gnome-terminal --`, `konsole -e`, `xfce4-terminal -x`, `xterm -e`. It runs `sh -c '<command>; printf "\nPress Enter to close"; read _'`; with System Certificates on, the script starts with `export <NAME>=<bundle path> …;`, since some terminals do not pass Weaver's environment on. If none exists, Weaver shows the command to copy, certificate variables included.
- **Quoting:** these terminal commands do pass through a shell. One tested function quotes for the target shell. Only omp's absolute path, a provider id from the fixed list, the bundle path and the installer marker path are inserted.
- **Commands:**
  - Installer: it downloads the official installer to a temporary file and runs it, never piping it into a shell, so that a failed download counts as a failure; then it writes the exit code of the first step that failed, else `0`, to `<app local data>/omp-install/<nonce>.code`, with a new nonce at each `INSTALL_OMP`, and deletes the temporary file. Weaver never writes `ompPath` afterwards.
    - macOS and Linux: `curl -fsSL https://omp.sh/install -o <temp>`, then `sh <temp>`, in `/bin/sh`.
    - Windows: `Invoke-RestMethod https://omp.sh/install.ps1 -OutFile <temp>.ps1`, then a child `powershell.exe -NoProfile -ExecutionPolicy Bypass -File <temp>.ps1`, so that the installer's `exit` ends only the child; its `$LASTEXITCODE` is the code, and a terminating error of the download is `1`.
  - Login: `omp login <provider>`.
- **Providers,** in this order: `openai-codex` (ChatGPT), `anthropic`, `github-copilot`, `google-gemini-cli`, `google-antigravity`, `cursor`, `xai-oauth`.
  - A provider is connected when any model from `omp models --json` has it as `provider`.
  - `cursor` is offered through login only. Weaver has no API key field and stores no secret.
- **Re-checks:** every 3 s while Installing or Logging in, one at a time. A check runs the lookup, `omp --version` and `omp models --json`. Re-checks stop when the state is left.
  - While Installing, a check that finds no omp then reads the marker of the current nonce: a code other than `0` is `omp.INSTALL_FAILED`; `0`, no marker or an unreadable one is `omp.MISSING`. Weaver keeps the code until the next `INSTALL_OMP` or until omp is found.
- **Updates:** the check runs at most once per hour app-wide, and its result is cached in memory. One `omp update` runs at a time app-wide.

