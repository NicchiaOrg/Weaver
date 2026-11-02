// Tests of scripts/check-versions.mjs on temporary repositories: `node --test scripts/`.

import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { after, test } from "node:test";
import { fileURLToPath } from "node:url";

import { collectVersions, manifestVersion, mismatches } from "./check-versions.mjs";

const script = path.join(path.dirname(fileURLToPath(import.meta.url)), "check-versions.mjs");
const temporary = [];
after(() => temporary.forEach((dir) => rmSync(dir, { recursive: true, force: true })));

/** A repository where every file carries `version`, with `overrides` replacing single files. */
function repository(version, overrides = {}) {
  const root = mkdtempSync(path.join(tmpdir(), "weaver-versions-"));
  temporary.push(root);
  const files = {
    "Cargo.toml": `[workspace]\nmembers = []\n\n[workspace.package]\nversion = "${version}"\nedition = "2021"\n`,
    "src-tauri/tauri.conf.json": JSON.stringify({ productName: "Weaver", version }),
    "src-tauri/Cargo.toml": `[package]\nname = "weaver"\nversion.workspace = true\n`,
    "crates/core/Cargo.toml": `[package]\nname = "trama-core"\nversion.workspace = true\n`,
    "crates/cli/Cargo.toml": `[package]\nname = "trama-cli"\nversion.workspace = true\n`,
    "ui/package.json": JSON.stringify({ name: "weaver-ui", version }),
    ...overrides,
  };
  for (const [file, text] of Object.entries(files)) {
    mkdirSync(path.dirname(path.join(root, file)), { recursive: true });
    writeFileSync(path.join(root, file), text);
  }
  return root;
}

const run = (root) => spawnSync(process.execPath, [script, root], { encoding: "utf8" });

test("aligned versions pass", () => {
  const result = run(repository("1.2.3"));
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /aligned at 1\.2\.3/);
});

test("a crate with its own version is compared too", () => {
  const root = repository("1.2.3", { "crates/cli/Cargo.toml": `[package]\nname = "trama-cli"\nversion = "1.2.3" # same\n` });
  assert.deepEqual(mismatches(collectVersions(root)), []);
});

test("a version that differs in any of the four followers fails and names the file", () => {
  const cases = {
    "src-tauri/Cargo.toml": `[package]\nname = "weaver"\nversion = "1.2.4"\n`,
    "crates/core/Cargo.toml": `[package]\nname = "trama-core"\nversion = "0.9.0"\n`,
    "crates/cli/Cargo.toml": `[package]\nname = "trama-cli"\nversion = "1.2.3-rc.1"\n`,
    "ui/package.json": JSON.stringify({ name: "weaver-ui", version: "2.0.0" }),
  };
  for (const [file, text] of Object.entries(cases)) {
    const result = run(repository("1.2.3", { [file]: text }));
    assert.equal(result.status, 1, `${file} should fail`);
    assert.ok(result.stderr.includes(file), result.stderr);
  }
});

test("the version of tauri.conf.json differing from the workspace version fails", () => {
  const root = repository("1.2.3", { "src-tauri/tauri.conf.json": JSON.stringify({ version: "1.3.0" }) });
  const problems = mismatches(collectVersions(root));
  assert.equal(problems.length, 4, problems.join("\n"));
  assert.equal(run(root).status, 1);
});

test("a missing version or file fails instead of passing", () => {
  assert.equal(run(repository("1.2.3", { "ui/package.json": JSON.stringify({ name: "weaver-ui" }) })).status, 1);
  assert.equal(run(repository("1.2.3", { "src-tauri/tauri.conf.json": "{}" })).status, 1);
  const root = repository("1.2.3");
  rmSync(path.join(root, "crates/core/Cargo.toml"));
  assert.equal(run(root).status, 1);
});

test("a UTF-8 BOM in a JSON file is accepted", () => {
  const root = repository("1.2.3", { "ui/package.json": `\uFEFF${JSON.stringify({ version: "1.2.3" })}` });
  assert.deepEqual(mismatches(collectVersions(root)), []);
});

test("manifestVersion reads only the requested table", () => {
  const text = `[package]\nname = "x"\nversion = "1.0.0"\n\n[dependencies]\nfoo = { version = "9" }\n\n[dependencies.bar]\nversion = "8"\n`;
  assert.deepEqual(manifestVersion(text), { version: "1.0.0" });
  assert.deepEqual(manifestVersion(`[package]\r\nversion.workspace = true\r\n`), { workspace: true });
  assert.deepEqual(manifestVersion(`[workspace.package]\nversion = "3.0.0"\n`), {});
  assert.deepEqual(manifestVersion(`[workspace.package]\nversion = "3.0.0"\n`, "workspace.package"), { version: "3.0.0" });
});
