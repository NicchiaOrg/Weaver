import { EditorState, Transaction, type TransactionSpec } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { toast } from "sonner";

import {
  createEditorState,
  isEdit,
  languageCompartment,
  programmatic,
  readOnlyCompartment,
  readOnlyExtension,
  themeCompartment,
} from "@/components/files/editor-state";
import { editorTheme } from "@/components/files/editor-theme";
import { languageOf } from "@/components/files/languages";
import {
  canEdit,
  diskChanged,
  diskDeleted,
  fileName,
  lockReleased,
  lockTaken,
  newTab,
  readAsImageOrBinary,
  readFoundGone,
  readFromDisk,
  saved,
  type TabModel,
} from "@/components/files/tab-model";
import { useProject } from "@/components/project-context";
import { useResolvedTheme } from "@/components/theme-provider";
import { useViewShown } from "@/components/view-shown";
import { call, isAppError, on, type Commands, type EventPayloads } from "@/lib/backend";

type OpenResult = Commands["file_open"]["result"];
type ReloadResult = Commands["file_reload"]["result"];
type SaveMode = Commands["file_save"]["args"]["mode"];

/** A file `Save all` could not save, and why. */
export interface SaveFailure {
  path: string;
  reason: string;
}

/** Which dialog waits for the developer. */
export type TabDialog = { kind: "close"; id: string } | { kind: "changed"; id: string } | null;

/** What the tabs of the File view offer: to the tree (`useFileTabs`) and to the tab view itself. */
export interface FileTabsValue {
  /** The tabs in opening order. */
  tabs: readonly TabModel[];
  activeId: string | null;
  activePath: string | null;
  /** `developer.OPEN_FILE`: shows the file's tab, opening it first when it has none. */
  openFile: (path: string) => void;
  /** Closes the tab at `path`, or those inside the folder `path`, losing their unsaved changes. */
  closeUnder: (path: string) => void;
  /** The tabs on `oldPath`, or inside it, follow `newPath` and keep their unsaved changes. */
  moveUnder: (oldPath: string, newPath: string) => void;
  /** The paths at or inside `path` with unsaved changes. */
  unsavedUnder: (path: string) => string[];
  unsavedPaths: readonly string[];
  /** The paths of the tabs whose lock is held. */
  lockedPaths: ReadonlySet<string>;
  showTab: (path: string) => void;
  /** Saves every tab with unsaved changes; the files that were not saved come back with the reason, and the tab of the first one is shown. */
  saveAll: () => Promise<SaveFailure[]>;
  /** Closes every tab, losing what is unsaved. */
  closeAll: () => void;

  // What the tab view needs.
  showTabById: (id: string) => void;
  /** `developer.CLOSE_TAB`: closes the tab, asking first when it has unsaved changes. */
  requestClose: (id: string) => void;
  save: (id: string) => void;
  /** Overwrites the file with the tab's text (`Overwrite`, `Keep mine`). */
  overwrite: (id: string) => void;
  /** `developer.RELOAD_FILE`: takes the file as it is on disk. */
  reload: (id: string) => void;
  /** `Cancel` of the bar of an unlock: the unsaved changes stay, the next save passes through the check. */
  keepUnsaved: (id: string) => void;
  dismissSaveError: (id: string) => void;
  dialog: TabDialog;
  /** Answers the dialog of a tab with unsaved changes being closed. */
  answerClose: (answer: "save" | "discard" | "cancel") => void;
  answerChanged: (answer: "reload" | "overwrite" | "cancel") => void;
  /** The editor: the view that shows the active tab attaches itself here. */
  attachView: (view: EditorView | null) => void;
  /** The 1-based line of the cursor of a tab. */
  cursorLine: (id: string) => number | null;
  /** The text a text tab holds now, unsaved changes included. */
  textOf: (id: string) => string;
  /** `Code` or `Preview` of an SVG tab. */
  setSvgView: (id: string, view: "preview" | "code") => void;
  /** Switches an image between fitted to the panel and its real size. */
  toggleActualSize: (id: string) => void;
  /** The image of a tab loaded with this size in pixels (`null`: nothing is drawn). */
  imageDrawn: (id: string, pixels: { width: number; height: number } | null) => void;
  /** The image of a tab could not be drawn. */
  imageFailed: (id: string) => void;
}

