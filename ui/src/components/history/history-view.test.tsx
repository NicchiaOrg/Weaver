import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent, { type UserEvent } from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { HistoryEntry } from "@/bindings/HistoryEntry";
import type { HistorySnapshot } from "@/bindings/HistorySnapshot";
import type { HistoryTask } from "@/bindings/HistoryTask";
import { BoardActionsContext } from "@/components/board/board-ui";
import { groupByDay, inPeriod, localMidnight, plainSummary } from "@/components/history/history-model";
import { HistoryView } from "@/components/history/history-view";
import { ProjectContext } from "@/components/project-context";
import { TooltipProvider } from "@/components/ui/tooltip";
import { call, on } from "@/lib/backend";
import { fnv1a32, labelTone } from "@/lib/label-tone";

vi.mock("@/lib/backend", () => ({
  call: vi.fn(),
  on: vi.fn(),
  isAppError: (value: unknown) => typeof value === "object" && value !== null && "code" in value && "message" in value,
}));

const PROJECT = "0123456789abcdef";

/** Monday, 2 November 2026, 12:00, local time: the tests never depend on the time zone of the machine. */
const NOW = new Date(2026, 10, 2, 12, 0).getTime();
const local = (month: number, day: number, hour: number, minute: number) => new Date(2026, month - 1, day, hour, minute);
const instant = (date: Date) => date.toISOString();

function task(id: string, title: string, patch: Partial<HistoryTask> = {}): HistoryTask {
  return { id, title, label: null, finished: instant(local(11, 2, 8, 5)), refs: [], body: "", ...patch };
}

function entry(id: string, title: string, updated: Date, patch: Partial<HistoryEntry> = {}): HistoryEntry {
  return {
    id,
    title,
    summary: `Summary of ${title}`,
    text: "",
    created: instant(updated),
    updated: instant(updated),
    tasks: [task(`${id.slice(0, 4)}task1`, "A Task")],
    ...patch,
  };
}

let current: HistorySnapshot;
let handlers: Map<string, (payload: unknown) => void>;

function install(entries: HistoryEntry[], revision = 1) {
  current = { projectId: PROJECT, revision, entries };
  handlers = new Map();
  vi.mocked(on).mockImplementation(((name: string, handler: (payload: unknown) => void) => {
    handlers.set(name, handler);
    return Promise.resolve(() => {});
  }) as unknown as typeof on);
  vi.mocked(call).mockImplementation(((name: string) => {
    if (name === "history_snapshot") return Promise.resolve(current);
    return Promise.reject(new Error(`unexpected command ${name}`));
  }) as unknown as typeof call);
}

async function show() {
  render(
    <TooltipProvider>
      <ProjectContext.Provider value={{ projectId: PROJECT, showView: vi.fn(), openJob: vi.fn(), openedJob: null }}>
        <HistoryView now={() => NOW} />
      </ProjectContext.Provider>
    </TooltipProvider>,
  );
  await screen.findByRole("combobox", { name: "Period" });
}

async function choosePeriod(user: UserEvent, name: string) {
  await user.click(screen.getByRole("combobox", { name: "Period" }));
  await user.click(await screen.findByRole("option", { name }));
}

const group = (name: string) => screen.getByRole("region", { name });
const titles = () => screen.queryAllByRole("button").map((button) => button.textContent ?? "");

beforeEach(() => {
  vi.mocked(call).mockReset();
  vi.mocked(on).mockReset();
});

