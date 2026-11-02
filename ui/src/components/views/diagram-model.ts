import type { Detail } from "@/bindings/Detail";
import type { DiagramEdge } from "@/bindings/DiagramEdge";
import type { DiagramNode } from "@/bindings/DiagramNode";
import type { Level } from "@/bindings/Level";
import type { LevelKind } from "@/bindings/LevelKind";
import type { NodeKind } from "@/bindings/NodeKind";

import { trail } from "@/components/views/focus-model";

/** The key of the root level, `System`. */
export const ROOT = "";

const RANK: Record<LevelKind, number> = { root: 0, system: 1, actor: 1, feature: 2, machine: 3, state: 4 };

/** The path of a level as a link reaches it: `System › <Feature>`. */
export function pathFromLink(levels: readonly Level[], key: string): string[] {
  const path = trail(levels, key).map((l) => l.key);
  return path.length > 0 ? path : [ROOT];
}

/**
 * The path after opening `target` from `path`: it extends the path when it goes deeper, replaces
 * its tail when it opens a level beside the current one (a Feature seen as an outside end), and
 * cuts it back when `target` is already in it.
 */
export function openPath(levels: readonly Level[], path: readonly string[], target: string): string[] {
  const level = levels.find((l) => l.key === target);
  if (!level) return [...path];
  const at = path.indexOf(target);
  if (at >= 0) return path.slice(0, at + 1);
  const kindOf = (key: string) => levels.find((l) => l.key === key)?.kind;
  if (level.kind === "root") return [ROOT];
  if (level.kind === "system" || level.kind === "actor") return [ROOT, target];
  if (level.kind === "feature") {
    return [...path.filter((key) => RANK[kindOf(key) ?? "state"] < RANK.feature), target];
  }
  const parentAt = level.parent === null ? -1 : path.indexOf(level.parent);
  return parentAt >= 0 ? [...path.slice(0, parentAt + 1), target] : pathFromLink(levels, target);
}

/**
 * The path after the Model or the overlay changed: the longest prefix whose levels all still
 * exist (up to the root), and the first level that no longer does.
 */
export function settlePath(levels: readonly Level[], path: readonly string[]): { path: string[]; missing: string | null } {
  const kept: string[] = [ROOT];
  for (const key of path.slice(path[0] === ROOT ? 1 : 0)) {
    if (!levels.some((l) => l.key === key)) return { path: kept, missing: key };
    kept.push(key);
  }
  return { path: kept, missing: null };
}

/** The segments of the breadcrumb: `System › <Actor | Whole System> › <Feature> › <Machine> › <State>`. */
export function breadcrumb(levels: readonly Level[], path: readonly string[]): { key: string; title: string }[] {
  return path.map((key) => ({ key, title: levels.find((l) => l.key === key)?.title ?? key }));
}

/** The nodes and edges a level draws, with expanded Machines showing their States. */
export function visibleNodes(level: Level, expanded: ReadonlySet<string>): DiagramNode[] {
  return level.nodes.filter((n) => n.parent === null || level.kind !== "feature" || expanded.has(n.parent));
}

export function visibleEdges(level: Level, expanded: ReadonlySet<string>): DiagramEdge[] {
  const ids = new Set(visibleNodes(level, expanded).map((n) => n.id));
  return level.edges.filter((e) => (e.inside === null || expanded.has(e.inside)) && ids.has(e.source) && ids.has(e.target));
}

/** What can be selected, in the order the arrow keys walk: nodes, then edges. */
export type Element = { type: "node"; id: string; node: DiagramNode } | { type: "edge"; id: string; edge: DiagramEdge };

export function elementsOf(level: Level, expanded: ReadonlySet<string>): Element[] {
  return [
    ...visibleNodes(level, expanded).map((node) => ({ type: "node" as const, id: node.id, node })),
    ...visibleEdges(level, expanded).map((edge) => ({ type: "edge" as const, id: edge.id, edge })),
  ];
}

const NODE_KIND: Record<NodeKind, string> = {
  "whole-system": "Whole System",
  actor: "Actor",
  feature: "Feature",
  machine: "Machine",
  state: "State",
  "external-actor": "Actor, outside this level",
  "external-machine": "Machine, outside this level",
  "external-state": "State, other end",
};

const drifts = (count: number) => `${count} open ${count === 1 ? "Drift" : "Drifts"}`;

/** The accessible name of a node: its title, kind, subtitle, overlay change and open Drifts, never colour alone. */
export function nodeName(node: DiagramNode): string {
  const parts = [node.title, NODE_KIND[node.kind]];
  if (node.subtitle) parts.push(node.subtitle);
  if (node.initial) parts.push("initial");
  if (node.final) parts.push("final");
  if (node.change) parts.push(node.change);
  if (node.drifts > 0) parts.push(drifts(node.drifts));
  return parts.join(", ");
}

export function edgeName(edge: DiagramEdge): string {
  const parts = [edge.detail.title, edge.label];
  if (edge.change) parts.push(edge.change);
  return parts.filter((p, i) => p !== "" && parts.indexOf(p) === i).join(", ");
}

export function elementName(element: Element): string {
  return element.type === "node" ? nodeName(element.node) : edgeName(element.edge);
}

/** The first sentence of a Markdown text, as plain text. */
export function firstSentence(markdown: string): string {
  const plain = markdown
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/[`*_#>]/g, "")
    .trim();
  const paragraph = plain.split(/\n\s*\n/)[0]?.replace(/\s+/g, " ").trim() ?? "";
  const end = /[.!?](\s|$)/.exec(paragraph);
  return end ? paragraph.slice(0, end.index + 1) : paragraph;
}

/** The hover box: the title and the first sentence of the description (of the Transition's text for an edge). */
export function hoverOf(detail: Detail, edge: boolean): { title: string; text: string } {
  const source = edge ? (detail.transitions[0]?.prose ?? detail.description) : detail.description;
  const title = detail.change ? `${detail.title} (${detail.change})` : detail.title;
  return { title, text: firstSentence(source) };
}

/** The details of a node or an edge, from the Model. */
export function detailOf(details: Readonly<Record<string, Detail | undefined>>, element: Element): Detail | null {
  if (element.type === "edge") return element.edge.detail;
  return element.node.reference ? (details[element.node.reference] ?? null) : null;
}

/** Where the arrow keys go from `from` among `elements`: Right and Down forward, Left and Up back, Home and End to the ends. */
export function stepSelection(elements: readonly Element[], from: string | null, key: string): string | null {
  if (elements.length === 0) return null;
  const at = from === null ? -1 : elements.findIndex((e) => e.id === from);
  switch (key) {
    case "ArrowRight":
    case "ArrowDown":
      return elements[Math.min(at + 1, elements.length - 1)]!.id;
    case "ArrowLeft":
    case "ArrowUp":
      return elements[at <= 0 ? 0 : at - 1]!.id;
    case "Home":
      return elements[0]!.id;
    case "End":
      return elements[elements.length - 1]!.id;
    default:
      return from;
  }
}
