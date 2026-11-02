import { Layers } from "lucide-react";

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useViews } from "@/components/views/views-state";

const CURRENT = "current";

/**
 * The overlay picker the Requirements and Specs of a window share: `Current Model`, then
 * every open Proposal not attached to a Drift (the backend decides which), by id and title.
 */
export function OverlayPicker() {
  const { snapshot, overlay, setOverlay } = useViews();
  const options = snapshot?.overlays ?? [];
  return (
    <div className="flex items-center gap-2 text-sm">
      <Layers className="size-4 text-muted-foreground" aria-hidden />
      <Select value={overlay ?? CURRENT} onValueChange={(value) => setOverlay(value === CURRENT ? null : value)}>
        <SelectTrigger aria-label="Overlay" className="h-8 w-72">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={CURRENT}>Current Model</SelectItem>
          {options.map((option) => (
            <SelectItem key={option.id} value={option.id}>
              {option.shortNumber ?? option.id} – {option.title}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
