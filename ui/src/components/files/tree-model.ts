import type { FileEntry } from "@/bindings/FileEntry";
import type { FolderListing } from "@/bindings/FolderListing";
import type { TreeUpdate } from "@/bindings/TreeUpdate";

/** The project root, as a folder path. */
export const ROOT = "";

/** The reason shown for every action that would change `.weaver/`. */
export const WEAVER_READ_ONLY = ".weaver is written by Weaver: read-only here.";

/** The tooltip, and the accessible name, of the lock on a file the agent is changing. */
export const AGENT_EDITING = "The agent is editing this file";

/** The path of the row that stands for the entry being made. */
export const NEW_ROW = "\u0000new";

/** A folder the backend lists, with the generation of the reading the UI holds. */
export interface TreeFolder {
  entries: readonly FileEntry[];
  generation: number;
}

/** The folders listed so far, by path (`""` is the project root); a folder that is listed is expanded. */
export type Folders = Readonly<Record<string, TreeFolder>>;

/** One visible row: an entry, or the entry being made (`isNew`). */
export interface Row {
  entry: FileEntry;
  /** 1 for the root's entries. */
  level: number;
  /** The folder it is listed in. */
  parent: string;
  /** How many entries its folder lists, and its place among them (from 1). */
  setSize: number;
  position: number;
  isNew?: boolean;
}

export function parentOf(path: string): string {
  const cut = path.lastIndexOf("/");
  return cut < 0 ? ROOT : path.slice(0, cut);
}

export function nameOf(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

export function joinPath(parent: string, name: string): string {
  return parent === ROOT ? name : `${parent}/${name}`;
}

/** Whether `path` is `folder` or lies inside it (the root holds everything). */
export function isWithin(folder: string, path: string): boolean {
  return folder === ROOT || path === folder || path.startsWith(`${folder}/`);
}

/** Whether `path` is `.weaver` or lies inside it: what Weaver writes by itself. */
export function inWeaver(path: string): boolean {
  return path === ".weaver" || path.startsWith(".weaver/");
}

/** The entry being made, which has no path on disk yet. */
export function newEntry(folder: boolean): FileEntry {
  return { name: "", path: NEW_ROW, kind: folder ? "folder" : "file" };
}

/**
 * The rows the tree shows: the root's entries, and below each listed folder its entries. While an
 * entry is being made (`creating`), its row comes first in the folder it is made in.
 */
export function visibleRows(folders: Folders, creating: { parent: string; folder: boolean } | null = null): Row[] {
  const rows: Row[] = [];
  const walk = (parent: string, level: number) => {
    const listed = folders[parent]?.entries;
    if (!listed) return;
    const adding = creating?.parent === parent;
    const setSize = listed.length + (adding ? 1 : 0);
    let position = 1;
    if (adding) rows.push({ entry: newEntry(creating.folder), level, parent, setSize, position: position++, isNew: true });
    for (const entry of listed) {
      rows.push({ entry, level, parent, setSize, position: position++ });
      if (entry.kind === "folder" && folders[entry.path]) walk(entry.path, level + 1);
    }
  };
  walk(ROOT, 1);
  return rows;
}

/**
 * The folders after `update`. A listing is taken when it is at least as new as the one held, so
 * what an older answer says never undoes what a newer one did; a folder that is gone is dropped
 * the same way.
 */
export function applyUpdate(folders: Folders, update: TreeUpdate): Folders {
  const next: Record<string, TreeFolder> = { ...folders };
  for (const path of update.removed) {
    const held = next[path];
    if (held && update.generation >= held.generation) delete next[path];
  }
  for (const listing of update.folders) {
    const held = next[listing.path];
    if (!held || update.generation >= held.generation) {
      next[listing.path] = { entries: listing.entries, generation: update.generation };
    }
  }
  return next;
}

export function fromListings(listings: readonly FolderListing[], generation: number): Folders {
  const folders: Record<string, TreeFolder> = {};
  for (const listing of listings) folders[listing.path] = { entries: listing.entries, generation };
  return folders;
}

/** The folders without `path` and the ones inside it: the folder was collapsed. */
export function without(folders: Folders, path: string): Folders {
  const next: Record<string, TreeFolder> = {};
  for (const [key, folder] of Object.entries(folders)) if (!isWithin(path, key)) next[key] = folder;
  return next;
}

/**
 * Whether `source` may be dropped on the folder `target` (empty: the root). Not what the backend
 * refuses for a name or a lock, but the moves that can never be: out of or into `.weaver/`, to the
 * folder it is in, into itself or into a folder inside it.
 */
export function canDrop(source: FileEntry, target: string): boolean {
  if (inWeaver(source.path) || inWeaver(target)) return false;
  if (parentOf(source.path) === target) return false;
  if (source.kind === "folder" && isWithin(source.path, target)) return false;
  return true;
}

/** The part of a name selected when it is edited: the name without its extension for a file. */
export function selection(name: string, folder: boolean): [number, number] {
  const dot = name.lastIndexOf(".");
  return folder || dot <= 0 ? [0, name.length] : [0, dot];
}
