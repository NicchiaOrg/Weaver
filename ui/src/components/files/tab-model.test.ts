import { describe, expect, it } from "vitest";

import {
  diskChanged,
  formatWeight,
  isSvgName,
  isSvgTab,
  lockReleased,
  lockTaken,
  newTab,
  readAsImageOrBinary,
  svgDataUrl,
  tabState,
  utf8Length,
} from "@/components/files/tab-model";

const PNG = { dataUrl: "data:image/png;base64,AAAA", size: 4 };

describe("the SVG name rule", () => {
  it("is the end of the name, in any case, and never the content", () => {
    expect(isSvgName("a/b/logo.svg")).toBe(true);
    expect(isSvgName("LOGO.SVG")).toBe(true);
    expect(isSvgName("x.Svg")).toBe(true);
    expect(isSvgName("logo.svg.txt")).toBe(false);
    expect(isSvgName("svg")).toBe(false);
    expect(isSvgName("dir.svg/readme.md")).toBe(false);
    expect(isSvgTab(newTab("t", "a.svg", "text", "h", null, false))).toBe(true);
    // A binary file named `.svg` is not an SVG tab: only a text file has a drawing of its text.
    expect(isSvgTab(newTab("t", "a.svg", "binary", "h", null, false))).toBe(false);
  });
});

describe("the weight of an image", () => {
  it("is in B, KB or MB with 1 KB = 1,024 bytes and one decimal from 1 KB on", () => {
    expect(formatWeight(0)).toBe("0 B");
    expect(formatWeight(1023)).toBe("1023 B");
    expect(formatWeight(1024)).toBe("1.0 KB");
    expect(formatWeight(12_700)).toBe("12.4 KB");
    expect(formatWeight(1_048_575)).toBe("1.0 MB");
    expect(formatWeight(1_048_576)).toBe("1.0 MB");
    expect(formatWeight(10_485_760)).toBe("10.0 MB");
  });

  it("of an SVG is the length of its text in UTF-8 bytes", () => {
    expect(utf8Length("")).toBe(0);
    expect(utf8Length("abc")).toBe(3);
    expect(utf8Length("è")).toBe(2);
    expect(utf8Length("€")).toBe(3);
    expect(utf8Length("😀")).toBe(4);
    expect(utf8Length("a\nb")).toBe(3);
  });
});

describe("the drawing of an SVG", () => {
  it("is a data URL of the percent-encoded text, never anything inline", () => {
    expect(svgDataUrl("<svg/>")).toBe("data:image/svg+xml;charset=utf-8,%3Csvg%2F%3E");
  });
});

describe("a tab on an image or a binary file", () => {
  it("is in the state Image or Binary and has no lock", () => {
    const image = newTab("t", "a.png", "image", "h", null, true, PNG);
    expect(tabState(image)).toBe("image");
    expect(image.locked).toBe(false);
    expect(tabState(newTab("t", "a.bin", "binary", "h", null, true))).toBe("binary");
    expect(lockTaken(image)).toBe(image);
    expect(lockReleased(image)).toEqual({ tab: image, reload: false });
  });

  it("is read again after every change of the file, and the bar of a deletion goes", () => {
    for (const kind of ["image", "binary"] as const) {
      const tab = { ...newTab("t", "a.png", kind, "h", null, false, kind === "image" ? PNG : null), deleted: "disk" as const };
      const outcome = diskChanged(tab);
      expect(outcome.reload).toBe(true);
      expect(outcome.tab.deleted).toBeNull();
    }
  });

  it("keeps the size and the failure of an image that is read again identical, and forgets them for a new one", () => {
    const tab = { ...newTab("t", "a.png", "image", "h", null, false, PNG), pixels: { width: 4, height: 2 }, undrawable: true, actualSize: true };
    const same = readAsImageOrBinary(tab, "h2", "image", PNG, false);
    expect(same).toMatchObject({ kind: "image", pixels: { width: 4, height: 2 }, undrawable: true, actualSize: true, hash: "h2" });
    const other = readAsImageOrBinary(tab, "h3", "image", { dataUrl: "data:image/png;base64,BBBB", size: 9 }, false);
    expect(other).toMatchObject({ kind: "image", pixels: null, undrawable: false, actualSize: true });
    const binary = readAsImageOrBinary(tab, "h4", "binary", null, true);
    expect(binary).toMatchObject({ kind: "binary", image: null, imageTooLarge: true, pixels: null });
    expect(tabState(binary)).toBe("binary");
  });
});
