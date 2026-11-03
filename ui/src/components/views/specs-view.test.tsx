import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ViewSnapshot } from "@/bindings/ViewSnapshot";
import type { FocusSubject } from "@/components/views/focus-model";
import { fieldTitle, SpecsView } from "@/components/views/specs-view";
import { useViews } from "@/components/views/views-state";
import { snapshot } from "@/components/views/view-fixtures";
import { fakeBackend, renderViews, type FakeBackend } from "@/components/views/view-test-utils";

vi.mock("@/lib/backend", () => ({
  call: vi.fn(),
  on: vi.fn(),
  isAppError: (value: unknown) => typeof value === "object" && value !== null && "code" in value && "message" in value,
}));

let current: ViewSnapshot;
let backend: FakeBackend;

beforeEach(() => {
  current = snapshot();
  backend = fakeBackend(() => current);
});

function FocusOpener({ refs, subject }: { refs: string[]; subject: FocusSubject }) {
  const { focusOn } = useViews();
  return <button onClick={() => focusOn("specs", refs, subject)}>open focus</button>;
}

async function show(extra?: React.ReactNode) {
  renderViews(
    <>
      {extra}
      <SpecsView />
    </>,
  );
  await screen.findByRole("heading", { name: "Languages" });
}

describe("the Specs page", () => {
  it("shows the fields in their order and names them for people", async () => {
    await show();
    expect(screen.getAllByRole("heading", { level: 2 }).map((h) => h.textContent)).toEqual(["Languages", "Notes"]);
    expect(fieldTitle("ci")).toBe("CI");
    expect(fieldTitle("infrastructure")).toBe("Infrastructure");
  });

  it("has no control in its header, only the title Specs: no overlay picker and no `Ask more`", async () => {
    await show();
    const header = screen.getByRole("heading", { level: 1, name: "Specs" }).closest("header")!;
    expect(within(header).queryByRole("button")).toBeNull();
    expect(within(header).queryByRole("combobox")).toBeNull();
    expect(screen.queryByRole("combobox", { name: "Overlay" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Ask more/ })).not.toBeInTheDocument();
  });

  it("starts with the view header titled Specs, with the focus bar full width right under it", async () => {
    await show(<FocusOpener refs={["specs#languages"]} subject={{ kind: "ref", ref: "specs#languages" }} />);
    await userEvent.setup().click(screen.getByRole("button", { name: "open focus" }));
    const bar = await screen.findByRole("region", { name: "Focus" });
    const header = screen.getByRole("heading", { level: 1, name: "Specs" }).closest("header")!;
    expect(header).toHaveClass("h-14", "border-b");
    expect(header.nextElementSibling).toBe(bar);
  });

  it("shows an entry with its name, version and note, in a grid of columns of at least 300 px, at most 1100 px wide", async () => {
    await show();
    const field = screen.getByRole("region", { name: "Languages" });
    const entries = within(field).getAllByRole("listitem");
    expect(entries).toHaveLength(2);
    expect(within(entries[0]!).getByText("TypeScript")).toBeInTheDocument();
    expect(within(entries[0]!).getByText("5.9")).toBeInTheDocument();
    expect(within(entries[0]!).getByText("Strict")).toBeInTheDocument();
    expect(entries[0]!.parentElement).toHaveClass("grid-cols-[repeat(auto-fill,minmax(300px,1fr))]");
    expect(field.parentElement).toHaveClass("max-w-[1100px]");
  });

  it("marks what the agent chose with `agent's choice`, whose tooltip is the reason", async () => {
    const user = userEvent.setup();
    await show();
    const entries = within(screen.getByRole("region", { name: "Languages" })).getAllByRole("listitem");
    expect(within(entries[0]!).queryByText("agent's choice")).not.toBeInTheDocument();
    const mark = within(entries[1]!).getByText("agent's choice");
    await user.hover(mark);
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Fast and safe");
  });

  it("renders the notes as Markdown and opens the file at the field's line", async () => {
    const user = userEvent.setup();
    await show();
    const notes = screen.getByRole("region", { name: "Notes" });
    expect(within(notes).getByText("notes").tagName).toBe("EM");
    await user.click(within(notes).getByRole("button", { name: "Open in editor" }));
    await waitFor(() => expect(backend.opened).toEqual([{ file: ".weaver/specs.yaml", line: 9 }]));
  });

  it("says so when the project has no Specs", async () => {
    current = snapshot({}, { specs: null });
    renderViews(<SpecsView />);
    expect(await screen.findByText("The project has no Specs.")).toBeInTheDocument();
  });
});

