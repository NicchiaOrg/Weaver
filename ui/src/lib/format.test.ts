import { describe, expect, it } from "vitest";

import { formatDate, formatDuration, formatRunning, formatStarted, formatTime } from "@/lib/format";

// Local-time dates, so the tests hold in any time zone.
const at = (y: number, mo: number, d: number, h = 0, mi = 0, s = 0) => new Date(y, mo - 1, d, h, mi, s);

describe("times and dates", () => {
  it("writes the time as HH:MM on 24 hours", () => {
    expect(formatTime(at(2026, 11, 2, 7, 5))).toBe("07:05");
    expect(formatTime(at(2026, 11, 2, 0, 0))).toBe("00:00");
    expect(formatTime(at(2026, 11, 2, 23, 59))).toBe("23:59");
  });

  it("writes the date out in English", () => {
    expect(formatDate(at(2026, 11, 2))).toBe("Monday, 2 November 2026");
    expect(formatDate(at(2027, 1, 31))).toBe("Sunday, 31 January 2027");
  });

  it("writes durations in words", () => {
    expect(formatDuration(0)).toBe("less than a minute");
    expect(formatDuration(59_999)).toBe("less than a minute");
    expect(formatDuration(60_000)).toBe("1 min");
    expect(formatDuration(59 * 60_000 + 59_000)).toBe("59 min");
    expect(formatDuration(60 * 60_000)).toBe("1 h 0 min");
    expect(formatDuration(135 * 60_000)).toBe("2 h 15 min");
  });

  it("writes the start of a job as HH:MM today and D MMM HH:MM otherwise", () => {
    const now = at(2026, 11, 2, 18, 0);
    expect(formatStarted(at(2026, 11, 2, 7, 5), now)).toBe("07:05");
    expect(formatStarted(at(2026, 11, 1, 23, 59), now)).toBe("1 Nov 23:59");
    expect(formatStarted(at(2025, 11, 2, 7, 5), now)).toBe("2 Nov 07:05");
  });

  it("writes a running time as 45s, 4m 05s, 1h 12m", () => {
    expect(formatRunning(45_000)).toBe("45s");
    expect(formatRunning(245_000)).toBe("4m 05s");
    expect(formatRunning(72 * 60_000)).toBe("1h 12m");
    expect(formatRunning(-5)).toBe("0s");
  });
});
