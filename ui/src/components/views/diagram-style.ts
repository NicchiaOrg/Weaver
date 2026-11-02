import type cytoscape from "cytoscape";

import type { Change } from "@/bindings/Change";

// Colours that carry a meaning come from the Tailwind palette, shade 600 in light and 400 in dark;
// the rest are the zinc tokens of the theme. The canvas cannot read CSS variables (they are
// oklch), so the values are written here. A change also has its text label in the details, the
// hover box and the accessible name: colour never stands alone.

interface Palette {
  text: string;
  muted: string;
  border: string;
  fill: string;
  edge: string;
  selected: string;
  cyan: string;
  change: Record<Change, string>;
  drift: string;
}

const LIGHT: Palette = {
  text: "#18181b",
  muted: "#52525b",
  border: "#71717a",
  fill: "#ffffff",
  edge: "#71717a",
  selected: "#2563eb",
  cyan: "#0891b2",
  change: { added: "#16a34a", changed: "#d97706", removed: "#dc2626" },
  drift: "#dc2626",
};

const DARK: Palette = {
  text: "#fafafa",
  muted: "#a1a1aa",
  border: "#a1a1aa",
  fill: "#18181b",
  edge: "#a1a1aa",
  selected: "#3b82f6",
  cyan: "#22d3ee",
  change: { added: "#4ade80", changed: "#fbbf24", removed: "#f87171" },
  drift: "#f87171",
};

export function paletteOf(dark: boolean): Palette {
  return dark ? DARK : LIGHT;
}

const CHANGES: Change[] = ["added", "changed", "removed"];

/** The cytoscape style of the Diagram in the light or the dark theme. */
export function stylesheet(dark: boolean): cytoscape.StylesheetJson {
  const p = paletteOf(dark);
  const font = "ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, sans-serif";
  const rules: cytoscape.StylesheetJson = [
    {
      selector: "node",
      style: {
        label: "data(label)",
        "text-valign": "center",
        "text-halign": "center",
        "text-wrap": "wrap",
        "text-max-width": "190px",
        "font-family": font,
        "font-size": 12,
        color: p.text,
        "background-color": p.fill,
        "border-width": 1.5,
        "border-color": p.border,
        shape: "round-rectangle",
        width: "data(width)",
        height: "data(height)",
        "underlay-color": p.cyan,
        "underlay-opacity": 0,
        "underlay-padding": 6,
      },
    },
    {
      selector: "node:parent",
      style: { "text-valign": "top", "text-margin-y": 6, "background-opacity": 0.6, padding: "14px" },
    },
    { selector: "node.compact", style: { "font-size": 11 } },
    { selector: "node.dashed", style: { "border-style": "dashed" } },
    { selector: "node.muted", style: { color: p.muted } },
    { selector: "node.final", style: { "border-style": "double", "border-width": 5 } },
    {
      selector: "edge",
      style: {
        width: 1.5,
        "line-color": p.edge,
        "target-arrow-color": p.edge,
        "target-arrow-shape": "triangle",
        "curve-style": "bezier",
        label: "data(label)",
        "font-family": font,
        "font-size": 11,
        color: p.text,
        "text-background-color": p.fill,
        "text-background-opacity": 1,
        "text-background-padding": "2px",
        "text-rotation": "autorotate",
        "text-wrap": "ellipsis",
        "text-max-width": "220px",
        "underlay-color": p.cyan,
        "underlay-opacity": 0,
        "underlay-padding": 4,
      },
    },
    ...CHANGES.flatMap((change): cytoscape.StylesheetJson => {
      const colour = p.change[change];
      return [
        { selector: `node.${change}`, style: { "border-color": colour, "border-width": 3, ...(change === "removed" ? { "border-style": "dashed" as const } : {}) } },
        {
          selector: `edge.${change}`,
          style: { "line-color": colour, "target-arrow-color": colour, ...(change === "removed" ? { "line-style": "dashed" as const } : {}) },
        },
        { selector: `.focused.${change}`, style: { "underlay-color": colour, "underlay-opacity": 0.18 } },
      ];
    }),
    { selector: "node.focused", style: { "border-width": 5, "border-color": p.cyan, "underlay-opacity": 0.18 } },
    { selector: "edge.focused", style: { width: 3.5, "line-color": p.cyan, "target-arrow-color": p.cyan, "underlay-opacity": 0.18 } },
    ...CHANGES.map((change): cytoscape.StylesheetStyle => ({
      selector: `node.focused.${change}`,
      style: { "border-color": p.change[change] },
    })),
    ...CHANGES.map((change): cytoscape.StylesheetStyle => ({
      selector: `edge.focused.${change}`,
      style: { "line-color": p.change[change], "target-arrow-color": p.change[change] },
    })),
    { selector: ".dim", style: { opacity: 0.3 } },
    { selector: "node.picked", style: { "border-color": p.selected, "border-width": 4 } },
    { selector: "edge.picked", style: { width: 4, "line-color": p.selected, "target-arrow-color": p.selected } },
  ];
  return rules;
}
