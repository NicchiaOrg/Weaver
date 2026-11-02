import { describe, expect, it } from "vitest";

import { centres, elkGraph, gridPositions, layoutLevel } from "@/components/views/diagram-layout";
import {
  breadcrumb,
  edgeName,
  elementsOf,
  firstSentence,
  hoverOf,
  nodeName,
  openPath,
  pathFromLink,
  settlePath,
  stepSelection,
  visibleEdges,
  visibleNodes,
} from "@/components/views/diagram-model";
import { cyElements, nodeLabel } from "@/components/views/diagram-graph";
import { detail, edge, levels, node } from "@/components/views/view-fixtures";

describe("the path of levels and the breadcrumb", () => {
  const all = levels();

  it("starts at System and goes through the containers a link starts from", () => {
    expect(pathFromLink(all, "feature:order")).toEqual(["", "feature:order"]);
    expect(pathFromLink(all, "state:cart/Paid")).toEqual(["", "feature:order", "machine:cart", "state:cart/Paid"]);
    expect(breadcrumb(all, pathFromLink(all, "machine:cart")).map((c) => c.title)).toEqual(["System", "order", "cart"]);
  });

  it("extends the path when a level is opened from the one above it", () => {
    expect(openPath(all, [""], "system")).toEqual(["", "system"]);
    expect(openPath(all, ["", "system"], "feature:order")).toEqual(["", "system", "feature:order"]);
    expect(openPath(all, ["", "actor:user", "feature:order"], "machine:cart")).toEqual(["", "actor:user", "feature:order", "machine:cart"]);
  });

  it("opens a Feature seen from another Feature beside it, and cuts back when the level is already in the path", () => {
    expect(openPath(all, ["", "system", "feature:order"], "feature:order")).toEqual(["", "system", "feature:order"]);
    expect(openPath(all, ["", "system", "feature:order", "machine:cart"], "system")).toEqual(["", "system"]);
  });

  it("falls back to the nearest container that still exists", () => {
    const without = all.filter((l) => l.key !== "machine:cart" && !l.key.startsWith("state:"));
    const settled = settlePath(without, ["", "system", "feature:order", "machine:cart", "state:cart/Paid"]);
    expect(settled).toEqual({ path: ["", "system", "feature:order"], missing: "machine:cart" });
    expect(settlePath(all, ["", "system"]).missing).toBeNull();
    expect(settlePath(all.slice(0, 1), ["", "feature:order"]).path).toEqual([""]);
  });
});

describe("what a level draws", () => {
  const feature = levels().find((l) => l.key === "feature:order")!;

  it("shows the States of a Machine only while it is expanded", () => {
    expect(visibleNodes(feature, new Set()).map((n) => n.id)).toEqual(["machine:cart", "machine:payment", "ext-actor:user"]);
    expect(visibleNodes(feature, new Set(["machine:cart"])).map((n) => n.id)).toContain("state:cart/Empty");
    expect(visibleEdges(feature, new Set()).map((e) => e.id)).not.toContain("t:cart/Empty/0");
    expect(visibleEdges(feature, new Set(["machine:cart"])).map((e) => e.id)).toContain("t:cart/Empty/0");
  });

  it("lists nodes and then edges for the keyboard", () => {
    const ids = elementsOf(feature, new Set()).map((e) => e.id);
    expect(ids).toEqual(["machine:cart", "machine:payment", "ext-actor:user", "e:machine:cart>machine:payment", "e:ext-actor:user>machine:cart"]);
  });
});

describe("walking the selection with the arrow keys", () => {
  const elements = elementsOf(levels()[1]!, new Set());

  it("moves forward and back and stops at the ends", () => {
    expect(stepSelection(elements, null, "ArrowRight")).toBe("actor:user");
    expect(stepSelection(elements, "actor:user", "ArrowDown")).toBe("feature:order");
    expect(stepSelection(elements, "feature:order", "ArrowLeft")).toBe("actor:user");
    expect(stepSelection(elements, "actor:user", "ArrowUp")).toBe("actor:user");
    expect(stepSelection(elements, "actor:user", "End")).toBe(elements[elements.length - 1]?.id);
    expect(stepSelection(elements, elements[elements.length - 1]!.id, "ArrowRight")).toBe(elements[elements.length - 1]?.id);
    expect(stepSelection([], null, "ArrowRight")).toBeNull();
  });
});

