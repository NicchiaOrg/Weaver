// Tests of scripts/release.mjs, and of what the release workflow and the README promise:
// `node --test scripts/`.

import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import path from "node:path";
import { after, test } from "node:test";
import { fileURLToPath } from "node:url";

import { buildLatestJson, downloads, MANIFEST_PLATFORMS, missingAssets, releaseNotes } from "./release.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const script = path.join(here, "release.mjs");
const root = path.join(here, "..");
const temporary = [];
after(() => temporary.forEach((dir) => rmSync(dir, { recursive: true, force: true })));

const V = "1.2.3";
const UPDATER = ["Weaver_universal.app.tar.gz", `Weaver_${V}_x64-setup.exe`, `Weaver_${V}_amd64.AppImage`];
const UPLOADED = [
  `Weaver_${V}_universal.dmg`,
  `Weaver_${V}_amd64.deb`,
  ...UPDATER,
  ...UPDATER.map((name) => `${name}.sig`),
];

test("the download names are the ones Tauri produces for the product Weaver", () => {
  assert.deepEqual(downloads(V), {
    macos: ["Weaver_1.2.3_universal.dmg"],
    windows: ["Weaver_1.2.3_x64-setup.exe"],
    linux: ["Weaver_1.2.3_amd64.AppImage", "Weaver_1.2.3_amd64.deb"],
  });
  assert.throws(() => downloads("1.2"), /not a version/);
  assert.throws(() => downloads("1.2.3; rm -rf /"), /not a version/);
});

test("the release notes name each file to download and the steps to open an unsigned build", () => {
  const notes = releaseNotes(V);
  for (const file of Object.values(downloads(V)).flat()) assert.match(notes, new RegExp(file.replace(/\./g, "\\.")));
  for (const step of ["**Open**", "**Open Anyway**", "**More info**", "**Run anyway**", "chmod +x Weaver_1.2.3_amd64.AppImage"]) {
    assert.ok(notes.includes(step), step);
  }
  assert.doesNotMatch(notes, /\b(I|we|my|our)\b/i);
});

test("a draft with every file passes; a missing file or signature is named", () => {
  assert.deepEqual(missingAssets(V, UPLOADED), []);
  assert.deepEqual(missingAssets(V, UPLOADED.filter((n) => n !== `Weaver_${V}_amd64.deb`)), [`Weaver_${V}_amd64.deb`]);
  assert.deepEqual(missingAssets(V, UPLOADED.filter((n) => n !== `Weaver_${V}_x64-setup.exe.sig`)), [
    `Weaver_${V}_x64-setup.exe.sig`,
  ]);
  const withoutDmg = UPLOADED.filter((n) => !n.endsWith(".dmg"));
  assert.deepEqual(missingAssets(V, withoutDmg), [`Weaver_${V}_universal.dmg`]);
  const noWindows = UPLOADED.filter((n) => !n.includes("setup.exe"));
  assert.equal(missingAssets(V, noWindows).length, 2);
});

test("a second MSI-less Windows file or a second AppImage is refused as ambiguous", () => {
  assert.match(missingAssets(V, [...UPLOADED, "Other.AppImage"]).join("\n"), /exactly one updater file of linux/);
});

test("latest.json has the four platforms, their urls and the signatures of the .sig files", () => {
  const signatures = Object.fromEntries(UPDATER.map((name) => [name, `sig-of-${name}\n`]));
  const manifest = buildLatestJson({ version: V, pubDate: "2026-11-02T07:00:00.000Z", names: UPLOADED, signatures });
  assert.equal(manifest.version, V);
  assert.equal(manifest.pub_date, "2026-11-02T07:00:00.000Z");
  assert.deepEqual(Object.keys(manifest.platforms).sort(), Object.keys(MANIFEST_PLATFORMS).sort());
  assert.deepEqual(Object.keys(manifest.platforms).sort(), ["darwin-aarch64", "darwin-x86_64", "linux-x86_64", "windows-x86_64"]);
  const base = "https://github.com/nicchia-code/weaver/releases/download/v1.2.3";
  assert.deepEqual(manifest.platforms["darwin-aarch64"], {
    signature: "sig-of-Weaver_universal.app.tar.gz",
    url: `${base}/Weaver_universal.app.tar.gz`,
  });
  assert.deepEqual(manifest.platforms["darwin-x86_64"], manifest.platforms["darwin-aarch64"]);
  assert.equal(manifest.platforms["windows-x86_64"].url, `${base}/Weaver_1.2.3_x64-setup.exe`);
  assert.equal(manifest.platforms["linux-x86_64"].url, `${base}/Weaver_1.2.3_amd64.AppImage`);
});

