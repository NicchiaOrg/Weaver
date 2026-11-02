import { Sparkles } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import type { SpecEntryView } from "@/bindings/SpecEntryView";
import type { SpecsFieldView } from "@/bindings/SpecsFieldView";
import { Markdown } from "@/components/markdown";
import { OpenInEditor } from "@/components/open-in-editor";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { ChangeBadge, focusClass } from "@/components/views/change-badge";
import { ViewFrame } from "@/components/views/view-frame";
import { useViews } from "@/components/views/views-state";
import { cn } from "@/lib/utils";

/** `languages` → `Languages`; `ci` → `CI`. */
export function fieldTitle(name: string): string {
  return name === "ci" ? "CI" : name.charAt(0).toUpperCase() + name.slice(1);
}

function Entry({ entry }: { entry: SpecEntryView }) {
  const [before, setBefore] = useState(false);
  const removed = entry.change === "removed";
  return (
    <li
      aria-label={`${entry.name}${entry.change ? `, ${entry.change}` : ""}`}
      className={cn("space-y-1 rounded-md border p-3", removed && "border-dashed text-muted-foreground")}
    >
      <div className="flex flex-wrap items-center gap-2">
        <span className={cn("font-medium", removed && "line-through")}>{entry.name}</span>
        {entry.version ? <Badge variant="muted">{entry.version}</Badge> : null}
        {entry.change ? <ChangeBadge change={entry.change} /> : null}
      </div>
      {entry.note ? <p className="text-sm text-muted-foreground">{entry.note}</p> : null}
      {entry.chosenReason !== null ? (
        <Tooltip>
          <TooltipTrigger asChild>
            <span
              tabIndex={0}
              className="inline-flex items-center gap-1 rounded-sm text-xs text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
              <Sparkles className="size-3.5" aria-hidden />
              agent's choice
            </span>
          </TooltipTrigger>
          <TooltipContent>{entry.chosenReason}</TooltipContent>
        </Tooltip>
      ) : null}
      {entry.change === "changed" && entry.before !== null ? (
        <div>
          <Button variant="ghost" size="sm" aria-pressed={before} onClick={() => setBefore((v) => !v)}>
            Show before
          </Button>
          {before ? <p className="text-sm text-muted-foreground">Before: {entry.before}</p> : null}
        </div>
      ) : null}
    </li>
  );
}

function Field({ field, focused }: { field: SpecsFieldView; focused: boolean }) {
  const [before, setBefore] = useState(false);
  const title = fieldTitle(field.name);
  return (
    <section
      data-step-key={field.name}
      aria-labelledby={`specs-${field.name}`}
      className={cn("space-y-3 rounded-md py-2 pl-4 pr-3", focusClass(focused, field.change) || "border-l-4 border-transparent")}
    >
      <div className="flex flex-wrap items-center gap-2">
        <h2 id={`specs-${field.name}`} className="text-lg font-semibold">
          {title}
        </h2>
        {field.change ? <ChangeBadge change={field.change} /> : null}
        {field.change === "changed" && field.beforeNotes !== null ? (
          <Button variant="ghost" size="sm" aria-pressed={before} onClick={() => setBefore((v) => !v)}>
            Show before
          </Button>
        ) : null}
        {field.file ? <OpenInEditor file={field.file} line={field.line} /> : null}
      </div>
      {field.notes !== null ? (
        <div className={cn(field.change === "removed" && "text-muted-foreground line-through")}>
          <Markdown holderHeadingLevel={2}>{before && field.beforeNotes !== null ? field.beforeNotes : field.notes}</Markdown>
        </div>
      ) : (
        <ul className="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-3">
          {field.entries.map((entry) => (
            <Entry key={entry.name} entry={entry} />
          ))}
        </ul>
      )}
    </section>
  );
}

function SpecsPage() {
  const { snapshot, focus } = useViews();
  const page = useRef<HTMLDivElement>(null);
  const active = focus.specs;
  const focusedKeys = new Set(active?.plan.steps.map((s) => s.key));
  const current = active?.plan.steps[active.index]?.key;
  const nonce = active?.nonce;

  useEffect(() => {
    if (current === undefined || !page.current) return;
    const target = [...page.current.querySelectorAll<HTMLElement>("[data-step-key]")].find((el) => el.dataset.stepKey === current);
    target?.scrollIntoView({ block: "start" });
  }, [current, nonce]);

  const fields = snapshot!.data.specs?.fields ?? [];
  return (
    <div ref={page} className="h-full overflow-y-auto">
      <div className="mx-auto max-w-[1100px] space-y-6 p-6">
        {fields.length === 0 ? <p className="text-sm text-muted-foreground">The project has no Specs.</p> : null}
        {fields.map((field) => (
          <Field key={field.name} field={field} focused={focusedKeys.has(field.name)} />
        ))}
      </div>
    </div>
  );
}

/** The Specs: the fields in their fixed order, empty ones hidden, entries in a grid of columns of at least 300 px. */
export function SpecsView() {
  return (
    <ViewFrame view="specs">
      <SpecsPage />
    </ViewFrame>
  );
}
