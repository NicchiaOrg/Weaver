import { useEffect, useRef, useState } from "react";

import type { AreaView } from "@/bindings/AreaView";
import type { SectionView } from "@/bindings/SectionView";
import { AskRequirementsButton } from "@/components/ask/ask-button";
import { Markdown } from "@/components/markdown";
import { OpenInEditor } from "@/components/open-in-editor";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ChangeBadge, focusClass } from "@/components/views/change-badge";
import { sectionKey } from "@/components/views/focus-model";
import { ViewFrame } from "@/components/views/view-frame";
import { useViews } from "@/components/views/views-state";
import { cn } from "@/lib/utils";

function SectionBlock({ area, section, focused }: { area: AreaView; section: SectionView; focused: boolean }) {
  const [before, setBefore] = useState(false);
  const removed = section.change === "removed";
  const key = sectionKey(area.id, section.title);
  return (
    <section
      data-step-key={key}
      aria-label={`${area.title}, ${section.title}${section.change ? `, ${section.change}` : ""}`}
      className={cn("space-y-2 rounded-md py-2 pl-4 pr-3", focusClass(focused, section.change) || "border-l-4 border-transparent")}
    >
      <div className="flex flex-wrap items-center gap-2">
        <h4 className="font-mono text-sm font-semibold">{section.title}</h4>
        {section.change ? <ChangeBadge change={section.change} /> : null}
        {section.movedFrom ? <Badge variant="muted">moved from {section.movedFrom}</Badge> : null}
        {section.change === "changed" && section.before !== null ? (
          <Button variant="ghost" size="sm" aria-pressed={before} onClick={() => setBefore((v) => !v)}>
            Show before
          </Button>
        ) : null}
        {section.file ? <OpenInEditor file={section.file} line={section.line} /> : null}
      </div>
      {section.body.trim() !== "" ? (
        <div className={cn(removed && "text-muted-foreground line-through")}>
          <Markdown holderHeadingLevel={4}>{before && section.before !== null ? section.before : section.body}</Markdown>
        </div>
      ) : null}
    </section>
  );
}

function AreaBlock({ area, focusedKeys, only }: { area: AreaView; focusedKeys: ReadonlySet<string>; only: boolean }) {
  const areaFocused = focusedKeys.has(sectionKey(area.id, null));
  const fragments = area.fragments
    .map((fragment) => ({
      fragment,
      sections: only && !areaFocused ? fragment.sections.filter((s) => focusedKeys.has(sectionKey(area.id, s.title))) : fragment.sections,
    }))
    .filter(({ sections }) => sections.length > 0 || !only);
  if (only && !areaFocused && fragments.length === 0) return null;
  return (
    <section
      data-step-key={sectionKey(area.id, null)}
      aria-labelledby={`area-${area.id}`}
      className="space-y-4 border-b pb-8 last:border-b-0"
    >
      <header className={cn("space-y-2 rounded-md py-2 pl-4 pr-3", focusClass(areaFocused, null) || "border-l-4 border-transparent")}>
        <div className="flex flex-wrap items-center gap-2">
          <h2 id={`area-${area.id}`} className="text-xl font-semibold">
            {area.title}
          </h2>
          {area.file ? <OpenInEditor file={area.file} /> : null}
        </div>
        {area.intro.trim() !== "" ? <Markdown holderHeadingLevel={2}>{area.intro}</Markdown> : null}
      </header>
      {fragments.map(({ fragment, sections }) => (
        <div key={fragment.name} className="space-y-2">
          <div className="flex flex-wrap items-baseline gap-2 pl-4">
            <h3 className={cn("text-base font-semibold", fragment.change === "removed" && "text-muted-foreground line-through")}>{fragment.title}</h3>
            <span className="font-mono text-xs text-muted-foreground">{fragment.name}.md</span>
            {fragment.change === "new" ? <ChangeBadge change="added" label="new fragment" /> : null}
            {fragment.change === "removed" ? <ChangeBadge change="removed" label="fragment removed" /> : null}
            {fragment.file ? <OpenInEditor file={fragment.file} /> : null}
          </div>
          {sections.map((section) => (
            <SectionBlock
              key={`${fragment.name}/${section.title}`}
              area={area}
              section={section}
              focused={focusedKeys.has(sectionKey(area.id, section.title))}
            />
          ))}
        </div>
      ))}
    </section>
  );
}

function RequirementsPage() {
  const { snapshot, focus } = useViews();
  const page = useRef<HTMLDivElement>(null);
  const active = focus.requirements;
  const focusedKeys = new Set(active?.plan.steps.map((s) => s.key));
  const current = active?.plan.steps[active.index]?.key;
  const nonce = active?.nonce;

  // Going to a step scrolls it to the top of the view.
  useEffect(() => {
    if (current === undefined || !page.current) return;
    const target = [...page.current.querySelectorAll<HTMLElement>("[data-step-key]")].find((el) => el.dataset.stepKey === current);
    target?.scrollIntoView({ block: "start" });
  }, [current, nonce]);

  const areas = snapshot!.data.requirements;
  return (
    <div ref={page} className="h-full overflow-y-auto">
      <div className="mx-auto max-w-4xl space-y-8 p-6">
        {areas.length === 0 ? <p className="text-sm text-muted-foreground">The project has no Requirements.</p> : null}
        {areas.map((area) => (
          <AreaBlock key={area.id} area={area} focusedKeys={focusedKeys} only={active?.only ?? false} />
        ))}
      </div>
    </div>
  );
}

/**
 * The Requirements: Areas in folder order, fragments in file order, sections with `General` first,
 * each section with its title and `Open in editor`, and the labels of an overlay. The only
 * `Ask more` is the one in the header, about the Requirements as a whole.
 */
export function RequirementsView() {
  return (
    <ViewFrame view="requirements" toolbar={<AskRequirementsButton />}>
      <RequirementsPage />
    </ViewFrame>
  );
}