describe("accessible names and the hover box", () => {
  it("never use colour alone: the change, the Drifts and the kind are words", () => {
    expect(nodeName(node("feature:order", "order", { subtitle: "2 machines", change: "removed", drifts: 2 }))).toBe(
      "order, Feature, 2 machines, removed, 2 open Drifts",
    );
    expect(nodeName(node("s", "Paid", { kind: "state", final: true, drifts: 1 }))).toBe("Paid, State, final, 1 open Drift");
    expect(edgeName(edge("e", "a", "b", "PAY", { change: "added", detail: detail("a → b") }))).toBe("a → b, PAY, added");
  });

  it("is the title and the first sentence, nothing more", () => {
    expect(firstSentence("Everything about one order. It starts in a cart.")).toBe("Everything about one order.");
    expect(firstSentence("**Bold** and [a link](http://x). More.")).toBe("Bold and a link.");
    expect(firstSentence("No full stop here\n\nSecond paragraph.")).toBe("No full stop here");
    expect(hoverOf(detail("order", { description: "One. Two." }), false)).toEqual({ title: "order", text: "One." });
    expect(hoverOf(detail("order"), false)).toEqual({ title: "order", text: "" });
    expect(hoverOf(detail("order", { change: "added", description: "One." }), false).title).toBe("order (added)");
  });

  it("takes the text of the Transition for an edge", () => {
    const transition = { event: "user.PAY", machine: null, from: "A", target: "B", prose: "Takes the payment. Then more.", change: null, file: null, line: null };
    expect(hoverOf(detail("A → B", { description: "ignored", transitions: [transition] }), true).text).toBe("Takes the payment.");
  });
});

describe("the layout of a level", () => {
  const all = levels();

  it("puts the root on a grid with no edges and without the layout engine", async () => {
    const root = all[0]!;
    const positions = await layoutLevel(root, new Set(), () => Promise.reject(new Error("not for the grid")));
    expect([...positions.keys()]).toEqual(["whole", "actor:user"]);
    expect(positions.get("whole")?.x).not.toBe(positions.get("actor:user")?.x);
    expect(gridPositions([]).size).toBe(0);
  });

  it("hands the layered graph to the engine, with States inside the Machine they belong to", async () => {
    const feature = all.find((l) => l.key === "feature:order")!;
    const graph = elkGraph(feature, new Set(["machine:cart"]));
    expect(graph.layoutOptions?.["elk.algorithm"]).toBe("layered");
    expect(graph.layoutOptions?.["elk.direction"]).toBe("RIGHT");
    const cart = graph.children?.find((c) => c.id === "machine:cart");
    expect(cart?.children?.map((c) => c.id)).toEqual(["state:cart/Empty"]);
    expect(graph.edges?.map((e) => e.id)).toContain("t:cart/Empty/0");
  });

  it("turns what the engine answers into the centre of each leaf, in canvas coordinates", () => {
    const positions = centres({
      id: "root",
      children: [
        { id: "a", x: 10, y: 20, width: 100, height: 40 },
        { id: "m", x: 200, y: 0, children: [{ id: "s", x: 5, y: 6, width: 20, height: 10 }] },
      ],
    });
    expect(positions.get("a")).toEqual({ x: 60, y: 40 });
    expect(positions.get("s")).toEqual({ x: 215, y: 11 });
    expect(positions.has("m")).toBe(false);
  });

  it("builds the cytoscape elements with their change classes and hover texts", () => {
    const system = all[1]!;
    const elements = cyElements(system, new Set(), { "feature:order": detail("order", { description: "One. Two." }) }, new Map());
    const order = elements.find((e) => e.data.id === "feature:order");
    expect(order?.data.hoverText).toBe("One.");
    expect(order?.data.label).toBe("order\n2 machines");
    expect(elements.filter((e) => e.group === "edges")).toHaveLength(1);
    expect(nodeLabel(node("s", "Empty", { kind: "state", initial: true }))).toBe("● Empty");
  });
});