describe("the periods", () => {
  it("run from the local midnight of their first day to now: today, 6 days ago, 29 days ago", () => {
    const at = (month: number, day: number, hour: number, minute: number) => local(month, day, hour, minute).getTime();
    expect(inPeriod(at(11, 2, 0, 0), "today", NOW)).toBe(true);
    expect(inPeriod(at(11, 1, 23, 59), "today", NOW)).toBe(false);

    expect(inPeriod(at(10, 27, 0, 0), "last-7-days", NOW)).toBe(true);
    expect(inPeriod(at(10, 26, 23, 59), "last-7-days", NOW)).toBe(false);

    expect(inPeriod(at(10, 4, 0, 0), "last-30-days", NOW)).toBe(true);
    expect(inPeriod(at(10, 3, 23, 59), "last-30-days", NOW)).toBe(false);

    expect(inPeriod(at(1, 1, 0, 0), "all", NOW)).toBe(true);
  });

  it("show an entry dated in the future only under All", () => {
    const future = NOW + 60 * 60 * 1000;
    for (const period of ["today", "last-7-days", "last-30-days"] as const) expect(inPeriod(future, period, NOW)).toBe(false);
    expect(inPeriod(future, "all", NOW)).toBe(true);
    expect(inPeriod(NOW, "today", NOW)).toBe(true);
  });

  it("count whole local days across a change of clock", () => {
    // The midnight of a day is the start of that local day whatever its length in hours.
    expect(new Date(localMidnight(NOW)).getHours()).toBe(0);
    expect(new Date(localMidnight(NOW, 6)).getDate()).toBe(27);
  });
});

describe("the grouping", () => {
  it("names Today, Yesterday and then the long date, each with its count, keeping the order of the backend", () => {
    const entries = [
      entry("aaaaaaaa", "One", local(11, 2, 11, 0)),
      entry("bbbbbbbb", "Two", local(11, 2, 9, 30)),
      entry("cccccccc", "Three", local(11, 1, 18, 0)),
      entry("dddddddd", "Four", local(10, 30, 10, 0)),
    ];
    const groups = groupByDay(entries, "all", NOW);
    expect(groups.map((g) => g.header)).toEqual(["Today · 2", "Yesterday · 1", "Friday, 30 October 2026 · 1"]);
    expect(groups.map((g) => g.showTime)).toEqual([true, true, false]);
    expect(groups[0]?.entries.map((e) => e.title)).toEqual(["One", "Two"]);
  });

  it("writes the summary on one line without its inline marks", () => {
    expect(plainSummary("**Bold** and `code` with [a link](https://example.com)")).toBe("Bold and code with a link");
  });
});

describe("the label colour", () => {
  it("is the FNV-1a 32-bit hash of the UTF-8 bytes modulo 6, in the order blue, green, amber, violet, cyan, pink", () => {
    expect(fnv1a32("")).toBe(0x811c9dc5);
    expect(fnv1a32("a")).toBe(0xe40c292c);
    expect(labelTone("")).toBe("green");
    expect(labelTone("a")).toBe("cyan");
  });
});

