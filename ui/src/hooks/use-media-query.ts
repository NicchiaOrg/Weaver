import { useSyncExternalStore } from "react";

/** Whether a CSS media query matches now; follows it live. */
export function useMediaQuery(query: string): boolean {
  return useSyncExternalStore(
    (notify) => {
      const media = window.matchMedia(query);
      media.addEventListener("change", notify);
      return () => media.removeEventListener("change", notify);
    },
    () => window.matchMedia(query).matches,
  );
}

/** Width, in CSS pixels, below which the sidebar shows icons only. */
export const COMPACT_SIDEBAR_BELOW = 1024;

/** The window is narrower than 1024 px: the sidebar shows icons only. */
export function useCompactSidebar(): boolean {
  return useMediaQuery(`(max-width: ${COMPACT_SIDEBAR_BELOW - 0.02}px)`);
}
