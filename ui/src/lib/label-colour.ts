import type { Tone } from "@/lib/tones";

/** The colours a label can take, in the order the hash picks them. */
export const LABEL_TONES = ["blue", "green", "amber", "violet", "cyan", "pink"] as const satisfies readonly Tone[];
export type LabelTone = (typeof LABEL_TONES)[number];

/** The 32-bit FNV-1a hash of the UTF-8 bytes of a text. */
export function fnv1a32(text: string): number {
  let hash = 0x811c9dc5;
  for (const byte of new TextEncoder().encode(text)) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash >>> 0;
}

/** The colour of a label: the hash modulo 6 picks blue, green, amber, violet, cyan or pink. */
export function labelTone(label: string): LabelTone {
  return LABEL_TONES[fnv1a32(label) % LABEL_TONES.length]!;
}

// Full class names, so Tailwind sees them: 3 px left border, shade 600 in light and 400 in dark.
const LEFT_BORDER: Record<LabelTone, string> = {
  blue: "border-l-blue-600 dark:border-l-blue-400",
  green: "border-l-green-600 dark:border-l-green-400",
  amber: "border-l-amber-600 dark:border-l-amber-400",
  violet: "border-l-violet-600 dark:border-l-violet-400",
  cyan: "border-l-cyan-600 dark:border-l-cyan-400",
  pink: "border-l-pink-600 dark:border-l-pink-400",
};

/** The left border of a card: in the label's colour, or the `border` token outside any design. */
export function leftBorderClass(label: string | null): string {
  return `border-l-[3px] ${label ? LEFT_BORDER[labelTone(label)] : "border-l-border"}`;
}
