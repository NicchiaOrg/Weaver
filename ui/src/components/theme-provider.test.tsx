import { act, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { resolveTheme, ThemeProvider, useResolvedTheme, type ThemePreference } from "@/components/theme-provider";
import { setSystemDark } from "@/test/media";

function Probe() {
  return <p>theme: {useResolvedTheme()}</p>;
}

function renderWith(preference: ThemePreference) {
  const view = render(
    <ThemeProvider preference={preference}>
      <Probe />
    </ThemeProvider>,
  );
  return (next: ThemePreference) =>
    view.rerender(
      <ThemeProvider preference={next}>
        <Probe />
      </ThemeProvider>,
    );
}

const isDark = () => document.documentElement.classList.contains("dark");

describe("theme", () => {
  it("resolves auto from the system and light or dark as they are", () => {
    expect(resolveTheme("auto", false)).toBe("light");
    expect(resolveTheme("auto", true)).toBe("dark");
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });

  it("follows the system in auto at once, without a reload", () => {
    renderWith("auto");
    expect(isDark()).toBe(false);
    expect(screen.getByText("theme: light")).toBeInTheDocument();

    act(() => setSystemDark(true));
    expect(isDark()).toBe(true);
    expect(document.documentElement.style.colorScheme).toBe("dark");
    expect(screen.getByText("theme: dark")).toBeInTheDocument();

    act(() => setSystemDark(false));
    expect(isDark()).toBe(false);
    expect(screen.getByText("theme: light")).toBeInTheDocument();
  });

  it("applies a change of the setting at once and ignores the system when it is not auto", () => {
    const setPreference = renderWith("auto");
    setPreference("dark");
    expect(isDark()).toBe(true);

    act(() => setSystemDark(true));
    setPreference("light");
    expect(isDark()).toBe(false);
    expect(document.documentElement.style.colorScheme).toBe("light");

    act(() => setSystemDark(false));
    expect(isDark()).toBe(false);
    setPreference("auto");
    expect(isDark()).toBe(false);
    act(() => setSystemDark(true));
    expect(isDark()).toBe(true);
  });
});
