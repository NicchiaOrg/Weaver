import { useCallback, useEffect, useMemo, useRef, type RefObject } from "react";

/** How long a collapsed folder stays under the pointer before it expands. */
export const EXPAND_AFTER_MS = 1000;

/** How near the top or bottom edge of the tree the pointer must be for the tree to scroll. */
export const EDGE_PX = 24;

/** How often the tree scrolls while the pointer is near an edge, and the most it scrolls each time (at the edge itself). */
export const SCROLL_TICK_MS = 16;
export const SCROLL_MAX_PX = 16;

/** Where the pointer is, in the coordinates of the window (CSS pixels). */
export interface Point {
  x: number;
  y: number;
}

/**
 * How far the tree scrolls for a pointer at `point`: negative up, positive down, zero when the
 * pointer is not within `EDGE_PX` of the top or bottom edge of the tree. The nearer the edge (or
 * beyond it), the faster.
 */
export function scrollSpeed(point: Point, rect: Pick<DOMRect, "top" | "bottom" | "left" | "right">): number {
  if (point.x < rect.left || point.x > rect.right) return 0;
  const fromTop = point.y - rect.top;
  const fromBottom = rect.bottom - point.y;
  const speed = (near: number) => Math.max(1, Math.round(SCROLL_MAX_PX * Math.min(1, (EDGE_PX - near) / EDGE_PX)));
  if (fromTop < EDGE_PX && fromTop >= -EDGE_PX) return -speed(fromTop);
  if (fromBottom < EDGE_PX && fromBottom >= -EDGE_PX) return speed(fromBottom);
  return 0;
}

export interface DragAids {
  /**
   * The pointer moved while something is dragged over the tree: `folder` is the folder under it
   * (`null` where there is none, the empty string for the tree's empty space). A collapsed folder
   * that stays under the pointer for 1 s is expanded, and the tree scrolls while the pointer is
   * within 24 px of its top or bottom edge. The same aids serve a row dragged inside the tree and a
   * drop dragged in from outside.
   */
  update: (point: Point, folder: string | null) => void;
  /** The drag ended: no folder waits to expand and the tree stops scrolling. */
  stop: () => void;
}

/** The drag aids of the tree `scroller` shows; `expand` opens a folder, `isExpanded` says whether it is open. */
export function useDragAids(scroller: RefObject<HTMLElement | null>, expand: (path: string) => void, isExpanded: (path: string) => boolean): DragAids {
  const latest = useRef({ expand, isExpanded });
  latest.current = { expand, isExpanded };
  const hover = useRef<{ path: string; timer: number } | null>(null);
  const scrolling = useRef<{ speed: number; timer: number } | null>(null);

  const stopHover = useCallback(() => {
    if (hover.current) window.clearTimeout(hover.current.timer);
    hover.current = null;
  }, []);

  const stopScroll = useCallback(() => {
    if (scrolling.current) window.clearInterval(scrolling.current.timer);
    scrolling.current = null;
  }, []);

  const stop = useCallback(() => {
    stopHover();
    stopScroll();
  }, [stopHover, stopScroll]);

  const update = useCallback(
    (point: Point, folder: string | null) => {
      // A collapsed folder under the pointer: opened once it stayed there for a second.
      const waiting = folder !== null && folder !== "" && !latest.current.isExpanded(folder) ? folder : null;
      if (hover.current && hover.current.path !== waiting) stopHover();
      if (waiting !== null && !hover.current) {
        hover.current = {
          path: waiting,
          timer: window.setTimeout(() => {
            hover.current = null;
            latest.current.expand(waiting);
          }, EXPAND_AFTER_MS),
        };
      }

      // Within reach of an edge of the tree: it scrolls, faster the nearer the edge.
      const node = scroller.current;
      const speed = node ? scrollSpeed(point, node.getBoundingClientRect()) : 0;
      if (speed === 0) {
        stopScroll();
      } else if (scrolling.current) {
        scrolling.current.speed = speed;
      } else {
        const state = {
          speed,
          timer: window.setInterval(() => {
            const target = scroller.current;
            if (target) target.scrollTop += state.speed;
          }, SCROLL_TICK_MS),
        };
        scrolling.current = state;
      }
    },
    [scroller, stopHover, stopScroll],
  );

  useEffect(() => stop, [stop]);

  return useMemo(() => ({ update, stop }), [update, stop]);
}