describe("HistoryView", () => {
  it("has the view header with the title History and the Period filter on its right", async () => {
    install([entry("aaaaaaaa", "Checkout works", local(11, 2, 9, 30))]);
    await show();
    const header = screen.getByRole("heading", { level: 1, name: "History" }).closest("header")!;
    expect(within(header).getByRole("combobox", { name: "Period" })).toBeInTheDocument();
  });

  it("groups the entries by local day under Today, Yesterday and the long date, and shows the time only in the first two", async () => {
    install([
      entry("aaaaaaaa", "Checkout works", local(11, 2, 9, 30)),
      entry("bbbbbbbb", "Cart totals", local(11, 2, 8, 15), { tasks: [task("bbbb0001", "One"), task("bbbb0002", "Two")] }),
      entry("cccccccc", "Search", local(11, 1, 18, 0)),
      entry("dddddddd", "Emails", local(10, 30, 10, 0)),
    ]);
    await show();

    expect(within(group("Today · 2")).getAllByRole("listitem")).toHaveLength(2);
    expect(group("Yesterday · 1")).toBeInTheDocument();
    expect(group("Friday, 30 October 2026 · 1")).toBeInTheDocument();

    const today = group("Today · 2");
    expect(within(today).getByText("09:30")).toBeInTheDocument();
    expect(within(today).getByText("08:15")).toBeInTheDocument();
    expect(within(today).getByText("2 Tasks")).toBeInTheDocument();
    expect(within(today).getByText("1 Task")).toBeInTheDocument();
    expect(within(group("Yesterday · 1")).getByText("18:00")).toBeInTheDocument();
    expect(within(group("Friday, 30 October 2026 · 1")).queryByText("10:00")).not.toBeInTheDocument();
  });

  it("shows Last 7 days first and filters by the period chosen, entries in the future only under All", async () => {
    const user = userEvent.setup();
    install([
      entry("aaaaaaaa", "Now", local(11, 2, 11, 0)),
      entry("bbbbbbbb", "Soon", new Date(NOW + 2 * 60 * 60 * 1000)),
      entry("cccccccc", "Yesterday", local(11, 1, 18, 0)),
      entry("dddddddd", "Edge of the week", local(10, 27, 0, 0)),
      entry("eeeeeeee", "Just outside the week", local(10, 26, 23, 59)),
      entry("ffffffff", "Edge of the month", local(10, 4, 0, 0)),
      entry("gggggggg", "Long ago", local(8, 15, 12, 0)),
    ]);
    await show();
    expect(screen.getByRole("combobox", { name: "Period" })).toHaveTextContent("Last 7 days");
    expect(titles().join("|")).toMatch(/Now.*Yesterday.*Edge of the week/);
    expect(screen.queryByText("Soon")).not.toBeInTheDocument();
    expect(screen.queryByText("Just outside the week")).not.toBeInTheDocument();

    await choosePeriod(user, "Today");
    expect(screen.getByText("Now")).toBeInTheDocument();
    expect(screen.queryByText("Yesterday")).not.toBeInTheDocument();

    await choosePeriod(user, "Last 30 days");
    expect(screen.getByText("Just outside the week")).toBeInTheDocument();
    expect(screen.getByText("Edge of the month")).toBeInTheDocument();
    expect(screen.queryByText("Long ago")).not.toBeInTheDocument();
    expect(screen.queryByText("Soon")).not.toBeInTheDocument();

    await choosePeriod(user, "All");
    expect(screen.getByText("Long ago")).toBeInTheDocument();
    expect(screen.getByText("Soon")).toBeInTheDocument();
  });

  it("says so when the period has no entry", async () => {
    const user = userEvent.setup();
    install([entry("aaaaaaaa", "Old", local(9, 1, 12, 0))]);
    await show();
    expect(screen.getByText("Nothing done in this period")).toBeInTheDocument();
    await choosePeriod(user, "All");
    expect(screen.queryByText("Nothing done in this period")).not.toBeInTheDocument();
  });

  it("gives the row the title, the one-line summary, the time and the number of Tasks, and the whole summary as tooltip", async () => {
    const user = userEvent.setup();
    const summary = "A very long summary that the row has to cut but that the tooltip shows whole, to the last word.";
    install([entry("aaaaaaaa", "Checkout works", local(11, 2, 9, 30), { summary })]);
    await show();
    const row = screen.getByRole("button", { name: /Checkout works/ });
    expect(row).toHaveTextContent(summary);
    expect(within(row).getByText(summary)).toHaveClass("truncate");
    await user.hover(row);
    expect(await screen.findByRole("tooltip")).toHaveTextContent(summary);
  });

  it("opens an entry in a Dialog with its title, date, summary, rendered text and its Tasks from the copies, in the order cited", async () => {
    const user = userEvent.setup();
    install([
      entry("aaaaaaaa", "Checkout works", local(11, 2, 9, 30), {
        summary: "Orders can be paid",
        text: "The **checkout** now works.",
        tasks: [
          task("bbbbbbbb", "Pay with a card", { label: "Checkout", finished: instant(local(11, 2, 8, 5)), body: "Card payments **succeed**.", refs: ["order/Paid", "requirements/ui#order"] }),
          task("cccccccc", "Receipt", { finished: instant(local(11, 1, 17, 40)) }),
        ],
      }),
    ]);
    await show();
    await user.click(screen.getByRole("button", { name: /Checkout works/ }));

    const dialog = await screen.findByRole("dialog", { name: "Checkout works" });
    expect(within(dialog).getByText("Monday, 2 November 2026, 09:30")).toBeInTheDocument();
    expect(within(dialog).getByText("Orders can be paid")).toBeInTheDocument();
    expect(within(dialog).getByText("checkout", { selector: "strong" })).toBeInTheDocument();

    const rows = within(dialog).getAllByRole("listitem").filter((item) => item.querySelector("button[aria-expanded]"));
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent("bbbbbbbb");
    expect(rows[0]).toHaveTextContent("Pay with a card");
    expect(rows[0]).toHaveTextContent("Checkout");
    expect(rows[0]).toHaveTextContent("Finished 08:05");
    expect(rows[1]).toHaveTextContent("cccccccc");
    expect(rows[1]).toHaveTextContent("Receipt");
    expect(rows[1]).toHaveTextContent(/Finished 1 Nov 17:40/);

    // A row expands to the rendered body and the refs; nothing in it can be edited.
    const expander = within(rows[0] as HTMLElement).getByRole("button");
    expect(expander).toHaveAttribute("aria-expanded", "false");
    expect(within(rows[0] as HTMLElement).queryByText("Card payments", { exact: false })).not.toBeVisible();
    await user.click(expander);
    expect(expander).toHaveAttribute("aria-expanded", "true");
    expect(within(rows[0] as HTMLElement).getByText("succeed", { selector: "strong" })).toBeVisible();
    expect(within(rows[0] as HTMLElement).getByText("order/Paid")).toBeVisible();
    expect(within(dialog).queryByRole("textbox")).not.toBeInTheDocument();

    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(screen.getByRole("button", { name: /Checkout works/ })).toHaveFocus();
  });

  it("asks the backend again when a project-changed event names the history slice, and not for other slices", async () => {
    install([entry("aaaaaaaa", "First", local(11, 2, 9, 30))]);
    await show();
    const asked = () => vi.mocked(call).mock.calls.filter(([name]) => name === "history_snapshot").length;
    expect(asked()).toBe(1);

    await act(async () => handlers.get("project-changed")?.({ projectId: PROJECT, revision: 2, slices: ["board"] }));
    expect(asked()).toBe(1);

    current = { projectId: PROJECT, revision: 3, entries: [entry("bbbbbbbb", "Second", local(11, 2, 10, 0)), ...current.entries] };
    await act(async () => handlers.get("project-changed")?.({ projectId: PROJECT, revision: 3, slices: ["history"] }));
    expect(await screen.findByText("Second")).toBeInTheDocument();
    expect(asked()).toBe(2);
  });
});

