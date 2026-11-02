import { useCallback, useEffect, useRef, useState } from "react";

import type { HistorySnapshot } from "@/bindings/HistorySnapshot";
import { call, isAppError, on } from "@/lib/backend";

export interface HistoryState {
  /** The History as the backend worked it out; `null` until the first answer. */
  snapshot: HistorySnapshot | null;
  /** Why the snapshot could not be read. */
  loadError: string | null;
  /** Asks again after a failed load. */
  reload: () => void;
}

/**
 * The History of one project, as the backend holds it. The UI asks for a snapshot when the view
 * opens and again whenever a `project-changed` event names the `history` slice (or shows a gap in
 * the revisions), and drops an answer older than the newest one it already shows.
 */
export function useHistory(projectId: string): HistoryState {
  const [snapshot, setSnapshot] = useState<HistorySnapshot | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const shown = useRef(-1);
  const seen = useRef(-1);
  const alive = useRef(true);

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  const fetchHistory = useCallback(() => {
    call("history_snapshot", { projectId })
      .then((incoming) => {
        if (!alive.current || incoming.projectId !== projectId || incoming.revision < shown.current) return;
        shown.current = incoming.revision;
        seen.current = Math.max(seen.current, incoming.revision);
        setSnapshot(incoming);
        setLoadError(null);
      })
      .catch((error: unknown) => {
        if (alive.current) setLoadError(isAppError(error) ? error.message : "The History could not be read.");
      });
  }, [projectId]);

  useEffect(() => {
    // Listen first, then ask: a change between the two is not lost.
    const stop = on("project-changed", (event) => {
      if (event.projectId !== projectId) return;
      const gap = seen.current >= 0 && event.revision > seen.current + 1;
      seen.current = Math.max(seen.current, event.revision);
      if (gap || event.slices.includes("history")) fetchHistory();
    });
    fetchHistory();
    return () => {
      stop.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [projectId, fetchHistory, attempt]);

  const reload = useCallback(() => setAttempt((n) => n + 1), []);
  return { snapshot, loadError, reload };
}
