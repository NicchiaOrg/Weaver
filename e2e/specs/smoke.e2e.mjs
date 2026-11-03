// End-to-end smoke test of the release build of Weaver (specs#testing, requirements/quality#Tests):
// the app starts, the welcome window shows, a fixture project opens in its window, the Board and
// the Requirements render, a card opens, the window closes and the app quits without error. With
// the project window at an odd width and height, the details dialog of the opened card has its left
// and top on whole CSS pixels.
//
// The fixture project is opened the way a second launch of Weaver does it (`weaver <folder>`,
// handed to the running instance by the single-instance plugin): the system folder picker of
// `Open Folder` cannot be driven through WebDriver.
//
// The windows are told apart by their WebDriver handles, never by the WebDriver title (the page's
// `document.title`, `Weaver` in every window): the project window is the handle that is not the
// welcome window's, and its native title, read through Tauri, must be the fixture folder's name.
//
// Each stage needs the interface of a Task that builds it. A stage waits on its entry point: when
// the control that starts it does not exist yet (the project window has no `Board` or
// `Requirements` entry), the stage is skipped and the log says why. Once the entry point exists,
// every assertion of the stage is strict.

import assert from "node:assert/strict";
import { spawn } from "node:child_process";

import {
  appProcessName,
  appVersion,
  applicationPath,
  closeWindowKeepingSession,
  copyFixtureProject,
  driverOutput,
  findProjectWindow,
  hermeticEnvironment,
  isProcessRunning,
  nativeWindowTitle,
  retryWhileNoSuchWindow,
} from "../lib.mjs";

const FIXTURE_FOLDER = "weaver-fixture";
const FIXTURE_TASK_TITLE = "Greet the user";

/** Shared by the stages, which run in order. */
const state = { welcomeWindow: undefined, projectWindow: undefined, boardOpened: false };

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

/** Resizes the window until the page itself (its inner width and height) is an odd number of CSS pixels. */
async function useOddViewport() {
  await browser.setWindowSize(1101, 701);
  for (let attempt = 0; attempt < 4; attempt += 1) {
    const { width, height } = await browser.execute(() => ({ width: window.innerWidth, height: window.innerHeight }));
    if (width % 2 === 1 && height % 2 === 1) return;
    const outer = await browser.getWindowSize();
    await browser.setWindowSize(outer.width + (width % 2 === 0 ? 1 : 0), outer.height + (height % 2 === 0 ? 1 : 0));
  }
}

describe("Weaver smoke test", function () {
  const projectFolder = copyFixtureProject(FIXTURE_FOLDER);

  it("starts and shows the welcome window with the name and the version of the app", async () => {
    await browser.waitUntil(async () => (await browser.getWindowHandles()).length === 1, { timeoutMsg: "the welcome window did not appear" });
    state.welcomeWindow = await browser.getWindowHandle();
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
    state.projectWindow = await findProjectWindow(browser, state.welcomeWindow);
    assert.equal(await nativeWindowTitle(browser), FIXTURE_FOLDER, "the native title of the project window is the folder's name");
    // The welcome window is destroyed once the project window is open; its handle goes away.
    await browser.waitUntil(
      async () => {
        const handles = await retryWhileNoSuchWindow(() => browser.getWindowHandles());
        return handles.length === 1 && handles[0] === state.projectWindow;
      },
      { timeoutMsg: "the welcome window did not close once the project window opened" },
    );
    await retryWhileNoSuchWindow(() => browser.switchToWindow(state.projectWindow));
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

  it("draws the Requirements of the fixture project", async function () {
    if (!state.projectWindow) return skipBecause(this, "no project window was opened");
    await browser.switchToWindow(state.projectWindow);
    const requirements = entry("Requirements");
    if (!(await requirements.isExisting())) {
      return skipBecause(this, "the project window has no `Requirements` entry yet");
    }
    await requirements.click();
    await browser.waitUntil(
      async () => {
        const text = await bodyText();
        return text.includes("Product") && text.includes("Greeting") && text.includes("The greeting is always shown in English.");
      },
      { timeoutMsg: "the Requirements do not show the Product Area, its Greeting fragment and the General section" },
    );
  });

  it("opens a card from the Board with whole-pixel corners at an odd window size and closes it with Esc", async function () {
    if (!state.boardOpened) return skipBecause(this, "the Board did not render in the earlier stage");
    await browser.switchToWindow(state.projectWindow);
    await useOddViewport();
    await entry("Board").click();
    const card = await $(`//*[contains(normalize-space(.), "${FIXTURE_TASK_TITLE}")][not(*[contains(normalize-space(.), "${FIXTURE_TASK_TITLE}")])]`);
    await card.waitForDisplayed({ timeoutMsg: "the Task card is not on the Board" });
    await card.click();
    const dialog = await $('[role="dialog"]');
    await dialog.waitForDisplayed({ timeoutMsg: "the card did not open its details dialog" });
    assert.ok((await dialog.getText()).includes(FIXTURE_TASK_TITLE));
    const viewport = await browser.execute(() => ({ width: window.innerWidth, height: window.innerHeight }));
    assert.ok(viewport.width % 2 === 1 && viewport.height % 2 === 1, `the viewport must have an odd width and height: ${viewport.width} × ${viewport.height}`);
    // The panel is centred by layout, not by a percentage shift: its corner is on whole CSS pixels.
    const box = await browser.execute(() => {
      const { left, top } = document.querySelector('[role="dialog"]').getBoundingClientRect();
      return { left, top };
    });
    assert.ok(Number.isInteger(box.left) && Number.isInteger(box.top), `the details dialog is between pixels: left ${box.left}, top ${box.top}`);
    await browser.keys("Escape");
    await dialog.waitForDisplayed({ reverse: true, timeoutMsg: "Esc did not close the details dialog" });
  });

  it("closes the windows and the app quits without error", async function () {
    assert.ok(state.projectWindow, "no project window was found by the earlier stages, so closing it cannot be checked");
    // Closing the last project window opens the welcome window.
    await retryWhileNoSuchWindow(() => browser.switchToWindow(state.projectWindow));
    await closeWindowKeepingSession(browser);
    await browser.waitUntil(
      async () => {
        const handles = await retryWhileNoSuchWindow(() => browser.getWindowHandles());
        return handles.length === 1 && handles[0] !== state.projectWindow;
      },
      { timeoutMsg: "closing the last project window did not bring back the welcome window" },
    );
    const [welcome] = await browser.getWindowHandles();
    await retryWhileNoSuchWindow(() => browser.switchToWindow(welcome));
    await browser.waitUntil(async () => (await bodyText()).includes(appVersion), {
      timeoutMsg: "the window that came back does not show the welcome window (name and version of the app)",
    });
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
