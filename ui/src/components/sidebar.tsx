import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useCompactSidebar } from "@/hooks/use-media-query";
import { TONE_TEXT } from "@/lib/tones";
import { cn } from "@/lib/utils";
import { VIEWS, type ShellCounts, type ViewEntry, type ViewId } from "@/views";

/** The number next to an entry, and what it counts in words (for assistive technologies). */
function countOf(view: ViewId, counts: ShellCounts): { value: number; meaning: string; warning: boolean } | null {
  if (view === "board" && counts.backlog > 0) {
    return { value: counts.backlog, meaning: counts.backlog === 1 ? "card in Backlog" : "cards in Backlog", warning: false };
  }
  if (view === "jobs" && counts.jobsWaiting > 0) {
    return { value: counts.jobsWaiting, meaning: counts.jobsWaiting === 1 ? "job waiting for you" : "jobs waiting for you", warning: true };
  }
  return null;
}

function Entry({
  view,
  selected,
  compact,
  counts,
  onSelect,
}: {
  view: ViewEntry;
  selected: boolean;
  compact: boolean;
  counts: ShellCounts;
  onSelect: (id: ViewId) => void;
}) {
  const Icon = view.icon;
  const count = countOf(view.id, counts);
  const button = (
    <button
      type="button"
      aria-current={selected ? "page" : undefined}
      onClick={() => onSelect(view.id)}
      className={cn(
        "relative flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm font-medium transition-colors duration-150 hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        compact && "justify-center px-0",
        selected && "bg-accent text-accent-foreground",
      )}
    >
      <Icon className="size-4 shrink-0" aria-hidden />
      <span className={cn(compact ? "sr-only" : "truncate")}>{view.label}</span>
      {count ? (
        <>
          <span
            aria-hidden
            className={cn(
              "text-xs font-semibold tabular-nums",
              count.warning ? TONE_TEXT.amber : "text-muted-foreground",
              compact ? "absolute right-1 top-0.5" : "ml-auto",
            )}
          >
            {count.value}
          </span>
          <span className="sr-only">
            , {count.value} {count.meaning}
          </span>
        </>
      ) : null}
    </button>
  );
  // Names are tooltips only while they are not visible; sidebar entries have none otherwise.
  if (!compact) return <li>{button}</li>;
  return (
    <li>
      <Tooltip>
        <TooltipTrigger asChild>{button}</TooltipTrigger>
        <TooltipContent side="right">{view.label}</TooltipContent>
      </Tooltip>
    </li>
  );
}

export interface SidebarProps {
  selected: ViewId;
  onSelect: (id: ViewId) => void;
  counts: ShellCounts;
}

/**
 * The left sidebar of a project window: 208 px wide with the name of each view, and 56 px with
 * icons only (names as tooltips) below a window width of 1024 px. Board shows the number of real
 * cards in Backlog, Jobs (in amber) the number of jobs waiting for the developer; nothing at 0.
 */
export function Sidebar({ selected, onSelect, counts }: SidebarProps) {
  const compact = useCompactSidebar();
  return (
    <nav
      aria-label="Views"
      data-compact={compact}
      className={cn("shrink-0 border-r bg-muted/40 p-2", compact ? "w-[56px]" : "w-[208px]")}
    >
      <ul className="flex flex-col gap-1">
        {VIEWS.map((view) => (
          <Entry key={view.id} view={view} selected={view.id === selected} compact={compact} counts={counts} onSelect={onSelect} />
        ))}
      </ul>
    </nav>
  );
}
