// The colour of a label (`requirements/ui#board`): the FNV-1a 32-bit hash of the label's UTF-8
// bytes, modulo 6, picks in order blue, green, amber, violet, cyan, pink.

import type { Tone } from "@/lib/tones";

const LABEL_TONES = ["blue", "green", "amber", "violet", "cyan", "pink"] as const satisfies readonly Tone[];

/** FNV-1a, 32 bits, over the UTF-8 bytes of `text`. */
export function fnv1a32(text: string): number {
  let hash = 0x811c9dc5;
  for (const byte of new TextEncoder().encode(text)) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash >>> 0;
}

/** The tone of a label. */
export function labelTone(label: string): (typeof LABEL_TONES)[number] {
  return LABEL_TONES[fnv1a32(label) % LABEL_TONES.length] ?? "blue";
}
