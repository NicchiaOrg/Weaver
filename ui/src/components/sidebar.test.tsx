import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Gauge } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { describe, expect, it, vi } from "vitest";

import { ProjectWindow } from "@/components/project-window";
import { Sidebar } from "@/components/sidebar";
import { TooltipProvider } from "@/components/ui/tooltip";
import { ViewHeader } from "@/components/view-header";
import { call } from "@/lib/backend";
import { setViewportWidth } from "@/test/media";
import { VIEWS, type ShellCounts, type ViewId } from "@/views";

vi.mock("@/lib/backend", () => ({
  call: vi.fn(() => Promise.resolve([])),
  on: vi.fn(() => Promise.resolve(() => {})),
  isAppError: (error: unknown) => typeof error === "object" && error !== null && "message" in error && "code" in error,
}));

vi.mock("sonner", () => ({ toast: { error: vi.fn() }, Toaster: () => null }));

// The Usage dialog reads the status of omp: not ready here, so it says why and offers Open Settings.
vi.mock("@/hooks/use-omp", () => ({ useOmp: () => ({ status: { state: "missing", providers: [], models: [], version: null, reason: null } }) }));

const ORDER = ["Board", "Chat", "Jobs", "Files", "Requirements", "Specs", "History", "Settings"];

function renderSidebar(counts: ShellCounts = { backlog: 0, jobsWaiting: 0 }, selected: ViewId = "board") {
  return render(
    <TooltipProvider>
      <Sidebar selected={selected} onSelect={() => {}} counts={counts} />
    </TooltipProvider>,
  );
}

const navigation = () => screen.getByRole("navigation", { name: "Views" });

