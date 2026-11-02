#!/usr/bin/env node
// Smoke of the `weaver-cli` binary just built (requirements/release#General, workflow `ci`): with a fresh
// temporary folder as `--root`, `weaver-cli --help`, `weaver-cli project new --input '{}'` and
// `weaver-cli model check` must each exit 0. Any other exit code fails the step.
//
// Usage: node scripts/smoke-weaver-cli.mjs <path to weaver-cli[.exe]>

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const requested = process.argv[2];
if (!requested || !existsSync(requested)) {
  console.error(`::error::usage: node scripts/smoke-weaver-cli.mjs <path to the built weaver-cli>; not found: ${requested}`);
  process.exit(2);
}
const binary = path.resolve(requested); // The steps run in another folder.

const sandbox = mkdtempSync(path.join(tmpdir(), "weaver-cli-smoke-"));
const root = path.join(sandbox, "project");
mkdirSync(root);

// Not the developer's or the runner's own settings: no WEAVER_* variable, homes in the sandbox.
const env = {};
for (const [name, value] of Object.entries(process.env)) {
  if (!/^WEAVER_/i.test(name)) env[name] = value;
}
Object.assign(env, {
  HOME: sandbox,
  XDG_CONFIG_HOME: path.join(sandbox, "config"),
  XDG_DATA_HOME: path.join(sandbox, "data"),
  XDG_STATE_HOME: path.join(sandbox, "state"),
  XDG_CACHE_HOME: path.join(sandbox, "cache"),
});
delete env.XDG_RUNTIME_DIR;

const steps = [
  ["--help"],
  ["project", "new", "--input", "{}", "--root", root],
  ["model", "check", "--root", root],
];

let failed = false;
for (const args of steps) {
  const label = `weaver-cli ${args.join(" ")}`;
  const result = spawnSync(binary, args, { env, cwd: root, encoding: "utf8", timeout: 60_000, windowsHide: true, input: "" });
  const status = result.error ? `error: ${result.error.message}` : result.status;
  console.log(`$ ${label}\n  exit ${status}`);
  if (result.stdout) console.log(result.stdout.trimEnd().replace(/^/gm, "  | "));
  if (result.stderr) console.log(result.stderr.trimEnd().replace(/^/gm, "  ! "));
  if (status !== 0) {
    console.error(`::error::${label} exited with ${status}, expected 0`);
    failed = true;
    break;
  }
}

rmSync(sandbox, { recursive: true, force: true });
process.exit(failed ? 1 : 0);
