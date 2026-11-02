import { Bot, History, ListChecks, MessageSquare, Network, Package, Settings, SquareKanban, type LucideIcon } from "lucide-react";

export type ViewId = "board" | "diagram" | "requirements" | "specs" | "history" | "jobs" | "general" | "settings";

export interface ViewEntry {
  id: ViewId;
  /** The name in the sidebar and the heading of the view. */
  label: string;
  icon: LucideIcon;
}

/** The views of a project window, in the order of the sidebar. */
export const VIEWS: readonly ViewEntry[] = [
  { id: "board", label: "Board", icon: SquareKanban },
  { id: "diagram", label: "Diagram", icon: Network },
  { id: "requirements", label: "Requirements", icon: ListChecks },
  { id: "specs", label: "Specs", icon: Package },
  { id: "history", label: "History", icon: History },
  { id: "jobs", label: "Jobs", icon: Bot },
  { id: "general", label: "General", icon: MessageSquare },
  { id: "settings", label: "Settings", icon: Settings },
];

/** The numbers the sidebar shows next to Board and Jobs; the backend holds them. */
export interface ShellCounts {
  /** Real cards in Backlog (ghosts never count). */
  backlog: number;
  /** Jobs waiting for the developer. */
  jobsWaiting: number;
}

export const NO_COUNTS: ShellCounts = { backlog: 0, jobsWaiting: 0 };