describe("sidebar", () => {
  it("lists Board, Chat, Jobs, Files, Requirements, Specs, History, Settings in this order", () => {
    expect(VIEWS.map((v) => v.label)).toEqual(ORDER);
    renderSidebar();
    const names = within(navigation())
      .getAllByRole("button")
      .map((b) => b.textContent);
    expect(names).toEqual(ORDER);
  });

  it("keeps Settings alone in a bottom group apart from the top group, and last", () => {
    renderSidebar();
    const lists = within(navigation()).getAllByRole("list");
    const top = lists[0] as HTMLElement;
    const bottom = lists[1] as HTMLElement;
    expect(within(top).getAllByRole("button").map((b) => b.textContent)).toEqual(ORDER.slice(0, 7));
    expect(within(bottom).getAllByRole("button").map((b) => b.textContent)).toEqual(["Settings"]);
    expect(top.compareDocumentPosition(bottom) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(navigation()).toHaveClass("h-full", "flex-col", "justify-between");
  });

  it("gives the bottom group entries that are not views: above Settings, never selected, without aria-current", async () => {
    const onActivate = vi.fn();
    const onSelect = vi.fn();
    render(
      <TooltipProvider>
        <Sidebar selected="settings" onSelect={onSelect} counts={{ backlog: 0, jobsWaiting: 0 }} actions={[{ id: "usage", label: "Usage", icon: Gauge, onActivate }]} />
      </TooltipProvider>,
    );
    const bottom = within(navigation()).getAllByRole("list")[1] as HTMLElement;
    expect(within(bottom).getAllByRole("button").map((b) => b.textContent)).toEqual(["Usage", "Settings"]);
    const usage = screen.getByRole("button", { name: "Usage" });
    expect(usage).not.toHaveAttribute("aria-current");
    expect(screen.getByRole("button", { name: "Settings" })).toHaveAttribute("aria-current", "page");

    await userEvent.setup().click(usage);
    expect(onActivate).toHaveBeenCalledTimes(1);
    expect(onSelect).not.toHaveBeenCalled();
    expect(usage).not.toHaveAttribute("aria-current");
  });

  it("gives Settings its name as tooltip when the sidebar shows icons only", async () => {
    renderSidebar();
    act(() => setViewportWidth(1023));
    const user = userEvent.setup();
    await user.hover(screen.getByRole("button", { name: "Settings" }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Settings");
    act(() => setViewportWidth(1024));
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
    // Still the same seven entries, now named by hidden text.
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
        <ProjectWindow
          projectId="0123456789abcdef"
          views={{
            board: <Counter />,
            jobs: (
              <>
                <ViewHeader view="jobs" />
                <p>jobs content</p>
              </>
            ),
          }}
        />
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

  it("opens Settings from the bottom of the sidebar and keeps the other views' state", async () => {
    const user = userEvent.setup();
    render(
      <TooltipProvider>
        <ProjectWindow projectId="0123456789abcdef" views={{ board: <Counter />, settings: <p>settings content</p> }} />
      </TooltipProvider>,
    );
    await user.click(screen.getByRole("button", { name: "Clicked 0" }));

    await user.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.getByText("settings content")).toBeVisible();
    expect(screen.getByRole("button", { name: "Settings" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: "Board" })).not.toHaveAttribute("aria-current");

    await user.click(screen.getByRole("button", { name: "Board" }));
    expect(screen.getByRole("button", { name: "Clicked 1" })).toBeVisible();
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

describe("the Usage entry of a project window", () => {
  const renderWindow = () =>
    render(
      <TooltipProvider>
        <ProjectWindow projectId="0123456789abcdef" views={{ board: <p>board content</p>, settings: <p>settings content</p> }} />
      </TooltipProvider>,
    );

  it("sits in the bottom group, right above Settings, with the Gauge icon", () => {
    renderWindow();
    const bottom = within(navigation()).getAllByRole("list")[1] as HTMLElement;
    expect(within(bottom).getAllByRole("button").map((b) => b.textContent)).toEqual(["Hide", "Usage", "Settings"]);
    expect(screen.getByRole("button", { name: "Usage" }).querySelector("svg")).toHaveClass("lucide-gauge");
  });

  it("is never the selected entry and never has aria-current, even when it is clicked", async () => {
    renderWindow();
    const usage = screen.getByRole("button", { name: "Usage" });
    expect(usage).not.toHaveAttribute("aria-current");
    await userEvent.setup().click(usage);
    // The dialog is open: the rest of the window is hidden from the accessibility tree.
    expect(usage).not.toHaveAttribute("aria-current");
    expect(screen.getAllByRole("button", { current: "page", hidden: true })).toHaveLength(1);
  });

  it("gives Usage its name as tooltip when the sidebar shows icons only", async () => {
    renderWindow();
    act(() => setViewportWidth(1023));
    const user = userEvent.setup();
    await user.hover(screen.getByRole("button", { name: "Usage" }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Usage");
    act(() => setViewportWidth(1024));
  });

  it("opens the dialog over the view shown without changing the view, and the focus comes back to the entry", async () => {
    const user = userEvent.setup();
    renderWindow();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Usage" }));
    expect(await screen.findByRole("dialog", { name: "Usage" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Board", hidden: true })).toHaveAttribute("aria-current", "page");
    expect(screen.getByText("board content")).toBeInTheDocument();

    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Usage" })).toHaveFocus();
    expect(screen.getByRole("button", { name: "Board" })).toHaveAttribute("aria-current", "page");
  });

  it("shows Settings and closes the dialog with Open Settings while omp is not ready", async () => {
    const user = userEvent.setup();
    renderWindow();
    await user.click(screen.getByRole("button", { name: "Usage" }));
    await user.click(await screen.findByRole("button", { name: "Open Settings" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Settings" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByText("settings content")).toBeVisible();
  });
});

describe("the Hide entry of a project window", () => {
  const renderWindow = () =>
    render(
      <TooltipProvider>
        <ProjectWindow projectId="0123456789abcdef" views={{ board: <Counter />, settings: <p>settings content</p> }} />
      </TooltipProvider>,
    );

  it("sits in the bottom group before Usage and Settings, with the EyeOff icon", () => {
    renderWindow();
    const bottom = within(navigation()).getAllByRole("list")[1] as HTMLElement;
    expect(within(bottom).getAllByRole("button").map((b) => b.textContent)).toEqual(["Hide", "Usage", "Settings"]);
    expect(screen.getByRole("button", { name: "Hide" }).querySelector("svg")).toHaveClass("lucide-eye-off");
  });

  it("sends this window to the background at once, without confirmation, and is never selected nor current", async () => {
    const user = userEvent.setup();
    vi.mocked(call).mockClear();
    renderWindow();
    await user.click(screen.getByRole("button", { name: "Clicked 0" }));
    const entry = screen.getByRole("button", { name: "Hide" });
    expect(entry).toBeEnabled();
    expect(entry).not.toHaveAttribute("aria-current");

    await user.click(entry);
    expect(call).toHaveBeenCalledWith("send_to_background", { projectId: "0123456789abcdef" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(entry).not.toHaveAttribute("aria-current");
    // The window shows the same view, with its state as it was.
    expect(screen.getByRole("button", { name: "Board" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: "Clicked 1" })).toBeVisible();
  });

  it("is also available while another view is shown, and leaves that view selected", async () => {
    const user = userEvent.setup();
    renderWindow();
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.click(screen.getByRole("button", { name: "Hide" }));
    expect(screen.getByRole("button", { name: "Settings" })).toHaveAttribute("aria-current", "page");
    expect(screen.getAllByRole("button", { current: "page" })).toHaveLength(1);
  });

  it("gives its name as tooltip when the sidebar shows icons only", async () => {
    renderWindow();
    act(() => setViewportWidth(1023));
    const user = userEvent.setup();
    await user.hover(screen.getByRole("button", { name: "Hide" }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Hide");
    act(() => setViewportWidth(1024));
  });

  it("tells the reason in an error toast when the window cannot be sent to the background", async () => {
    vi.mocked(call).mockImplementation(((name: string) =>
      name === "send_to_background" ? Promise.reject({ code: "internal", message: "the window is gone" }) : Promise.resolve([])) as never);
    renderWindow();
    await userEvent.setup().click(screen.getByRole("button", { name: "Hide" }));
    expect(toast.error).toHaveBeenCalledWith("Could not hide the window.", { description: "the window is gone" });
    vi.mocked(call).mockImplementation((() => Promise.resolve([])) as never);
  });
});
