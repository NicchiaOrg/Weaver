import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import type { ProjectStatus } from "@/bindings/ProjectStatus";
import { call, on } from "@/lib/backend";

/** What a project window knows of the Machine `project`: its state, the files it installs, the opening check. */
export interface ProjectStatusState {
  projectId: string;
  /** The status as the backend last sent it; `null` until the first answer. */
  status: ProjectStatus | null;
  /** The first answer could not be read (the window then shows the views as they are). */
  failed: boolean;
  /** Asks the backend again (after `Initialise`); an answer older than the one shown is dropped. */
  refresh: () => void;
}

const ProjectStatusContext = createContext<ProjectStatusState | null>(null);

export function useProjectStatus(): ProjectStatusState {
  const value = useContext(ProjectStatusContext);
  if (!value) throw new Error("useProjectStatus needs a ProjectStatusProvider");
  return value;
}

/**
 * The status of the project of this window. The UI keeps no state of its own: the status arrives
 * whole from `project_status` and from `project-state`, and the one with the highest revision wins.
 */
export function ProjectStatusProvider({ projectId, children }: { projectId: string; children: ReactNode }) {
  const [status, setStatus] = useState<ProjectStatus | null>(null);
  const [failed, setFailed] = useState(false);
  const revision = useRef(-1);
  const alive = useRef(true);

  const accept = useCallback(
    (incoming: ProjectStatus) => {
      if (!alive.current || incoming.projectId !== projectId || incoming.revision < revision.current) return;
      revision.current = incoming.revision;
      setFailed(false);
      setStatus(incoming);
    },
    [projectId],
  );

  const refresh = useCallback(() => {
    call("project_status", { projectId })
      .then(accept)
      .catch(() => {
        if (alive.current && revision.current < 0) setFailed(true);
      });
  }, [accept, projectId]);

  useEffect(() => {
    alive.current = true;
    revision.current = -1;
    // Listen first, then ask: a change between the two is not lost.
    const unlisten = on("project-state", accept);
    refresh();
    return () => {
      alive.current = false;
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [accept, refresh]);

  const value = useMemo(() => ({ projectId, status, failed, refresh }), [projectId, status, failed, refresh]);
  return <ProjectStatusContext.Provider value={value}>{children}</ProjectStatusContext.Provider>;
}
