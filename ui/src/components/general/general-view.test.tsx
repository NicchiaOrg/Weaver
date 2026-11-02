import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { Entry } from "@/bindings/Entry";
import type { GeneralState } from "@/bindings/GeneralState";
import type { OmpStatus } from "@/bindings/OmpStatus";
import { GeneralView } from "@/components/general/general-view";
import { ProjectContext } from "@/components/project-context";
import { TooltipProvider } from "@/components/ui/tooltip";
import { call, on } from "@/lib/backend";

vi.mock("@/lib/backend", () => ({
  call: vi.fn(),
  on: vi.fn(),
  isAppError: (value: unknown) => typeof value === "object" && value !== null && "code" in value && "message" in value,
}));

const ompStatus = vi.hoisted(() => ({ current: null as OmpStatus | null }));
vi.mock("@/hooks/use-omp", () => ({ useOmp: () => ({ status: ompStatus.current }) }));

const PROJECT = "0123456789abcdef";

function state(patch: Partial<GeneralState> = {}): GeneralState {
  return { projectId: PROJECT, working: false, starting: false, kept: false, ...patch };
}

const handlers = new Map<string, (payload: never) => void>();

function mount(entries: Entry[], general: GeneralState = state(), openJob = vi.fn()) {
  handlers.clear();
  vi.mocked(on).mockImplementation(async (name: string, handler: unknown) => {
    handlers.set(name, handler as (payload: never) => void);
    return () => {};
  });
  vi.mocked(call).mockImplementation(async (name: string) => {
    if (name === "general_snapshot") return { state: general, seq: 0, entries };
    return null;
  });
  render(
    <TooltipProvider>
      <ProjectContext.Provider value={{ projectId: PROJECT, showView: vi.fn(), openJob, openedJob: null }}>
        <GeneralView />
      </ProjectContext.Provider>
    </TooltipProvider>,
  );
  return { openJob };
}

const developer = (text: string, status: "queued" | "delivered" = "delivered"): Entry => ({
  kind: "developer",
  message: 1,
  text,
  delivery: { status },
});

describe("GeneralView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    ompStatus.current = null;
  });

  it("shows the invitation when the conversation is empty, with the message box enabled and Clear disabled", async () => {
    mount([]);
    expect(await screen.findByText("Talk with omp in this project. Ask it to start a job of Weaver when you want one.")).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Message omp" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Clear" })).toHaveAttribute("aria-disabled", "true");
    expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
  });

  it("keeps Clear usable while a kept session waits for the first message", async () => {
    mount([], state({ kept: true }));
    await screen.findByText(/Talk with omp/);
    await waitFor(() => expect(screen.getByRole("button", { name: "Clear" })).not.toHaveAttribute("aria-disabled"));
  });

  it("sends the message with Enter and keeps Shift+Enter for a new line", async () => {
    mount([]);
    const user = userEvent.setup();
    const box = await screen.findByRole("textbox", { name: "Message omp" });
    await user.type(box, "Hello{Shift>}{Enter}{/Shift}omp{Enter}");
    await waitFor(() => expect(call).toHaveBeenCalledWith("general_send_message", { projectId: PROJECT, text: "Hello\nomp" }));
    expect(box).toHaveValue("");
  });

  it("shows Starting omp… under the message that waits while omp starts", async () => {
    mount([developer("Hi omp", "queued")], state({ working: true, starting: true }));
    expect(await screen.findByText("Hi omp")).toBeInTheDocument();
    expect(screen.getByText("Starting omp…")).toBeInTheDocument();
  });

  it("stops the turn at once, without a dialog", async () => {
    mount([developer("Do it")], state({ working: true }));
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "Stop" }));
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(call).toHaveBeenCalledWith("general_stop", { projectId: PROJECT });
  });

  it("asks before clearing, with another text during a turn, and clears only when confirmed", async () => {
    mount([developer("One")]);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "Clear" }));
    let dialog = await screen.findByRole("alertdialog");
    expect(within(dialog).getByText("Clear General?")).toBeInTheDocument();
    expect(within(dialog).getByText("The conversation is deleted and a new one starts.")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Cancel" })).toHaveFocus();
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(call).not.toHaveBeenCalledWith("general_clear", expect.anything());

    act(() => handlers.get("general-changed")?.(state({ working: true }) as never));
    await user.click(screen.getByRole("button", { name: "Clear" }));
    dialog = await screen.findByRole("alertdialog");
    expect(within(dialog).getByText("The agent's turn stops, the conversation is deleted and a new one starts.")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Clear" }));
    await waitFor(() => expect(call).toHaveBeenCalledWith("general_clear", { projectId: PROJECT }));
  });

  it("shows why omp stopped, then says the next message starts it again", async () => {
    mount([
      developer("Hi"),
      { kind: "omp-stopped", reason: "omp exited with code 3", details: "boom" },
      { kind: "turn-failed", reason: "rate limited" },
      { kind: "notice", text: "The conversation could not be reloaded (gone). A new one started." },
    ]);
    expect(await screen.findByText("omp stopped: omp exited with code 3.")).toBeInTheDocument();
    expect(screen.getByText("Your next message starts omp again.")).toBeInTheDocument();
    expect(screen.getByText("The turn failed: rate limited.")).toBeInTheDocument();
    expect(screen.getByText("The conversation could not be reloaded (gone). A new one started.")).toBeInTheDocument();
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Show details" }));
    expect(screen.getByText("boom")).toBeInTheDocument();
  });

  it("offers to install omp when it is missing", async () => {
    ompStatus.current = { state: "missing" } as OmpStatus;
    mount([]);
    const user = userEvent.setup();
    expect(await screen.findByText("omp is not installed.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Install omp" }));
    expect(call).toHaveBeenCalledWith("omp_install");
  });

  it("shows a started job with a button that opens its conversation", async () => {
    const { openJob } = mount([developer("Scan please"), { kind: "job-started", job: "aaaaaaaa", name: "Scan System" }]);
    expect(await screen.findByText("Started Scan System")).toBeInTheDocument();
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Open conversation" }));
    expect(openJob).toHaveBeenCalledWith("aaaaaaaa");
  });

  it("shows a refused start in the warning colour", async () => {
    mount([{ kind: "job-refused", jobKind: "run", reason: "A run is in progress" }]);
    const line = await screen.findByText("Weaver did not start run: A run is in progress.");
    expect(line).toHaveClass("text-amber-600");
  });

  it("answers a permission card through General", async () => {
    mount([
      {
        kind: "permission",
        request: 4,
        tool: null,
        state: { status: "asking" },
        options: [{ id: "allow", name: "Allow once", rejects: false }],
      },
    ]);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "Allow once" }));
    expect(call).toHaveBeenCalledWith("general_answer_permission", { projectId: PROJECT, request: 4, option: "allow" });
  });

  it("applies the batches of entries that follow the snapshot and fetches again on a reset", async () => {
    mount([developer("One")]);
    await screen.findByText("One");
    act(() =>
      handlers.get("general-entries")?.({
        projectId: PROJECT,
        seq: 1,
        reset: false,
        changes: [{ index: 1, entry: { kind: "agent", text: "Answer", complete: true } }],
      } as never),
    );
    expect(await screen.findByText("Answer")).toBeInTheDocument();
    const fetches = () => vi.mocked(call).mock.calls.filter(([name]) => name === "general_snapshot").length;
    expect(fetches()).toBe(1);
    act(() => handlers.get("general-entries")?.({ projectId: PROJECT, seq: 2, reset: true, changes: [] } as never));
    await waitFor(() => expect(fetches()).toBe(2));
  });
});