const FileTabsContext = createContext<FileTabsValue | null>(null);

export function useFileTabs(): FileTabsValue {
  const value = useContext(FileTabsContext);
  if (!value) throw new Error("useFileTabs needs a FileTabsProvider");
  return value;
}

let nextTab = 1;

function reasonOf(error: unknown, fallback: string): string {
  return isAppError(error) ? error.message : fallback;
}

/** Where the path `path` goes when the entry `oldPath` is now `newPath`; `null` when it is not inside. */
function moved(path: string, oldPath: string, newPath: string): string | null {
  if (path === oldPath) return newPath;
  return path.startsWith(`${oldPath}/`) ? newPath + path.slice(oldPath.length) : null;
}

function isUnder(path: string, folder: string): boolean {
  return path === folder || path.startsWith(`${folder}/`);
}

/**
 * The tabs of the File view of a project window (the machine `file-tab`): which files are open, the
 * text of each one in its editor state, what the disk and the agent do to them, saving and closing.
 * It exists while the File view is shown. It tells the backend, at every change, in which files
 * there are unsaved changes, and answers its request for the texts when the window closes.
 */
export function FileTabsProvider({ children }: { children: ReactNode }) {
  const { projectId } = useProject();
  const theme = useResolvedTheme();
  const shown = useViewShown();
  const [tabs, setTabs] = useState<readonly TabModel[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [dialog, setDialog] = useState<TabDialog>(null);
  const tabsRef = useRef<readonly TabModel[]>([]);
  const activeRef = useRef<string | null>(null);
  /** The ids of the tabs, the least recently shown first. */
  const recent = useRef<string[]>([]);
  /** The editor state of each text tab. */
  const states = useRef(new Map<string, EditorState>());
  const view = useRef<EditorView | null>(null);
  /** The tab whose state the view shows. */
  const viewId = useRef<string | null>(null);
  const generation = useRef(0);
  /** The texts sent to the backend for `Save all`, by path, until it says which were saved. */
  const flushed = useRef<Record<string, string>>({});
  const alive = useRef(true);
  const themeRef = useRef(theme);
  themeRef.current = theme;

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  // --- the editor states --------------------------------------------------------------------

  const textOf = useCallback((id: string): string => states.current.get(id)?.doc.toString() ?? "", []);

  /** Applies a transaction to the state of a tab: through the view when it is shown, else to the state. */
  const apply = useCallback((id: string, spec: TransactionSpec) => {
    const state = states.current.get(id);
    if (!state) return;
    if (id === viewId.current && view.current) {
      view.current.dispatch(spec);
      return;
    }
    states.current.set(id, state.update(spec).state);
  }, []);

  /** Puts the state of the active tab in the view, when the view shows another one. */
  const syncView = useCallback(() => {
    const current = view.current;
    const id = activeRef.current;
    if (!current || !id || viewId.current === id) return;
    const state = states.current.get(id);
    if (!state) return;
    current.setState(state);
    viewId.current = id;
  }, []);

  /** The view holds no text of a tab that is gone. */
  const showNothing = useCallback(() => {
    view.current?.setState(EditorState.create({ doc: "" }));
    viewId.current = null;
  }, []);

  const replaceText = useCallback(
    (id: string, text: string) => {
      const state = states.current.get(id);
      if (!state) return;
      apply(id, {
        changes: { from: 0, to: state.doc.length, insert: text },
        annotations: [programmatic.of(true), Transaction.addToHistory.of(false)],
        selection: { anchor: Math.min(state.selection.main.head, text.length) },
      });
    },
    [apply],
  );

  // --- the tabs -----------------------------------------------------------------------------

  /** Replaces a tab with what `change` makes of it; the editor follows when it can be changed no more. */
  const mutate = useCallback(
    (id: string, change: (tab: TabModel) => TabModel) => {
      const before = tabsRef.current.find((tab) => tab.id === id);
      if (!before) return;
      const after = change(before);
      if (after === before) return;
      const next = tabsRef.current.map((tab) => (tab.id === id ? after : tab));
      tabsRef.current = next;
      setTabs(next);
      if (canEdit(before) !== canEdit(after) && states.current.has(id)) {
        apply(id, { effects: readOnlyCompartment.reconfigure(readOnlyExtension(!canEdit(after))) });
      }
    },
    [apply],
  );

  const tabById = useCallback((id: string) => tabsRef.current.find((tab) => tab.id === id), []);
  const tabAt = useCallback((path: string) => tabsRef.current.find((tab) => tab.path === path), []);

  const show = useCallback((id: string) => {
    recent.current = [...recent.current.filter((other) => other !== id), id];
    activeRef.current = id;
    setActiveId(id);
  }, []);

  const showTab = useCallback(
    (path: string) => {
      const tab = tabAt(path);
      if (tab) show(tab.id);
    },
    [show, tabAt],
  );

  const dropTab = useCallback((id: string, notify: boolean) => {
    const tab = tabsRef.current.find((other) => other.id === id);
    if (!tab) return;
    const next = tabsRef.current.filter((other) => other.id !== id);
    tabsRef.current = next;
    setTabs(next);
    if (viewId.current === id) showNothing();
    states.current.delete(id);
    recent.current = recent.current.filter((other) => other !== id);
    if (activeRef.current === id) {
      const neighbour = recent.current[recent.current.length - 1] ?? null;
      activeRef.current = neighbour;
      setActiveId(neighbour);
    }
    setDialog((current) => (current && current.id === id ? null : current));
    if (notify) call("file_close", { projectId, path: tab.path }).catch(() => {});
  }, [projectId]);

  /** Makes the editor state of a text tab, and loads the colours of its kind. */
  const startEditor = useCallback(
    (tab: TabModel, text: string) => {
      const id = tab.id;
      states.current.set(
        id,
        createEditorState(text, themeRef.current, !canEdit(tab), (update) => {
          states.current.set(id, update.state);
          if (isEdit(update) && alive.current) mutate(id, (current) => (current.modified ? current : { ...current, modified: true }));
        }),
      );
      if (tab.readOnly !== "tooLarge") {
        languageOf(fileName(tab.path)).then((language) => {
          if (!language || !alive.current || !states.current.has(id)) return;
          apply(id, { effects: languageCompartment.reconfigure(language.support) });
          mutate(id, (current) => ({ ...current, language: language.name }));
        });
      }
    },
    [apply, mutate],
  );

  const addTab = useCallback(
    (tab: TabModel, text: string | null) => {
      if (text !== null) startEditor(tab, text);
      const next = [...tabsRef.current, tab];
      tabsRef.current = next;
      setTabs(next);
      show(tab.id);
    },
    [show, startEditor],
  );

  const openFile = useCallback(
    (path: string) => {
      const existing = tabAt(path);
      if (existing) {
        show(existing.id);
        return;
      }
      const order = recent.current
        .map((id) => tabsRef.current.find((tab) => tab.id === id))
        .filter((tab): tab is TabModel => tab !== undefined)
        .map((tab) => ({ path: tab.path, unsaved: tab.modified }));
      call("file_open", { projectId, path, tabs: order })
        .then((result: OpenResult) => {
          if (!alive.current) return;
          if (result.status === "full") {
            toast.warning("Every tab has unsaved changes: save or close one to open another file.");
            return;
          }
          if (result.status === "alreadyOpen") {
            showTab(result.path);
            return;
          }
          if (result.closed !== null) {
            const gone = tabAt(result.closed);
            if (gone) dropTab(gone.id, false);
          }
          const id = `tab-${nextTab++}`;
          const image = result.kind === "image" ? result.image : null;
          addTab(
            newTab(id, result.path, result.kind, result.hash, result.readOnly, result.locked, image, result.imageTooLarge),
            result.kind === "text" ? (result.text ?? "") : null,
          );
        })
        .catch((error: unknown) => {
          toast.error(`Could not open ${fileName(path)}.`, { description: reasonOf(error, "The file could not be read.") });
        });
    },
    [addTab, dropTab, projectId, show, showTab, tabAt],
  );

  // --- reading and saving -------------------------------------------------------------------

  /** `developer.RELOAD_FILE` and the reads after a change: takes the file as it is on disk. */
  const readAgain = useCallback(
    async (id: string, byAgent: boolean) => {
      const tab = tabById(id);
      if (!tab) return;
      let result: ReloadResult;
      try {
        result = await call("file_reload", { projectId, path: tab.path });
      } catch (error) {
        toast.error(`Could not read ${fileName(tab.path)}.`, { description: reasonOf(error, "The file could not be read.") });
        return;
      }
      if (!alive.current || !tabById(id)) return;
      if (result.status === "gone") {
        mutate(id, (current) => readFoundGone(current, byAgent));
        return;
      }
      if (result.kind === "binary" || result.kind === "image") {
        // Not text (any more): the tab drops its editor and keeps the image, if there is one, for itself.
        if (viewId.current === id) showNothing();
        states.current.delete(id);
        const image = result.kind === "image" ? result.image : null;
        mutate(id, (current) => readAsImageOrBinary(current, result.hash, image ? "image" : "binary", image, result.imageTooLarge));
        return;
      }
      const now = tabById(id);
      if (now && (now.kind !== "text" || !states.current.has(id))) {
        // A file that was binary or an image is text now: it gets an editor.
        const next: TabModel = {
          ...readFromDisk(now, result.hash),
          kind: "text",
          readOnly: result.readOnly,
          image: null,
          imageTooLarge: false,
          undrawable: false,
          pixels: null,
          textRev: now.textRev + 1,
        };
        startEditor(next, result.text ?? "");
        mutate(id, () => next);
        return;
      }
      replaceText(id, result.text ?? "");
      mutate(id, (current) => ({ ...readFromDisk(current, result.hash), readOnly: result.readOnly, textRev: current.textRev + 1 }));
    },
    [mutate, projectId, replaceText, startEditor, tabById],
  );

  /** `developer.SAVE_FILE` and its variants; says what came of it, never shows the conflict. */
  const write = useCallback(
    async (id: string, mode: SaveMode): Promise<"saved" | "changed" | "gone" | "failed" | "skipped"> => {
      const tab = tabById(id);
      if (!tab || !canEdit(tab)) return "skipped";
      const text = textOf(id);
      const fail = (reason: string) => {
        toast.error(`Could not save ${fileName(tab.path)}.`, { description: reason });
      };
      let result: Commands["file_save"]["result"];
      try {
        result = await call("file_save", { projectId, path: tab.path, text, hash: tab.hash, mode });
      } catch (error) {
        fail(reasonOf(error, "The file could not be written."));
        return "failed";
      }
      if (!alive.current || !tabById(id)) return result.status === "saved" ? "saved" : "skipped";
      switch (result.status) {
        case "saved":
          mutate(id, (current) => saved(current, result.hash, textOf(id) !== text));
          return "saved";
        case "gone":
          mutate(id, (current) => readFoundGone(current, false));
          return "gone";
        case "changed":
          return "changed";
        case "failed":
          fail(result.reason);
          return "failed";
      }
    },
    [mutate, projectId, tabById, textOf],
  );

  /** `developer.SAVE_FILE` from the tab: a file changed on disk asks what to keep. */
  const save = useCallback(
    (id: string) => {
      const tab = tabById(id);
      if (!tab || !canEdit(tab)) return;
      if (!tab.modified && !tab.deleted) return;
      void write(id, tab.deleted ? "again" : "check").then((outcome) => {
        if (outcome !== "changed") return;
        if (tab.deleted) {
          // The file is back: what the tab has is no longer the file's last version.
          mutate(id, (current) => ({ ...current, deleted: null, changedOnDisk: current.modified }));
          if (!tab.modified) void readAgain(id, false);
          return;
        }
        if (alive.current) setDialog({ kind: "changed", id });
      });
    },
    [mutate, readAgain, tabById, write],
  );

  const overwrite = useCallback((id: string) => void write(id, "overwrite"), [write]);
  const reload = useCallback((id: string) => void readAgain(id, false), [readAgain]);
  const keepUnsaved = useCallback((id: string) => mutate(id, (tab) => ({ ...tab, unlocked: false })), [mutate]);
  const dismissSaveError = useCallback((id: string) => mutate(id, (tab) => ({ ...tab, saveError: null })), [mutate]);

  // --- what a tab shows: the image, the drawing of an SVG ------------------------------------

  /** An SVG tab shows its drawing or its text; the choice holds until the tab closes and never touches the file. */
  const setSvgView = useCallback((id: string, view: "preview" | "code") => mutate(id, (tab) => (tab.svgView === view ? tab : { ...tab, svgView: view })), [mutate]);
  /** Fitted to the panel, or one image pixel per CSS pixel: the choice survives the image being read again. */
  const toggleActualSize = useCallback((id: string) => mutate(id, (tab) => ({ ...tab, actualSize: !tab.actualSize })), [mutate]);
  /** The image of the tab loaded (`pixels`), or the drawing of its text is gone (`null`). */
  const imageDrawn = useCallback(
    (id: string, pixels: { width: number; height: number } | null) =>
      mutate(id, (tab) => (tab.pixels?.width === pixels?.width && tab.pixels?.height === pixels?.height ? tab : { ...tab, pixels })),
    [mutate],
  );
  /** The `<img>` of an image tab raised `error`: the tab says that the file is binary and the image cannot be shown. */
  const imageFailed = useCallback((id: string) => mutate(id, (tab) => (tab.undrawable ? tab : { ...tab, undrawable: true, pixels: null })), [mutate]);

  const requestClose = useCallback(
    (id: string) => {
      const tab = tabById(id);
      if (!tab) return;
      if (tab.modified) setDialog({ kind: "close", id });
      else dropTab(id, true);
    },
    [dropTab, tabById],
  );

  const answerClose = useCallback(
    (answer: "save" | "discard" | "cancel") => {
      const current = dialog;
      setDialog(null);
      if (!current || current.kind !== "close" || answer === "cancel") return;
      if (answer === "discard") {
        dropTab(current.id, true);
        return;
      }
      const tab = tabById(current.id);
      if (!tab) return;
      void write(current.id, tab.deleted ? "again" : "check").then((outcome) => {
        if (outcome === "saved") dropTab(current.id, true);
        else if (outcome === "changed" && alive.current) setDialog({ kind: "changed", id: current.id });
      });
    },
    [dialog, dropTab, tabById, write],
  );

  const answerChanged = useCallback(
    (answer: "reload" | "overwrite" | "cancel") => {
      const current = dialog;
      setDialog(null);
      if (!current || current.kind !== "changed" || answer === "cancel") return;
      if (answer === "reload") void readAgain(current.id, false);
      else void write(current.id, "overwrite");
    },
    [dialog, readAgain, write],
  );

  const saveAll = useCallback(async (): Promise<SaveFailure[]> => {
    const failures: SaveFailure[] = [];
    for (const tab of tabsRef.current.filter((candidate) => candidate.modified)) {
      if (!canEdit(tab)) {
        failures.push({ path: tab.path, reason: "the agent is editing this file" });
        continue;
      }
      const outcome = await write(tab.id, tab.deleted ? "again" : "check");
      if (outcome === "saved") continue;
      const reason =
        outcome === "changed"
          ? "the file changed on disk since it was opened"
          : outcome === "gone"
            ? "the file no longer exists"
            : "the file could not be written";
      failures.push({ path: tab.path, reason });
    }
    if (alive.current) {
      for (const failure of failures) {
        const tab = tabAt(failure.path);
        if (tab) mutate(tab.id, (current) => ({ ...current, saveError: failure.reason }));
      }
      const first = failures[0] && tabAt(failures[0].path);
      if (first) show(first.id);
    }
    return failures;
  }, [mutate, show, tabAt, write]);

  const closeAll = useCallback(() => {
    for (const tab of tabsRef.current) dropTab(tab.id, true);
  }, [dropTab]);

  const closeUnder = useCallback(
    (path: string) => {
      for (const tab of tabsRef.current.filter((candidate) => isUnder(candidate.path, path))) dropTab(tab.id, false);
    },
    [dropTab],
  );

  const moveUnder = useCallback((oldPath: string, newPath: string) => {
    const next = tabsRef.current.map((tab) => {
      const path = moved(tab.path, oldPath, newPath);
      return path === null ? tab : { ...tab, path };
    });
    tabsRef.current = next;
    setTabs(next);
  }, []);

  // --- what the backend says ----------------------------------------------------------------

  useEffect(() => {
    const fresh = (event: { projectId: string; generation: number }) => {
      if (event.projectId !== projectId || !alive.current) return false;
      if (event.generation <= generation.current) return false;
      generation.current = event.generation;
      return true;
    };
    const stops = [
      on("file-tab-changed", (event: EventPayloads["file-tab-changed"]) => {
        if (!fresh(event)) return;
        const tab = tabAt(event.path);
        if (!tab) return;
        if (event.change === "deleted") {
          mutate(tab.id, diskDeleted);
          return;
        }
        const outcome = diskChanged(tab);
        mutate(tab.id, () => outcome.tab);
        if (outcome.reload) void readAgain(tab.id, false);
      }),
      on("file-lock-changed", (event: EventPayloads["file-lock-changed"]) => {
        if (!fresh(event)) return;
        const tab = tabAt(event.path);
        if (!tab) return;
        if (event.locked) {
          mutate(tab.id, lockTaken);
          return;
        }
        if (event.movedTo) {
          const target = event.movedTo;
          mutate(tab.id, (current) => ({ ...current, path: target }));
        }
        const released = lockReleased(tabById(tab.id) ?? tab);
        mutate(tab.id, () => released.tab);
        if (released.reload) void readAgain(tab.id, true);
      }),
      on("files-flush", (event) => {
        if (event.projectId !== projectId) return;
        const texts = tabsRef.current.filter((tab) => tab.modified).map((tab) => ({ path: tab.path, text: textOf(tab.id) }));
        flushed.current = Object.fromEntries(texts.map((tab) => [tab.path, tab.text]));
        call("files_flushed", { projectId, tabs: texts }).catch(() => {});
      }),
      on("files-save-failed", (event) => {
        if (event.projectId !== projectId || !alive.current) return;
        // What was saved meanwhile shows no unsaved change, unless it was typed in since it was sent.
        for (const file of event.saved) {
          const tab = tabAt(file.path);
          if (tab && textOf(tab.id) === flushed.current[file.path]) mutate(tab.id, (current) => saved(current, file.hash, false));
        }
        for (const failure of event.failures) {
          const tab = tabAt(failure.path);
          if (tab) mutate(tab.id, (current) => ({ ...current, saveError: failure.reason }));
        }
        const first = event.failures[0] && tabAt(event.failures[0].path);
        if (first) show(first.id);
      }),
      on("files-close-tab", (event) => {
        if (event.projectId !== projectId || !shown || !activeRef.current) return;
        requestClose(activeRef.current);
      }),
    ];
    return () => {
      for (const stop of stops) void stop.then((unlisten) => unlisten());
    };
  }, [mutate, projectId, readAgain, requestClose, shown, show, tabAt, tabById, textOf]);

  // --- what the backend is told ---------------------------------------------------------------

  const unsavedPaths = useMemo(() => tabs.filter((tab) => tab.modified).map((tab) => tab.path), [tabs]);
  const lockedPaths = useMemo(() => new Set(tabs.filter((tab) => tab.locked).map((tab) => tab.path)), [tabs]);
  const unsavedKey = JSON.stringify(unsavedPaths);
  useEffect(() => {
    call("files_unsaved", { projectId, paths: JSON.parse(unsavedKey) as string[] }).catch(() => {});
  }, [projectId, unsavedKey]);
  const claimsClose = shown && tabs.length > 0;
  useEffect(() => {
    call("files_close_key", { projectId, claimed: claimsClose }).catch(() => {});
  }, [projectId, claimsClose]);
  useEffect(
    () => () => {
      call("files_unsaved", { projectId, paths: [] }).catch(() => {});
      call("files_close_key", { projectId, claimed: false }).catch(() => {});
    },
    [projectId],
  );

  // The theme of the window reaches every editor at once.
  useEffect(() => {
    for (const id of [...states.current.keys()]) apply(id, { effects: themeCompartment.reconfigure(editorTheme(theme)) });
  }, [apply, theme]);

  // --- shortcuts, only while the File view is shown ---------------------------------------------

  useEffect(() => {
    if (!shown) return;
    const onKey = (event: KeyboardEvent) => {
      if (document.querySelector('[role="alertdialog"], [role="dialog"]')) return;
      const modifier = event.metaKey || event.ctrlKey;
      const key = event.key.toLowerCase();
      if (modifier && !event.altKey && !event.shiftKey && key === "s") {
        event.preventDefault();
        if (activeRef.current) save(activeRef.current);
      } else if (modifier && !event.altKey && !event.shiftKey && key === "w") {
        event.preventDefault();
        if (activeRef.current) requestClose(activeRef.current);
      } else if (event.ctrlKey && !event.metaKey && !event.altKey && event.key === "Tab") {
        event.preventDefault();
        const all = tabsRef.current;
        if (all.length === 0) return;
        const at = all.findIndex((tab) => tab.id === activeRef.current);
        const step = event.shiftKey ? -1 : 1;
        const next = all[(at + step + all.length) % all.length];
        if (next) show(next.id);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [requestClose, save, shown, show]);

  const attachView = useCallback(
    (next: EditorView | null) => {
      view.current = next;
      viewId.current = null;
      syncView();
    },
    [syncView],
  );

  // The view shows the active tab, once the state of a tab that opens is there.
  useEffect(() => {
    syncView();
  }, [activeId, tabs, syncView]);
  const cursorLine = useCallback((id: string) => {
    const state = id === viewId.current && view.current ? view.current.state : states.current.get(id);
    return state ? state.doc.lineAt(state.selection.main.head).number : null;
  }, []);
  const unsavedUnder = useCallback((path: string) => tabsRef.current.filter((tab) => tab.modified && isUnder(tab.path, path)).map((tab) => tab.path), []);
  const activePath = tabs.find((tab) => tab.id === activeId)?.path ?? null;

  const value = useMemo<FileTabsValue>(
    () => ({
      tabs,
      activeId,
      activePath,
      openFile,
      closeUnder,
      moveUnder,
      unsavedUnder,
      unsavedPaths,
      lockedPaths,
      showTab,
      saveAll,
      closeAll,
      showTabById: show,
      requestClose,
      save,
      overwrite,
      reload,
      keepUnsaved,
      dismissSaveError,
      dialog,
      answerClose,
      answerChanged,
      attachView,
      cursorLine,
      textOf,
      setSvgView,
      toggleActualSize,
      imageDrawn,
      imageFailed,
    }),
    [
      tabs, activeId, activePath, openFile, closeUnder, moveUnder, unsavedUnder, unsavedPaths, lockedPaths, showTab, saveAll, closeAll, show, requestClose,
      save, overwrite, reload, keepUnsaved, dismissSaveError, dialog, answerClose, answerChanged, attachView, cursorLine,
      textOf, setSvgView, toggleActualSize, imageDrawn, imageFailed,
    ],
  );
  return (
    <FileTabsContext.Provider value={value}>
      {children}
    </FileTabsContext.Provider>
  );
}
