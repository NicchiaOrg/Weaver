import { describe, expect, it } from "vitest";

import {
  addedNames,
  isModifier,
  linkTooltip,
  markdownNames,
  notesImageName,
  refusalText,
  statusText,
  trashFailure,
  unnamedImages,
} from "@/components/notes/notes-model";

describe("statusText", () => {
  it("is empty before the first change and tells the save state after it", () => {
    expect(statusText({ kind: "none" })).toBe("");
    expect(statusText({ kind: "saving" })).toBe("Saving…");
    expect(statusText({ kind: "saved" })).toBe("Saved");
    expect(statusText({ kind: "error", reason: "No space left on device." })).toBe("Could not save: No space left on device.");
    expect(statusText({ kind: "error", reason: "disk full" })).toBe("Could not save: disk full.");
  });
});

describe("notesImageName", () => {
  it("accepts only a destination that is exactly a plain file name", () => {
    expect(notesImageName("pic.png")).toBe("pic.png");
    expect(notesImageName("with space.png")).toBe("with space.png");
    for (const other of ["", "notes.md", ".hidden.png", "sub/pic.png", "..\\pic.png", "../pic.png", "/abs.png", "https://example.com/a.png", "data:image/png;base64,AAAA"]) {
      expect(notesImageName(other)).toBeNull();
    }
  });
});

describe("images the written Markdown still names", () => {
  it("names an image by its whole file name, in any syntax", () => {
    expect(markdownNames("![](<a.png>)", "a.png")).toBe(true);
    expect(markdownNames("| ![x](a.png) |", "a.png")).toBe(true);
    expect(markdownNames('<img src="a.png">', "a.png")).toBe(true);
    expect(markdownNames("see a.png.", "a.png")).toBe(true);
    expect(markdownNames("![](<banana.png>) and a.png.bak and xa.png", "a.png")).toBe(false);
    expect(markdownNames("nothing here", "a.png")).toBe(false);
    expect(markdownNames("![](<a (1).png>)", "a (1).png")).toBe(true);
  });

  it("keeps the removed images that are still named and returns the others", () => {
    expect(unnamedImages(new Set(["a.png", "b.png", "c.png"]), "![](<b.png>)\n")).toEqual(["a.png", "c.png"]);
    expect(unnamedImages([], "text")).toEqual([]);
  });
});

describe("toast texts", () => {
  it("says which images the trash refused", () => {
    expect(trashFailure([{ name: "a.png", reason: "in use" }])).toBe('Could not move "a.png" to the trash: in use');
    expect(trashFailure([{ name: "a.png", reason: "in use" }, { name: "b.png", reason: "in use" }, { name: "c.png", reason: "gone" }])).toBe(
      "Could not move 3 images to the trash: in use; gone",
    );
  });

  it("says why an image was refused or not copied, and nothing for the others", () => {
    expect(refusalText({ status: "refused", source: "big.png", reason: "tooLarge" })).toBe('"big.png" is larger than 10 MB: notes images can be at most 10 MB.');
    expect(refusalText({ status: "refused", source: "a.bmp", reason: "notAccepted" })).toBe('"a.bmp" is not a PNG, JPEG, GIF or WebP image.');
    expect(refusalText({ status: "failed", source: "a.png", message: "permission denied." })).toBe('Could not add "a.png": permission denied.');
    expect(refusalText({ status: "leftOut" })).toBeNull();
    expect(refusalText({ status: "added", name: "a.png" })).toBeNull();
  });

  it("lists the names of the images that were added, in order", () => {
    expect(addedNames([{ status: "added", name: "a.png" }, { status: "leftOut" }, { status: "added", name: "b-2.png" }])).toEqual(["a.png", "b-2.png"]);
  });
});

describe("keys and tooltips by platform", () => {
  it("uses Cmd on macOS and Ctrl elsewhere", () => {
    expect(isModifier({ metaKey: true, ctrlKey: false }, "MacIntel")).toBe(true);
    expect(isModifier({ metaKey: false, ctrlKey: true }, "MacIntel")).toBe(false);
    expect(isModifier({ metaKey: false, ctrlKey: true }, "Win32")).toBe(true);
    expect(isModifier({ metaKey: true, ctrlKey: false }, "Linux x86_64")).toBe(false);
    expect(linkTooltip("https://a.b", "MacIntel")).toBe("https://a.b\nCmd+click to open");
    expect(linkTooltip("https://a.b", "Win32")).toBe("https://a.b\nCtrl+click to open");
  });
});
