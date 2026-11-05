// Helpers of the `release` workflow (.github/workflows/release.yml, requirements/release#General):
// the names of the files Tauri produces for the product `Weaver`, the release notes, the check
// that a draft release holds every one of them, and the updater manifest `latest.json`.
//
//   node scripts/release.mjs notes <version>
//   node scripts/release.mjs verify <version> <assets.json>
//   node scripts/release.mjs manifest <version> <assets.json> <signatures-dir> <out.json>
//
// `assets.json` is the `assets` array of the release as the GitHub API returns it. The signature of
// an updater file `<name>` is read from `<signatures-dir>/<name>.sig`.

import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

export const REPO = "nicchia-code/weaver";
const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

function checkVersion(version) {
  if (!SEMVER.test(version)) throw new Error(`"${version}" is not a version like 1.2.3`);
  return version;
}

/** The files a user downloads, as Tauri names them for the product `Weaver`. */
export function downloads(version) {
  checkVersion(version);
  return {
    macos: [`Weaver_${version}_universal.dmg`],
    windows: [`Weaver_${version}_x64-setup.exe`],
    linux: [`Weaver_${version}_amd64.AppImage`, `Weaver_${version}_amd64.deb`],
  };
}

/** What each platform's updater downloads: the file matching a name, and `<file>.sig` beside it. */
export const UPDATER_FILES = {
  macos: /\.app\.tar\.gz$/,
  windows: /-setup\.exe$/,
  linux: /\.AppImage$/,
};

/** The platforms of `latest.json` and the updater file each one takes. */
export const MANIFEST_PLATFORMS = {
  "darwin-aarch64": "macos",
  "darwin-x86_64": "macos",
  "windows-x86_64": "windows",
  "linux-x86_64": "linux",
};

export function releaseNotes(version) {
  const { macos, windows, linux } = downloads(version);
  const [appImage, deb] = linux;
  return `## Download

| System | File to download |
|---|---|
| macOS (Apple silicon and Intel) | \`${macos[0]}\` |
| Windows 10 and 11 (x64) | \`${windows[0]}\` |
| Linux (x64) | \`${appImage}\`, or \`${deb}\` |

The other files of this release (\`.sig\`, \`.app.tar.gz\` and \`latest.json\`) are read by the updater of Weaver; they are not meant to be downloaded by hand.

## Opening an unsigned build

This build is not signed or notarised yet, so the system warns that it comes from an unidentified developer. The warning does not mean the file is damaged.

**macOS** (\`${macos[0]}\`): open the disk image and drag Weaver to Applications. Right-click Weaver and choose **Open**, then **Open** again. If macOS still refuses, open System Settings → Privacy & Security and choose **Open Anyway** next to Weaver.

**Windows** (\`${windows[0]}\`): run the installer. When SmartScreen shows "Windows protected your PC", choose **More info** and then **Run anyway**.

**Linux** (\`${appImage}\`): make the file executable with \`chmod +x ${appImage}\`, then run it. For \`${deb}\`, install it with \`sudo apt install ./${deb}\`.

Weaver then keeps itself up to date from the releases of this repository.
`;
}

/** The names the release lacks of what it must hold for `version`, given the names it holds. */
export function missingAssets(version, names) {
  const have = new Set(names);
  const missing = [];
  for (const file of Object.values(downloads(version)).flat()) {
    if (!have.has(file)) missing.push(file);
  }
  for (const [platform, pattern] of Object.entries(UPDATER_FILES)) {
    const files = names.filter((name) => pattern.test(name));
    if (files.length !== 1) {
      missing.push(`exactly one updater file of ${platform} (${pattern}), found ${files.length}`);
      continue;
    }
    if (!have.has(`${files[0]}.sig`)) missing.push(`${files[0]}.sig`);
  }
  return missing;
}

/**
 * The updater manifest. `signatures` maps an updater file name to the content of its `.sig`.
 * Every platform of `MANIFEST_PLATFORMS` must be present.
 */
export function buildLatestJson({ version, pubDate, names, signatures, repo = REPO }) {
  checkVersion(version);
  const platforms = {};
  for (const [target, platform] of Object.entries(MANIFEST_PLATFORMS)) {
    const files = names.filter((name) => UPDATER_FILES[platform].test(name));
    if (files.length !== 1) throw new Error(`expected one updater file for ${target}, found ${files.length}`);
    const signature = (signatures[files[0]] ?? "").trim();
    if (!signature) throw new Error(`the signature of ${files[0]} is missing or empty`);
    platforms[target] = {
      signature,
      url: `https://github.com/${repo}/releases/download/v${version}/${encodeURIComponent(files[0])}`,
    };
  }
  return {
    version,
    notes: `Weaver v${version}. Release notes: https://github.com/${repo}/releases/tag/v${version}`,
    pub_date: pubDate,
    platforms,
  };
}

function main(argv) {
  const [command, version, ...rest] = argv;
  switch (command) {
    case "notes": {
      process.stdout.write(releaseNotes(version));
      return;
    }
    case "verify": {
      const names = JSON.parse(readFileSync(rest[0], "utf8")).map((asset) => asset.name);
      const missing = missingAssets(version, names);
      if (missing.length > 0) {
        for (const item of missing) console.error(`::error::the release lacks ${item}`);
        process.exitCode = 1;
      }
      return;
    }
    case "manifest": {
      const [assetsFile, signaturesDir, out] = rest;
      const names = JSON.parse(readFileSync(assetsFile, "utf8")).map((asset) => asset.name);
      const signatures = {};
      for (const pattern of Object.values(UPDATER_FILES)) {
        for (const name of names.filter((n) => pattern.test(n))) {
          signatures[name] = readFileSync(join(signaturesDir, `${name}.sig`), "utf8");
        }
      }
      const manifest = buildLatestJson({ version, pubDate: new Date().toISOString(), names, signatures });
      writeFileSync(out, `${JSON.stringify(manifest, null, 2)}\n`);
      return;
    }
    default:
      throw new Error("usage: release.mjs notes <version> | verify <version> <assets.json> | manifest <version> <assets.json> <signatures-dir> <out.json>");
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(`::error::${error.message}`);
    process.exitCode = 1;
  }
}
