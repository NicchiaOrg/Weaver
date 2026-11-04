// The state of the Notes view of one project window (the Machine `notes-view`), held in UI memory
// only: the editor with its undo history, cursor and scroll, the hash of the sheet it last read or
// wrote, the images removed during the visit and the images already read. It talks to the backend
// and knows nothing of TipTap: the editor module (`notes-editor.tsx`) loads the first time the
// view opens and hands this class a `NotesPort`.
import type { ComponentType } from "react";
import { toast } from "sonner";

import type { NotesChanged } from "@/bindings/NotesChanged";
import type { NotesImageOutcome } from "@/bindings/NotesImageOutcome";
import type { NotesSheet } from "@/bindings/NotesSheet";
import {
  addedNames,
  AUTOSAVE_MS,
  KEPT_TEXT,
  NO_STATUS,
  notesImageName,
  refusalText,
  trashFailure,
  unnamedImages,
  type SaveStatus,
} from "@/components/notes/notes-model";
import { localTimeOf, toBase64 } from "@/components/requests/request-model";
import { call, isAppError, on } from "@/lib/backend";

/** What the images of the sheet need to show: the `data:` URL of one file of the notes folder, read once. */
export type ImageEntry = { state: "loading" } | { state: "ready"; url: string } | { state: "missing" };

/** The images read so far, by name. A name is read once, again only after a `notes-changed` names it. */
export class ImageStore {
  private entries = new Map<string, ImageEntry>();
  private listeners = new Set<() => void>();

  constructor(private readonly projectId: string) {}

  peek(name: string): ImageEntry | undefined {
    return this.entries.get(name);
  }

  /** Reads the image when it has not been read yet; only a plain file name is ever asked for. */
  ensure(name: string): void {
    if (this.entries.has(name) || notesImageName(name) === null) return;
    this.entries.set(name, { state: "loading" });
    call("notes_image", { projectId: this.projectId, name })
      .then((url) => this.settle(name, { state: "ready", url }))
      .catch(() => this.settle(name, { state: "missing" }));
  }

  /** The images a `notes-changed` names are read again the next time they are shown. */
  invalidate(names: readonly string[]): void {
    let changed = false;
    for (const name of names) changed = this.entries.delete(name) || changed;
    if (changed) this.emit();
  }

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  private settle(name: string, entry: ImageEntry): void {
    // An image invalidated while it was being read is read again, never settled with the old bytes.
    if (this.entries.get(name)?.state !== "loading") return;
    this.entries.set(name, entry);
    this.emit();
  }

  private emit(): void {
    for (const listener of [...this.listeners]) listener();
  }
}

/** Where images are added: the cursor, or the point of a drop, followed while the text around changes. */
export interface InsertAnchor {
  /** Inserts the images, written `![](<name>)`, as blocks at the anchor, in order. */
  insert(names: string[]): void;
  release(): void;
}

/** A point of the sheet where something can be dropped, with the line that shows it. */
export interface DropPoint {
  pos: number;
  /** Viewport rectangle of the line: its left, top and width. */
  line: { left: number; top: number; width: number };
}

/** What the session asks of the editor. */
export interface NotesPort {
  /** The sheet as the editor writes it. */
  getMarkdown(): string;
  /** Replaces the whole content in one transaction that is not the developer's (cursor kept where the text around it exists). */
  replaceFromDisk(markdown: string): void;
  focus(): void;
  /** Starts following `pos`, or the cursor. */
  track(pos?: number): InsertAnchor;
  /** Where something dropped at the viewport point `x`, `y` would go; `null` outside the sheet. */
  dropPoint(x: number, y: number): DropPoint | null;
  destroy(): void;
}

/** What the editor reports of the developer's doings. */
export interface EditorEvents {
  /** The developer changed the sheet (typing, formatting, paste, undo, redo…). */
  edited(): void;
  /** The developer ticked or unticked an item: the sheet is written at once. */
  ticked(): void;
  /** The editor lost the focus. */
  blurred(): void;
  /** Images left the document through the developer's own transactions. */
  imagesRemoved(names: string[]): void;
  /** Image files were pasted; `anchor` follows the cursor. */
  pasted(files: File[], anchor: InsertAnchor): void;
}

