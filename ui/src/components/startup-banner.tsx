import { TriangleAlert } from "lucide-react";
import { useEffect, useState } from "react";

import type { StartupFailure } from "@/bindings/StartupFailure";
import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { call, on } from "@/lib/backend";

/** What the developer reads: `Weaver could not start: <reason>` or `<project> could not open: <reason>`. */
export function startupMessage(failure: StartupFailure): string {
  return failure.projectName
    ? `${failure.projectName} could not open: ${failure.reason}`
    : `Weaver could not start: ${failure.reason}`;
}

/**
 * The failed startup steps that concern this window: the app's own (in every window) and, in a
 * project window, that project's. The welcome window has no project and shows every failure
 * (`allProjects`): a project of the last session that could not open is told there. Each has an
 * **Open log** button that reveals the log file. A failure is fetched when the window loads and is
 * also pushed while the app starts.
 */
export function StartupBanner({ projectId, allProjects = false }: { projectId?: string; allProjects?: boolean }) {
  const [failures, setFailures] = useState<StartupFailure[]>([]);

  useEffect(() => {
    let current = true;
    const concerns = (failure: StartupFailure) => allProjects || failure.projectId === null || failure.projectId === projectId;
    const add = (incoming: StartupFailure[]) =>
      setFailures((known) => [
        ...known,
        ...incoming.filter(
          (f) => concerns(f) && !known.some((k) => k.step === f.step && k.projectId === f.projectId && k.reason === f.reason),
        ),
      ]);
    // Subscribe first, then ask: a failure between the two is not lost (duplicates are dropped).
    const unlisten = on("startup-failed", (failure) => current && add([failure]));
    call("startup_failures")
      .then((known) => current && add(known))
      .catch(() => {});
    return () => {
      current = false;
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [projectId, allProjects]);

  if (failures.length === 0) return null;
  return (
    <div className="flex flex-col gap-2 p-3">
      {failures.map((failure) => (
        <Alert key={`${failure.projectId ?? "app"}/${failure.step}/${failure.reason}`} variant="error">
          <TriangleAlert aria-hidden />
          <p className="min-w-0 flex-1">{startupMessage(failure)}</p>
          <Button variant="outline" size="sm" onClick={() => void call("open_log").catch(() => {})}>
            Open log
          </Button>
        </Alert>
      ))}
    </div>
  );
}
