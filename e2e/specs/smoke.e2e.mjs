// End-to-end smoke test of the release build of Weaver (specs#testing, requirements/quality#Tests):
// the app starts, the welcome window shows, a fixture project opens in its window, the Board and
// the Diagram render, a card opens, the window closes and the app quits without error.
//
// The fixture project is opened the way a second launch of Weaver does it (`weaver <folder>`,
// handed to the running instance by the single-instance plugin): the system folder picker of
// `Open Folder` cannot be driven through WebDriver.
//
// Each stage needs the interface of a Task that builds it. A stage waits on its entry point: when
// the control that starts it does not exist yet (the project window has no `Board` or `Diagram`
// entry), the stage is skipped and the log says why. Once the entry point exists, every assertion
// of the stage is strict.

import assert from "node:assert/strict";
import { spawn } from "node:child_process";

import {
  appProcessName,
  appVersion,
  applicationPath,
  copyFixtureProject,
  driverOutput,
  hermeticEnvironment,
  isProcessRunning,
} from "../lib.mjs";

const FIXTURE_FOLDER = "weaver-fixture";
const FIXTURE_TASK_TITLE = "Greet the user";

/** Shared by the stages, which run in order. */
const state = { projectWindow: undefined, boardOpened: false };

/** Marks the running stage as skipped and says why in the log (Mocha has no reason field). */
function skipBecause(test, reason) {
  console.log(`[e2e] SKIPPED "${test.test.title}": ${reason}`);
  test.skip();
}

/** A navigation entry, by its name or accessible name (a count badge may follow the name). */
function entry(name) {
  return $(
    `//*[(self::button or self::a or @role="tab" or @role="link" or @role="button" or @role="menuitem") and ` +
      `(starts-with(normalize-space(.), "${name}") or starts-with(@aria-label, "${name}"))]`,
  );
}

const bodyText = () => browser.execute(() => document.body.textContent ?? "");

/** Switches to the window with this title and returns its handle; fails after the timeout. */
async function switchToWindowTitled(title, timeoutMsg) {
  let found;
  await browser.waitUntil(
    async () => {
      for (const handle of await browser.getWindowHandles()) {
        await browser.switchToWindow(handle);
        if ((await browser.getTitle()) === title) {
          found = handle;
          return true;
        }
      }
      return false;
    },
    { timeoutMsg },
  );
  return found;
}

/** Starts `weaver <folder>`: with an instance running, it hands the folder over and exits 0. */
function secondLaunch(folder) {
  return new Promise((resolve, reject) => {
    const child = spawn(applicationPath(), [folder], { env: hermeticEnvironment(), stdio: "ignore", windowsHide: true });
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error("the second launch of Weaver did not exit within 30 s"));
    }, 30_000);
    child.on("error", reject);
    child.on("close", (code) => {
      clearTimeout(timer);
      resolve(code);
    });
  });
}

