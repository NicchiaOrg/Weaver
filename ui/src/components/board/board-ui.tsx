import { createContext, useContext } from "react";

import type { FocusSubject, ViewName } from "@/components/views/focus-model";

/** The views a focus on references can open. */
export type FocusView = ViewName;

/** What a card on the Board can ask of the Board it sits in. */
export interface BoardUi {
  /** Opens the details dialog of a card, by its permanent id. */
  openDetails: (cardId: string) => void;
}

export const BoardUiContext = createContext<BoardUi | null>(null);

export function useBoardUi(): BoardUi {
  const value = useContext(BoardUiContext);
  if (!value) throw new Error("useBoardUi needs a Board");
  return value;
}

/**
 * What the footers and the header of the Board start. The work behind them lives with the agent's
 * jobs and the design request, which provide these functions; a missing one leaves its control
 * enabled and without effect, as the control's conditions do not depend on it.
 */
export interface BoardActions {
  /** `+ Work Item`: opens a new design request. */
  newRequest?: () => void;
  /** `Run all`. */
  runAll?: () => void;
  /** `Scan System`, once confirmed. */
  scanSystem?: () => void;
  /** `Run` on one Task, by its permanent id. */
  runTask?: (taskId: string) => void;
  /**
   * How many steps the focus on these references has in a view. `View on diagram` and `View
   * requirements` exist only when `viewOn` is set and this answers at least 1 for their view.
   * `subject` names what the focus is about when it is a Proposal or a Task.
   */
  focusSteps?: (view: FocusView, refs: readonly string[], subject?: FocusSubject) => number;
  /** Opens a view focused on these references; without `subject` the focus is about the reference. */
  viewOn?: (view: FocusView, refs: readonly string[], subject?: FocusSubject) => void;
}

export const BoardActionsContext = createContext<BoardActions>({});

export function useBoardActions(): BoardActions {
  return useContext(BoardActionsContext);
}
