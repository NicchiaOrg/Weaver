import { createContext, useContext } from "react";

/**
 * Whether the view a component sits in is the one the window shows. The views stay mounted while
 * another is shown; a dialog that belongs to a hidden view hides with it and shows again, as it
 * was, when the view does.
 */
export const ViewShownContext = createContext(true);

export function useViewShown(): boolean {
  return useContext(ViewShownContext);
}
