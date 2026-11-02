// WebdriverIO configuration of the end-to-end smoke test (specs#testing).
//
// It drives the release build of the app (`target/release/weaver`) through `tauri-driver`, which
// wraps the platform WebDriver: `WebKitWebDriver` on Linux (run under `xvfb-run`) and
// `msedgedriver` on Windows. It runs on GitHub Actions only: the workflow `ci` installs the
// drivers and builds the app first. macOS has no WebDriver for its webview.

import { spawn } from "node:child_process";
import path from "node:path";

import { applicationPath, driverOutput, hermeticEnvironment, isWindows } from "./lib.mjs";

const PORT = 4444;
const application = applicationPath();
const environment = hermeticEnvironment();

/** Where `msedgedriver.exe` is on Windows: `E2E_NATIVE_DRIVER`, else the one found on the PATH. */
function nativeDriverArguments() {
  if (process.env.E2E_NATIVE_DRIVER) return ["--native-driver", process.env.E2E_NATIVE_DRIVER];
  return [];
}

let tauriDriver;

function stopDriver() {
  if (tauriDriver && !tauriDriver.killed) {
    tauriDriver.kill();
  }
  tauriDriver = undefined;
}

export const config = {
  runner: "local",
  hostname: "127.0.0.1",
  port: PORT,
  path: "/",
  specs: ["./specs/**/*.e2e.mjs"],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      // WebKitWebDriver and msedgedriver speak classic WebDriver only.
      "wdio:enforceWebDriverClassic": true,
      "tauri:options": { application },
    },
  ],
  logLevel: "warn",
  reporters: ["spec"],
  framework: "mocha",
  mochaOpts: { ui: "bdd", timeout: 120_000 },
  waitforTimeout: 30_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 1,

  beforeSession() {
    const driver = process.env.E2E_TAURI_DRIVER ?? "tauri-driver";
    tauriDriver = spawn(driver, ["--port", String(PORT), ...nativeDriverArguments()], {
      env: environment,
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
    for (const [stream, target] of [
      [tauriDriver.stdout, process.stdout],
      [tauriDriver.stderr, process.stderr],
    ]) {
      stream.on("data", (chunk) => {
        driverOutput.push(chunk.toString("utf8"));
        target.write(chunk);
      });
    }
    tauriDriver.on("error", (error) => {
      console.error("tauri-driver failed to start:", error);
      process.exit(1);
    });
  },

  afterSession() {
    stopDriver();
  },
};

// A crash of the runner must not leave `tauri-driver` (and the app under it) running.
for (const signal of ["SIGINT", "SIGTERM", "exit"]) {
  process.on(signal, () => {
    stopDriver();
    if (signal !== "exit") process.exit(1);
  });
}

// Only for readers of the logs: where the app comes from.
console.log(`[e2e] application: ${path.relative(process.cwd(), application) || application} (${isWindows ? "Windows" : "Linux"})`);
