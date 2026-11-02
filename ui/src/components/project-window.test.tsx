import { act, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ShowRequest } from "@/bindings/ShowRequest";
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
