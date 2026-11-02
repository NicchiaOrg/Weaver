import cytoscape from "cytoscape";
import { afterEach, describe, expect, it, vi } from "vitest";

import { cyElements } from "@/components/views/diagram-graph";
import { paletteOf, stylesheet } from "@/components/views/diagram-style";
import { details, levels } from "@/components/views/view-fixtures";

afterEach(() => vi.restoreAllMocks());

/** The feature level with an added, a changed and a removed element, drawn headless (no canvas) with the real style. */
function draw(dark: boolean) {
  const feature = levels().find((l) => l.key === "feature:order")!;
  const byId = (id: string) => feature.nodes.find((n) => n.id === id)!;
  byId("machine:cart").change = "added";
  byId("machine:payment").change = "changed";
  byId("ext-actor:user").change = "removed";
  feature.edges[0]!.change = "removed";
  const positions = new Map(feature.nodes.map((n, i) => [n.id, { x: i * 100, y: 0 }]));
  return cytoscape({ headless: true, styleEnabled: true, elements: cyElements(feature, new Set(), details(), positions), style: stylesheet(dark) });
}

describe("the style of the Diagram", () => {
  it("is accepted by cytoscape as written, with no property it does not know", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    draw(false);
    expect(warn).not.toHaveBeenCalled();
  });

  it("draws added green, changed amber and removed red with a dashed border, from the 600 shades in light and 400 in dark", () => {
    for (const dark of [false, true]) {
      const cy = draw(dark);
      const change = paletteOf(dark).change;
      expect(cy.getElementById("machine:cart").style("border-color")).toBe(hex(change.added));
      expect(cy.getElementById("machine:payment").style("border-color")).toBe(hex(change.changed));
      const removed = cy.getElementById("ext-actor:user");
      expect(removed.style("border-color")).toBe(hex(change.removed));
      expect(removed.style("border-style")).toBe("dashed");
      expect(cy.getElementById("e:machine:cart>machine:payment").style("line-style")).toBe("dashed");
    }
    expect(paletteOf(false).change).toEqual({ added: "#16a34a", changed: "#d97706", removed: "#dc2626" });
    expect(paletteOf(true).change).toEqual({ added: "#4ade80", changed: "#fbbf24", removed: "#f87171" });
  });

  it("focuses in cyan, or in the change colour, and drops everything else to 30%", () => {
    const cy = draw(false);
    cy.getElementById("machine:cart").addClass("focused");
    cy.getElementById("machine:payment").addClass("dim");
    const palette = paletteOf(false);
    expect(cy.getElementById("machine:cart").style("border-width")).toBe("5px");
    expect(cy.getElementById("machine:cart").style("border-color")).toBe(hex(palette.change.added));
    cy.getElementById("ext-actor:user").removeClass("removed").addClass("focused");
    expect(cy.getElementById("ext-actor:user").style("border-color")).toBe(hex(palette.cyan));
    expect(cy.getElementById("machine:payment").style("opacity")).toBe("0.3");
  });

  it("shows a final State with a double border", () => {
    const state = levels().find((l) => l.key === "machine:cart")!;
    const cy = cytoscape({
      headless: true,
      styleEnabled: true,
      elements: cyElements(state, new Set(), details(), new Map(state.nodes.map((n, i) => [n.id, { x: i * 100, y: 0 }]))),
      style: stylesheet(false),
    });
    expect(cy.getElementById("state:cart/Paid").style("border-style")).toBe("double");
    expect(cy.getElementById("state:cart/Empty").data("label")).toBe("● Empty");
  });
});

/** cytoscape answers colours as `rgb(r,g,b)`. */
function hex(colour: string): string {
  const n = Number.parseInt(colour.slice(1), 16);
  return `rgb(${n >> 16},${(n >> 8) & 255},${n & 255})`;
}
