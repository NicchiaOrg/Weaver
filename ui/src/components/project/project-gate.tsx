import type { ReactNode } from "react";

import { GetStarted } from "@/components/project/get-started";
import { useProjectStatus } from "@/components/project/use-project-status";
import { ViewHeader, viewTitleId } from "@/components/view-header";
import type { ViewId } from "@/views";

/**
 * Every view of the window except Settings goes through it: a folder without `.weaver` shows Get
 * Started instead of the view, header included (its heading labels the view's region). While the
 * backend has not answered only the view header shows (the answer is one local call); when it
 * cannot be read the view is shown as it is.
 */
export function ProjectGate({ view, children }: { view: ViewId; children: ReactNode }) {
  const { status, failed } = useProjectStatus();
  if (!status) return failed ? <>{children}</> : <ViewHeader view={view} />;
  return status.state === "not-initialised" ? <GetStarted headingId={viewTitleId(view)} /> : <>{children}</>;
}
