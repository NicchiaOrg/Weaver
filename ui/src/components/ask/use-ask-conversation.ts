import { useEffect, useRef, useState } from "react";

import type { Entry } from "@/bindings/Entry";
import { call, on } from "@/lib/backend";

/** The conversation of one Ask more conversation: the snapshot, then the batches of changes that follow it. */
export function useAskConversation(askId: string): readonly Entry[] {
  const [entries, setEntries] = useState<readonly Entry[]>([]);
  const seq = useRef(0);

  useEffect(() => {
    setEntries([]);
    seq.current = 0;
    let alive = true;
    const fetchSnapshot = () =>
      call("ask_snapshot", { askId })
        .then((snapshot) => {
          if (!alive || snapshot.row.id !== askId) return;
          seq.current = snapshot.seq;
          setEntries(snapshot.entries);
        })
        .catch(() => {});
    // Listen first, then ask: a change between the two is not lost.
    const stop = on("ask-entries", (event) => {
      if (!alive || event.id !== askId) return;
      if (event.reset || event.seq > seq.current + 1) {
        void fetchSnapshot();
        return;
      }
      if (event.seq <= seq.current) return;
      seq.current = event.seq;
      setEntries((previous) => {
        const next = [...previous];
        for (const { index, entry } of event.changes) next[index] = entry;
        return next;
      });
    });
    void fetchSnapshot();
    return () => {
      alive = false;
      stop.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [askId]);

  return entries;
}
