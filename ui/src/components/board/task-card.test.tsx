import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { BoardCard } from "@/bindings/BoardCard";
import type { BoardSnapshot } from "@/bindings/BoardSnapshot";
import { BoardProvider } from "@/components/board/board-state";
import { BoardActionsContext, type BoardActions } from "@/components/board/board-ui";
import { BoardView } from "@/components/board/board-view";
import { ProjectContext } from "@/components/project-context";
import { TooltipProvider } from "@/components/ui/tooltip";
import { call, on } from "@/lib/backend";
import { leftBorderClass } from "@/lib/label-colour";
import { driftView } from "@/components/board/drift-fixtures";

vi.mock("@/lib/backend", () => ({
  call: vi.fn(),
  on: vi.fn(),
  isAppError: (value: unknown) => typeof value === "object" && value !== null && "code" in value && "message" in value,
}));

const PROJECT = "0123456789abcdef";

function head(id: string, shortNumber: string | null, title: string, body = "") {
  return { id, shortNumber, title, body, created: "2026-11-02T07:00:00.000Z", agentFinishing: false };
}

type TaskCard = Extract<BoardCard, { kind: "task" }>;

function task(id: string, shortNumber: string | null, title: string, patch: Partial<TaskCard> = {}): BoardCard {
  return {
    kind: "task",
    head: head(id, shortNumber, title),
    doing: false,
    heldByAgent: false,
    technical: false,
    refs: [],
    label: null,
    blockedBy: [],
    waitingFor: [],
    runReasons: [],
    deleteReasons: [],
    ...patch,
  };
}

function drift(id: string, shortNumber: string, title: string): BoardCard {
  return { kind: "drift", head: head(id, shortNumber, title), origin: null, view: driftView() };
}

function snapshot(patch: Partial<BoardSnapshot>): BoardSnapshot {
  return {
    projectId: PROJECT,
    revision: 1,
    backlog: [],
    active: [],
    ghosts: [],
    gathered: [],
    runAllReasons: [],
    doneSinceLastScan: 0,
    warnings: [],
    ...patch,
  };
}

async function show(board: BoardSnapshot, actions: BoardActions = {}) {
  vi.mocked(on).mockImplementation((() => Promise.resolve(() => {})) as unknown as typeof on);
  vi.mocked(call).mockImplementation(((name: string) => {
    if (name === "board_snapshot") return Promise.resolve(board);
    if (name === "delete_task") return Promise.resolve(null);
    return Promise.reject(new Error(`unexpected command ${name}`));
  }) as unknown as typeof call);
  render(
    <TooltipProvider>
      <ProjectContext.Provider value={{ projectId: PROJECT, showView: vi.fn(), openJob: vi.fn(), openedJob: null }}>
        <BoardActionsContext.Provider value={actions}>
          <BoardProvider projectId={PROJECT}>
            <BoardView />
          </BoardProvider>
        </BoardActionsContext.Provider>
      </ProjectContext.Provider>
    </TooltipProvider>,
  );
  await screen.findByRole("region", { name: "Backlog" });
}

const active = () => screen.getByRole("region", { name: "Active" });
const backlog = () => screen.getByRole("region", { name: "Backlog" });

/** The reasons a guarded control lists, one per line, as its accessible description holds them. */
function reasonsOf(button: HTMLElement): string[] {
  const own = (button.getAttribute("aria-describedby") ?? "").split(" ")[0] ?? "";
  const lines = [...(document.getElementById(own)?.querySelectorAll(":scope > .block") ?? [])];
  return lines.map((line) => line.textContent ?? "");
}

beforeEach(() => {
  vi.mocked(call).mockReset();
  vi.mocked(on).mockReset();
});

