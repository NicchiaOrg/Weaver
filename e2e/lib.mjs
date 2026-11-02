// Helpers of the end-to-end suite: where the release build is, a hermetic environment for the
// app, a copy of the fixture project, and process lookups. Plain Node, no dependencies.

import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const e2eRoot = path.dirname(fileURLToPath(import.meta.url));
export const repositoryRoot = path.resolve(e2eRoot, "..");
export const isWindows = process.platform === "win32";

/** Everything `tauri-driver` and the app (which inherits its streams) wrote to stdout and stderr. */
export const driverOutput = [];

/** Version of the app, from `src-tauri/tauri.conf.json`. */
export const appVersion = JSON.parse(readFileSync(path.join(repositoryRoot, "src-tauri", "tauri.conf.json"), "utf8")).version;

/** Name of the app executable (also the process name looked for after the app quits). */
export const appProcessName = isWindows ? "weaver.exe" : "weaver";

/**
 * The release build of the app: `target/release/weaver[.exe]` (built by `pnpm tauri build`),
 * never a dev server. `E2E_APP` overrides the path (not a `TRAMA_*`/`WEAVER_*` variable on purpose).
 */
export function applicationPath() {
  if (process.platform === "darwin") {
    throw new Error("The end-to-end tests do not run on macOS: it has no WebDriver for its webview.");
  }
  const application = process.env.E2E_APP ?? path.join(repositoryRoot, "target", "release", appProcessName);
  if (!existsSync(application)) {
    throw new Error(`The release build of the app is missing: ${application}. Build it first with \`pnpm tauri build --no-bundle\` from the repository root.`);
  }
  return path.resolve(application);
}

/** Path of `tauri-driver` (`cargo install tauri-driver --locked`). */
export function tauriDriverPath() {
  const fromEnv = process.env.E2E_TAURI_DRIVER;
  if (fromEnv) return fromEnv;
  const home = process.env.CARGO_HOME ?? path.join(process.env.HOME ?? process.env.USERPROFILE ?? "", ".cargo");
  const installed = path.join(home, "bin", isWindows ? "tauri-driver.exe" : "tauri-driver");
  return existsSync(installed) ? installed : "tauri-driver";
}

/**
 * The environment of the app: the inherited one without `TRAMA_*` and `WEAVER_*`, and on Linux
 * the XDG folders (settings, recent projects, runtime file) in a temporary sandbox, so a run never
 * touches the developer's own files. Windows has no such override: its folders are the account's
 * Known Folders, which is why this suite is meant for CI runners.
 */
export function hermeticEnvironment() {
  const env = {};
  for (const [name, value] of Object.entries(process.env)) {
    if (!/^(TRAMA_|WEAVER_)/i.test(name)) env[name] = value;
  }
  if (!isWindows) {
    // One sandbox for the whole run: the launcher creates it when the config loads and the workers
    // it starts inherit the variable, so the driver and the spec see the same folders.
    process.env.E2E_SANDBOX ??= realpathSync(mkdtempSync(path.join(tmpdir(), "weaver-e2e-home-")));
    const sandbox = process.env.E2E_SANDBOX;
    for (const [name, folder] of Object.entries({ XDG_CONFIG_HOME: "config", XDG_DATA_HOME: "data", XDG_STATE_HOME: "state", XDG_CACHE_HOME: "cache" })) {
      env[name] = path.join(sandbox, folder);
      mkdirSync(env[name], { recursive: true });
    }
    delete env.XDG_RUNTIME_DIR;
  }
  return env;
}

/** A copy of the fixture project in a new temporary folder (the app writes into `.trama/`). */
export function copyFixtureProject(name = "weaver-fixture") {
  const parent = realpathSync(mkdtempSync(path.join(tmpdir(), "weaver-e2e-project-")));
  const project = path.join(parent, name);
  cpSync(path.join(e2eRoot, "fixtures", "project"), project, { recursive: true });
  return project;
}

/** Whether a process with this image name is running. */
export function isProcessRunning(imageName) {
  if (isWindows) {
    const result = spawnSync("tasklist", ["/FI", `IMAGENAME eq ${imageName}`, "/NH", "/FO", "CSV"], { encoding: "utf8", windowsHide: true });
    return result.stdout.toLowerCase().includes(`"${imageName.toLowerCase()}"`);
  }
  return spawnSync("pgrep", ["-x", imageName]).status === 0;
}
