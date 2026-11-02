#!/usr/bin/env node
// Fails (exit 1) when the version of `src-tauri/tauri.conf.json` differs from the one of
// `src-tauri/Cargo.toml`, `crates/core/Cargo.toml`, `crates/cli/Cargo.toml` and `ui/package.json`
// (requirements/release#General). A crate that says `version.workspace = true` carries the
// `[workspace.package]` version of the root `Cargo.toml`.
//
// Usage: node scripts/check-versions.mjs [repository root]

import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/** The file that holds the version of the release, and the files that must carry the same. */
export const REFERENCE = "src-tauri/tauri.conf.json";
export const FOLLOWERS = [
  "src-tauri/Cargo.toml",
  "crates/core/Cargo.toml",
  "crates/cli/Cargo.toml",
  "ui/package.json",
];

/** Text of a TOML basic string value such as `"0.1.0"` (comments after it are ignored). */
function tomlString(raw) {
  const match = /^\s*"((?:[^"\\]|\\.)*)"\s*(?:#.*)?$/.exec(raw);
  return match ? JSON.parse(`"${match[1]}"`) : undefined;
}

/**
 * Reads the `version` of the `[package]` (or `[workspace.package]`) table of a Cargo manifest.
 * Returns `{ version }`, or `{ workspace: true }` for `version.workspace = true`, or `{}`.
 */
export function manifestVersion(text, table = "package") {
  let current = "";
  let found = {};
  for (const line of text.split(/\r?\n/)) {
    const header = /^\s*\[\s*([^\]]+?)\s*\]\s*(?:#.*)?$/.exec(line);
    if (header) {
      current = header[1];
      continue;
    }
    if (current !== table) continue;
    const key = /^\s*([A-Za-z0-9_.\-"]+?)\s*=\s*(.*)$/.exec(line);
    if (!key) continue;
    if (key[1] === "version") {
      const version = tomlString(key[2]);
      if (version !== undefined) found = { version };
    } else if (key[1] === "version.workspace" && /^\s*true\s*(?:#.*)?$/.test(key[2])) {
      found = { workspace: true };
    }
  }
  return found;
}

/** `{ file: version | undefined }` for the reference and every follower under `root`. */
export function collectVersions(root) {
  const read = (file) => readFileSync(path.join(root, file), "utf8");
  const workspaceVersion = manifestVersion(read("Cargo.toml"), "workspace.package").version;
  const versions = {};
  for (const file of [REFERENCE, ...FOLLOWERS]) {
    try {
      const text = read(file);
      if (file.endsWith(".json")) {
        const version = JSON.parse(text.replace(/^\uFEFF/, "")).version;
        versions[file] = typeof version === "string" ? version : undefined;
      } else {
        const found = manifestVersion(text);
        versions[file] = found.workspace ? workspaceVersion : found.version;
      }
    } catch (error) {
      versions[file] = undefined;
      versions[`${file}#error`] = String(error.message ?? error);
    }
  }
  return versions;
}

/** One line per file whose version is missing or differs from the reference; empty when aligned. */
export function mismatches(versions) {
  const expected = versions[REFERENCE];
  const problems = [];
  if (expected === undefined) {
    problems.push(`${REFERENCE}: no "version" found${versions[`${REFERENCE}#error`] ? ` (${versions[`${REFERENCE}#error`]})` : ""}`);
    return problems;
  }
  for (const file of FOLLOWERS) {
    const actual = versions[file];
    if (actual === undefined) {
      problems.push(`${file}: no version found${versions[`${file}#error`] ? ` (${versions[`${file}#error`]})` : ""}`);
    } else if (actual !== expected) {
      problems.push(`${file}: version ${actual} differs from ${expected} in ${REFERENCE}`);
    }
  }
  return problems;
}

function main() {
  const root = path.resolve(process.argv[2] ?? path.join(path.dirname(fileURLToPath(import.meta.url)), ".."));
  const versions = collectVersions(root);
  const problems = mismatches(versions);
  if (problems.length > 0) {
    for (const problem of problems) console.error(`::error::${problem}`);
    console.error("Versions are not aligned: set the same version in the five files.");
    process.exit(1);
  }
  console.log(`Versions aligned at ${versions[REFERENCE]} (${[REFERENCE, ...FOLLOWERS].join(", ")}).`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
