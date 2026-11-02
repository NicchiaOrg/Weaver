import { Minus, Pencil, Plus } from "lucide-react";

import type { Change } from "@/bindings/Change";
import { Badge } from "@/components/ui/badge";

const CHANGE: Record<Change, { variant: "success" | "warning" | "error"; icon: typeof Plus }> = {
  added: { variant: "success", icon: Plus },
  changed: { variant: "warning", icon: Pencil },
  removed: { variant: "error", icon: Minus },
};

/** The overlay change as text with an icon and its colour (green, amber, red): never colour alone. */
export function ChangeBadge({ change, label }: { change: Change; label?: string }) {
  const { variant, icon: Icon } = CHANGE[change];
  return (
    <Badge variant={variant}>
      <Icon aria-hidden />
      {label ?? change}
    </Badge>
  );
}

// Full class names, so Tailwind sees them.
const FOCUS_CYAN = "border-l-4 border-cyan-600 bg-cyan-500/10 dark:border-cyan-400";
const FOCUS_CHANGE: Record<Change, string> = {
  added: "border-l-4 border-green-600 bg-green-500/10 dark:border-green-400",
  changed: "border-l-4 border-amber-600 bg-amber-500/10 dark:border-amber-400",
  removed: "border-l-4 border-red-600 bg-red-500/10 dark:border-red-400",
};

/** A focused section or field: a left border and tint in cyan, or in its change colour under an overlay. */
export function focusClass(focused: boolean, change: Change | null): string {
  if (!focused) return "";
  return change ? FOCUS_CHANGE[change] : FOCUS_CYAN;
}
