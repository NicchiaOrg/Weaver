import { describe, expect, it } from "vitest";

import type { TreeEntry } from "@/bindings/TreeEntry";
import { excludedLabel, hasIncluded, stateOf, toggle, visibleRows, type Listing } from "@/components/project/import-tree-model";

const dir = (path: string): TreeEntry => ({ name: path.split("/").pop() ?? path, path, folder: true });
const file = (path: string): TreeEntry => ({ name: path.split("/").pop() ?? path, path, folder: false });

const DOCS = dir("docs");
const SRC = dir("src");
const README = file("README.md");

const listing: Listing = {
  "": [DOCS, SRC, README],
  src: [dir("src/ui"), file("src/main.ts")],
  "src/ui": [file("src/ui/a.ts"), file("src/ui/b.ts")],
};

describe("import tree exclusions", () => {
  it("excludes a folder with everything inside it, and includes it again", () => {
    const src = SRC;
    const excluded = toggle(new Set(), listing, src);
    expect([...excluded]).toEqual(["src"]);
    expect(stateOf(excluded, listing, file("src/main.ts"))).toBe("excluded");
    expect(stateOf(excluded, listing, dir("src/ui"))).toBe("excluded");
    expect(toggle(excluded, listing, src).size).toBe(0);
  });

  it("shows a partly excluded folder as mixed", () => {
    const excluded = toggle(new Set(), listing, file("src/main.ts"));
    expect(stateOf(excluded, listing, SRC)).toBe("mixed");
    expect(excludedLabel(excluded)).toBe("1 excluded");
    expect(excludedLabel(new Set())).toBe("Everything is included");
  });

  it("includes a whole mixed folder when it is toggled", () => {
    const excluded = toggle(new Set(), listing, file("src/ui/a.ts"));
    expect(toggle(excluded, listing, SRC).size).toBe(0);
  });

  it("folds a folder whose every child is excluded into the folder", () => {
    let excluded = toggle(new Set(), listing, file("src/ui/a.ts"));
    excluded = toggle(excluded, listing, file("src/ui/b.ts"));
    expect([...excluded]).toEqual(["src/ui"]);
  });

  it("includes one entry of an excluded folder and keeps its siblings excluded", () => {
    const excluded = toggle(new Set(), listing, SRC);
    const next = toggle(excluded, listing, file("src/ui/a.ts"));
    expect(stateOf(next, listing, file("src/ui/a.ts"))).toBe("included");
    expect(stateOf(next, listing, file("src/ui/b.ts"))).toBe("excluded");
    expect(stateOf(next, listing, file("src/main.ts"))).toBe("excluded");
    expect([...next].sort()).toEqual(["src/main.ts", "src/ui/b.ts"]);
  });

  it("has nothing included when every first-level entry is excluded, or the folder is empty", () => {
    let excluded: ReadonlySet<string> = new Set();
    expect(hasIncluded(excluded, listing)).toBe(true);
    for (const entry of [DOCS, SRC, README]) excluded = toggle(excluded, listing, entry);
    expect(hasIncluded(excluded, listing)).toBe(false);
    expect(hasIncluded(new Set(), { "": [] })).toBe(false);
    expect(hasIncluded(new Set(), {})).toBe(false);
  });

  it("lists the children of an open folder right after it", () => {
    const rows = visibleRows(listing, new Set(["src"]));
    expect(rows.map((row) => [row.entry.path, row.level])).toEqual([
      ["docs", 1],
      ["src", 1],
      ["src/ui", 2],
      ["src/main.ts", 2],
      ["README.md", 1],
    ]);
  });
});
