/**
 * The selection of the Assets grid (`requirements/ui#asset-view`, **Selection**), as pure
 * functions over the names of the assets in grid order. The grid keeps one of these per window, in
 * memory only: choosing another view forgets it.
 */
export interface Selection {
  /** The selected names, in grid order. */
  selected: readonly string[];
  /** Where a `Shift` range starts: the tile last clicked (or reached without `Shift`). */
  anchor: string | null;
  /** The tile the arrow keys move and the keyboard acts on; it owns the focus of the grid. */
  active: string | null;
}

export const NOTHING: Selection = { selected: [], anchor: null, active: null };

/** The names of `order` that `names` holds, in grid order. */
const inOrder = (order: readonly string[], names: ReadonlySet<string>): string[] => order.filter((name) => names.has(name));

/** The names from `from` to `to` in grid order, both included, whichever comes first. */
function between(order: readonly string[], from: string, to: string): string[] {
  const a = order.indexOf(from);
  const b = order.indexOf(to);
  if (a < 0 || b < 0) return [to];
  return order.slice(Math.min(a, b), Math.max(a, b) + 1);
}

/** Only `name` is selected. */
export function only(name: string): Selection {
  return { selected: [name], anchor: name, active: name };
}

/** How a click on a tile was made. */
export interface ClickModifiers {
  /** `Cmd` on macOS, `Ctrl` elsewhere: adds or removes the tile. */
  toggle: boolean;
  /** `Shift`: selects the range from the anchor. */
  range: boolean;
}

/** A click on the tile `name`: one tile, one added or removed, or a range from the last clicked one. */
export function click(current: Selection, order: readonly string[], name: string, modifiers: ClickModifiers): Selection {
  if (modifiers.range && current.anchor !== null && order.includes(current.anchor)) {
    return { selected: between(order, current.anchor, name), anchor: current.anchor, active: name };
  }
  if (modifiers.toggle) {
    const held = new Set(current.selected);
    if (!held.delete(name)) held.add(name);
    return { selected: inOrder(order, held), anchor: name, active: name };
  }
  return only(name);
}

/** The keys that move the active tile. */
export type Move = "left" | "right" | "up" | "down" | "home" | "end";

/**
 * The tile an arrow key goes to from `from`: one step sideways, one row (`columns` tiles) up or
 * down, the first or the last. `null` when there is nowhere to go. Without an active tile the
 * first one is reached.
 */
export function target(order: readonly string[], from: string | null, move: Move, columns: number): string | null {
  if (order.length === 0) return null;
  const at = from === null ? -1 : order.indexOf(from);
  if (at < 0) return order[0] ?? null;
  const last = order.length - 1;
  const step = Math.max(1, columns);
  const to = { left: at - 1, right: at + 1, up: at - step, down: at + step, home: 0, end: last }[move];
  // Down from the last row stays where it is; sideways stops at the ends.
  return order[Math.min(last, Math.max(0, to))] ?? null;
}

/** The arrow key `move`: the active tile goes there and is the only one selected, or with `extend` (`Shift`) the range from the anchor. */
export function moveActive(current: Selection, order: readonly string[], move: Move, columns: number, extend: boolean): Selection {
  const to = target(order, current.active, move, columns);
  if (to === null) return current;
  if (!extend) return only(to);
  const anchor = current.anchor !== null && order.includes(current.anchor) ? current.anchor : (current.active ?? to);
  return { selected: between(order, anchor, to), anchor, active: to };
}

/** `Cmd+A` / `Ctrl+A`: every asset. */
export function selectAll(current: Selection, order: readonly string[]): Selection {
  if (order.length === 0) return current;
  return { selected: [...order], anchor: current.anchor ?? order[0] ?? null, active: current.active ?? order[0] ?? null };
}

/** `names` are the selection, those of them that are in `order`; the first one is the active tile. */
export function selectNames(names: readonly string[], order: readonly string[]): Selection {
  const held = inOrder(order, new Set(names));
  const first = held[0] ?? null;
  return { selected: held, anchor: first, active: first };
}

/**
 * The selection once the list of assets is `order`: an asset that is gone leaves it, and the
 * anchor and the active tile fall back to what is left. The same object comes back when nothing changed.
 */
export function prune(current: Selection, order: readonly string[]): Selection {
  const present = new Set(order);
  const selected = current.selected.filter((name) => present.has(name));
  const anchor = current.anchor !== null && present.has(current.anchor) ? current.anchor : (selected[0] ?? null);
  const active = current.active !== null && present.has(current.active) ? current.active : (selected[0] ?? null);
  if (selected.length === current.selected.length && anchor === current.anchor && active === current.active) return current;
  return { selected, anchor, active };
}

/** `name` is gone from the selection (renamed away, trashed). */
export function without(current: Selection, names: readonly string[]): Selection {
  const gone = new Set(names);
  const selected = current.selected.filter((name) => !gone.has(name));
  return {
    selected,
    anchor: current.anchor !== null && gone.has(current.anchor) ? (selected[0] ?? null) : current.anchor,
    active: current.active !== null && gone.has(current.active) ? (selected[0] ?? null) : current.active,
  };
}
