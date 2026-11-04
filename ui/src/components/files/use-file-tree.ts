import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";

import type { FileEntry } from "@/bindings/FileEntry";
import type { TreeUpdate } from "@/bindings/TreeUpdate";
import { useFileTabs } from "@/components/files/file-tabs-context";
import { applyUpdate, fromListings, parentOf, ROOT, visibleRows, without, type Folders, type Row } from "@/components/files/tree-model";
import { call, isAppError, on } from "@/lib/backend";

/** What the tree is doing to a name: making an entry, or renaming one. */
export type Editing = { mode: "create"; parent: string; folder: boolean } | { mode: "rename"; path: string } | null;

function reasonOf(error: unknown, fallback: string): string {
  return isAppError(error) ? error.message : fallback;
}

/**
 * The calls that show and leave the view go one after the other: the backend's view of a window is
 * one state, and `leave` must never overtake the `open` that came before it.
 */
const queues = new Map<string, Promise<unknown>>();
function inOrder<T>(projectId: string, run: () => Promise<T>): Promise<T> {
  const done = (queues.get(projectId) ?? Promise.resolve()).then(run, run);
  queues.set(
    projectId,
    done.catch(() => {}),
  );
  return done;
}

export interface FileTree {
  /** Whether the backend answered `files_open_view`. */
  ready: boolean;
  /** Why the root could not be read, when it could not. */
  failure: string | null;
  rows: Row[];
  folders: Folders;
  selected: string | null;
  select: (path: string | null) => void;
  editing: Editing;
  startRename: (path: string) => void;
  startCreate: (parent: string, folder: boolean) => void;
  stopEditing: () => void;
  isExpanded: (path: string) => boolean;
  expand: (path: string) => void;
  collapse: (path: string) => void;
  toggle: (path: string) => void;
  entryOf: (path: string) => FileEntry | null;
  /** Each action resolves `true` when it was done, and shows the reason in a toast when it was refused. */
  create: (parent: string, name: string, folder: boolean) => Promise<boolean>;
  rename: (path: string, name: string) => Promise<boolean>;
  move: (path: string, target: string) => Promise<boolean>;
  remove: (path: string) => Promise<boolean>;
  /**
   * `developer.DROP_EXTERNAL_ENTRIES`: copies the dropped `paths` into the folder `target` (empty:
   * the root). A loading toast says `Copying <n> items…` until the copy ends; a refusal or a
   * failure turns it into the error toast. When it worked and the view is still shown, the folder
   * expands if it was collapsed and the first copied entry is selected and revealed.
   */
  copyIn: (target: string, paths: string[]) => Promise<void>;
  /** The path of an entry to scroll into view once its row is on screen. */
  reveal: string | null;
  clearReveal: () => void;
}

/**
 * The tree of the File view of a window: the folders the backend lists (it holds them; the UI
 * renders what it sends), which are expanded (the ones listed), the selected row and the name
 * being edited. Showing the view asks the backend to open it; leaving it (this hook ends) tells the
 * backend, which keeps the tree for 10 s.
 */
