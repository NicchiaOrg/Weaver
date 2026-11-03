import { useCallback, useEffect, useRef, useState } from "react";

import type { UsageStatus } from "@/bindings/UsageStatus";
import { call, on } from "@/lib/backend";

export interface UsageReading {
  /** The status of usage as the backend last sent it; `null` until it did. */
  status: UsageStatus | null;
  /**
   * `status` once it can answer the reading this window asked for last: a reading that runs, or
   * any status newer than the request. `null` before: the status the window holds is from before
   * the request (a failure the developer is retrying) and says nothing of the new reading.
   */
  current: UsageStatus | null;
  /** `developer.OPEN_USAGE` / `developer.RETRY_USAGE`: asks the backend to read (or join the reading that runs). */
  read: () => void;
}

/**
 * The usage limits of the connected providers, for the Usage dialog. The UI keeps no copy of its
 * own: the whole status arrives from the `usage-changed` event and from the end of `omp_usage`,
 * and the one with the highest revision wins. While `ompReady`, a reading starts when this is first
 * used and each time omp becomes ready again (a dialog that waited for omp reads by itself); never
 * on a timer.
 */
export function useUsage(ompReady: boolean): UsageReading {
  const [status, setStatus] = useState<UsageStatus | null>(null);
  const [since, setSince] = useState(-1);
  const latest = useRef(-1);
  const mounted = useRef(true);

  const apply = useCallback((incoming: UsageStatus) => {
    if (!mounted.current || incoming.revision < latest.current) return;
    latest.current = incoming.revision;
    setStatus(incoming);
  }, []);

  useEffect(() => {
    mounted.current = true;
    const unlisten = on("usage-changed", apply);
    return () => {
      mounted.current = false;
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [apply]);

  const read = useCallback(() => {
    setSince(latest.current);
    call("omp_usage")
      .then(apply)
      .catch(() => {});
  }, [apply]);

  useEffect(() => {
    if (ompReady) read();
  }, [ompReady, read]);

  const current = status && (status.revision > since || status.phase === "reading") ? status : null;
  return { status, current, read };
}
