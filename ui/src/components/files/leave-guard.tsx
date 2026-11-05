import { createContext, useContext, useEffect, useRef } from "react";

/**
 * Asks to leave the File view for another view: the guard calls `proceed` when the window may show
 * the other view (the tabs closed, after the developer chose what to do with unsaved changes), and
 * never when the developer cancelled or a file could not be saved.
 */
export type LeaveGuard = (proceed: () => void) => void;

/** What the window gives the File view to register its guard with. */
export interface LeaveGuardSlot {
  set: (guard: LeaveGuard | null) => void;
}

export const LeaveGuardContext = createContext<LeaveGuardSlot | null>(null);

/**
 * Registers the guard of the File view with its window while the view exists. The latest `guard`
 * is the one called, whatever render registered it.
 */
export function useLeaveGuard(guard: LeaveGuard) {
  const slot = useContext(LeaveGuardContext);
  const latest = useRef(guard);
  latest.current = guard;
  useEffect(() => {
    slot?.set((proceed) => latest.current(proceed));
    return () => slot?.set(null);
  }, [slot]);
}
