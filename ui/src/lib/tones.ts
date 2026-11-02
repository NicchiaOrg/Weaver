// Colours that carry a meaning (states, change kinds, label colours, the Drift and resolution
// marks) come from the Tailwind palette: shade 600 in light, 400 in dark. Everything else uses the
// theme tokens. Colour never stands alone: whoever uses a tone also shows a text, an icon and an
// accessible name.

export const TONES = ["blue", "green", "amber", "violet", "cyan", "pink", "red", "yellow"] as const;
export type Tone = (typeof TONES)[number];

// Full class names, so Tailwind sees them.
export const TONE_TEXT: Record<Tone, string> = {
  blue: "text-blue-600 dark:text-blue-400",
  green: "text-green-600 dark:text-green-400",
  amber: "text-amber-600 dark:text-amber-400",
  violet: "text-violet-600 dark:text-violet-400",
  cyan: "text-cyan-600 dark:text-cyan-400",
  pink: "text-pink-600 dark:text-pink-400",
  red: "text-red-600 dark:text-red-400",
  yellow: "text-yellow-600 dark:text-yellow-400",
};

export const TONE_BORDER: Record<Tone, string> = {
  blue: "border-blue-600 dark:border-blue-400",
  green: "border-green-600 dark:border-green-400",
  amber: "border-amber-600 dark:border-amber-400",
  violet: "border-violet-600 dark:border-violet-400",
  cyan: "border-cyan-600 dark:border-cyan-400",
  pink: "border-pink-600 dark:border-pink-400",
  red: "border-red-600 dark:border-red-400",
  yellow: "border-yellow-600 dark:border-yellow-400",
};

/** Meanings of the interface, by tone: success, warning, error, information. */
export const SEMANTIC_TONE = { success: "green", warning: "amber", error: "red", info: "blue" } as const satisfies Record<
  string,
  Tone
>;
export type Semantic = keyof typeof SEMANTIC_TONE;
