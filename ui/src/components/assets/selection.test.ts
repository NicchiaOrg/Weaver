import { describe, expect, it } from "vitest";

import { click, moveActive, NOTHING, only, prune, selectAll, selectNames, target, without } from "@/components/assets/selection";

const ORDER = ["a", "b", "c", "d", "e", "f", "g"];

describe("clicking a tile", () => {
  it("selects that tile alone", () => {
    expect(click(only("a"), ORDER, "c", { toggle: false, range: false })).toEqual({ selected: ["c"], anchor: "c", active: "c" });
  });

  it("adds a tile with the modifier, and removes one that is selected", () => {
    const first = click(only("c"), ORDER, "a", { toggle: true, range: false });
    expect(first.selected).toEqual(["a", "c"]);
    expect(first.anchor).toBe("a");
    const second = click(first, ORDER, "c", { toggle: true, range: false });
    expect(second.selected).toEqual(["a"]);
  });

  it("selects the range from the last clicked tile in grid order, either way", () => {
    const from = only("c");
    expect(click(from, ORDER, "e", { toggle: false, range: true }).selected).toEqual(["c", "d", "e"]);
    expect(click(from, ORDER, "a", { toggle: false, range: true }).selected).toEqual(["a", "b", "c"]);
    // The anchor stays, so the range can be changed.
    const wide = click(from, ORDER, "f", { toggle: false, range: true });
    expect(wide.anchor).toBe("c");
    expect(click(wide, ORDER, "d", { toggle: false, range: true }).selected).toEqual(["c", "d"]);
  });

  it("selects only the tile for a range with no anchor", () => {
    expect(click(NOTHING, ORDER, "d", { toggle: false, range: true }).selected).toEqual(["d"]);
  });
});

describe("the arrow keys", () => {
  it("move one tile sideways and stop at the ends", () => {
    expect(target(ORDER, "c", "right", 3)).toBe("d");
    expect(target(ORDER, "c", "left", 3)).toBe("b");
    expect(target(ORDER, "a", "left", 3)).toBe("a");
    expect(target(ORDER, "g", "right", 3)).toBe("g");
  });

  it("move one row up or down, the row being the columns of the grid", () => {
    expect(target(ORDER, "b", "down", 3)).toBe("e");
    expect(target(ORDER, "e", "up", 3)).toBe("b");
    expect(target(ORDER, "b", "up", 3)).toBe("a");
    expect(target(ORDER, "f", "down", 3)).toBe("g");
  });

  it("go to the first or the last tile, and to the first when none is active", () => {
    expect(target(ORDER, "d", "home", 3)).toBe("a");
    expect(target(ORDER, "d", "end", 3)).toBe("g");
    expect(target(ORDER, null, "down", 3)).toBe("a");
    expect(target([], null, "down", 3)).toBeNull();
  });

  it("select the tile they reach, or with Shift the range from the anchor", () => {
    expect(moveActive(only("b"), ORDER, "right", 3, false)).toEqual({ selected: ["c"], anchor: "c", active: "c" });
    const extended = moveActive(only("b"), ORDER, "down", 3, true);
    expect(extended).toEqual({ selected: ["b", "c", "d", "e"], anchor: "b", active: "e" });
    expect(moveActive(extended, ORDER, "up", 3, true).selected).toEqual(["b"]);
    expect(moveActive(extended, ORDER, "left", 3, true).selected).toEqual(["b", "c", "d"]);
  });

  it("extend from the active tile when nothing was clicked", () => {
    const next = moveActive({ selected: [], anchor: null, active: "c" }, ORDER, "right", 3, true);
    expect(next).toEqual({ selected: ["c", "d"], anchor: "c", active: "d" });
  });
});

describe("select all", () => {
  it("selects every asset", () => {
    expect(selectAll(NOTHING, ORDER).selected).toEqual(ORDER);
    expect(selectAll(only("c"), ORDER).active).toBe("c");
  });

  it("does nothing without assets", () => {
    expect(selectAll(NOTHING, [])).toBe(NOTHING);
  });
});

describe("keeping the selection with the list", () => {
  it("lets an asset that is gone leave the selection, and moves the anchor and the active tile to what is left", () => {
    const held = { selected: ["b", "c"], anchor: "a", active: "c" };
    expect(prune(held, ["b", "c", "d"])).toEqual({ selected: ["b", "c"], anchor: "b", active: "c" });
    expect(prune(held, ["a", "d"])).toEqual({ selected: [], anchor: "a", active: null });
  });

  it("answers the same object when nothing changed", () => {
    const held = only("b");
    expect(prune(held, ORDER)).toBe(held);
  });

  it("takes names out of the selection", () => {
    expect(without({ selected: ["a", "b", "c"], anchor: "a", active: "b" }, ["a", "b"])).toEqual({ selected: ["c"], anchor: "c", active: "c" });
  });

  it("selects names in grid order, the first being active, and drops the unknown ones", () => {
    expect(selectNames(["d", "b", "zz"], ORDER)).toEqual({ selected: ["b", "d"], anchor: "b", active: "b" });
    expect(selectNames(["zz"], ORDER)).toEqual({ selected: [], anchor: null, active: null });
  });
});
