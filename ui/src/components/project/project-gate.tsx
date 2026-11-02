import type { ReactNode } from "react";

import { GetStarted } from "@/components/project/get-started";
import { useProjectStatus } from "@/components/project/use-project-status";

/**
 * Every view of the window except Settings goes through it: a folder without `.weaver` shows Get
 * Started instead of the view. While the backend has not answered nothing is shown (the answer is
 * one local call); when it cannot be read the view is shown as it is.
 */
export function ProjectGate({ children }: { children: ReactNode }) {
  const { status, failed } = useProjectStatus();
  if (!status) return failed ? <>{children}</> : null;
  return status.state === "not-initialised" ? <GetStarted /> : <>{children}</>;
}
