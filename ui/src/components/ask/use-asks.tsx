import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import type { AskRow } from "@/bindings/AskRow";
import type { AskTarget } from "@/bindings/AskTarget";
import { call, on } from "@/lib/backend";

/** The Ask more conversations of a project as the backend holds them. */
export interface AsksState {
  /** The most recent first. */
  asks: readonly AskRow[];
  /** The conversation opened last, for the Jobs view to select and focus; `nonce` grows with every opening. */
  opened: { id: string; nonce: number } | null;
  /**
   * `Ask more` on a thing: the backend reads it and opens a new conversation. Resolves to its
   * identity, or `null` when the thing no longer exists (nothing opens and nothing changes).
   */
  askMore: (target: AskTarget) => Promise<string | null>;
  /** Closes a conversation (the confirmation, when its agent works, is the caller's). */
  close: (askId: string) => void;
}

const AsksContext = createContext<AsksState | null>(null);

export function useAsks(): AsksState {
  const value = useContext(AsksContext);
  if (!value) throw new Error("useAsks needs an AsksProvider");
  return value;
}

/** The conversations when the window has them; views that also render without them (the question round) use this. */
export function useOptionalAsks(): AsksState | null {
  return useContext(AsksContext);
}

/** The conversations of a project: the list, then the rows and closings the backend announces. */
export function AsksProvider({ projectId, children }: { projectId: string; children: ReactNode }) {
  const [asks, setAsks] = useState<readonly AskRow[]>([]);
  const [opened, setOpened] = useState<AsksState["opened"]>(null);
  const alive = useRef(true);
  const nonce = useRef(0);

  useEffect(() => {
    alive.current = true;
    // Listen first, then ask: a change between the two is not lost.
    const changed = on("ask-changed", (row) => {
      if (!alive.current || row.projectId !== projectId) return;
      setAsks((previous) =>
        previous.some((ask) => ask.id === row.id) ? previous.map((ask) => (ask.id === row.id ? row : ask)) : [row, ...previous],
      );
    });
    const closed = on("ask-closed", (event) => {
      if (!alive.current || event.projectId !== projectId) return;
      setAsks((previous) => previous.filter((ask) => ask.id !== event.id));
    });
    call("ask_list", { projectId })
      .then((snapshot) => {
        if (!alive.current) return;
        // What an event already told stays; the list fills in the rest.
        setAsks((previous) => [...previous, ...snapshot.asks.filter((ask) => !previous.some((known) => known.id === ask.id))]);
      })
      .catch(() => {});
    return () => {
      alive.current = false;
      changed.then((unlisten) => unlisten()).catch(() => {});
      closed.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [projectId]);

  const askMore = useCallback(
    async (target: AskTarget) => {
      try {
        const id = await call("ask_open", { projectId, target });
        nonce.current += 1;
        if (alive.current) setOpened({ id, nonce: nonce.current });
        return id;
      } catch {
        return null;
      }
    },
    [projectId],
  );
  const close = useCallback((askId: string) => void call("ask_close", { askId }).catch(() => {}), []);

  const value = useMemo<AsksState>(() => ({ asks, opened, askMore, close }), [asks, opened, askMore, close]);
  return <AsksContext.Provider value={value}>{children}</AsksContext.Provider>;
}
