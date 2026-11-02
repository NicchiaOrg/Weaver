import type cytoscape from "cytoscape";

import type { Detail } from "@/bindings/Detail";
import type { DiagramNode } from "@/bindings/DiagramNode";
import type { Level } from "@/bindings/Level";
import { nodeSize, type Positions } from "@/components/views/diagram-layout";
import { hoverOf, visibleEdges, visibleNodes } from "@/components/views/diagram-model";

/** The text of a node on the canvas: a filled dot before the title of an initial State, then the subtitle. */
export function nodeLabel(node: DiagramNode): string {
  const title = node.initial ? `● ${node.title}` : node.title;
  return node.subtitle && !node.compact ? `${title}\n${node.subtitle}` : title;
}

function nodeClasses(node: DiagramNode): string {
  const classes: string[] = [node.kind];
  if (node.compact) classes.push("compact");
  if (node.dashed) classes.push("dashed");
  if (node.muted) classes.push("muted");
  if (node.final) classes.push("final");
  if (node.change) classes.push(node.change);
  return classes.join(" ");
}

/** The cytoscape elements of a level as it is drawn: expanded Machines show their States inside them. */
export function cyElements(
  level: Level,
  expanded: ReadonlySet<string>,
  details: Readonly<Record<string, Detail | undefined>>,
  positions: Positions,
): cytoscape.ElementDefinition[] {
  const nodes = visibleNodes(level, expanded);
  const shown = new Set(nodes.map((n) => n.id));
  const elements: cytoscape.ElementDefinition[] = nodes.map((node) => {
    const detail = node.reference ? details[node.reference] : undefined;
    const hover = detail ? hoverOf(detail, false) : { title: node.change ? `${node.title} (${node.change})` : node.title, text: "" };
    const { width, height } = nodeSize(node);
    const position = positions.get(node.id);
    return {
      group: "nodes",
      data: {
        id: node.id,
        label: nodeLabel(node),
        width,
        height,
        drifts: node.drifts,
        hoverTitle: hover.title,
        hoverText: hover.text,
        ...(node.parent !== null && shown.has(node.parent) ? { parent: node.parent } : {}),
      },
      classes: nodeClasses(node),
      ...(position ? { position } : {}),
    };
  });
  for (const edge of visibleEdges(level, expanded)) {
    const hover = hoverOf(edge.detail, true);
    elements.push({
      group: "edges",
      data: { id: edge.id, source: edge.source, target: edge.target, label: edge.label, hoverTitle: hover.title, hoverText: hover.text },
      classes: edge.change ?? "",
    });
  }
  return elements;
}
