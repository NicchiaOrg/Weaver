import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ShowRequest } from "@/bindings/ShowRequest";
import { useLeaveGuard } from "@/components/files/leave-guard";
import { ProjectWindow } from "@/components/project-window";
import { TooltipProvider } from "@/components/ui/tooltip";
import { call, on } from "@/lib/backend";

vi.mock("@/lib/backend", () => ({ call: vi.fn(), on: vi.fn() }));

const PROJECT = "0123456789abcdef";
let push: (request: ShowRequest) => void;

beforeEach(() => {
  vi.mocked(call).mockResolvedValue([] as never);
  vi.mocked(on).mockImplementation(((name: string, handler: (request: ShowRequest) => void) => {
    if (name === "show-requested") push = handler;
    return Promise.resolve(() => {});
  }) as unknown as typeof on);
});

function mount() {
  render(
    <TooltipProvider>
      <ProjectWindow projectId={PROJECT} />
    </TooltipProvider>,
  );
}

const shown = () => screen.getByRole("button", { current: "page" }).textContent;

describe("a window asked to show something", () => {
  it("shows the view a notification was about", () => {
    mount();
    expect(shown()).toMatch(/^Board/);
    act(() => push({ projectId: PROJECT, view: "history", jobId: null }));
    expect(shown()).toMatch(/^History/);
    expect(screen.getByRole("heading", { level: 1, name: "History" }).closest("section")).not.toHaveAttribute("hidden");
  });

  it("opens the Settings view when the System Certificates notification is clicked", () => {
    mount();
    act(() => push({ projectId: PROJECT, view: "settings", jobId: null }));
    expect(shown()).toMatch(/^Settings/);
    expect(screen.getByRole("heading", { level: 1, name: "Settings" }).closest("section")).not.toHaveAttribute("hidden");
  });

  it("shows the System Certificates notice at the top and Open Settings shows the Settings view", async () => {
    vi.mocked(call).mockImplementation(((name: string) =>
      Promise.resolve(
        name === "system_certificates_status"
          ? {
              revision: 1,
              phase: "ready",
              automatic: true,
              count: 3,
              writtenAt: null,
              error: null,
              turnOnNotice: true,
              failureBanner: false, failureReason: null,
            }
          : [],
      )) as never);
    mount();
    await act(async () => {});
    expect(shown()).toMatch(/^Board/);
    fireEvent.click(screen.getByRole("button", { name: "Open Settings" }));
    expect(shown()).toMatch(/^Settings/);
    expect(call).toHaveBeenCalledWith("system_certificates_dismiss_notice");
  });

  it("opens the conversation of the job in Jobs", () => {
    mount();
    act(() => push({ projectId: PROJECT, view: "jobs", jobId: "job-7" }));
    expect(shown()).toMatch(/^Jobs/);
  });

  it("ignores what was meant for another project", () => {
    mount();
    act(() => push({ projectId: "ffffffffffffffff", view: "history", jobId: null }));
    expect(shown()).toMatch(/^Board/);
  });
});

describe("the region of a view", () => {
  it("is labelled by the visible title of the view, with no hidden heading repeating the name", () => {
    mount();
    const title = screen.getByRole("heading", { level: 1, name: "Board" });
    expect(title).not.toHaveClass("sr-only");
    expect(screen.getByRole("region", { name: "Board" })).toContainElement(title);
    expect(document.querySelectorAll("h1.sr-only")).toHaveLength(0);
  });
});

describe("the File view in a window", () => {
  it("is not kept when another entry is chosen, and the guard of the view decides whether the other entry is shown", () => {
    let decide: (proceed: () => void) => void = (proceed) => proceed();
    const Files = () => {
      useLeaveGuard((proceed) => decide(proceed));
      return <p>Files content</p>;
    };
    render(
      <TooltipProvider>
        <ProjectWindow projectId={PROJECT} initialView="files" views={{ files: <Files />, board: <p>Board content</p> }} />
      </TooltipProvider>,
    );
    expect(screen.getByText("Files content")).toBeInTheDocument();
    // The guard keeps the view: the other entry is not shown.
    decide = () => {};
    fireEvent.click(screen.getByRole("button", { name: "Board" }));
    expect(screen.getByText("Files content")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Files" })).toHaveAttribute("aria-current", "page");
    // The guard lets it go: the File view is unmounted, not hidden.
    decide = (proceed) => proceed();
    fireEvent.click(screen.getByRole("button", { name: "Board" }));
    expect(screen.queryByText("Files content")).not.toBeInTheDocument();
    expect(screen.getByText("Board content")).toBeInTheDocument();
  });
});
