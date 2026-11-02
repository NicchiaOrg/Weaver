import { createContext, useContext, useLayoutEffect, useSyncExternalStore, type ReactNode } from "react";

/** The Theme setting: `auto` follows the system. */
export type ThemePreference = "auto" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

const DARK_QUERY = "(prefers-color-scheme: dark)";

export function resolveTheme(preference: ThemePreference, systemDark: boolean): ResolvedTheme {
  if (preference === "auto") return systemDark ? "dark" : "light";
  return preference;
}

/** Whether the system asks for a dark appearance; follows it live. */
export function useSystemDark(): boolean {
  return useSyncExternalStore(
    (notify) => {
      const media = window.matchMedia(DARK_QUERY);
      media.addEventListener("change", notify);
      return () => media.removeEventListener("change", notify);
    },
    () => window.matchMedia(DARK_QUERY).matches,
  );
}

const ThemeContext = createContext<ResolvedTheme>("light");

/** The theme in force (`light` or `dark`), for what is drawn outside CSS (the mermaid diagrams). */
export function useResolvedTheme(): ResolvedTheme {
  return useContext(ThemeContext);
}

/**
 * Applies the Theme setting to the window: the `dark` class of `<html>` (the shadcn/ui tokens of
 * `index.css` switch on it) and `color-scheme` (scroll bars, form controls). The `preference` is
 * what the backend holds in settings; it comes with every settings change, which the backend sends
 * to every window, so a change applies at once everywhere and a system change applies at once in
 * `auto`, with no reload.
 */
export function ThemeProvider({ preference, children }: { preference: ThemePreference; children: ReactNode }) {
  const systemDark = useSystemDark();
  const theme = resolveTheme(preference, systemDark);

  useLayoutEffect(() => {
    const root = document.documentElement;
    root.classList.toggle("dark", theme === "dark");
    root.style.colorScheme = theme;
  }, [theme]);

  return <ThemeContext.Provider value={theme}>{children}</ThemeContext.Provider>;
}
