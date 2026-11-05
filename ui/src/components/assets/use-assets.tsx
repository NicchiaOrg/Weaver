import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import type { Asset } from "@/bindings/Asset";
import type { AssetsSnapshot } from "@/bindings/AssetsSnapshot";
import { call, on } from "@/lib/backend";

export interface AssetsValue {
  /** The assets of the project as the backend last told, in the order of the grid. Empty until it answered. */
  assets: readonly Asset[];
  /** Whether the backend has answered at least once. */
  loaded: boolean;
  /**
   * Applies a list the backend sent along with the result of an action. A list older than the one
   * held (a lower generation) is ignored, so a late answer never takes the view back.
   */
  apply: (snapshot: AssetsSnapshot) => void;
}

const NONE: AssetsValue = { assets: [], loaded: false, apply: () => {} };

const AssetsContext = createContext<AssetsValue>(NONE);

interface Held {
  generation: number;
  assets: readonly Asset[];
  loaded: boolean;
}

/**
 * The assets of a project window: the list the backend holds (`assets_list`), kept current by
 * `assets-changed` (`workspace.ASSETS_CHANGED`). It lives as long as the window, whether or not
 * the Assets view is shown, so what is written in a message box can be recognised against it.
 * The UI holds no other copy of the assets.
 */
export function AssetsProvider({ projectId, children }: { projectId: string; children: ReactNode }) {
  const [held, setHeld] = useState<Held>({ generation: 0, assets: [], loaded: false });

  const apply = useCallback((snapshot: AssetsSnapshot) => {
    setHeld((now) => (now.loaded && snapshot.generation <= now.generation ? now : { generation: snapshot.generation, assets: snapshot.assets, loaded: true }));
  }, []);

  useEffect(() => {
    let current = true;
    const unlisten = on("assets-changed", (changed) => {
      if (current && changed.projectId === projectId) apply(changed);
    });
    // The event goes first, so a change that happens while the list is asked for is not lost: the
    // higher generation wins whichever arrives last.
    call("assets_list", { projectId })
      .then((snapshot) => current && apply(snapshot))
      .catch(() => {});
    return () => {
      current = false;
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [projectId, apply]);

  const value = useMemo<AssetsValue>(() => ({ assets: held.assets, loaded: held.loaded, apply }), [held.assets, held.loaded, apply]);
  return <AssetsContext.Provider value={value}>{children}</AssetsContext.Provider>;
}

/** The assets of the project of this window; nothing, outside an `AssetsProvider`. */
export function useAssets(): AssetsValue {
  return useContext(AssetsContext);
}
