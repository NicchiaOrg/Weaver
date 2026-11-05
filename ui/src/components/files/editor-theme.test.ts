import { describe, expect, it } from "vitest";

import { PALETTES, type EditorPalette } from "@/components/files/editor-theme";

function channel(value: number): number {
  const c = value / 255;
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function luminance(hex: string): number {
  const n = Number.parseInt(hex.slice(1), 16);
  return 0.2126 * channel((n >> 16) & 255) + 0.7152 * channel((n >> 8) & 255) + 0.0722 * channel(n & 255);
}

/** WCAG contrast ratio of two `#rrggbb` colours. */
function contrast(a: string, b: string): number {
  const [light = 0, dark = 0] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

const TEXT: (keyof EditorPalette)[] = [
  "foreground",
  "punctuation",
  "comment",
  "keyword",
  "string",
  "number",
  "definition",
  "type",
  "link",
  "invalid",
];

describe("editor palettes", () => {
  for (const theme of ["light", "dark"] as const) {
    it(`keeps every ${theme} text colour at 4.5:1 or more on the background and on the selection`, () => {
      const palette = PALETTES[theme];
      for (const name of TEXT) {
        expect(contrast(palette[name], palette.background), `${name} on the background`).toBeGreaterThanOrEqual(4.5);
        expect(contrast(palette[name], palette.selection), `${name} on the selection`).toBeGreaterThanOrEqual(4.5);
      }
    });
  }
});