/** The module that loads the first time the view opens: it alone imports TipTap. */
export interface NotesEditorModule {
  createNotesEditor(options: { markdown: string; images: ImageStore; events: EditorEvents }): NotesPort;
  NotesSheet: ComponentType<{ port: NotesPort }>;
}

export interface NotesSnapshot {
  status: SaveStatus;
  /** The editor, once it has loaded. */
  editor: { port: NotesPort; module: NotesEditorModule } | null;
  /** Why the sheet could not be read, until it is read. */
  loadError: string | null;
}

let loading: Promise<NotesEditorModule> | null = null;

/** One `import()` for the whole window: the first time any Notes view opens. */
export function loadEditorModule(): Promise<NotesEditorModule> {
  loading ??= import("@/components/notes/notes-editor");
  return loading;
}

const reasonOf = (error: unknown, fallback: string): string => (isAppError(error) ? error.message : fallback);

export class NotesSession {
  readonly images: ImageStore;
  private snap: NotesSnapshot = { status: NO_STATUS, editor: null, loadError: null };
  private listeners = new Set<() => void>();
  /** The view is shown: Weaver follows the notes on disk. */
  private isOpen = false;
  private visit = 0;
  private opening: Promise<void> | null = null;
  /** The hash of the sheet as last read or written (`null`: the file was missing); unknown until the first read. */
  private diskHash: string | null = null;
  private known = false;
  private generation = -1;
  /** A change of the developer waits to be written. */
  private dirty = false;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private running: Promise<void> | null = null;
  /** The images that left the document through the developer's own transactions during the visit. */
  private removed = new Set<string>();
  private stops: Array<() => void> = [];
  private flushStop: (() => void) | null = null;
  private alive = false;

  constructor(
    private readonly projectId: string,
    private readonly loadModule: () => Promise<NotesEditorModule> = loadEditorModule,
  ) {
    this.images = new ImageStore(projectId);
  }

  /** The window's provider mounted: the backend may now ask for the waiting text before the window closes. */
  start(): void {
    this.alive = true;
    if (this.flushStop) return;
    // The backend asks for it before the window closes or the app quits, whether or not the view was ever opened.
    const stop = on("notes-flush", (event) => {
      if (event.projectId === this.projectId) void this.answerFlush();
    });
    this.flushStop = () => void stop.then((unlisten) => unlisten()).catch(() => {});
  }

  // --- what React reads -----------------------------------------------------------------------

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  getSnapshot = (): NotesSnapshot => this.snap;

  private update(patch: Partial<NotesSnapshot>): void {
    this.snap = { ...this.snap, ...patch };
    for (const listener of [...this.listeners]) listener();
  }

  /** The window's view is gone for good (the project window unmounts). */
  dispose(): void {
    this.alive = false;
    clearTimeout(this.timer);
    this.release();
    this.flushStop?.();
    this.flushStop = null;
    this.snap.editor?.port.destroy();
  }

  // --- developer.OPEN_NOTES_VIEW --------------------------------------------------------------

  /** The view is shown: the sheet is read, and the editor built the first time or brought up to date. */
  open(): Promise<void> {
    if (this.isOpen) return this.opening ?? Promise.resolve();
    this.isOpen = true;
    const visit = ++this.visit;
    this.update({ loadError: null });
    // A visit that starts while the last one is still loading waits for it: one editor only.
    const run = (this.opening ?? Promise.resolve()).then(() => this.begin(visit));
    this.opening = run;
    void run.finally(() => {
      if (this.opening === run) this.opening = null;
    });
    return run;
  }

