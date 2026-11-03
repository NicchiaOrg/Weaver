import { describe, expect, it } from "vitest";

import type { FileEntry } from "@/bindings/FileEntry";
import {
  applyUpdate,
  canDrop,
  fromListings,
  inWeaver,
  isWithin,
  nameOf,
  parentOf,
  selection,
  visibleRows,
  without,
} from "@/components/files/tree-model";

const dir = (path: string): FileEntry => ({ name: nameOf(path), path, kind: "folder" });
const file = (path: string): FileEntry => ({ name: nameOf(path), path, kind: "file" });

const FOLDERS = fromListings(
  [
    { path: "", entries: [dir("docs"), dir("src"), file("README.md")] },
    { path: "src", entries: [dir("src/deep"), file("src/main.rs")] },
    { path: "src/deep", entries: [file("src/deep/leaf.rs")] },
  ],
  1,
);

describe("the rows of the tree", () => {
  it("lists the root's entries, and under a listed folder its entries one level deeper", () => {
    const rows = visibleRows(FOLDERS);
    expect(rows.map((row) => [row.entry.path, row.level])).toEqual([
      ["docs", 1],
      ["src", 1],
      ["src/deep", 2],
      ["src/deep/leaf.rs", 3],
      ["src/main.rs", 2],
      ["README.md", 1],
    ]);
    expect(rows[2]).toMatchObject({ parent: "src", setSize: 2, position: 1 });
    expect(rows[4]).toMatchObject({ parent: "src", setSize: 2, position: 2 });
  });

  it("shows the entry being made first in its folder", () => {
    const rows = visibleRows(FOLDERS, { parent: "src", folder: false });
    const at = rows.findIndex((row) => row.isNew);
    expect(rows[at - 1]?.entry.path).toBe("src");
    expect(rows[at]).toMatchObject({ level: 2, parent: "src", setSize: 3, position: 1 });
    expect(rows[at]?.entry.kind).toBe("file");
    expect(visibleRows(FOLDERS, { parent: "", folder: true })[0]).toMatchObject({ isNew: true, level: 1 });
  });

  it("shows nothing of a folder that is not listed", () => {
    expect(visibleRows(without(FOLDERS, "src")).map((row) => row.entry.path)).toEqual(["docs", "src", "README.md"]);
    expect(visibleRows({})).toEqual([]);
  });
});

describe("what the backend says", () => {
  it("takes a listing as new as the one held, and never an older one", () => {
    const next = applyUpdate(FOLDERS, { generation: 5, folders: [{ path: "src", entries: [file("src/new.rs")] }], removed: [] });
    expect(next.src?.entries).toEqual([file("src/new.rs")]);
    expect(next.src?.generation).toBe(5);
    const older = applyUpdate(next, { generation: 3, folders: [{ path: "src", entries: [file("src/old.rs")] }], removed: [] });
    expect(older.src?.entries).toEqual([file("src/new.rs")]);
    const same = applyUpdate(next, { generation: 5, folders: [{ path: "src", entries: [file("src/same.rs")] }], removed: [] });
    expect(same.src?.entries).toEqual([file("src/same.rs")]);
  });

  it("drops a folder that is gone, unless a newer listing came after", () => {
    const gone = applyUpdate(FOLDERS, { generation: 4, folders: [], removed: ["src", "src/deep"] });
    expect(Object.keys(gone)).toEqual([""]);
    const newer = applyUpdate(FOLDERS, { generation: 0, folders: [], removed: ["src"] });
    expect(newer.src).toBeDefined();
  });

  it("collapsing forgets the folder and the folders inside it", () => {
    expect(Object.keys(without(FOLDERS, "src"))).toEqual([""]);
    expect(Object.keys(without(FOLDERS, "src/deep")).sort()).toEqual(["", "src"]);
  });
});

describe("paths", () => {
  it("names the parent and the name, and tells what is inside a folder", () => {
    expect(parentOf("a/b/c.txt")).toBe("a/b");
    expect(parentOf("c.txt")).toBe("");
    expect(nameOf("a/b/c.txt")).toBe("c.txt");
    expect(isWithin("a", "a/b")).toBe(true);
    expect(isWithin("a", "a")).toBe(true);
    expect(isWithin("a", "ab/c")).toBe(false);
    expect(isWithin("", "anything")).toBe(true);
    expect(inWeaver(".weaver")).toBe(true);
    expect(inWeaver(".weaver/tasks/t.md")).toBe(true);
    expect(inWeaver(".weaver-old")).toBe(false);
  });

  it("selects a file's name without its extension, a folder's whole name", () => {
    expect(selection("readme.md", false)).toEqual([0, 6]);
    expect(selection("archive.tar.gz", false)).toEqual([0, 11]);
    expect(selection(".gitignore", false)).toEqual([0, 10]);
    expect(selection("Makefile", false)).toEqual([0, 8]);
    expect(selection("v1.2", true)).toEqual([0, 4]);
  });
});

describe("where a row may be dropped", () => {
  it("allows a folder or the root that is not where it is, nor inside it", () => {
    expect(canDrop(file("a.txt"), "src")).toBe(true);
    expect(canDrop(file("src/a.txt"), "")).toBe(true);
    expect(canDrop(dir("src"), "docs")).toBe(true);
  });

  it("refuses the folder it is in, itself, a folder inside it and anything of .weaver", () => {
    expect(canDrop(file("src/a.txt"), "src")).toBe(false);
    expect(canDrop(file("a.txt"), "")).toBe(false);
    expect(canDrop(dir("src"), "src")).toBe(false);
    expect(canDrop(dir("src"), "src/deep")).toBe(false);
    expect(canDrop(dir("src"), "src-extra")).toBe(true);
    expect(canDrop(file("a.txt"), ".weaver")).toBe(false);
    expect(canDrop(file("a.txt"), ".weaver/tasks")).toBe(false);
    expect(canDrop(file(".weaver/system.md"), "")).toBe(false);
  });
});