export function useFileTree(projectId: string): FileTree {
  const tabs = useFileTabs();
  const tabsRef = useRef(tabs);
  tabsRef.current = tabs;
  const [folders, setFolders] = useState<Folders>({});
  const foldersRef = useRef<Folders>({});
  const [ready, setReady] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [editing, setEditing] = useState<Editing>(null);
  const [reveal, setReveal] = useState<string | null>(null);
  const alive = useRef(true);
  const expanding = useRef(new Set<string>());

  const commit = useCallback((next: Folders) => {
    foldersRef.current = next;
    setFolders(next);
  }, []);

  const apply = useCallback(
    (update: TreeUpdate) => {
      if (alive.current) commit(applyUpdate(foldersRef.current, update));
    },
    [commit],
  );

  useEffect(() => {
    alive.current = true;
    let current = true;
    inOrder(projectId, () => call("files_open_view", { projectId }))
      .then((snapshot) => {
        if (!current) return;
        commit(fromListings(snapshot.folders, snapshot.generation));
        setReady(true);
      })
      .catch((error: unknown) => {
        if (current) setFailure(reasonOf(error, "The project's files could not be read."));
      });
    const unlisten = on("files-tree-changed", (changed) => {
      if (current && changed.projectId === projectId) apply(changed);
    });
    return () => {
      current = false;
      alive.current = false;
      unlisten.then((stop) => stop()).catch(() => {});
      inOrder(projectId, () => call("files_leave", { projectId })).catch(() => {});
    };
  }, [projectId, apply, commit]);

  const rows = useMemo(
    () => visibleRows(folders, editing?.mode === "create" ? { parent: editing.parent, folder: editing.folder } : null),
    [folders, editing],
  );

  const isExpanded = useCallback((path: string) => folders[path] !== undefined, [folders]);

  const expand = useCallback(
    (path: string) => {
      if (foldersRef.current[path] || expanding.current.has(path)) return;
      expanding.current.add(path);
      call("files_expand", { projectId, path })
        .then(apply)
        .catch((error: unknown) => toast.error(reasonOf(error, "The folder could not be read.")))
        .finally(() => expanding.current.delete(path));
    },
    [projectId, apply],
  );

  const collapse = useCallback(
    (path: string) => {
      if (path === ROOT || !foldersRef.current[path]) return;
      commit(without(foldersRef.current, path));
      call("files_collapse", { projectId, path }).catch(() => {});
    },
    [projectId, commit],
  );

  const toggle = useCallback((path: string) => (foldersRef.current[path] ? collapse(path) : expand(path)), [collapse, expand]);

  const entryOf = useCallback(
    (path: string) => folders[parentOf(path)]?.entries.find((entry) => entry.path === path) ?? null,
    [folders],
  );

  const act = useCallback(async <T extends { update: TreeUpdate }>(run: () => Promise<T>, fallback: string): Promise<T | null> => {
    try {
      const result = await run();
      apply(result.update);
      return result;
    } catch (error) {
      toast.error(reasonOf(error, fallback));
      return null;
    }
  }, [apply]);

  const create = useCallback(
    async (parent: string, name: string, folder: boolean) => {
      const made = await act(() => call("files_create", { projectId, parent, name, folder }), "The entry could not be created.");
      if (made) setSelected(made.path);
      return made !== null;
    },
    [projectId, act],
  );

  const rename = useCallback(
    async (path: string, name: string) => {
      const done = await act(() => call("files_rename", { projectId, path, name }), "The entry could not be renamed.");
      if (!done) return false;
      if (done.path !== path) tabsRef.current.moveUnder(path, done.path);
      setSelected((now) => (now === path ? done.path : now));
      return true;
    },
    [projectId, act],
  );

  const move = useCallback(
    async (path: string, target: string) => {
      const done = await act(() => call("files_move", { projectId, path, target }), "The entry could not be moved.");
      if (!done) return false;
      tabsRef.current.moveUnder(path, done.path);
      setSelected((now) => (now === path ? done.path : now));
      return true;
    },
    [projectId, act],
  );

  const remove = useCallback(
    async (path: string) => {
      try {
        const update = await call("files_delete", { projectId, path });
        tabsRef.current.closeUnder(path);
        apply(update);
        setSelected((now) => (now !== null && (now === path || now.startsWith(`${path}/`)) ? null : now));
        return true;
      } catch (error) {
        toast.error(reasonOf(error, "The entry could not be moved to the trash."));
        return false;
      }
    },
    [projectId, apply],
  );

  const copyIn = useCallback(
    async (target: string, paths: string[]) => {
      const toastId = toast.loading(`Copying ${paths.length} ${paths.length === 1 ? "item" : "items"}…`);
      try {
        const done = await call("files_copy_in", { projectId, target, paths });
        toast.dismiss(toastId);
        // The copy goes to its end even when the view was left; what follows is for a view still shown.
        if (!alive.current) return;
        apply(done.update);
        if (target !== ROOT && !foldersRef.current[target]) expand(target);
        const first = done.paths[0];
        if (first !== undefined) {
          setSelected(first);
          setReveal(first);
        }
      } catch (error) {
        toast.error(reasonOf(error, "The dropped items could not be copied."), { id: toastId });
      }
    },
    [projectId, apply, expand],
  );

  const startCreate = useCallback(
    (parent: string, folder: boolean) => {
      expand(parent);
      setEditing({ mode: "create", parent, folder });
    },
    [expand],
  );

  return {
    ready,
    failure,
    rows,
    folders,
    selected,
    select: setSelected,
    editing,
    startRename: (path) => setEditing({ mode: "rename", path }),
    startCreate,
    stopEditing: () => setEditing(null),
    isExpanded,
    expand,
    collapse,
    toggle,
    entryOf,
    create,
    rename,
    move,
    remove,
    copyIn,
    reveal,
    clearReveal: () => setReveal(null),
  };
}
