import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ConversationSnapshot } from "@/bindings/ConversationSnapshot";
import type { JobRow } from "@/bindings/JobRow";
import { JobsView } from "@/components/jobs/jobs-view";
import { JobsProvider } from "@/components/jobs/use-jobs";
import { ProjectContext } from "@/components/project-context";
import { TooltipProvider } from "@/components/ui/tooltip";
import { call, on } from "@/lib/backend";

vi.mock("@/lib/backend", () => ({
  call: vi.fn(),
  on: vi.fn(),
  isAppError: (value: unknown) => typeof value === "object" && value !== null && "code" in value && "message" in value,
}));

const PROJECT = "0123456789abcdef";

function job(id: string, state: JobRow["state"], patch: Partial<JobRow> = {}): JobRow {
  return { id, kind: "run", name: `Run ${id}`, state, reason: null, created: "2026-11-02T07:00:00.000Z", started: null, ended: null, note: null, target: null, ...patch };
}

function mount(jobs: JobRow[], conversation: ConversationSnapshot) {
  vi.mocked(on).mockResolvedValue(() => {});
  vi.mocked(call).mockImplementation(async (name: string) => {
    if (name === "jobs_snapshot") return { projectId: PROJECT, jobs, ompReady: true };
    if (name === "job_conversation") return conversation;
    return null;
  });
  render(
    <TooltipProvider>
      <ProjectContext.Provider value={{ projectId: PROJECT, showView: vi.fn(), openJob: vi.fn(), openedJob: null }}>
        <JobsProvider projectId={PROJECT}>
          <JobsView />
        </JobsProvider>
      </ProjectContext.Provider>
    </TooltipProvider>,
  );
}

describe("JobsView", () => {
  beforeEach(() => vi.clearAllMocks());

  it("says there are no jobs", async () => {
    mount([], { jobId: "x", seq: 0, entries: [], notes: [] });
    expect(await screen.findByText("No jobs yet")).toBeInTheDocument();
  });

  it("lists the jobs with their state badge and latest note, and shows the conversation of the first", async () => {
    mount([job("aaaaaaaa", "running", { note: "Reading the Model" }), job("bbbbbbbb", "done")], {
      jobId: "aaaaaaaa",
      seq: 1,
      notes: [],
      entries: [
        { kind: "instruction", text: "Use the trama-scan skill on the whole System." },
        { kind: "agent", text: "Reading **code**", complete: false },
        { kind: "end", outcome: "failed", reason: "omp exited with code 3", details: "boom" },
      ],
    });
    const list = await screen.findByRole("navigation", { name: "Jobs" });
    expect(within(list).getByText("running")).toBeInTheDocument();
    expect(within(list).getByText("Reading the Model")).toBeInTheDocument();
    expect(within(list).getByRole("button", { name: "Remove Run bbbbbbbb" })).toBeInTheDocument();
    expect(within(list).queryByRole("button", { name: "Remove Run aaaaaaaa" })).toBeNull();
    expect(await screen.findByText("Use the trama-scan skill on the whole System.")).toBeInTheDocument();
    expect(await screen.findByText("code")).toBeInTheDocument();
    expect(screen.getByText(/Failed: omp exited with code 3\./)).toBeInTheDocument();
  });

  it("asks before stopping the agent and stops it only when confirmed", async () => {
    mount([job("aaaaaaaa", "running")], { jobId: "aaaaaaaa", seq: 0, entries: [], notes: [] });
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "Stop the agent" }));
    const dialog = await screen.findByRole("alertdialog");
    expect(within(dialog).getByText("Stop Run aaaaaaaa?")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(call).not.toHaveBeenCalledWith("job_stop", expect.anything());
    await user.click(screen.getByRole("button", { name: "Stop the agent" }));
    await user.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Stop" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("job_stop", { projectId: PROJECT, jobId: "aaaaaaaa" }));
  });

  it("sends a message with Enter and keeps Shift+Enter for a new line", async () => {
    mount([job("aaaaaaaa", "running")], { jobId: "aaaaaaaa", seq: 0, entries: [], notes: [] });
    const user = userEvent.setup();
    const box = await screen.findByPlaceholderText("Message the agent…");
    await user.type(box, "Use the old parser{Shift>}{Enter}{/Shift}now{Enter}");
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("job_send_message", { projectId: PROJECT, jobId: "aaaaaaaa", text: "Use the old parser\nnow" }),
    );
  });

  it("offers omp's own options on a permission card and sends the choice", async () => {
    mount([job("aaaaaaaa", "waiting")], {
      jobId: "aaaaaaaa",
      seq: 1,
      notes: [],
      entries: [
        {
          kind: "permission",
          request: 4,
          tool: null,
          state: { status: "asking" },
          options: [
            { id: "allow", name: "Allow once", rejects: false },
            { id: "no", name: "Reject", rejects: true },
          ],
        },
      ],
    });
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "Allow once" }));
    expect(call).toHaveBeenCalledWith("job_answer_permission", { projectId: PROJECT, jobId: "aaaaaaaa", request: 4, option: "allow" });
  });

  it("makes a finished conversation read-only and points to General", async () => {
    mount([job("aaaaaaaa", "done")], { jobId: "aaaaaaaa", seq: 0, entries: [], notes: [] });
    expect(await screen.findByText("This job has ended. Continue in General.")).toBeInTheDocument();
    expect(screen.queryByPlaceholderText("Message the agent…")).toBeNull();
    expect(screen.getByRole("button", { name: "Open General" })).toBeInTheDocument();
  });
});