  private async begin(visit: number): Promise<void> {
    if (!this.isOpen || this.visit !== visit) return;
    // Listen first, then read: a change between the two is not lost.
    this.follow();
    try {
      const [sheet, module] = await Promise.all([call("notes_read", { projectId: this.projectId }), this.loadModule()]);
      if (!this.alive) return;
      if (!this.snap.editor) {
        const port = module.createNotesEditor({ markdown: sheet.text ?? "", images: this.images, events: this.events() });
        this.diskHash = sheet.hash;
        this.known = true;
        this.update({ editor: { port, module } });
      } else {
        this.reconcile(sheet);
      }
      if (this.isOpen && this.visit === visit) this.snap.editor?.port.focus();
    } catch (error) {
      if (!this.alive) return;
      this.update({ loadError: reasonOf(error, "the notes could not be read") });
      // Nothing was shown: the view is closed again, and opening it once more reads again.
      if (this.visit === visit) {
        this.isOpen = false;
        this.release();
      }
    }
  }

  private events(): EditorEvents {
    return {
      edited: () => this.edited(),
      ticked: () => {
        this.dirty = true;
        void this.flush();
      },
      blurred: () => void this.flush(),
      imagesRemoved: (names) => {
        for (const name of names) if (notesImageName(name) !== null) this.removed.add(name);
      },
      pasted: (files, anchor) => void this.addPasted(files, anchor),
    };
  }

  // --- workspace.NOTES_CHANGED ----------------------------------------------------------------

  private follow(): void {
    const stop = on("notes-changed", (event) => {
      if (this.isOpen && event.projectId === this.projectId) void this.changedOnDisk(event);
    });
    this.stops.push(() => void stop.then((unlisten) => unlisten()).catch(() => {}));
  }

  private release(): void {
    for (const stop of this.stops.splice(0)) stop();
  }

  private async changedOnDisk(event: NotesChanged): Promise<void> {
    if (event.generation < this.generation) return;
    this.generation = event.generation;
    if (event.images.length > 0) this.images.invalidate(event.images);
    if (!event.sheetChanged) return;
    // The sheet Weaver last wrote or read is not a change.
    if (this.known && event.hash === this.diskHash) return;
    if (this.dirty || this.running) {
      this.keepOwn();
      return;
    }
    try {
      const sheet = await call("notes_read", { projectId: this.projectId });
      if (this.isOpen && this.snap.editor) this.reconcile(sheet);
    } catch {
      // The next change of the sheet reads it again.
    }
  }

  /** The disk has `sheet`; the editor shows what it last knew of the disk, and maybe a change of the developer on top. */
  private reconcile(sheet: NotesSheet): void {
    const port = this.snap.editor?.port;
    if (!port) return;
    if (this.known && sheet.hash === this.diskHash) {
      // Still the sheet the view left: it comes back as it was, with a change that failed to be written still waiting.
      if (this.dirty) this.schedule();
      return;
    }
    if (this.dirty || this.running) {
      this.keepOwn();
      return;
    }
    port.replaceFromDisk(sheet.text ?? "");
    this.diskHash = sheet.hash;
    this.known = true;
  }

  /** The sheet changed on disk while a change of the developer waits: his version is written over it, with a warning. */
  private keepOwn(): void {
    toast.warning(KEPT_TEXT);
    this.dirty = true;
    void this.flush();
  }

  // --- developer.EDIT_NOTES / developer.TICK_NOTES_ITEM ---------------------------------------

  private edited(): void {
    this.dirty = true;
    this.schedule();
  }

  private schedule(): void {
    clearTimeout(this.timer);
    this.timer = setTimeout(() => void this.flush(), AUTOSAVE_MS);
  }

  /** Writes the waiting text now, after the write that runs, when there is one. */
  async flush(): Promise<void> {
    clearTimeout(this.timer);
    if (this.running) {
      await this.running;
      // What arrived during that write was written by the same loop; anything left is written now.
      if (this.dirty && !this.running) await this.flush();
      return;
    }
    if (!this.dirty) return;
    const run = this.writeLoop().finally(() => {
      this.running = null;
    });
    this.running = run;
    await run;
  }

