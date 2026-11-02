import type { TreeEntry } from "@/bindings/TreeEntry";

/** The folders read so far, by path relative to the project folder (`""` is the project folder itself). */
export type Listing = Readonly<Record<string, readonly TreeEntry[]>>;

export const ROOT = "";

/** What the checkbox of an entry shows: everything in it, nothing, or a part. */
export type CheckState = "included" | "excluded" | "mixed";

/** `a/b/c` → `["a", "a/b"]`. */
export function ancestorsOf(path: string): string[] {
  const parts = path.split("/");
  return parts.slice(0, -1).map((_, i) => parts.slice(0, i + 1).join("/"));
}

export function parentOf(path: string): string {
  const cut = path.lastIndexOf("/");
  return cut < 0 ? ROOT : path.slice(0, cut);
}

function within(path: string, folder: string): boolean {
  return path.startsWith(`${folder}/`);
}

/** The state of an entry given the paths the developer unchecked (a folder stands for everything inside it). */
export function stateOf(excluded: ReadonlySet<string>, listing: Listing, entry: TreeEntry): CheckState {
  if (excluded.has(entry.path) || ancestorsOf(entry.path).some((a) => excluded.has(a))) return "excluded";
  if (!entry.folder) return "included";
  const children = listing[entry.path];
  if (children === undefined) {
    for (const path of excluded) if (within(path, entry.path)) return "mixed";
    return "included";
  }
  if (children.length === 0) return "included";
  const states = children.map((child) => stateOf(excluded, listing, child));
  if (states.every((s) => s === "excluded")) return "excluded";
  return states.every((s) => s === "included") ? "included" : "mixed";
}

/** A folder whose every known child is excluded is excluded itself, so the count and the list stay short. */
function normalise(excluded: Set<string>, listing: Listing): Set<string> {
  let changed = true;
  while (changed) {
    changed = false;
    for (const path of [...excluded]) {
      const parent = parentOf(path);
      if (parent === ROOT || excluded.has(parent)) continue;
      const children = listing[parent];
      if (children && children.length > 0 && children.every((child) => excluded.has(child.path))) {
        for (const child of children) excluded.delete(child.path);
        excluded.add(parent);
        changed = true;
      }
    }
  }
  return excluded;
}

/**
 * `Space` on an entry: an included one is excluded with everything inside it; an excluded or partly
 * excluded one is included again with everything inside it. Including an entry inside an excluded
 * folder leaves that folder's other known children excluded.
 */
export function toggle(excluded: ReadonlySet<string>, listing: Listing, entry: TreeEntry): Set<string> {
  const state = stateOf(excluded, listing, entry);
  const next = new Set<string>();
  for (const path of excluded) if (!within(path, entry.path)) next.add(path);
  next.delete(entry.path);
  if (state === "included") {
    next.add(entry.path);
    return normalise(next, listing);
  }
  const chain = [...ancestorsOf(entry.path), entry.path];
  const top = chain.findIndex((path) => path !== entry.path && excluded.has(path));
  if (top >= 0) {
    // Every level from the excluded folder down to the entry keeps its other known children excluded.
    next.delete(chain[top] ?? "");
    chain.slice(top, -1).forEach((folder, i) => {
      for (const sibling of listing[folder] ?? []) if (sibling.path !== chain[top + i + 1]) next.add(sibling.path);
    });
  }
  return normalise(next, listing);
}

/** `Start import` needs something left: a first-level entry that is not excluded. */
export function hasIncluded(excluded: ReadonlySet<string>, listing: Listing): boolean {
  const top = listing[ROOT];
  return top !== undefined && top.some((entry) => stateOf(excluded, listing, entry) !== "excluded");
}

/** `<N> excluded` or `Everything is included`. */
export function excludedLabel(excluded: ReadonlySet<string>): string {
  return excluded.size === 0 ? "Everything is included" : `${excluded.size} excluded`;
}

/** A row of the tree as shown: the entry, its depth (1 for the first level) and its folder's path. */
export interface Row {
  entry: TreeEntry;
  level: number;
  parent: string;
}

/** The rows in view, in order: the children of an open folder follow it. */
export function visibleRows(listing: Listing, open: ReadonlySet<string>, folder: string = ROOT, level = 1): Row[] {
  const rows: Row[] = [];
  for (const entry of listing[folder] ?? []) {
    rows.push({ entry, level, parent: folder });
    if (entry.folder && open.has(entry.path)) rows.push(...visibleRows(listing, open, entry.path, level + 1));
  }
  return rows;
}
