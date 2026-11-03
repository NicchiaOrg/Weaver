/**
 * What the UI knows of a tab, and what each event does to it: the machine `file-tab`. The states
 * are told by flags (a tab can be modified and locked at once); [`tabState`] names the state the
 * machine is in. Pure functions: the provider applies them and does what they ask (reading again).
 */

export type ReadOnlyReason = "tooLarge" | "weaver";

export type TabState = "saved" | "modified" | "locked" | "deleted" | "read-only" | "binary";

export interface TabModel {
  /** Stable while the tab lives, whatever its path becomes. */
  id: string;
  /** Relative to the project, with `/`. */
  path: string;
  kind: "text" | "binary";
  /** The SHA-256 of the raw bytes the tab read, or wrote last: what a save checks against. */
  hash: string;
  readOnly: ReadOnlyReason | null;
  /** An agent is changing the file. */
  locked: boolean;
  /** The text has unsaved changes. */
  modified: boolean;
  /** The file is gone from the disk: deleted by the agent after a lock, or in another way. */
  deleted: "agent" | "disk" | null;
  /** Bar: the file changed on disk while the tab has unsaved changes. */
  changedOnDisk: boolean;
  /** Bar: the lock was released while the tab has unsaved changes. */
  unlocked: boolean;
  /** Bar: a `Save all` could not save this tab, and why. */
  saveError: string | null;
  /** What the disk did while the tab was locked, settled when the lock is released. */
  whileLocked: "changed" | "deleted" | null;
  /** The name of the language that colours the text, once loaded. */
  language: string | null;
}

export function newTab(
  id: string,
  path: string,
  kind: "text" | "binary",
  hash: string,
  readOnly: ReadOnlyReason | null,
  locked: boolean,
): TabModel {
  return {
    id,
    path,
    kind,
    hash,
    readOnly,
    locked,
    modified: false,
    deleted: null,
    changedOnDisk: false,
    unlocked: false,
    saveError: null,
    whileLocked: null,
    language: null,
  };
}

/** The last part of a path. */
export function fileName(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

/** The state of the machine `file-tab` the tab is in. */
export function tabState(tab: TabModel): TabState {
  if (tab.kind === "binary") return "binary";
  if (tab.readOnly) return "read-only";
  if (tab.locked) return "locked";
  if (tab.deleted) return "deleted";
  return tab.modified ? "modified" : "saved";
}

/** The text can be changed and saved. */
export function canEdit(tab: TabModel): boolean {
  return tab.kind === "text" && tab.readOnly === null && !tab.locked;
}

/** A bar waits for the developer (the `TriangleAlert` of the tab). */
export function needsAttention(tab: TabModel): boolean {
  return tab.changedOnDisk || tab.unlocked || tab.saveError !== null || (tab.deleted !== null && !tab.locked);
}

/** `workspace.OPEN_FILE_CHANGED`: the tab, and whether the text must be read again. */
export function diskChanged(tab: TabModel): { tab: TabModel; reload: boolean } {
  if (tab.kind === "binary") return { tab: { ...tab, deleted: null }, reload: false };
  if (tab.locked) return { tab: { ...tab, whileLocked: "changed" }, reload: false };
  if (tab.readOnly) return { tab: { ...tab, deleted: null }, reload: true };
  if (tab.modified) return { tab: { ...tab, deleted: null, changedOnDisk: true }, reload: false };
  // Saved, or Deleted without unsaved changes that appeared again.
  return { tab: { ...tab, deleted: null }, reload: true };
}

/** `workspace.OPEN_FILE_DELETED`: the text stays, a bar says so. */
export function diskDeleted(tab: TabModel): TabModel {
  if (tab.locked) return { ...tab, whileLocked: "deleted" };
  return { ...tab, deleted: "disk", changedOnDisk: false };
}

/** `file-lock.LOCKED`: the text cannot be changed, the unsaved changes stay. */
export function lockTaken(tab: TabModel): TabModel {
  if (tab.kind === "binary") return tab;
  return { ...tab, locked: true, whileLocked: null };
}

/**
 * `file-lock.RELEASED`: what the tab does. `reload` asks to read the file again (Saved, Read-only);
 * a tab with unsaved changes keeps them and shows the bar of the unlock, unless the file was
 * deleted meanwhile.
 */
export function lockReleased(tab: TabModel): { tab: TabModel; reload: boolean } {
  if (tab.kind === "binary" || !tab.locked) return { tab, reload: false };
  const seen = tab.whileLocked;
  const free = { ...tab, locked: false, whileLocked: null };
  if (seen === "deleted") return { tab: { ...free, deleted: "agent", unlocked: false }, reload: false };
  if (tab.readOnly) return { tab: free, reload: true };
  if (tab.modified) return { tab: { ...free, unlocked: true }, reload: false };
  return { tab: free, reload: true };
}

/** The file is not there when the tab reads it again (after a release or a change). */
export function readFoundGone(tab: TabModel, byAgent: boolean): TabModel {
  return { ...tab, deleted: byAgent ? "agent" : "disk", changedOnDisk: false };
}

/** The text was read from the disk, with this hash: nothing is unsaved, no bar waits. */
export function readFromDisk(tab: TabModel, hash: string): TabModel {
  return { ...tab, hash, modified: false, deleted: null, changedOnDisk: false, unlocked: false, saveError: null, whileLocked: null };
}

/** The text was written to the disk, and the file now has this hash. */
export function saved(tab: TabModel, hash: string, stillModified: boolean): TabModel {
  return { ...tab, hash, modified: stillModified, deleted: null, changedOnDisk: false, unlocked: false, saveError: null };
}
