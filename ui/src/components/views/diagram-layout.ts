import type { ElkNode } from "elkjs/lib/elk-api";

import type { DiagramNode } from "@/bindings/DiagramNode";
import type { Level } from "@/bindings/Level";
import { visibleEdges, visibleNodes } from "@/components/views/diagram-model";

/** The centre of every node of a level, in the coordinates of the canvas. */
export type Positions = Map<string, { x: number; y: number }>;

/** Lays a graph out; it runs in a worker, so the interface thread never waits for it. */
export type LayoutEngine = (graph: ElkNode) => Promise<ElkNode>;

/** The size a node is laid out with: from its title, smaller for the compact ends of another container. */
export function nodeSize(node: DiagramNode): { width: number; height: number } {
  if (node.compact) return { width: Math.max(96, 7 * node.title.length + 24), height: 32 };
  const width = Math.max(128, 8 * Math.max(node.title.length, node.subtitle?.length ?? 0) + 32);
  return { width, height: node.subtitle ? 64 : 48 };
}

const GAP = 32;

/** The root level: a grid with no edges, as wide as it is tall as far as it can be. */
export function gridPositions(nodes: readonly DiagramNode[]): Positions {
  const positions: Positions = new Map();
  const columns = Math.max(1, Math.ceil(Math.sqrt(nodes.length)));
  const sizes = nodes.map(nodeSize);
  const cell = {
    width: Math.max(0, ...sizes.map((s) => s.width)),
    height: Math.max(0, ...sizes.map((s) => s.height)),
  };
  nodes.forEach((node, index) => {
    positions.set(node.id, {
      x: (index % columns) * (cell.width + GAP) + cell.width / 2,
      y: Math.floor(index / columns) * (cell.height + GAP) + cell.height / 2,
    });
  });
  return positions;
}

const ROOT_OPTIONS = {
  "elk.algorithm": "layered",
  "elk.direction": "RIGHT",
  "elk.hierarchyHandling": "INCLUDE_CHILDREN",
  "elk.spacing.nodeNode": "36",
  "elk.layered.spacing.nodeNodeBetweenLayers": "72",
  "elk.padding": "[top=24,left=24,bottom=24,right=24]",
};

/** The graph elkjs lays out: the visible nodes (States nested in the node that holds them) and edges. */
export function elkGraph(level: Level, expanded: ReadonlySet<string>): ElkNode {
  const nodes = visibleNodes(level, expanded);
  const shown = new Set(nodes.map((n) => n.id));
  const build = (node: DiagramNode): ElkNode => {
    const children = nodes.filter((child) => child.parent === node.id).map(build);
    if (children.length > 0) {
      return { id: node.id, children, layoutOptions: { "elk.padding": "[top=32,left=16,bottom=16,right=16]" } };
    }
    const { width, height } = nodeSize(node);
    return { id: node.id, width, height };
  };
  return {
    id: "root",
    layoutOptions: ROOT_OPTIONS,
    children: nodes.filter((n) => n.parent === null || !shown.has(n.parent)).map(build),
    edges: visibleEdges(level, expanded).map((e) => ({ id: e.id, sources: [e.source], targets: [e.target] })),
  };
}

/** The centres of the leaves of a laid-out graph, in the coordinates of the canvas (a parent follows its children). */
export function centres(graph: ElkNode): Positions {
  const positions: Positions = new Map();
  const walk = (node: ElkNode, dx: number, dy: number) => {
    const x = dx + (node.x ?? 0);
    const y = dy + (node.y ?? 0);
    if (node.children && node.children.length > 0) node.children.forEach((child) => walk(child, x, y));
    else positions.set(node.id, { x: x + (node.width ?? 0) / 2, y: y + (node.height ?? 0) / 2 });
  };
  graph.children?.forEach((child) => walk(child, 0, 0));
  return positions;
}

/** The positions of a level: the grid for the root, else the layered layout of `engine`. */
export async function layoutLevel(level: Level, expanded: ReadonlySet<string>, engine: LayoutEngine): Promise<Positions> {
  if (level.grid) return gridPositions(visibleNodes(level, expanded));
  return centres(await engine(elkGraph(level, expanded)));
}
