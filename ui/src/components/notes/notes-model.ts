// What the Notes view works out without the editor: the texts it shows, which image destinations
// are files of the notes folder, which removed images the written Markdown no longer names, and
// the sentences of its toasts. Nothing here imports TipTap: the editor is a separate module that
// loads the first time the view opens.
import type { NotesImageOutcome } from "@/bindings/NotesImageOutcome";
import type { NotesTrashRefusal } from "@/bindings/NotesTrashRefusal";
import { isMacPlatform } from "@/components/assets/asset-kinds";

/** After the last change the sheet is written (the same pause as the request editor). */
export { AUTOSAVE_MS } from "@/components/requests/request-pane";

/** The first line of an empty sheet. */
export const EMPTY_TEXT = "Write notes for this project. Start a line with [ ] for a checklist.";

/** The warning when the developer's version is written over a sheet that changed on disk. */
export const KEPT_TEXT = "The notes changed on disk; Weaver kept your version.";

/** The sheet's own file name: never an image. */
export const SHEET_FILE = "notes.md";

/** The save status shown at the right of the header. */
export type SaveStatus = { kind: "none" } | { kind: "saving" } | { kind: "saved" } | { kind: "error"; reason: string };

export const NO_STATUS: SaveStatus = { kind: "none" };

/** `Saving…`, `Saved` or `Could not save: <reason>.`; empty before the first change of the session. */
export function statusText(status: SaveStatus): string {
  switch (status.kind) {
    case "saving":
      return "Saving…";
    case "saved":
      return "Saved";
    case "error":
      return `Could not save: ${status.reason.replace(/\.+$/, "")}.`;
    default:
      return "";
  }
}

/**
 * The name of a file in the notes folder that an image destination is, or `null` when it is any
 * other destination (another path, `http`, `https`, `data:`…), which is never read nor fetched.
 * Only a destination that is exactly a plain file name counts: no separator, no leading `.`, not
 * the sheet itself.
 */
export function notesImageName(destination: string): string | null {
  if (destination === "" || destination === SHEET_FILE) return null;
  if (destination.startsWith(".") || /[/\\]/.test(destination)) return null;
  if (/[\u0000-\u001f]/.test(destination)) return null;
  return destination;
}

const escapeRegex = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/**
 * Whether the Markdown names the image: its file name appears as a whole name, not as the end of
 * a longer one. Anything that is not a plain image link (a table cell, raw HTML) counts too, so
 * that an image the sheet still shows somewhere is never trashed.
 */
export function markdownNames(markdown: string, name: string): boolean {
  return new RegExp(`(?<![\\p{L}\\p{N}._-])${escapeRegex(name)}(?![\\p{L}\\p{N}_-]|\\.[\\p{L}\\p{N}])`, "u").test(markdown);
}

/** The removed images that the written Markdown no longer names: the ones that go to the trash. */
export function unnamedImages(removed: Iterable<string>, markdown: string): string[] {
  return [...removed].filter((name) => !markdownNames(markdown, name));
}

/** The toast for the images the trash refused: the one image, or how many and the reasons. */
export function trashFailure(refused: readonly NotesTrashRefusal[]): string {
  const [first] = refused;
  if (refused.length === 1 && first) return `Could not move "${first.name}" to the trash: ${first.reason}`;
  const reasons = [...new Set(refused.map((one) => one.reason))].join("; ");
  return `Could not move ${refused.length} images to the trash: ${reasons}`;
}

/** The error toast of an image the backend did not add (`added` and `leftOut` show none). */
export function refusalText(outcome: NotesImageOutcome): string | null {
  switch (outcome.status) {
    case "refused":
      return outcome.reason === "tooLarge"
        ? `"${outcome.source}" is larger than 10 MB: notes images can be at most 10 MB.`
        : `"${outcome.source}" is not a PNG, JPEG, GIF or WebP image.`;
    case "failed":
      return `Could not add "${outcome.source}": ${outcome.message.replace(/\.+$/, "")}.`;
    default:
      return null;
  }
}

/** The names the backend gave to the images it added, in the order of the files. */
export function addedNames(outcomes: readonly NotesImageOutcome[]): string[] {
  return outcomes.flatMap((outcome) => (outcome.status === "added" ? [outcome.name] : []));
}

/** Whether `Cmd` (macOS) or `Ctrl` (elsewhere) is held in this event. */
export function isModifier(event: { metaKey: boolean; ctrlKey: boolean }, platform?: string): boolean {
  return isMacPlatform(platform) ? event.metaKey : event.ctrlKey;
}

/** The tooltip of a link: its address and how to open it. */
export function linkTooltip(href: string, platform?: string): string {
  return `${href}\n${isMacPlatform(platform) ? "Cmd" : "Ctrl"}+click to open`;
}
