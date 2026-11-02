import type { Ghost } from "@/bindings/Ghost";
import type { GhostPhase } from "@/bindings/GhostPhase";
import { KindMark, kindLabel } from "@/components/board/kind-mark";
import { useProject } from "@/components/project-context";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

const PHASES: Record<GhostPhase, string> = {
  preparing: "Preparing",
  writing: "Writing",
  "waiting-for-answers": "Waiting for your answers",
  "being-refined": "Being refined",
};

/** The phase line of a ghost. */
export function phaseText(phase: GhostPhase): string {
  return PHASES[phase];
}

/**
 * A card a job announced and has not filed yet: a skeleton at the bottom of its column (dashed
 * border, 70% opacity, shimmer on its text lines) that shows the kind, the title, the phase and
 * the job's latest note on one line. The whole ghost is one button that opens the job's
 * conversation in Jobs.
 */
export function GhostCard({ ghost }: { ghost: Ghost }) {
  const { openJob } = useProject();
  const kind = kindLabel(ghost.kind);
  const phase = phaseText(ghost.phase);
  return (
    <button
      type="button"
      data-ghost-id={ghost.id}
      aria-label={`${kind} being prepared by the agent: ${ghost.title}, ${phase}`}
      onClick={() => openJob(ghost.jobId)}
      className="flex w-full flex-col gap-1 rounded-lg border border-dashed bg-card p-3 text-left text-card-foreground opacity-70 transition-colors duration-150 hover:bg-accent/40 focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
    >
      <KindMark kind={ghost.kind} />
      <span className="shimmer line-clamp-3 rounded-sm text-sm font-medium">{ghost.title}</span>
      <span className="shimmer w-fit rounded-sm text-xs text-muted-foreground">{phase}</span>
      {ghost.note ? (
        <Tooltip>
          <TooltipTrigger asChild>
            <span className="block truncate text-xs text-muted-foreground">{ghost.note}</span>
          </TooltipTrigger>
          <TooltipContent>{ghost.note}</TooltipContent>
        </Tooltip>
      ) : null}
    </button>
  );
}
