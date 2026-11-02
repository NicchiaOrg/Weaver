import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { toast } from "sonner";

import type { RequestsSnapshot } from "@/bindings/RequestsSnapshot";
import { localTimeOf } from "@/components/requests/request-model";
import { call, isAppError, on } from "@/lib/backend";

/** What the editor of design requests shows and asks of the backend, for one project window. */
export interface RequestsState {
  /** The open requests as the backend lists them; `null` until the first answer. */
  snapshot: RequestsSnapshot | null;
  /** Grows with every `requests-changed`: an editor with a request open reads it again. */
  changes: number;
  /** The editor is shown (in place of the Board). */
  editing: boolean;
  /** The request shown in the editor. */
  selected: string | null;
  /** `developer.NEW_REQUEST`: creates an empty request and opens it for writing (`+ Work Item`, Start from Scratch). */
  newRequest: () => void;
  /** Shows the editor on a request (the first one when none is named). */
  openEditor: (requestId?: string) => void;
  /** Back to the Board; nothing is sent. */
  closeEditor: () => void;
  select: (requestId: string) => void;
  /** The pane that holds the text being written says what to save before the window closes. */
  setFlusher: (flush: (() => Promise<void>) | null) => void;
}

const RequestsContext = createContext<RequestsState | null>(null);

export function useRequests(): RequestsState {
  const value = useContext(RequestsContext);
  if (!value) throw new Error("useRequests needs a RequestsProvider");
  return value;
}

export function RequestsProvider({ projectId, children }: { projectId: string; children: ReactNode }) {
  const [snapshot, setSnapshot] = useState<RequestsSnapshot | null>(null);
  const [changes, setChanges] = useState(0);
  const [editing, setEditing] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const alive = useRef(true);
  const flusher = useRef<(() => Promise<void>) | null>(null);
  const latest = useRef<RequestsSnapshot | null>(null);

  const refresh = useCallback(() => {
    call("requests_list", { projectId })
      .then((next) => {
        if (!alive.current) return;
        latest.current = next;
        setSnapshot(next);
      })
      .catch(() => {});
  }, [projectId]);

  useEffect(() => {
    alive.current = true;
    // Listen first, then ask: a change between the two is not lost.
    const changed = on("requests-changed", (event) => {
      if (!alive.current || event.projectId !== projectId) return;
      setChanges((n) => n + 1);
      refresh();
    });
    const flush = on("requests-flush", (event) => {
      if (!alive.current || event.projectId !== projectId) return;
      // The window is closing: what is pending is saved, never sent; then the backend goes on.
      Promise.resolve(flusher.current?.())
        .catch(() => {})
        .finally(() => {
          call("requests_flushed", { projectId }).catch(() => {});
        });
    });
    refresh();
    return () => {
      alive.current = false;
      for (const stop of [changed, flush]) stop.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [projectId, refresh]);

  const newRequest = useCallback(() => {
    // The editor opens when the list already holds the new request.
    call("request_new", { projectId, localTime: localTimeOf(new Date()) })
      .then(async (id) => {
        const next = await call("requests_list", { projectId }).catch(() => null);
        if (!alive.current) return;
        if (next) {
          latest.current = next;
          setSnapshot(next);
        }
        setSelected(id);
        setEditing(true);
      })
      .catch((error: unknown) => {
        toast.error(isAppError(error) ? error.message : "The request could not be created.");
      });
  }, [projectId]);

  const openEditor = useCallback((requestId?: string) => {
    const first = latest.current?.requests[0]?.id ?? null;
    setSelected((current) => requestId ?? current ?? first);
    setEditing(true);
  }, []);

  const closeEditor = useCallback(() => setEditing(false), []);
  const setFlusher = useCallback((flush: (() => Promise<void>) | null) => {
    flusher.current = flush;
  }, []);

  const value = useMemo<RequestsState>(
    () => ({ snapshot, changes, editing, selected, newRequest, openEditor, closeEditor, select: setSelected, setFlusher }),
    [snapshot, changes, editing, selected, newRequest, openEditor, closeEditor, setFlusher],
  );
  return <RequestsContext.Provider value={value}>{children}</RequestsContext.Provider>;
}
