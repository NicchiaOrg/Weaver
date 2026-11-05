/**
 * What the UI knows of a tab, and what each event does to it: the machine `file-tab`. The states
 * are told by flags (a tab can be modified and locked at once); [`tabState`] names the state the
 * machine is in. Pure functions: the provider applies them and does what they ask (reading again).
 */

export type ReadOnlyReason = "tooLarge" | "weaver";

/** What a tab shows: text, a binary file, or an image Weaver can show. */
export type TabKind = "text" | "binary" | "image";

export type TabState = "saved" | "modified" | "locked" | "deleted" | "read-only" | "binary" | "image";

/** An image of a tab: kept in the state of that tab only, and dropped with it. */
export interface TabImage {
  /** `data:<mime>;base64,<…>`. */
  dataUrl: string;
  /** The size of the file in bytes. */
  size: number;
}

export interface TabModel {
  /** Stable while the tab lives, whatever its path becomes. */
  id: string;
  /** Relative to the project, with `/`. */
  path: string;
  kind: TabKind;
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
  /** The image of an `image` tab. */
  image: TabImage | null;
  /** A binary file with an image signature that is larger than 10 MB. */
  imageTooLarge: boolean;
  /** The image's `<img>` raised `error`: the tab says that the image cannot be shown. */
  undrawable: boolean;
  /** The size of the drawn image (an image, or the drawing of an SVG), once it loaded. */
  pixels: { width: number; height: number } | null;
  /** The image is at its real size (one image pixel per CSS pixel), not fitted to the panel. */
  actualSize: boolean;
  /** An SVG tab: the drawing or the text. It opens on the drawing. */
  svgView: "preview" | "code";
  /** Counts the texts put in the editor by the code (a read from disk): the drawing of an SVG is built again then. */
  textRev: number;
}

export function newTab(
  id: string,
  path: string,
  kind: TabKind,
  hash: string,
  readOnly: ReadOnlyReason | null,
  locked: boolean,
  image: TabImage | null = null,
  imageTooLarge = false,
): TabModel {
  return {
    id,
    path,
    kind,
    hash,
    readOnly,
    // The lock shows on text only: an image or a binary file has no lock line.
    locked: kind === "text" && locked,
    modified: false,
    deleted: null,
    changedOnDisk: false,
    unlocked: false,
    saveError: null,
    whileLocked: null,
    language: null,
    image,
    imageTooLarge,
    undrawable: false,
    pixels: null,
    actualSize: false,
    svgView: "preview",
    textRev: 0,
  };
}

/** The last part of a path. */
export function fileName(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

/** The state of the machine `file-tab` the tab is in. */
export function tabState(tab: TabModel): TabState {
  if (tab.kind === "image") return "image";
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

/**
 * `workspace.OPEN_FILE_CHANGED`: the tab, and whether the file must be read again. An image or a
 * binary file is always read again: the result says whether it is still an image, now binary, or text.
 */
export function diskChanged(tab: TabModel): { tab: TabModel; reload: boolean } {
  if (tab.kind !== "text") return { tab: { ...tab, deleted: null }, reload: true };
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
  if (tab.kind !== "text") return tab;
  return { ...tab, locked: true, whileLocked: null };
}

/**
 * `file-lock.RELEASED`: what the tab does. `reload` asks to read the file again (Saved, Read-only);
 * a tab with unsaved changes keeps them and shows the bar of the unlock, unless the file was
 * deleted meanwhile.
 */
export function lockReleased(tab: TabModel): { tab: TabModel; reload: boolean } {
  if (tab.kind !== "text" || !tab.locked) return { tab, reload: false };
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

/** The tab read the file again and it is an image Weaver can show (or a binary file): what the tab keeps of it. */
export function readAsImageOrBinary(tab: TabModel, hash: string, kind: "binary" | "image", image: TabImage | null, imageTooLarge: boolean): TabModel {
  const sameImage = image !== null && tab.image !== null && tab.image.dataUrl === image.dataUrl;
  return {
    ...readFromDisk(tab, hash),
    kind,
    readOnly: null,
    locked: false,
    image,
    imageTooLarge,
    // A new picture is drawn again: its size comes with its `load`.
    undrawable: sameImage ? tab.undrawable : false,
    pixels: sameImage ? tab.pixels : null,
  };
}

/** Whether a file name is the one of an SVG file: it ends in `.svg`, whatever the case. The content never decides it. */
export function isSvgName(path: string): boolean {
  return fileName(path).toLowerCase().endsWith(".svg");
}

/** A tab on a text file that has the drawing of its text. */
export function isSvgTab(tab: TabModel): boolean {
  return tab.kind === "text" && isSvgName(tab.path);
}

/** The `data:` URL an `<img>` draws the SVG `text` from: scripts never run and nothing external loads in an image. */
export function svgDataUrl(text: string): string {
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(text)}`;
}

/** The length of `text` in bytes once encoded as UTF-8: the weight of an SVG tab. */
export function utf8Length(text: string): number {
  let bytes = 0;
  for (const char of text) {
    const code = char.codePointAt(0) ?? 0;
    bytes += code < 0x80 ? 1 : code < 0x800 ? 2 : code < 0x10000 ? 3 : 4;
  }
  return bytes;
}

/** A weight in `B`, `KB` or `MB` (1 KB = 1,024 bytes), with one decimal from 1 KB on: `12.4 KB`. */
export function formatWeight(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kilobytes = bytes / 1024;
  if (kilobytes.toFixed(1) === "1024.0" || kilobytes > 1024) return `${(bytes / 1_048_576).toFixed(1)} MB`;
  return `${kilobytes.toFixed(1)} KB`;
}
