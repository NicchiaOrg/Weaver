import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { ProjectWindow } from "@/components/project-window";
import { Sidebar } from "@/components/sidebar";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setViewportWidth } from "@/test/media";
import { VIEWS, type ShellCounts, type ViewId } from "@/views";

vi.mock("@/lib/backend", () => ({
  call: vi.fn(() => Promise.resolve([])),
  on: vi.fn(() => Promise.resolve(() => {})),
}));

const ORDER = ["Board", "Chat", "Jobs", "Requirements", "Specs", "History", "Settings"];

function renderSidebar(counts: ShellCounts = { backlog: 0, jobsWaiting: 0 }, selected: ViewId = "board") {
  return render(
    <TooltipProvider>
      <Sidebar selected={selected} onSelect={() => {}} counts={counts} />
    </TooltipProvider>,
  );
}

const navigation = () => screen.getByRole("navigation", { name: "Views" });

describe("sidebar", () => {
  it("lists Board, Chat, Jobs, Requirements, Specs, History, Settings in this order", () => {
    expect(VIEWS.map((v) => v.label)).toEqual(ORDER);
    renderSidebar();
    const names = within(navigation())
      .getAllByRole("button")
      .map((b) => b.textContent);
    expect(names).toEqual(ORDER);
  });

  it("is 208 px wide with the names, and marks the selected entry", () => {
    renderSidebar({ backlog: 0, jobsWaiting: 0 }, "specs");
    expect(navigation()).toHaveClass("w-[208px]");
    expect(navigation()).toHaveAttribute("data-compact", "false");
    expect(screen.getByRole("button", { name: "Specs" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: "Board" })).not.toHaveAttribute("aria-current");
  });

  it("shrinks to 56 px with icons only below 1024 px, the names kept as accessible names and tooltips", async () => {
    renderSidebar();
    expect(navigation()).toHaveClass("w-[208px]");

    act(() => setViewportWidth(1023));
    expect(navigation()).toHaveClass("w-[56px]");
    expect(navigation()).toHaveAttribute("data-compact", "true");
    // Still the same eight entries, now named by hidden text.
    const buttons = within(navigation()).getAllByRole("button");
    expect(buttons.map((b) => b.textContent)).toEqual(ORDER);
    for (const button of buttons) expect(within(button).getByText(button.textContent ?? "")).toHaveClass("sr-only");

    const user = userEvent.setup();
    await user.tab();
    expect(buttons[0]).toHaveFocus();
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Board");

    act(() => setViewportWidth(1024));
    expect(navigation()).toHaveClass("w-[208px]");
  });

  it("shows the Backlog count on Board and the waiting jobs in amber on Jobs, and nothing at 0", () => {
    const { rerender } = renderSidebar();
    expect(within(navigation()).queryByText(/^\d+$/)).not.toBeInTheDocument();

    rerender(
      <TooltipProvider>
        <Sidebar selected="board" onSelect={() => {}} counts={{ backlog: 4, jobsWaiting: 2 }} />
      </TooltipProvider>,
    );
    const board = screen.getByRole("button", { name: /^Board/ });
    expect(board).toHaveTextContent("4");
    expect(board).toHaveAccessibleName("Board, 4 cards in Backlog");
    const jobs = screen.getByRole("button", { name: /^Jobs/ });
    expect(jobs).toHaveAccessibleName("Jobs, 2 jobs waiting for you");
    expect(within(jobs).getByText("2")).toHaveClass("text-amber-600", "dark:text-amber-400");
    expect(within(board).getByText("4")).not.toHaveClass("text-amber-600");
    for (const other of ["Chat", "Requirements", "Specs", "History", "Settings"]) {
      expect(screen.getByRole("button", { name: other })).not.toHaveTextContent(/\d/);
    }
  });
});

function Counter() {
  const [count, setCount] = useState(0);
  return <button onClick={() => setCount(count + 1)}>Clicked {count}</button>;
}

describe("project window", () => {
  it("keeps the state of a view when another entry is selected and back", async () => {
    const user = userEvent.setup();
    render(
      <TooltipProvider>
        <ProjectWindow projectId="0123456789abcdef" views={{ board: <Counter />, jobs: <p>jobs content</p> }} />
      </TooltipProvider>,
    );
    await user.click(screen.getByRole("button", { name: "Clicked 0" }));
    await user.click(screen.getByRole("button", { name: "Clicked 1" }));

    await user.click(screen.getByRole("button", { name: "Jobs" }));
    expect(screen.getByText("jobs content")).toBeVisible();
    expect(screen.queryByRole("button", { name: /Clicked/ })).not.toBeInTheDocument(); // hidden from the tree
    expect(screen.getByRole("heading", { name: "Jobs" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Board" }));
    expect(screen.getByRole("button", { name: "Clicked 2" })).toBeVisible();
    expect(screen.queryByText("jobs content")).not.toBeVisible();
  });

  it("starts on Board", () => {
    render(
      <TooltipProvider>
        <ProjectWindow projectId="0123456789abcdef" />
      </TooltipProvider>,
    );
    expect(screen.getByRole("button", { name: "Board" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("heading", { name: "Board" })).toBeInTheDocument();
  });
});
