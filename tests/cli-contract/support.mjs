// Support of the CLI contract tests: they run the compiled `weaver-cli` binary, never the library.
//
// Every run is hermetic: the environment of the child loses every `WEAVER_*` and `WEAVER_*`
// variable, and the folders where Weaver keeps its files (home, config, local data) point into a
// temporary sandbox, so a run never reads or writes the developer's home or settings and the
// suite passes the same with or without those variables in the environment.

import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, realpathSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const executable = process.platform === "win32" ? "weaver-cli.exe" : "weaver-cli";

/** Where the contract tests find the binary under test. */
export function locateBinary() {
  // Not a WEAVER_* name on purpose: the suite never reads those variables from its environment.
  const chosen = process.env.CLI_CONTRACT_BIN;
  if (chosen) {
    if (!existsSync(chosen)) throw new Error(`CLI_CONTRACT_BIN names a file that does not exist: ${chosen}`);
    return path.resolve(chosen);
  }
  for (const profile of ["release", "debug"]) {
    const candidate = path.join(repositoryRoot, "target", profile, executable);
    if (existsSync(candidate)) return candidate;
  }
  throw new Error(
    `No compiled weaver-cli binary: build it (cargo build -p weaver-cli) or set CLI_CONTRACT_BIN to its path. Looked in ${path.join(repositoryRoot, "target")}.`,
  );
}

/** The `weaver-cli` under test. */
export const WEAVER_CLI = locateBinary();

const temporaryFolders = [];

/** A new empty folder under the system temporary folder, removed by [`cleanup`]. */
export function temporaryFolder(prefix = "cli-contract-") {
  const folder = realpathSync(mkdtempSync(path.join(tmpdir(), prefix)));
  temporaryFolders.push(folder);
  return folder;
}

/** Removes every folder made by [`temporaryFolder`]. */
export function cleanup() {
  for (const folder of temporaryFolders.splice(0)) rmSync(folder, { recursive: true, force: true });
}

/** `{ env, sandbox }`: the inherited environment without `WEAVER_*`/`WEAVER_*`, homes in a sandbox. */
export function hermeticEnvironment(extra = {}) {
  const sandbox = temporaryFolder("cli-contract-home-");
  const folders = {
    HOME: sandbox,
    USERPROFILE: sandbox,
    XDG_CONFIG_HOME: path.join(sandbox, "config"),
    XDG_DATA_HOME: path.join(sandbox, "data"),
    XDG_STATE_HOME: path.join(sandbox, "state"),
    XDG_CACHE_HOME: path.join(sandbox, "cache"),
    LOCALAPPDATA: path.join(sandbox, "local"),
    APPDATA: path.join(sandbox, "roaming"),
  };
  if (process.platform === "win32") {
    // Windows resolves the config and local data folders of the user through the shell, which
    // verifies that they exist: with the sandbox homes missing, Weaver finds no folder to keep its
    // project lock in.
    for (const name of ["LOCALAPPDATA", "APPDATA"]) mkdirSync(folders[name], { recursive: true });
  }
  const env = {};
  for (const [name, value] of Object.entries(process.env)) {
    if (!/^(WEAVER_|WEAVER_)/i.test(name)) env[name] = value;
  }
  delete env.XDG_RUNTIME_DIR;
  return { env: { ...env, ...folders, ...extra }, sandbox };
}

/**
 * Runs `weaver-cli` and returns `{ status, stdout, stderr, timedOut }`.
 *
 * `stdin`: a string or Buffer to send; `"open"` leaves the pipe open and silent until the process
 * exits (a command that reads it would hang and time out); `undefined` closes it at once.
 */
export function run(args, { stdin, cwd, env, timeoutMs = 30_000 } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(WEAVER_CLI, args, { cwd, env, windowsHide: true, stdio: ["pipe", "pipe", "pipe"] });
    const out = [];
    const err = [];
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      child.kill();
    }, timeoutMs);
    child.stdout.on("data", (chunk) => out.push(chunk));
    child.stderr.on("data", (chunk) => err.push(chunk));
    child.stdin.on("error", () => {}); // The child may exit before reading its input.
    child.on("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.on("close", (status) => {
      clearTimeout(timer);
      resolve({
        status: timedOut ? null : status,
        stdout: Buffer.concat(out).toString("utf8"),
        stderr: Buffer.concat(err).toString("utf8"),
        timedOut,
      });
    });
    if (stdin === "open") return;
    child.stdin.end(stdin);
  });
}

/** Names of the entries of a folder (empty when it does not exist). */
export function entries(folder) {
  return existsSync(folder) ? readdirSync(folder).sort() : [];
}

/** Creates a folder (and parents) and returns it. */
export function makeFolder(...parts) {
  const folder = path.join(...parts);
  mkdirSync(folder, { recursive: true });
  return folder;
}

/** A fresh project: `weaver project new --input '{}'` in a new folder. Returns the root. */
export async function newProject(environment) {
  const root = temporaryFolder("cli-contract-project-");
  const result = await run(["project", "new", "--input", "{}", "--root", root], { env: environment.env, cwd: root });
  if (result.status !== 0) {
    throw new Error(`project new failed (exit ${result.status}): ${result.stderr || result.stdout}`);
  }
  return root;
}

/** Runs real git in `folder` with no global or system configuration; returns its stdout. */
export function git(folder, args) {
  return execFileSync("git", args, {
    cwd: folder,
    encoding: "utf8",
    env: { ...process.env, GIT_CONFIG_GLOBAL: path.join(tmpdir(), "no-such-git-config"), GIT_CONFIG_NOSYSTEM: "1" },
  });
}

/** A new empty git repository under the system temporary folder, removed by [`cleanup`]. Returns the root. */
export function gitRepository(prefix = "cli-contract-repo-") {
  const folder = temporaryFolder(prefix);
  git(folder, ["init", "-q"]);
  return folder;
}