describe("the card of a Task", () => {
  it("shows the status dot with its name: To do, In progress, Agent working", async () => {
    await show(
      snapshot({
        active: [
          task("aaaaaaaa", "T1", "First"),
          task("bbbbbbbb", "T2", "Second", { doing: true }),
          task("cccccccc", "T3", "Third", { doing: true, heldByAgent: true }),
        ],
      }),
    );
    const names = within(active())
      .getAllByRole("img")
      .map((img) => img.getAttribute("aria-label"))
      .filter((name) => ["To do", "In progress", "Agent working"].includes(name ?? ""));
    expect(names).toEqual(["To do", "In progress", "Agent working"]);
  });

  it("shows technical, blocked and waiting badges, and the label badge, with their names", async () => {
    await show(
      snapshot({
        active: [
          task("aaaaaaaa", "T1", "First", {
            technical: true,
            label: "Checkout",
            blockedBy: [
              { id: "d1111111", shortNumber: "D3", title: "Total is wrong" },
              { id: "d2222222", shortNumber: "D7", title: "Tax is wrong" },
            ],
            waitingFor: [{ id: "ffffffff", shortNumber: "T4", title: "Fix the total" }],
          }),
        ],
      }),
    );
    const card = within(active());
    expect(card.getByText("technical")).toBeInTheDocument();
    expect(card.getByRole("button", { name: "Blocked by Drift D3, D7" })).toHaveTextContent("blocked");
    expect(card.getByRole("button", { name: "Waiting for Task T4" })).toHaveTextContent("waiting T4");
    expect(card.getByText("Checkout")).toBeInTheDocument();
  });

  it("has a left border in the label's colour, the border token outside a design, and the rotating border while held", async () => {
    await show(
      snapshot({
        active: [
          task("aaaaaaaa", "T1", "Labelled", { label: "Checkout" }),
          task("bbbbbbbb", "T2", "Plain"),
          task("cccccccc", "T3", "Held", { doing: true, heldByAgent: true }),
        ],
      }),
    );
    const frame = (id: string) => active().querySelector<HTMLElement>(`[data-card-id="${id}"]`)!;
    for (const name of leftBorderClass("Checkout").split(" ")) expect(frame("aaaaaaaa")).toHaveClass(name);
    for (const name of leftBorderClass(null).split(" ")) expect(frame("bbbbbbbb")).toHaveClass(name);
    expect(frame("cccccccc")).toHaveClass("task-held");
    expect(frame("aaaaaaaa")).not.toHaveClass("task-held");
    expect(frame("bbbbbbbb")).not.toHaveClass("task-held");
  });

  it("opens a popover from `blocked` listing the Drifts, and a row focuses that Drift's card", async () => {
    const user = userEvent.setup();
    await show(
      snapshot({
        backlog: [drift("d1111111", "D3", "Total is wrong")],
        active: [task("aaaaaaaa", "T1", "First", { blockedBy: [{ id: "d1111111", shortNumber: "D3", title: "Total is wrong" }] })],
      }),
    );
    await user.click(within(active()).getByRole("button", { name: "Blocked by Drift D3" }));
    const popover = await screen.findByRole("dialog", { name: "Blocked by" });
    await user.click(within(popover).getByRole("button", { name: /D3\s*Total is wrong/ }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Blocked by" })).not.toBeInTheDocument());
    const title = within(backlog()).getByRole("button", { name: "Total is wrong" });
    await waitFor(() => expect(title).toHaveFocus());
  });

  it("opens a popover from `waiting` listing the fix Task", async () => {
    const user = userEvent.setup();
    await show(
      snapshot({
        active: [
          task("aaaaaaaa", "T1", "First", { waitingFor: [{ id: "ffffffff", shortNumber: "T4", title: "Fix the total" }] }),
          task("ffffffff", "T4", "Fix the total"),
        ],
      }),
    );
    await user.click(within(active()).getByRole("button", { name: "Waiting for Task T4" }));
    const popover = await screen.findByRole("dialog", { name: "Waiting for" });
    expect(within(popover).getByRole("button", { name: /T4\s*Fix the total/ })).toBeInTheDocument();
    await user.click(within(popover).getByRole("button", { name: /T4\s*Fix the total/ }));
    await waitFor(() => expect(within(active()).getAllByRole("button", { name: "Fix the total" })[0]).toHaveFocus());
  });
});

describe("Run", () => {
  it("lists every reason it is disabled, in order, and does nothing while disabled", async () => {
    const user = userEvent.setup();
    const runTask = vi.fn();
    const reasons = ["The agent is still finishing", "Drifts are being resolved", "A run is in progress", "Blocked by Drift D3", "Waiting for Task T4"];
    await show(snapshot({ active: [task("aaaaaaaa", "T1", "First", { runReasons: reasons })] }), { runTask });
    const run = within(active()).getByRole("button", { name: "Run" });
    expect(run).toHaveAttribute("aria-disabled", "true");
    expect(reasonsOf(run)).toEqual(reasons);
    await user.click(run);
    expect(runTask).not.toHaveBeenCalled();
  });

  it("runs the Task when it has no reason", async () => {
    const user = userEvent.setup();
    const runTask = vi.fn();
    await show(snapshot({ active: [task("aaaaaaaa", "T1", "First")] }), { runTask });
    const run = within(active()).getByRole("button", { name: "Run" });
    expect(run).not.toHaveAttribute("aria-disabled");
    await user.click(run);
    expect(runTask).toHaveBeenCalledWith("aaaaaaaa");
  });

  it("shows `View on diagram` and `View requirements` only when their focus has a step", async () => {
    const user = userEvent.setup();
    const viewOn = vi.fn();
    const focusSteps = vi.fn((view: string) => (view === "diagram" ? 2 : 0));
    await show(snapshot({ active: [task("aaaaaaaa", "T1", "First", { refs: ["order/Paid"] })] }), { viewOn, focusSteps });
    expect(within(active()).queryByRole("button", { name: "View requirements" })).not.toBeInTheDocument();
    await user.click(within(active()).getByRole("button", { name: "View on diagram" }));
    expect(viewOn).toHaveBeenCalledWith("diagram", ["order/Paid"], { kind: "task", id: "aaaaaaaa", number: "T1", title: "First" });
  });
});

describe("the details of a Task", () => {
  const details = async (card: BoardCard, actions: BoardActions = {}) => {
    const user = userEvent.setup();
    await show(snapshot({ active: [card] }), actions);
    await user.click(within(active()).getByRole("button", { name: card.head.title }));
    return { user, dialog: await screen.findByRole("dialog") };
  };

  it("shows the status, the references as chips and the text", async () => {
    const card = task("aaaaaaaa", "T1", "First", { refs: ["order/Paid", "user"], label: "Checkout" });
    const withBody = { ...card, head: { ...card.head, body: "Carry out the **cart**." } } as BoardCard;
    const { dialog } = await details(withBody);
    expect(dialog).toHaveTextContent("To do");
    expect(dialog).toHaveTextContent("Carry out the cart.");
    expect(within(dialog).getByRole("list", { name: "References" })).toHaveTextContent("order/Paid");
    expect(within(dialog).getByRole("list", { name: "References" })).toHaveTextContent("user");
  });

  it("a chip focuses the diagram on that one reference", async () => {
    const viewOn = vi.fn();
    const { user, dialog } = await details(task("aaaaaaaa", "T1", "First", { refs: ["order/Paid", "user"] }), { viewOn, focusSteps: () => 1 });
    await user.click(within(dialog).getByRole("button", { name: "View on diagram: user" }));
    expect(viewOn).toHaveBeenCalledWith("diagram", ["user"]);
  });

  it("has Delete only in To do, lists its reasons in order, and confirms with `Delete T1 for good?`", async () => {
    const reasons = ["The agent is still finishing", "Origin of open Drift D3", "Fix Task of Drift D5, whose code is to be fixed"];
    const { user, dialog } = await details(task("aaaaaaaa", "T1", "First", { deleteReasons: reasons }));
    const remove = within(dialog).getByRole("button", { name: "Delete" });
    expect(remove).toHaveAttribute("aria-disabled", "true");
    expect(reasonsOf(remove)).toEqual(reasons);
    await user.click(remove);
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("confirms Delete and sends the decision", async () => {
    const { user, dialog } = await details(task("aaaaaaaa", "T1", "First"));
    await user.click(within(dialog).getByRole("button", { name: "Delete" }));
    const confirm = await screen.findByRole("alertdialog");
    expect(within(confirm).getByText("Delete T1 for good?")).toBeInTheDocument();
    expect(within(confirm).getByRole("button", { name: "Cancel" })).toHaveFocus();
    await user.click(within(confirm).getByRole("button", { name: "Delete" }));
    expect(vi.mocked(call)).toHaveBeenCalledWith("delete_task", { projectId: PROJECT, taskId: "aaaaaaaa" });
  });

  it("has no Delete once the Task is in progress, and shows `Run` with the agent's reason", async () => {
    const { dialog } = await details(task("aaaaaaaa", "T1", "First", { doing: true, heldByAgent: true, runReasons: ["The agent is working on it"] }));
    expect(dialog).toHaveTextContent("Agent working");
    expect(within(dialog).queryByRole("button", { name: "Delete" })).not.toBeInTheDocument();
    expect(reasonsOf(within(dialog).getByRole("button", { name: "Run" }))).toEqual(["The agent is working on it"]);
  });
});