describe("Weaver smoke test", function () {
  const projectFolder = copyFixtureProject(FIXTURE_FOLDER);

  it("starts and shows the welcome window with the name and the version of the app", async () => {
    await browser.waitUntil(async () => (await browser.getWindowHandles()).length === 1, { timeoutMsg: "the welcome window did not appear" });
    assert.equal(await browser.getTitle(), "Weaver");
    const url = await browser.getUrl();
    assert.ok(!/localhost:1420/.test(url), `the app must be the release build, not the dev server: ${url}`);
    await browser.waitUntil(async () => (await bodyText()).includes(appVersion), {
      timeoutMsg: `the welcome window does not show the version ${appVersion}`,
    });
    assert.ok((await bodyText()).includes("Weaver"));
  });

  it("opens the fixture project in its own window and closes the welcome window", async function () {
    if (!(await entry("Open Folder").isExisting())) {
      return skipBecause(this, "the welcome window has no `Open Folder` yet (Task \"Finestre e avvio\"), so project windows are not built");
    }
    assert.equal(await secondLaunch(projectFolder), 0, "the second launch must hand the folder over and exit 0");
    state.projectWindow = await switchToWindowTitled(FIXTURE_FOLDER, `no window titled ${FIXTURE_FOLDER} opened`);
    await browser.waitUntil(async () => (await browser.getWindowHandles()).length === 1, {
      timeoutMsg: "the welcome window did not close once the project window opened",
    });
  });

  it("draws the Board with its two columns and the Task of the fixture", async function () {
    if (!state.projectWindow) return skipBecause(this, "no project window was opened by the previous stage");
    await browser.switchToWindow(state.projectWindow);
    const board = entry("Board");
    if (!(await board.isExisting())) {
      return skipBecause(this, "the project window has no `Board` entry yet (Task \"Board\" and \"Shell e aspetto dell'app\")");
    }
    await board.click();
    await browser.waitUntil(
      async () => {
        const text = await bodyText();
        return text.includes("Backlog") && text.includes("Active") && text.includes(FIXTURE_TASK_TITLE);
      },
      { timeoutMsg: `the Board does not show Backlog, Active and the Task "${FIXTURE_TASK_TITLE}"` },
    );
    state.boardOpened = true;
  });

  it("draws the Diagram of the fixture Model", async function () {
    if (!state.projectWindow) return skipBecause(this, "no project window was opened");
    await browser.switchToWindow(state.projectWindow);
    const diagram = entry("Diagram");
    if (!(await diagram.isExisting())) {
      return skipBecause(this, "the project window has no `Diagram` entry yet (Task \"Diagramma, requisiti e specs\")");
    }
    await diagram.click();
    // The Diagram is a cytoscape canvas: it renders when a canvas of a real size has drawn pixels.
    await browser.waitUntil(
      () =>
        browser.execute(() =>
          [...document.querySelectorAll("canvas")].some((canvas) => {
            const { width, height } = canvas.getBoundingClientRect();
            if (width < 100 || height < 100 || canvas.width === 0 || canvas.height === 0) return false;
            const context = canvas.getContext("2d");
            if (!context) return false;
            const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
            for (let i = 3; i < pixels.length; i += 4) if (pixels[i] !== 0) return true;
            return false;
          }),
        ),
      { timeoutMsg: "the Diagram canvas drew nothing" },
    );
    assert.ok((await bodyText()).includes("System"), "the breadcrumb starts at System");
  });

  it("opens a card from the Board and closes it with Esc", async function () {
    if (!state.boardOpened) return skipBecause(this, "the Board did not render in the earlier stage");
    await browser.switchToWindow(state.projectWindow);
    await entry("Board").click();
    const card = await $(`//*[contains(normalize-space(.), "${FIXTURE_TASK_TITLE}")][not(*[contains(normalize-space(.), "${FIXTURE_TASK_TITLE}")])]`);
    await card.waitForDisplayed({ timeoutMsg: "the Task card is not on the Board" });
    await card.click();
    const dialog = await $('[role="dialog"]');
    await dialog.waitForDisplayed({ timeoutMsg: "the card did not open its details dialog" });
    assert.ok((await dialog.getText()).includes(FIXTURE_TASK_TITLE));
    await browser.keys("Escape");
    await dialog.waitForDisplayed({ reverse: true, timeoutMsg: "Esc did not close the details dialog" });
  });

  it("closes the windows and the app quits without error", async function () {
    const handles = await browser.getWindowHandles();
    assert.ok(handles.length >= 1, "a window is open");
    if (state.projectWindow) {
      // Closing the last project window opens the welcome window.
      await browser.switchToWindow(state.projectWindow);
      await browser.closeWindow();
      await browser.waitUntil(async () => (await browser.getWindowHandles()).length === 1, {
        timeoutMsg: "closing the last project window did not bring back the welcome window",
      });
      await browser.switchToWindow((await browser.getWindowHandles())[0]);
    }
    // Closing the welcome window quits Weaver on Windows and Linux. The driver session ends with
    // the window, so the command may fail once the app is gone.
    try {
      await browser.closeWindow();
    } catch {
      // The session may already be over.
    }
    const deadline = Date.now() + 30_000;
    while (isProcessRunning(appProcessName) && Date.now() < deadline) {
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
    assert.equal(isProcessRunning(appProcessName), false, "Weaver is still running 30 s after its last window closed");
    const output = driverOutput.join("");
    assert.ok(!/panicked at/.test(output), `the app panicked:\n${output}`);
  });
});