  /** At most one write at a time; the texts that arrive meanwhile become one more write of the latest. */
  private async writeLoop(): Promise<void> {
    const port = this.snap.editor?.port;
    if (!port) return;
    while (this.dirty) {
      this.dirty = false;
      this.update({ status: { kind: "saving" } });
      try {
        const done = await call("notes_write", { projectId: this.projectId, text: port.getMarkdown() });
        this.diskHash = done.hash;
        this.known = true;
      } catch (error) {
        // The text stays as it is; the next change tries again.
        this.dirty = true;
        this.update({ status: { kind: "error", reason: reasonOf(error, "the notes could not be written") } });
        return;
      }
    }
    this.update({ status: { kind: "saved" } });
  }

  // --- developer.LEAVE_NOTES_VIEW -------------------------------------------------------------

  /** Another view takes the place of Notes: what waits is written, then the removed images that are no longer named go to the trash. */
  async leave(): Promise<void> {
    if (!this.isOpen) return;
    this.isOpen = false;
    this.visit++;
    this.release();
    await this.opening?.catch(() => {});
    await this.flush();
    await this.trashRemoved();
  }

  private async trashRemoved(): Promise<void> {
    const port = this.snap.editor?.port;
    // A text that could not be written is not the written Markdown: nothing is trashed until it is.
    if (!port || this.dirty || this.removed.size === 0) return;
    const names = unnamedImages(this.removed, port.getMarkdown());
    this.removed.clear();
    if (names.length === 0) return;
    try {
      const done = await call("notes_trash", { projectId: this.projectId, names });
      if (done.refused.length > 0) toast.error(trashFailure(done.refused));
    } catch (error) {
      toast.error(trashFailure(names.map((name) => ({ name, reason: reasonOf(error, "the trash refused it") }))));
    }
  }

  // --- window.CLOSED --------------------------------------------------------------------------

  /** The backend asked for what waits before the window closes: the waiting text and the images to trash after it. */
  private async answerFlush(): Promise<void> {
    let text: string | null = null;
    let trash: string[] = [];
    try {
      const port = this.snap.editor?.port;
      if (port && this.isOpen) {
        clearTimeout(this.timer);
        await this.running?.catch(() => {});
        const current = port.getMarkdown();
        text = this.dirty ? current : null;
        trash = unnamedImages(this.removed, current);
        this.removed.clear();
        this.dirty = false;
      }
    } finally {
      await call("notes_flushed", { projectId: this.projectId, text, trash }).catch(() => {});
    }
  }

  // --- developer.ADD_NOTES_IMAGES -------------------------------------------------------------

  private async addPasted(files: File[], anchor: InsertAnchor): Promise<void> {
    try {
      const pasted = await Promise.all(
        files.map(async (file) => ({ name: file.name === "" ? null : file.name, data: toBase64(new Uint8Array(await file.arrayBuffer())) })),
      );
      const done = await call("notes_add_pasted", { projectId: this.projectId, files: pasted, localTime: localTimeOf(new Date()) });
      this.added(done.files, anchor);
    } catch (error) {
      anchor.release();
      toast.error(reasonOf(error, "The images could not be added."));
    }
  }

  /** Files dropped on the sheet at `pos` (absolute paths, as Tauri reports them). */
  async addDropped(paths: string[], pos: number): Promise<void> {
    const anchor = this.snap.editor?.port.track(pos);
    if (!anchor) return;
    try {
      const done = await call("notes_add_dropped", { projectId: this.projectId, paths, localTime: localTimeOf(new Date()) });
      this.added(done.files, anchor);
    } catch (error) {
      anchor.release();
      toast.error(reasonOf(error, "The images could not be added."));
    }
  }

  private added(outcomes: readonly NotesImageOutcome[], anchor: InsertAnchor): void {
    for (const outcome of outcomes) {
      const text = refusalText(outcome);
      if (text) toast.error(text);
    }
    const names = addedNames(outcomes);
    if (names.length > 0) anchor.insert(names);
    anchor.release();
  }
}
