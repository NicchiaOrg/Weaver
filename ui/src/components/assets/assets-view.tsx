import type { DragDropEvent } from "@tauri-apps/api/webview";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";

import type { AssetRefusal } from "@/bindings/AssetRefusal";
import { emptyText, inGridOrder } from "@/components/assets/asset-kinds";
import { AssetGrid } from "@/components/assets/asset-grid";
import { NOTHING, only, prune, selectNames, without, type Selection } from "@/components/assets/selection";
import { useAssets } from "@/components/assets/use-assets";
import { ConfirmDialog } from "@/components/dialogs";
import { useProject } from "@/components/project-context";
import { ViewHeader } from "@/components/view-header";
import { useViewShown } from "@/components/view-shown";
import { call, isAppError, onDragDrop } from "@/lib/backend";

/** A copy that ends sooner than this shows no toast: a drop of the project's own assets, which does nothing, shows nothing. */
export const COPY_TOAST_AFTER_MS = 150;

/** The reason a refused action shows in its error toast (the backend's sentence), or `fallback`. */
function reasonOf(error: unknown, fallback: string): string {
  return isAppError(error) ? error.message : fallback;
}

const reportRefusal = (error: unknown, fallback: string) => toast.error(reasonOf(error, fallback));

/** The toast text for the assets the trash refused: the one asset, or how many and the reasons. */
export function trashFailure(refused: readonly AssetRefusal[]): string {
  const [first] = refused;
  if (refused.length === 1 && first) return `Could not move "${first.name}" to the trash: ${first.reason}`;
  const reasons = [...new Set(refused.map((one) => one.reason))].join("; ");
  return `Could not move ${refused.length} assets to the trash: ${reasons}`;
}

/** The warning after a copy that brought assets over the size limit. */
export function largeWarning(large: readonly string[]): string {
  const [first] = large;
  if (large.length === 1 && first) return `"${first}" is larger than 10 MB: it goes into the repository.`;
  return `${large.length} copied files are larger than 10 MB: they go into the repository.`;
}

/**
 * The Assets view (`Assets` in the sidebar, the Machine `asset-view`): the project's assets as a
 * grid of icons. Files dragged in from the Finder or Explorer are copied into `.weaver/assets/`;
 * an asset is renamed, deleted to the system's trash after a confirmation, or dragged out to
 * another app, which receives copies. The list is the backend's (`assets-changed`); the view holds
 * only the selection, the name being edited and what a dialog is asking, and forgets all of it when
 * another view is chosen. A copy still running goes on to its end.
 */