test("latest.json is refused without a signature", () => {
  const signatures = { [UPDATER[0]]: "a", [UPDATER[1]]: "  \n" , [UPDATER[2]]: "c" };
  assert.throws(() => buildLatestJson({ version: V, pubDate: "x", names: UPLOADED, signatures }), /signature of .*setup\.exe is missing/);
});

test("the command line verifies a draft and writes latest.json", () => {
  const dir = mkdtempSync(path.join(tmpdir(), "weaver-release-"));
  temporary.push(dir);
  const assets = path.join(dir, "assets.json");
  writeFileSync(assets, JSON.stringify(UPLOADED.map((name, id) => ({ name, id }))));
  const signatures = path.join(dir, "signatures");
  mkdirSync(signatures);
  for (const name of UPDATER) writeFileSync(path.join(signatures, `${name}.sig`), `signature of ${name}\n`);

  const run = (...args) => spawnSync(process.execPath, [script, ...args], { encoding: "utf8" });
  assert.equal(run("verify", V, assets).status, 0);
  const out = path.join(dir, "latest.json");
  const made = run("manifest", V, assets, signatures, out);
  assert.equal(made.status, 0, made.stderr);
  const manifest = JSON.parse(readFileSync(out, "utf8"));
  assert.equal(manifest.platforms["linux-x86_64"].signature, `signature of Weaver_${V}_amd64.AppImage`);

  writeFileSync(assets, JSON.stringify(UPLOADED.slice(1).map((name, id) => ({ name, id }))));
  const failed = run("verify", V, assets);
  assert.equal(failed.status, 1);
  assert.match(failed.stderr, /::error::the release lacks Weaver_1\.2\.3_universal\.dmg/);

  assert.equal(run("notes", V).stdout, releaseNotes(V));
});

// --- what the workflow and the README promise ------------------------------------------------

const workflow = readFileSync(path.join(root, ".github", "workflows", "release.yml"), "utf8");

test("the workflow starts only from a successful ci run on main", () => {
  assert.match(workflow, /workflow_run:\n {4}workflows: \[ci\]\n {4}types: \[completed\]\n {4}branches: \[main\]/);
  assert.match(workflow, /workflow_run\.conclusion == 'success'/);
  assert.match(workflow, /workflow_run\.event == 'push'/);
  assert.match(workflow, /ref: \$\{\{ github\.event\.workflow_run\.head_sha \}\}/);
  assert.match(workflow, /target_commitish="\$HEAD_SHA"/);
});

test("the workflow never moves, deletes or recreates a tag and never edits a published release", () => {
  for (const forbidden of [/git tag\b/, /git push\b/, /--force/, /cleanup_tag/, /git ref/, /git\/refs/, /-X DELETE "[^"]*git\//]) {
    assert.doesNotMatch(workflow, forbidden);
  }
  // The only calls that change a release are on the draft: delete (discard) and publish.
  assert.equal([...workflow.matchAll(/-X DELETE/g)].length, 2);
  assert.equal([...workflow.matchAll(/-X PATCH/g)].length, 1);
  assert.match(workflow, /"\$\(gh api "repos\/\$\{GITHUB_REPOSITORY\}\/releases\/\$\{RELEASE_ID\}" --jq '\.draft'\)" != "true"/);
});

test("the workflow skips with a notice when the tag exists, and publishes only after every build", () => {
  assert.match(workflow, /::notice::Release of v\$\{VERSION\} skipped: the tag v\$\{VERSION\} exists\./);
  assert.match(workflow, /needs: \[prepare, build, manifest\]\n {4}if: \$\{\{ needs\.prepare\.outputs\.build == 'true' \}\}/);
  assert.match(workflow, /always\(\) && needs\.prepare\.result == 'success'.*needs\.build\.result != 'success'/);
});

test("the build matrix is the one of the release requirements, with a 25 minute limit", () => {
  assert.match(workflow, /--target universal-apple-darwin --bundles app,dmg/);
  assert.match(workflow, /--target x86_64-pc-windows-msvc --bundles nsis/);
  assert.match(workflow, /--target x86_64-unknown-linux-gnu --bundles appimage,deb/);
  assert.doesNotMatch(workflow, /msi/i);
  assert.equal([...workflow.matchAll(/timeout-minutes: 25/g)].length, 1);
  assert.equal([...workflow.matchAll(/timeout-minutes: 15/g)].length, 4);
  assert.match(workflow, /lipo -create -output src-tauri\/binaries\/weaver-cli-universal-apple-darwin/);
  assert.match(workflow, /uses: tauri-apps\/tauri-action@/);
});

test("the README tells how to open an unsigned build on each system", () => {
  const readme = readFileSync(path.join(root, "README.md"), "utf8");
  for (const step of ["**Open**", "**Open Anyway**", "**More info**", "**Run anyway**", "chmod +x"]) {
    assert.ok(readme.includes(step), step);
  }
});
