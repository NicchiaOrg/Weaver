import { useMemo, type ReactNode } from "react";

import { BoardActionsContext, useBoardActions } from "@/components/board/board-ui";
import { useJobs } from "@/components/jobs/use-jobs";

/**
 * Adds what the jobs start (`Run`, `Run all`, `Scan System`) to the actions the Board already has,
 * keeping the ones other parts of the window provide. A refusal of the backend shows as a notice
 * at the top of the Board.
 */
export function JobsBoardActions({ children }: { children: ReactNode }) {
  const parent = useBoardActions();
  const { runTask, runAll, scanSystem } = useJobs();
  const actions = useMemo(() => ({ ...parent, runTask, runAll, scanSystem }), [parent, runTask, runAll, scanSystem]);
  return <BoardActionsContext.Provider value={actions}>{children}</BoardActionsContext.Provider>;
}
