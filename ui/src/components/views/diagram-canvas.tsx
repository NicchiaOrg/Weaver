import cytoscape from "cytoscape";
import { useEffect, useRef, useState } from "react";

import type { Detail } from "@/bindings/Detail";
import type { Level } from "@/bindings/Level";
import { cyElements } from "@/components/views/diagram-graph";
import { layoutLevel, type LayoutEngine } from "@/components/views/diagram-layout";
import { paletteOf, stylesheet } from "@/components/views/diagram-style";
import { layoutInWorker } from "@/components/views/elk-client";
import type { Viewport } from "@/components/views/views-state";

/** The hover box: the title and the first sentence of the description, at a point of the canvas. */
export interface Hover {
  title: string;
  text: string;
  x: number;
  y: number;
}

export interface DiagramCanvasProps {
  level: Level;
  details: Readonly<Record<string, Detail | undefined>>;
  expanded: ReadonlySet<string>;
  selected: string | null;
  /** The nodes and edges of the current focus step on this level; `null` without a focus here. */
  focus: { nodes: readonly string[]; edges: readonly string[] } | null;
  /** Fit the focused elements once, when the focus asked for it. */
  fitFocus: boolean;
  onFocusFitted: () => void;
  /** Bumped by `Fit`. */
  fitNonce: number;
  dark: boolean;
  /** The viewport this level was left with, if it was visited. */
  viewport: Viewport | undefined;
  onViewport: (viewport: Viewport) => void;
  onSelect: (id: string | null) => void;
  onOpen: (id: string) => void;
  onHover: (hover: Hover | null) => void;
  /** Overridable for tests; the app uses elkjs in a Web Worker. */
  engine?: LayoutEngine;
}

const PADDING = 32;
const FOCUS_PADDING = 56;

/** Fits `eles` with a margin and never zooms in beyond what `Fit` shows of the whole level. */
function fitContaining(cy: cytoscape.Core, eles: cytoscape.Collection) {
  cy.fit(cy.elements(), PADDING);
  const fitZoom = cy.zoom();
  cy.fit(eles, FOCUS_PADDING);
  if (cy.zoom() > fitZoom) {
    cy.zoom(fitZoom);
    cy.center(eles);
  }
}

interface Badge {
  id: string;
  x: number;
  y: number;
  count: number;
}

/**
 * One level of the Diagram drawn with cytoscape, laid out by elkjs off the interface thread. The
 * canvas is aria-hidden: the same elements are in the listbox next to it, with accessible names.
 * It is mounted once per level (key = the level), so its viewport is its own; the viewport is
 * reported on every change and given back when the level is visited again.
 */
export function DiagramCanvas(props: DiagramCanvasProps) {
  const { level, details, expanded, selected, focus, fitFocus, fitNonce, dark, engine = layoutInWorker } = props;
  const host = useRef<HTMLDivElement>(null);
  const cyRef = useRef<cytoscape.Core | null>(null);
  const latest = useRef(props);
  latest.current = props;
  const [version, setVersion] = useState(0);
  const [badges, setBadges] = useState<Badge[]>([]);
  const placed = useRef(false);
  const sequence = useRef(0);

  useEffect(() => {
    const cy = cytoscape({
      container: host.current,
      elements: [],
      style: stylesheet(latest.current.dark),
      minZoom: 0.1,
      maxZoom: 2.5,
      wheelSensitivity: 0.3,
      boxSelectionEnabled: false,
      autoungrabify: true,
      autounselectify: true,
    });
    cyRef.current = cy;
    let frame = 0;
    const updateBadges = () => {
      frame = 0;
      const next: Badge[] = [];
      cy.nodes().forEach((node) => {
        const count = Number(node.data("drifts") ?? 0);
        if (count <= 0) return;
        const box = node.renderedBoundingBox({ includeLabels: false, includeOverlays: false });
        next.push({ id: node.id(), x: box.x2, y: box.y1, count });
      });
      setBadges((previous) => (JSON.stringify(previous) === JSON.stringify(next) ? previous : next));
    };
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(updateBadges);
    };
    cy.on("tap", "node, edge", (event) => latest.current.onSelect(event.target.id()));
    cy.on("tap", (event) => {
      if (event.target === cy) latest.current.onSelect(null);
    });
    cy.on("dbltap", "node", (event) => latest.current.onOpen(event.target.id()));
    cy.on("mouseover", "node, edge", (event) => {
      const element = event.target;
      latest.current.onHover({
        title: String(element.data("hoverTitle") ?? ""),
        text: String(element.data("hoverText") ?? ""),
        x: event.renderedPosition.x,
        y: event.renderedPosition.y,
      });
    });
    cy.on("mouseout", "node, edge", () => latest.current.onHover(null));
    cy.on("viewport", () => {
      latest.current.onViewport({ zoom: cy.zoom(), pan: { ...cy.pan() } });
      latest.current.onHover(null);
      schedule();
    });
    cy.on("render", schedule);
    return () => {
      if (frame) cancelAnimationFrame(frame);
      cy.destroy();
      cyRef.current = null;
    };
  }, []);

  // Draw the level: positions come from the layout engine (a worker), a stale answer is dropped.
  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    sequence.current += 1;
    const mine = sequence.current;
    let alive = true;
    layoutLevel(level, expanded, engine)
      .then((positions) => {
        if (!alive || mine !== sequence.current) return;
        cy.batch(() => {
          cy.elements().remove();
          cy.add(cyElements(level, expanded, details, positions));
        });
        if (!placed.current) {
          placed.current = true;
          const stored = latest.current.viewport;
          if (stored) cy.viewport({ zoom: stored.zoom, pan: stored.pan });
          else cy.fit(undefined, PADDING);
        }
        setVersion((v) => v + 1);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [level, expanded, details, engine]);

  // Classes: the picked element, the focus and what it dims.
  useEffect(() => {
    const cy = cyRef.current;
    if (!cy || version === 0) return;
    const focusedIds = focus ? new Set([...focus.nodes, ...focus.edges]) : null;
    cy.batch(() => {
      cy.elements().forEach((element) => {
        const id = element.id();
        element.toggleClass("picked", id === selected);
        element.toggleClass("focused", focusedIds !== null && focusedIds.has(id));
        element.toggleClass("dim", focusedIds !== null && !focusedIds.has(id));
      });
    });
  }, [version, selected, focus]);

  // Going to a step moves the viewport to contain it.
  useEffect(() => {
    const cy = cyRef.current;
    if (!cy || version === 0 || !fitFocus || !focus) return;
    const ids = new Set([...focus.nodes, ...focus.edges]);
    const eles = cy.elements().filter((element) => ids.has(element.id()));
    if (eles.length > 0) fitContaining(cy, eles);
    latest.current.onFocusFitted();
  }, [version, fitFocus, focus]);

  const first = useRef(true);
  useEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    cyRef.current?.fit(undefined, PADDING);
  }, [fitNonce]);

  useEffect(() => {
    cyRef.current?.style(stylesheet(dark));
  }, [dark]);

  const palette = paletteOf(dark);
  return (
    <div className="relative size-full">
      <div ref={host} aria-hidden className="size-full" data-testid="diagram-canvas" />
      {badges.map((badge) => (
        <span
          key={badge.id}
          aria-hidden
          className="pointer-events-none absolute flex size-5 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full text-[11px] font-semibold"
          style={{ left: badge.x, top: badge.y, backgroundColor: palette.drift, color: dark ? "#18181b" : "#ffffff" }}
        >
          {badge.count}
        </span>
      ))}
    </div>
  );
}