describe("the overlay labels of the Specs", () => {
  it("label entries added, changed and removed in words, and `Show before` shows how a changed one read", async () => {
    current = snapshot({ overlay: "pppppppp", overlays: [{ id: "pppppppp", shortNumber: "P3", title: "Pay" }] });
    const field = current.data.specs!.fields[0]!;
    field.entries = [
      { name: "TypeScript", version: "6.0", note: null, chosenReason: null, change: "changed", before: "TypeScript 5.9 — Strict" },
      { name: "Go", version: null, note: null, chosenReason: null, change: "added", before: null },
      { name: "Rust", version: null, note: null, chosenReason: null, change: "removed", before: null },
    ];
    const user = userEvent.setup();
    await show();
    expect(screen.getByRole("listitem", { name: "TypeScript, changed" })).toBeInTheDocument();
    expect(screen.getByRole("listitem", { name: "Go, added" })).toBeInTheDocument();
    const removed = screen.getByRole("listitem", { name: "Rust, removed" });
    expect(within(removed).getByText("Rust")).toHaveClass("line-through");
    await user.click(within(screen.getByRole("listitem", { name: "TypeScript, changed" })).getByRole("button", { name: "Show before" }));
    expect(screen.getByText("Before: TypeScript 5.9 — Strict")).toBeInTheDocument();
  });
});

describe("the focus of the Specs", () => {
  const subject: FocusSubject = { kind: "task", id: "tttttttt", number: "T4", title: "Fix" };
  const scrolled: HTMLElement[] = [];

  beforeEach(() => {
    scrolled.length = 0;
    Element.prototype.scrollIntoView = function (this: HTMLElement) {
      scrolled.push(this);
    };
  });

  async function focused(refs: string[]) {
    const user = userEvent.setup();
    await show(<FocusOpener refs={refs} subject={subject} />);
    await user.click(screen.getByRole("button", { name: "open focus" }));
    await screen.findByRole("region", { name: "Focus" });
    return user;
  }

  it("tints the focused fields, scrolls to the step, and counts the fields it cannot find", async () => {
    await focused(["specs#notes", "specs#languages", "specs#cloud"]);
    const bar = screen.getByRole("region", { name: "Focus" });
    expect(within(bar).getByText("languages")).toBeInTheDocument();
    expect(within(bar).getByText("1 of 2")).toBeInTheDocument();
    expect(within(bar).getByText("1 not found")).toBeInTheDocument();
    expect(within(bar).queryByRole("checkbox", { name: "Only these" })).not.toBeInTheDocument();
    await waitFor(() => expect(new Set(scrolled.map((el) => el.dataset.stepKey))).toEqual(new Set(["languages"])));
    expect(screen.getByRole("region", { name: "Languages" })).toHaveClass("border-cyan-600");
    const user = userEvent.setup();
    await user.click(within(bar).getByRole("button", { name: "Next" }));
    await waitFor(() => expect(scrolled[scrolled.length - 1]?.dataset.stepKey).toBe("notes"));
  });

  it("clears with Esc", async () => {
    const user = await focused(["specs#notes"]);
    within(screen.getByRole("region", { name: "Focus" })).getByRole("button", { name: "Clear" }).focus();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("region", { name: "Focus" })).not.toBeInTheDocument();
  });
});
