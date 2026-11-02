import { FileText, Layers, ListTodo, TriangleAlert, type LucideIcon } from "lucide-react";

import { cn } from "@/lib/utils";
import { TONE_TEXT, type Tone } from "@/lib/tones";

export type MarkKind = "proposal" | "resolution" | "drift" | "task";

const MARKS: Record<MarkKind, { label: string; icon: LucideIcon; tone: Tone }> = {
  proposal: { label: "Proposal", icon: FileText, tone: "blue" },
  resolution: { label: "Resolution", icon: Layers, tone: "yellow" },
  drift: { label: "Drift", icon: TriangleAlert, tone: "red" },
  task: { label: "Task", icon: ListTodo, tone: "green" },
};

/** The name of a kind of card, as the Board writes it (`Proposal`, `Task`, `Drift`, `Resolution`). */
export function kindLabel(kind: MarkKind): string {
  return MARKS[kind].label;
}

/** An icon and the kind in words, in the colour of the kind: the kind never stands on colour alone. */
export function KindMark({ kind, className }: { kind: MarkKind; className?: string }) {
  const { label, icon: Icon, tone } = MARKS[kind];
  return (
    <span className={cn("inline-flex items-center gap-1 text-xs", TONE_TEXT[tone], className)}>
      <Icon aria-hidden className="size-3.5" />
      {label}
    </span>
  );
}