describe("Ask more on a History entry", () => {
  function open(askMore = vi.fn().mockResolvedValue("askaskas")) {
    install([entry("eeeeeeee", "Checkout works", local(11, 2, 9, 30), { tasks: [task("bbbbbbbb", "Pay with a card", { body: "Card payments." })] })]);
    render(
      <TooltipProvider>
        <ProjectContext.Provider value={{ projectId: PROJECT, showView: vi.fn(), openJob: vi.fn(), openedJob: null }}>
          <BoardActionsContext.Provider value={{ askMore }}>
            <HistoryView now={() => NOW} />
          </BoardActionsContext.Provider>
        </ProjectContext.Provider>
      </TooltipProvider>,
    );
    return askMore;
  }

  it("is in the footer of the entry's dialog, with its tooltip and accessible name, and sends only the entry", async () => {
    const user = userEvent.setup();
    const askMore = open();
    await screen.findByRole("combobox", { name: "Period" });
    await user.click(screen.getByRole("button", { name: /Checkout works/ }));
    const dialog = await screen.findByRole("dialog", { name: "Checkout works" });
    const button = within(dialog).getByRole("button", { name: "Ask more about eeeeeeee" });
    expect(button).toHaveTextContent("Ask more");
    expect(button).toBeEnabled();
    await user.hover(button);
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Ask an agent about this History entry");
    await user.click(button);
    expect(askMore).toHaveBeenCalledWith({ kind: "history", entry: "eeeeeeee" });
    // The dialog stays as it was.
    expect(screen.getByRole("dialog", { name: "Checkout works" })).toBeInTheDocument();
  });

  it("is not on the Tasks of the entry, collapsed or expanded", async () => {
    const user = userEvent.setup();
    open();
    await screen.findByRole("combobox", { name: "Period" });
    await user.click(screen.getByRole("button", { name: /Checkout works/ }));
    const dialog = await screen.findByRole("dialog", { name: "Checkout works" });
    await user.click(within(dialog).getByRole("button", { name: /Pay with a card/ }));
    expect(within(dialog).getByText("Card payments.")).toBeVisible();
    expect(within(dialog).queryByRole("button", { name: "Ask more about bbbbbbbb" })).toBeNull();
    expect(within(dialog).getAllByRole("button", { name: /^Ask more/ })).toHaveLength(1);
  });
});
