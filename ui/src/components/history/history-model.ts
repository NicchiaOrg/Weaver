// What the History view works out from the entries the backend sends (newest first): the period
// filter and the grouping by local day. Times are local; the backend holds UTC instants.

import type { HistoryEntry } from "@/bindings/HistoryEntry";
import { formatDate } from "@/lib/format";

export type Period = "today" | "last-7-days" | "last-30-days" | "all";

export interface PeriodChoice {
  id: Period;
  label: string;
  /** Days before today where the period starts (`null`: no start). */
  daysBack: number | null;
}

/** The choices of the period filter, in order. A period has both bounds: its first local midnight and now. */
export const PERIODS: readonly PeriodChoice[] = [
  { id: "today", label: "Today", daysBack: 0 },
  { id: "last-7-days", label: "Last 7 days", daysBack: 6 },
  { id: "last-30-days", label: "Last 30 days", daysBack: 29 },
  { id: "all", label: "All", daysBack: null },
];

/** The period shown at each opening of the window. */
export const DEFAULT_PERIOD: Period = "last-7-days";

/** Local midnight of the day `daysBack` days before the day of `time`. */
export function localMidnight(time: number, daysBack = 0): number {
  const date = new Date(time);
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() - daysBack).getTime();
}

/** Whether an entry dated `time` is in `period` at `now`: from the first local midnight to now; a future date only in `all`. */
export function inPeriod(time: number, period: Period, now: number): boolean {
  const choice = PERIODS.find((candidate) => candidate.id === period);
  if (!choice || choice.daysBack === null) return true;
  return time >= localMidnight(now, choice.daysBack) && time <= now;
}

/** The date of an entry: when it was written or last updated. */
export function entryTime(entry: Pick<HistoryEntry, "updated">): number {
  return Date.parse(entry.updated);
}

export interface DayGroup {
  /** Local midnight of the day, as the key of the group. */
  day: number;
  /** `Today · 2`, `Yesterday · 1`, `Monday, 2 November 2026 · 3`. */
  header: string;
  /** The time `HH:MM` is shown on the rows of Today and Yesterday only. */
  showTime: boolean;
  entries: HistoryEntry[];
}

function dayName(day: number, now: number): { name: string; showTime: boolean } {
  if (day === localMidnight(now)) return { name: "Today", showTime: true };
  if (day === localMidnight(now, 1)) return { name: "Yesterday", showTime: true };
  return { name: formatDate(day), showTime: false };
}

/** The entries of `period`, grouped by local day in the order they come (newest first). */
export function groupByDay(entries: readonly HistoryEntry[], period: Period, now: number): DayGroup[] {
  const groups = new Map<number, DayGroup>();
  for (const entry of entries) {
    const time = entryTime(entry);
    if (!inPeriod(time, period, now)) continue;
    const day = localMidnight(time);
    let group = groups.get(day);
    if (!group) {
      group = { day, header: "", showTime: false, entries: [] };
      groups.set(day, group);
    }
    group.entries.push(entry);
  }
  return [...groups.values()].map((group) => {
    const { name, showTime } = dayName(group.day, now);
    return { ...group, header: `${name} · ${group.entries.length}`, showTime };
  });
}

/** A summary without its inline Markdown marks, for the one-line row. */
export function plainSummary(summary: string): string {
  return summary
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/(\*\*|__|~~|[*_`])/g, "")
    .trim();
}
