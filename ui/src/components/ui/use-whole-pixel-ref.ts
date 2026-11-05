import * as React from "react";

const EPSILON = 0.001;

/**
 * Keeps the top-left corner of `element` on whole CSS pixels. The dialogs are centred by layout, but
 * half of an odd free space is still a half pixel; this moves the element by the missing fraction
 * with `position: relative` offsets (never a transform, which would blur the text it carries).
 * Returns the function that stops the watching.
 */
function snapToWholePixels(element: HTMLElement): () => void {
  function snap() {
    const rect = element.getBoundingClientRect();
    const dx = Math.round(rect.left) - rect.left;
    const dy = Math.round(rect.top) - rect.top;
    if (Math.abs(dx) > EPSILON) element.style.left = `${(Number.parseFloat(element.style.left) || 0) + dx}px`;
    if (Math.abs(dy) > EPSILON) element.style.top = `${(Number.parseFloat(element.style.top) || 0) + dy}px`;
  }

  snap();
  window.addEventListener("resize", snap);
  const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(snap);
  observer?.observe(element);
  if (element.parentElement) observer?.observe(element.parentElement);
  return () => {
    window.removeEventListener("resize", snap);
    observer?.disconnect();
  };
}

/** A callback ref for a dialog panel: snaps it to whole pixels while mounted and forwards `forwarded`. */
export function useWholePixelRef<T extends HTMLElement>(forwarded: React.ForwardedRef<T>): React.RefCallback<T> {
  const stop = React.useRef<(() => void) | null>(null);
  return React.useCallback(
    (node: T | null) => {
      stop.current?.();
      stop.current = node ? snapToWholePixels(node) : null;
      if (typeof forwarded === "function") forwarded(node);
      else if (forwarded) forwarded.current = node;
    },
    [forwarded],
  );
}
