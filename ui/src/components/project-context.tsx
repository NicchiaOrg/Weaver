import { createContext, useContext } from "react";

import type { ViewId } from "@/views";

/** What a view may ask of its project window. */
export interface ProjectContextValue {
  /** The project hash (the window's label is `p-<hash>`). */
  projectId: string;
  /** Shows another view of the window (the sidebar entry becomes the selected one). */
  showView: (view: ViewId) => void;
  /** Opens the conversation of a job in Jobs: shows that view and names the job to open. */
  openJob: (jobId: string) => void;
  /** The job a view asked to open last, for the Jobs view to select; `null` until one was. */
  openedJob: string | null;
}

export const ProjectContext = createContext<ProjectContextValue | null>(null);

export function useProject(): ProjectContextValue {
  const value = useContext(ProjectContext);
  if (!value) throw new Error("useProject needs a ProjectWindow");
  return value;
}
