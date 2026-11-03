import { useEffect, useRef } from "react";

import { useAsks } from "@/components/ask/use-asks";
import { useProject } from "@/components/project-context";

/**
 * Shows the Jobs view whenever a new Ask more conversation opens; the view the developer was
 * reading (dialog, overlay, focus, scroll) stays mounted and is found again as it was.
 */
export function AskViewSwitch() {
  const { opened } = useAsks();
  const { showView } = useProject();
  const seen = useRef(opened?.nonce ?? 0);
  useEffect(() => {
    if (!opened || opened.nonce === seen.current) return;
    seen.current = opened.nonce;
    showView("jobs");
  }, [opened, showView]);
  return null;
}
