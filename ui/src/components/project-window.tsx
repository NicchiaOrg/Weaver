import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";

import { ProjectContext, type ProjectContextValue } from "@/components/project-context";
import { Sidebar } from "@/components/sidebar";
import { StartupBanner } from "@/components/startup-banner";
import { UpdateBanner } from "@/components/update-banner";
import { on } from "@/lib/backend";
import { NO_COUNTS, VIEWS, type ShellCounts, type ViewId } from "@/views";

export interface ProjectWindowProps {
  /** The project hash (the window's label is `p-<hash>`). */
  projectId: string;
  /** The content of each view; a view without one shows its heading only. */
  views?: Partial<Record<ViewId, ReactNode>>;
  counts?: ShellCounts;
  initialView?: ViewId;
  /** Wraps everything the window shows, inside the project context (what the views of the window share). */
  around?: (children: ReactNode) => ReactNode;
  /** Shown under the startup and update banners, at the top of the window (what the check of the agent files says). */
  banners?: ReactNode;
}

/**
 * The shell of a project window: the sidebar and the selected view. Every view that was opened
 * stays mounted while another is shown, so switching entries keeps the state of each view (level,
 * focus, overlay, scroll). When the window comes forward because of a notification
 * (`show-requested`), it shows what the notification was about.
 */
export function ProjectWindow({ projectId, views = {}, counts = NO_COUNTS, initialView = "board", around = (children) => children, banners }: ProjectWindowProps) {
  const [selected, setSelected] = useState<ViewId>(initialView);
  const [opened, setOpened] = useState<ReadonlySet<ViewId>>(() => new Set([initialView]));

  const select = useCallback((id: ViewId) => {
    setOpened((previous) => (previous.has(id) ? previous : new Set(previous).add(id)));
    setSelected(id);
  }, []);

  const [openedJob, setOpenedJob] = useState<string | null>(null);
  const openJob = useCallback(
    (jobId: string) => {
      setOpenedJob(jobId);
      select("jobs");
    },
    [select],
  );

  useEffect(() => {
    let current = true;
    const unlisten = on("show-requested", (request) => {
      if (!current || request.projectId !== projectId) return;
      if (request.jobId) openJob(request.jobId);
      else select(request.view);
    });
    return () => {
      current = false;
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [projectId, select, openJob]);

  const context = useMemo<ProjectContextValue>(
    () => ({ projectId, showView: select, openJob, openedJob }),
    [projectId, select, openJob, openedJob],
  );

  return (
    <ProjectContext.Provider value={context}>
      {around(
        <div className="flex h-full min-w-0">
          <Sidebar selected={selected} onSelect={select} counts={counts} />
          <div className="flex min-w-0 flex-1 flex-col">
            <StartupBanner projectId={projectId} />
            <UpdateBanner />
            {banners}
            <main className="relative min-h-0 flex-1">
              {VIEWS.filter((view) => opened.has(view.id)).map((view) => (
                <section
                  key={view.id}
                  aria-labelledby={`view-${view.id}`}
                  hidden={view.id !== selected}
                  className="h-full overflow-auto"
                >
                  <h1 id={`view-${view.id}`} className="sr-only">
                    {view.label}
                  </h1>
                  {views[view.id]}
                </section>
              ))}
            </main>
          </div>
        </div>,
      )}
    </ProjectContext.Provider>
  );
}
