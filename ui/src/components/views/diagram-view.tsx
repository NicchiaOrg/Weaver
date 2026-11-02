import { ChevronRight, Maximize } from "lucide-react";
import { useMemo, useRef, useState, type KeyboardEvent } from "react";

import type { Detail } from "@/bindings/Detail";
import { useResolvedTheme } from "@/components/theme-provider";
import { Button } from "@/components/ui/button";
import { DiagramCanvas, type Hover } from "@/components/views/diagram-canvas";
import { DetailsPanel } from "@/components/views/diagram-details";
import {
  breadcrumb,
  detailOf,
  elementName,
  elementsOf,
  stepSelection,
} from "@/components/views/diagram-model";
import { ViewFrame } from "@/components/views/view-frame";
import { useViews } from "@/components/views/views-state";

const NAVIGATION_KEYS = ["ArrowRight", "ArrowDown", "ArrowLeft", "ArrowUp", "Home", "End"];

function plainDetail(title: string): Detail {
  return {
    kind: "system",
    title,
    description: "",
    change: null,
    transitions: [],
    events: [],
    drifts: [],
    tasks: [],
    file: null,
    line: null,
  };
}

function DiagramBody({ fitNonce }: { fitNonce: number }) {
  const { snapshot, nav, focus, select, openLevel, goCrumb, toggleExpanded, viewports } = useViews();
  const theme = useResolvedTheme();
  const listbox = useRef<HTMLDivElement>(null);
  const handled = useRef(0);
  const [hover, setHover] = useState<Hover | null>(null);
  const data = snapshot!.data;
  const levels = data.model.levels;
  const levelKey = nav.path[nav.path.length - 1] ?? "";
  const level = levels.find((l) => l.key === levelKey) ?? levels[0];
  const elements = useMemo(() => (level ? elementsOf(level, nav.expanded) : []), [level, nav.expanded]);
  const step = focus.diagram?.plan.steps.find((s) => s.level === level?.key) ?? null;
  const focusHere = useMemo(() => (step ? { nodes: step.nodes, edges: step.edges } : null), [step]);

  if (!level) return <p className="p-4 text-sm text-muted-foreground">The Model has nothing to draw yet.</p>;

  const selected = elements.find((e) => e.id === nav.selected) ?? null;
  const detail = selected ? (detailOf(data.model.details, selected) ?? plainDetail(selected.type === "node" ? selected.node.title : selected.edge.label)) : null;
  const node = selected?.type === "node" ? selected.node : null;
  const opens = node?.opens && levels.some((l) => l.key === node.opens) ? node.opens : null;
  const crumbs = breadcrumb(levels, nav.path);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    if (NAVIGATION_KEYS.includes(event.key)) {
      event.preventDefault();
      select(stepSelection(elements, nav.selected, event.key));
    } else if (event.key === "Enter" && opens) {
      event.preventDefault();
      openLevel(opens);
    } else if ((event.key === " " || event.key === "+" || event.key === "-") && node?.expandable) {
      event.preventDefault();
      toggleExpanded(node.id);
    } else if (event.key === "Backspace" && nav.path.length > 1) {
      event.preventDefault();
      goCrumb(nav.path.length - 2);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <nav aria-label="Breadcrumb" className="shrink-0 border-b px-4 py-1">
        <ol className="flex flex-wrap items-center gap-1 text-sm">
          {crumbs.map((crumb, index) => (
            <li key={crumb.key} className="flex items-center gap-1">
              {index > 0 ? <ChevronRight className="size-3.5 text-muted-foreground" aria-hidden /> : null}
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2"
                aria-current={index === crumbs.length - 1 ? "page" : undefined}
                onClick={() => goCrumb(index)}
              >
                {crumb.title}
              </Button>
            </li>
          ))}
        </ol>
      </nav>
      <div className="flex min-h-0 flex-1">
        {/* The canvas takes the pointer; the listbox beside it takes the keyboard and speaks for it. */}
        <div
          className="relative min-w-0 flex-1 focus-within:ring-2 focus-within:ring-inset focus-within:ring-ring"
          onMouseDown={() => listbox.current?.focus({ preventScroll: true })}
        >
          <DiagramCanvas
            key={level.key}
            level={level}
            details={data.model.details}
            expanded={nav.expanded}
            selected={nav.selected}
            focus={focusHere}
            fitFocus={focusHere !== null && focus.diagram !== null && focus.diagram.nonce !== handled.current}
            onFocusFitted={() => {
              handled.current = focus.diagram?.nonce ?? handled.current;
            }}
            fitNonce={fitNonce}
            dark={theme === "dark"}
            viewport={viewports.current.get(level.key)}
            onViewport={(viewport) => viewports.current.set(level.key, viewport)}
            onSelect={select}
            onOpen={(id) => {
              const target = level.nodes.find((n) => n.id === id)?.opens;
              if (target && levels.some((l) => l.key === target)) openLevel(target);
            }}
            onHover={setHover}
          />
          {hover ? (
            <div
              role="tooltip"
              className="pointer-events-none absolute z-10 max-w-xs rounded-md border bg-popover px-3 py-2 text-xs text-popover-foreground shadow-lg"
              style={{ left: hover.x + 12, top: hover.y + 12 }}
            >
              <p className="font-medium">{hover.title}</p>
              {hover.text ? <p className="text-muted-foreground">{hover.text}</p> : null}
            </div>
          ) : null}
          <div
            ref={listbox}
            role="listbox"
            tabIndex={0}
            aria-label={`Elements of ${level.title}`}
            aria-activedescendant={selected ? `diagram-option-${selected.id}` : undefined}
            aria-orientation="horizontal"
            className="sr-only"
            onKeyDown={onKeyDown}
          >
            {elements.map((element) => (
              <div
                key={element.id}
                id={`diagram-option-${element.id}`}
                role="option"
                aria-selected={element.id === nav.selected}
                aria-label={elementName(element)}
              />
            ))}
          </div>
        </div>
        {selected && detail ? (
          <DetailsPanel
            detail={detail}
            opens={opens !== null}
            onOpen={() => opens && openLevel(opens)}
            expandable={node?.expandable ?? false}
            expanded={node ? nav.expanded.has(node.id) : false}
            onToggleExpanded={() => node && toggleExpanded(node.id)}
          />
        ) : null}
      </div>
    </div>
  );
}

/**
 * The Diagram: the level the window is on, drawn from what the backend computed, with its
 * breadcrumb, a right panel for the selected element and a keyboard path (arrows select, Enter
 * opens, Space expands a Machine, Backspace goes up, Esc clears the selection and then the focus).
 */
export function DiagramView() {
  const [fitNonce, setFitNonce] = useState(0);
  return (
    <ViewFrame
      view="diagram"
      toolbar={
        <Button variant="outline" size="sm" onClick={() => setFitNonce((n) => n + 1)}>
          <Maximize aria-hidden />
          Fit
        </Button>
      }
    >
      <DiagramBody fitNonce={fitNonce} />
    </ViewFrame>
  );
}
