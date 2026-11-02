import { describe, expect, it } from "vitest";

import { fnv1a32, labelTone, LABEL_TONES, leftBorderClass } from "@/lib/label-colour";

describe("label colour", () => {
  it("hashes the UTF-8 bytes with FNV-1a, 32 bits", () => {
    expect(fnv1a32("")).toBe(0x811c9dc5);
    expect(fnv1a32("a")).toBe(0xe40c292c);
    expect(fnv1a32("foobar")).toBe(0xbf9cf968);
    // Bytes, not UTF-16 units: `é` is the two bytes 0xC3 0xA9.
    let expected = 0x811c9dc5;
    for (const byte of [0xc3, 0xa9]) {
      expected = Math.imul(expected ^ byte, 0x01000193) >>> 0;
    }
    expect(fnv1a32("é")).toBe(expected);
  });

  it("picks blue, green, amber, violet, cyan or pink by the hash modulo 6", () => {
    expect(LABEL_TONES).toEqual(["blue", "green", "amber", "violet", "cyan", "pink"]);
    for (const label of ["Cart", "Checkout", "Payments", "a", "é"]) {
      expect(labelTone(label)).toBe(LABEL_TONES[fnv1a32(label) % 6]);
    }
    expect(labelTone("a")).toBe("cyan"); // 0xe40c292c % 6 = 4
    expect(labelTone("Cart")).toBe("pink"); // 5
  });

  it("gives a 3 px left border in the colour of the label, or the border token outside any design", () => {
    expect(leftBorderClass("a")).toBe("border-l-[3px] border-l-cyan-600 dark:border-l-cyan-400");
    expect(leftBorderClass(null)).toBe("border-l-[3px] border-l-border");
  });
});