export function AssetsView() {
  const { projectId } = useProject();
  const shown = useViewShown();
  const { assets, loaded, apply } = useAssets();
  const grid = useMemo(() => inGridOrder(assets), [assets]);
  const order = useMemo(() => grid.map((asset) => asset.name), [grid]);

  const [selection, setSelection] = useState<Selection>(NOTHING);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string[] | null>(null);
  const [reveal, setReveal] = useState<string | null>(null);
  const [dropActive, setDropActive] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const scroller = useRef<HTMLDivElement>(null);
  const scrolled = useRef(0);
  /** Where the focus goes back to when the dialog closes: the active tile. */
  const focusTarget = useRef<HTMLElement | null>(null);
  const shownNow = useRef(shown);
  shownNow.current = shown;

  // An asset that is gone leaves the selection, and the name being edited.
  useEffect(() => {
    setSelection((now) => prune(now, order));
    setRenaming((now) => (now !== null && order.includes(now) ? now : null));
  }, [order]);

  // `developer.LEAVE_ASSETS_VIEW`: the selection is forgotten. The grid's scroll is kept.
  useEffect(() => {
    if (shown) {
      if (scroller.current) scroller.current.scrollTop = scrolled.current;
      return;
    }
    setSelection(NOTHING);
    setRenaming(null);
    setDeleting(null);
    setDropActive(false);
  }, [shown]);

  // --- developer.DROP_ASSETS ------------------------------------------------------------------

  const copyIn = async (paths: string[]) => {
    const count = paths.length;
    let toastId: string | number | undefined;
    // The toast says the copy is running; a copy that ends at once (or does nothing) shows none.
    const timer = window.setTimeout(() => {
      toastId = toast.loading(`Copying ${count} ${count === 1 ? "file" : "files"}…`);
    }, COPY_TOAST_AFTER_MS);
    try {
      const done = await call("assets_copy_in", { projectId, paths });
      window.clearTimeout(timer);
      if (toastId !== undefined) toast.dismiss(toastId);
      apply(done.snapshot);
      if (done.copied.length === 0) return;
      // The copy goes to its end even when the view was left; what follows is for a view still shown.
      if (shownNow.current) {
        const next = selectNames(done.copied, inGridOrder(done.snapshot.assets).map((asset) => asset.name));
        setSelection(next);
        setReveal(next.active);
      }
      if (done.large.length > 0) toast.warning(largeWarning(done.large));
    } catch (error) {
      window.clearTimeout(timer);
      const message = reasonOf(error, "The dropped files could not be copied.");
      if (toastId !== undefined) toast.error(message, { id: toastId });
      else toast.error(message);
    }
  };

  const dropped = (event: DragDropEvent) => {
    if (event.type === "leave") {
      setDropActive(false);
      return;
    }
    const ratio = window.devicePixelRatio || 1;
    const element = document.elementFromPoint(event.position.x / ratio, event.position.y / ratio);
    const inside = element !== null && root.current !== null && root.current.contains(element);
    if (event.type === "drop") {
      setDropActive(false);
      if (inside && event.paths.length > 0) void copyIn(event.paths);
      return;
    }
    setDropActive(inside);
  };
  const droppedNow = useRef(dropped);
  droppedNow.current = dropped;

  // Files dragged in from outside arrive as events of Tauri's drag and drop handler; they are
  // listened to only while the view is shown.
  useEffect(() => {
    if (!shown) return;
    let listening = true;
    const stop = onDragDrop((event) => {
      if (listening) droppedNow.current(event);
    });
    return () => {
      listening = false;
      setDropActive(false);
      stop.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [shown]);

  // --- developer.RENAME_ASSET -----------------------------------------------------------------

  const commitRename = async (name: string, newName: string): Promise<boolean> => {
    if (newName === name) {
      setRenaming(null);
      return true;
    }
    try {
      const done = await call("assets_rename", { projectId, name, newName });
      apply(done.snapshot);
      setSelection(only(done.name));
      setReveal(done.name);
      setRenaming(null);
      return true;
    } catch (error) {
      reportRefusal(error, "The asset could not be renamed.");
      return false;
    }
  };

  // --- developer.DELETE_ASSETS ----------------------------------------------------------------

  const confirmDelete = async (names: string[]) => {
    try {
      const done = await call("assets_delete", { projectId, names });
      apply(done.snapshot);
      setSelection((now) => without(now, done.trashed));
      if (done.refused.length > 0) toast.error(trashFailure(done.refused));
    } catch (error) {
      reportRefusal(error, "The assets could not be moved to the trash.");
    }
  };

  const lastDeleting = useRef<string[]>([]);
  if (deleting !== null) lastDeleting.current = deleting;
  const asked = deleting ?? lastDeleting.current;

  // --- developer.DRAG_ASSETS_OUT --------------------------------------------------------------

  const dragOut = useCallback(
    (names: string[]) => {
      call("assets_drag_out", { projectId, names }).catch((error: unknown) => reportRefusal(error, "The assets could not be dragged out."));
    },
    [projectId],
  );

  return (
    <div ref={root} className="flex h-full min-h-0 flex-col">
      <ViewHeader view="assets" />
      <div className="relative min-h-0 flex-1" data-drop-target={dropActive ? "" : undefined}>
        <div
          ref={scroller}
          onScroll={(event) => {
            if (shownNow.current) scrolled.current = event.currentTarget.scrollTop;
          }}
          className="h-full overflow-auto"
        >
          {loaded && grid.length === 0 ? (
            <div className="flex h-full items-center justify-center p-4">
              <p className="text-center text-sm text-muted-foreground">{emptyText()}</p>
            </div>
          ) : (
            <AssetGrid
              assets={grid}
              selection={selection}
              onSelection={setSelection}
              renaming={renaming}
              onRenameRequest={setRenaming}
              onRenameCommit={commitRename}
              onRenameCancel={() => setRenaming(null)}
              onDeleteRequest={() => selection.selected.length > 0 && setDeleting([...selection.selected])}
              onDragOut={dragOut}
              reveal={reveal}
              onRevealed={() => setReveal(null)}
              focusRef={focusTarget}
            />
          )}
        </div>
        {dropActive ? <div aria-hidden className="pointer-events-none absolute inset-1 rounded-md border-2 border-dashed border-ring" /> : null}
      </div>
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={asked.length === 1 ? `Move "${asked[0]}" to the trash?` : `Move ${asked.length} assets to the trash?`}
        description={asked.length === 1 ? "It goes to the system's trash." : "They go to the system's trash."}
        confirmLabel="Move to Trash"
        destructive
        onConfirm={() => deleting !== null && void confirmDelete(deleting)}
        returnFocus={focusTarget}
      >
        {asked.length > 1 ? (
          <ul className="max-h-40 list-disc overflow-auto pl-5 text-sm">
            {asked.map((name) => (
              <li key={name} className="[overflow-wrap:anywhere]">
                {name}
              </li>
            ))}
          </ul>
        ) : null}
      </ConfirmDialog>
    </div>
  );
}
