import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { AgentNode } from "@/bindings/AgentNode";
import type { ConversationSnapshot } from "@/bindings/ConversationSnapshot";
import type { Entry } from "@/bindings/Entry";
import type { JobRow } from "@/bindings/JobRow";
import type { Step } from "@/bindings/Step";
import type { SubAgentConversation } from "@/bindings/SubAgentConversation";
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

function mount(
  jobs: JobRow[],
  conversation: ConversationSnapshot,
  handlers: Record<string, (args: never) => unknown> = {},
) {
  vi.mocked(on).mockResolvedValue(() => {});
  vi.mocked(call).mockImplementation(async (name: string, args?: unknown) => {
    if (name === "jobs_snapshot") return { projectId: PROJECT, jobs, ompReady: true };
    if (name === "job_conversation") return conversation;
    return (handlers[name]?.(args as never) ?? null) as null;
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

  it("starts with the view header across both panels, with the list and the conversation under it", async () => {
    mount([job("aaaaaaaa", "running")], { jobId: "aaaaaaaa", seq: 0, entries: [], notes: [] });
    const list = await screen.findByRole("navigation", { name: "Jobs" });
    const header = screen.getByRole("heading", { level: 1, name: "Jobs" }).closest("header")!;
    expect(header).toHaveClass("h-14", "border-b");
    expect(header.querySelector("button")).toBeNull();
    expect(header.compareDocumentPosition(list) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(header.parentElement).toContainElement(screen.getByRole("region", { name: "Conversation" }));
    expect(header.contains(list)).toBe(false);
  });

  it("puts 12 px of space above the first row inside the scrolling list, and none below the last", async () => {
    mount([job("aaaaaaaa", "running"), job("bbbbbbbb", "done")], { jobId: "aaaaaaaa", seq: 0, entries: [], notes: [] });
    const list = await screen.findByRole("navigation", { name: "Jobs" });
    const rows = within(list).getByRole("list");
    expect(list).toHaveClass("overflow-y-auto");
    expect(list.className).not.toMatch(/(^|\s)p[tby]?-/);
    expect(rows).toHaveClass("pt-3");
    expect(rows.className).not.toMatch(/(^|\s)(pb|py|p|my|mb)-/);
  });

  it("keeps the empty list text without the extra space", async () => {
    mount([], { jobId: "x", seq: 0, entries: [], notes: [] });
    const list = await screen.findByRole("navigation", { name: "Jobs" });
    expect(within(list).queryByRole("list")).toBeNull();
    expect(within(list).getByText("No jobs yet")).toHaveClass("p-4");
  });

  it("lists the jobs with their state badge and latest note, and shows the conversation of the first", async () => {
    mount([job("aaaaaaaa", "running", { note: "Reading the Model" }), job("bbbbbbbb", "done")], {
      jobId: "aaaaaaaa",
      seq: 1,
      notes: [],
      entries: [
        { kind: "instruction", text: "Use the weaver-scan skill on the whole System." },
        { kind: "agent", text: "Reading **code**", complete: false },
        { kind: "end", outcome: "failed", reason: "omp exited with code 3", details: "boom" },
      ],
    });
    const list = await screen.findByRole("navigation", { name: "Jobs" });
    expect(within(list).getByText("running")).toBeInTheDocument();
    expect(within(list).getByText("Reading the Model")).toBeInTheDocument();
    expect(within(list).getByRole("button", { name: "Remove Run bbbbbbbb" })).toBeInTheDocument();
    expect(within(list).queryByRole("button", { name: "Remove Run aaaaaaaa" })).toBeNull();
    expect(await screen.findByText("Use the weaver-scan skill on the whole System.")).toBeInTheDocument();
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

  it("makes a finished conversation read-only and points to Chat", async () => {
    mount([job("aaaaaaaa", "done")], { jobId: "aaaaaaaa", seq: 0, entries: [], notes: [] });
    expect(await screen.findByText("This job has ended. Continue in Chat.")).toBeInTheDocument();
    expect(screen.queryByPlaceholderText("Message the agent…")).toBeNull();
    expect(screen.getByRole("button", { name: "Open Chat" })).toBeInTheDocument();
  });

  describe("sub-agents", () => {
    const nodes: AgentNode[] = [
      {
        call: "t1",
        id: "0-Lead",
        name: "Lead",
        state: "working",
        started: "2026-11-02T07:05:00.000Z",
        children: [{ call: "c9", id: "1-Child", name: "A very long name that is truncated in the list", state: "succeeded", started: null, children: [] }],
      },
      { call: "t1", id: "2-Twin", name: "Twin", state: "failed", started: "2026-11-02T07:06:00.000Z", children: [] },
      { call: "t1", id: "3-Twin", name: "Twin", state: "interrupted", started: "2026-11-02T07:07:00.000Z", children: [] },
    ];
    const taskRow: Entry = {
      kind: "tool",
      id: "t1",
      title: "Task",
      icon: "sub-agent",
      command: null,
      status: "done",
      subAgents: [{ id: "0-Lead", name: "Lead", state: "working" }],
    };
    const reply = (path: Step[], patch: Partial<SubAgentConversation> = {}): SubAgentConversation => ({
      projectId: PROJECT,
      owner: { kind: "job", id: "aaaaaaaa" },
      path,
      seq: 0,
      entries: [{ kind: "assignment", from: "Run aaaaaaaa", text: "Read the code." }, { kind: "agent", text: "On it.", complete: true }],
      unavailable: null,
      ...patch,
    });
    const handlers = (conversation: (path: Step[]) => SubAgentConversation) => ({
      job_sub_agents: () => ({ projectId: PROJECT, owner: { kind: "job", id: "aaaaaaaa" }, nodes }),
      job_sub_agent_conversation: ({ path }: { path: Step[] }) => conversation(path),
    });

    it("lists them under the selected job only, nested, with the start time only between siblings of the same name", async () => {
      mount([job("aaaaaaaa", "running"), job("bbbbbbbb", "running")], { jobId: "aaaaaaaa", seq: 0, entries: [], notes: [] }, handlers((path) => reply(path)));
      const list = await screen.findByRole("list", { name: "Sub-agents" });
      expect(screen.getAllByRole("list", { name: "Sub-agents" })).toHaveLength(1);
      const lead = within(list).getByRole("button", { name: /^Lead/ });
      expect(lead).toHaveTextContent("working");
      expect(lead).not.toHaveTextContent("07:05");
      const twins = within(list).getAllByRole("button", { name: /^Twin/ });
      expect(twins[0]).toHaveTextContent(/\d\d:\d\d/);
      expect(twins[0]).toHaveTextContent("failed");
      expect(twins[1]).toHaveTextContent("interrupted");
      const child = within(list).getByTitle("A very long name that is truncated in the list");
      expect(child).toHaveClass("truncate");
      expect(list.closest("li")).toContainElement(screen.getByRole("button", { name: /Run aaaaaaaa/ }));
    });

    it("shows no list for a job without sub-agents, and tells the backend what is on screen", async () => {
      mount([job("aaaaaaaa", "running")], { jobId: "aaaaaaaa", seq: 0, entries: [], notes: [] }, {
        job_sub_agents: () => ({ projectId: PROJECT, owner: { kind: "job", id: "aaaaaaaa" }, nodes: [] }),
      });
      await screen.findByText("Run aaaaaaaa");
      expect(screen.queryByRole("list", { name: "Sub-agents" })).toBeNull();
      await waitFor(() =>
        expect(call).toHaveBeenCalledWith("job_sub_agents_view", { projectId: PROJECT, jobId: "aaaaaaaa", listShown: true, path: null }),
      );
    });

    it("opens a sub-agent's read-only conversation from its list row and goes back with the job's row", async () => {
      const user = userEvent.setup();
      mount([job("aaaaaaaa", "running")], { jobId: "aaaaaaaa", seq: 0, entries: [], notes: [] }, handlers((path) => reply(path)));
      const list = await screen.findByRole("list", { name: "Sub-agents" });
      const row = within(list).getByRole("button", { name: /^Lead/ });
      await user.click(row);
      expect(await screen.findByText("Read the code.")).toBeInTheDocument();
      expect(screen.getByText("On it.")).toBeInTheDocument();
      expect(row).toHaveAttribute("aria-current", "true");
      const header = screen.getByRole("heading", { name: "Run aaaaaaaa › Lead" });
      expect(header.parentElement).toHaveTextContent("working");
      expect(screen.getByText("Run aaaaaaaa", { selector: "div" })).toBeInTheDocument();
      const disabledBox = screen.getByPlaceholderText("Sub-agents take no messages.");
      expect(disabledBox).toBeDisabled();
      expect(screen.getByRole("button", { name: "Send" })).toBeDisabled();
      expect(screen.queryByPlaceholderText("Message the agent…")).toBeNull();
      expect(screen.queryByRole("button", { name: "Stop the agent" })).toBeNull();
      expect(call).toHaveBeenCalledWith("job_sub_agents_view", {
        projectId: PROJECT,
        jobId: "aaaaaaaa",
        listShown: true,
        path: [{ call: "t1", id: "0-Lead" }],
      });

      await user.click(within(list).getByRole("button", { name: /^A very long name/ }));
      expect(await screen.findByRole("heading", { name: "Run aaaaaaaa › Lead › A very long name that is truncated in the list" })).toBeInTheDocument();

      await user.click(screen.getByRole("button", { name: /Run aaaaaaaa/ }));
      expect(await screen.findByPlaceholderText("Message the agent…")).toBeInTheDocument();
    });

    it("opens the same conversation from the line under the task row, and says when it is not available", async () => {
      const user = userEvent.setup();
      mount(
        [job("aaaaaaaa", "running")],
        { jobId: "aaaaaaaa", seq: 0, entries: [taskRow], notes: [] },
        handlers((path) => reply(path, { entries: [], unavailable: "the transcript file was not found" })),
      );
      const line = await within(await screen.findByRole("region", { name: "Conversation" })).findByRole("button", { name: /^Lead\s*working$/ });
      await user.click(line);
      expect(await screen.findByText("Conversation not available: the transcript file was not found.")).toBeInTheDocument();
      expect(screen.getByPlaceholderText("Sub-agents take no messages.")).toBeDisabled();
      const list = screen.getByRole("list", { name: "Sub-agents" });
      expect(within(list).getByRole("button", { name: /^Lead/ })).toHaveAttribute("aria-current", "true");
    });
  });
});
