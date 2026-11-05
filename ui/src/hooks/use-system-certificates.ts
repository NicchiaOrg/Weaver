import { useCallback, useEffect, useRef, useState } from "react";

import type { SystemCertificatesStatus } from "@/bindings/SystemCertificatesStatus";
import { call, on } from "@/lib/backend";

/**
 * The status of System Certificates as the backend holds it: read at load and followed with
 * `system-certificates-changed`; the answer with the highest revision wins. The interface never
 * reads the bundle or the certificates. `null` until the backend answered.
 */
export function useSystemCertificates() {
  const [status, setStatus] = useState<SystemCertificatesStatus | null>(null);
  const revision = useRef(-1);
  const mounted = useRef(true);

  const accept = useCallback((incoming: SystemCertificatesStatus) => {
    if (!mounted.current || incoming.revision < revision.current) return;
    revision.current = incoming.revision;
    setStatus(incoming);
  }, []);

  useEffect(() => {
    mounted.current = true;
    // Subscribe first, then ask: a change between the two is not lost.
    const unlisten = on("system-certificates-changed", accept);
    call("system_certificates_status")
      .then(accept)
      .catch(() => {});
    return () => {
      mounted.current = false;
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [accept]);

  /** `Refresh`: the backend goes to Exporting and tells every window; a refusal leaves the line as it is. */
  const refresh = useCallback(() => {
    call("system_certificates_refresh")
      .then(accept)
      .catch(() => {});
  }, [accept]);

  /** Closes the turn-on notice in every window: the backend holds it and tells them all. */
  const dismissNotice = useCallback(() => {
    call("system_certificates_dismiss_notice")
      .then(accept)
      .catch(() => {});
  }, [accept]);

  /** Closes the banner of a failed automatic export in every window. */
  const dismissFailure = useCallback(() => {
    call("system_certificates_dismiss_failure")
      .then(accept)
      .catch(() => {});
  }, [accept]);

  return { status, refresh, dismissNotice, dismissFailure };
}
