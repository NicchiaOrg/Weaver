import { createContext, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import type { OmpStatus } from "@/bindings/OmpStatus";
import { call, on } from "@/lib/backend";

export interface OmpContextValue {
  /** The status of omp as the backend last sent it; `null` until the backend answered. */
  status: OmpStatus | null;
}

const OmpContext = createContext<OmpContextValue | null>(null);

/**
 * The readiness of omp (the Machine `omp-setup`, one for the whole app). The UI keeps no state of
 * its own: the status arrives whole from `omp_status` and from the `omp-changed` event, and the
 * one with the highest revision wins.
 */
export function OmpProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<OmpStatus | null>(null);
  const revision = useRef(-1);

  useEffect(() => {
    let current = true;
    const accept = (incoming: OmpStatus) => {
      if (!current || incoming.revision < revision.current) return;
      revision.current = incoming.revision;
      setStatus(incoming);
    };
    // Listen first, then ask: a change between the two is not lost.
    const unlisten = on("omp-changed", accept);
    call("omp_status")
      .then(accept)
      .catch(() => {});
    return () => {
      current = false;
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);

  const value = useMemo(() => ({ status }), [status]);
  return <OmpContext.Provider value={value}>{children}</OmpContext.Provider>;
}

export function useOmp(): OmpContextValue {
  const value = useContext(OmpContext);
  if (!value) throw new Error("useOmp needs an OmpProvider");
  return value;
}
