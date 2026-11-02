import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ProjectStatus } from "@/bindings/ProjectStatus";
import { GetStarted, initialiseFailure } from "@/components/project/get-started";
import { ProjectGate } from "@/components/project/project-gate";
import { ProjectStatusProvider } from "@/components/project/use-project-status";
import { call, on } from "@/lib/backend";

vi.mock("@/lib/backend", () => ({
  call: vi.fn(),
  on: vi.fn(),
  isAppError: (value: unknown) => typeof value === "object" && value !== null && "code" in value && "message" in value,
}));

const PROJECT = "0123456789abcdef";
const FILES = [".agents/skills/trama-setup/SKILL.md", ".omp/agents/trama-analysis.md", ".omp/WATCHDOG.yml"];

function status(patch: Partial<ProjectStatus> = {}): ProjectStatus {
  return {
    projectId: PROJECT,
    revision: 1,
    state: "not-initialised",
    files: FILES,
    folderHasFiles: false,
    check: { opening: 0, outcome: { kind: "none" } },
    ...patch,
  };
}

let push: (status: ProjectStatus) => void;
let initialise: (force: boolean) => Promise<unknown>;

beforeEach(() => {
  vi.mocked(call).mockReset();
  vi.mocked(on).mockImplementation(((_name: string, handler: (s: ProjectStatus) => void) => {
    push = handler;
    return Promise.resolve(() => {});
  }) as unknown as typeof on);
  initialise = () => Promise.resolve({ conflicts: [] });
  vi.mocked(call).mockImplementation(((name: string, args: { force: boolean }) => {
    if (name === "project_status") return Promise.resolve(status());
    if (name === "initialise_project") return initialise(args.force);
    return Promise.reject(new Error(`unexpected command ${name}`));
  }) as unknown as typeof call);
});

function renderGetStarted() {
  return render(
    <ProjectStatusProvider projectId={PROJECT}>
      <GetStarted />
    </ProjectStatusProvider>,
  );
}

describe("Get Started", () => {
  it("explains the folder, lists the agent files and offers Initialise", async () => {
    renderGetStarted();
    expect(await screen.findByRole("heading", { name: "Set up Weaver in this folder" })).toBeInTheDocument();
    expect(
      screen.getByText(
        "Weaver will create a .trama folder that describes this project. It holds no secrets and is meant to be committed, like the agent files it installs.",
      ),
    ).toBeInTheDocument();
    const files = within(screen.getByRole("list", { name: "Agent files" })).getAllByRole("listitem");
    expect(files.map((item) => item.textContent)).toEqual(FILES);
    expect(screen.getByRole("button", { name: "Initialise" })).toBeEnabled();
  });

  it("initialises and asks the backend for the new status", async () => {
    const user = userEvent.setup();
    renderGetStarted();
    await user.click(await screen.findByRole("button", { name: "Initialise" }));
    expect(call).toHaveBeenCalledWith("initialise_project", { projectId: PROJECT, force: false });
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("asks before overwriting agent files, with Cancel focused, and changes nothing on Cancel", async () => {
    const user = userEvent.setup();
    initialise = () => Promise.resolve({ conflicts: [".omp/WATCHDOG.yml", ".agents/skills/trama-setup/SKILL.md"] });
    renderGetStarted();
    const button = await screen.findByRole("button", { name: "Initialise" });
    await user.click(button);
    const dialog = await screen.findByRole("alertdialog", { name: "Some agent files already exist" });
    expect(within(dialog).getAllByRole("listitem").map((item) => item.textContent)).toEqual([
      ".omp/WATCHDOG.yml",
      ".agents/skills/trama-setup/SKILL.md",
    ]);
    expect(within(dialog).getByRole("button", { name: "Cancel" })).toHaveFocus();
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(vi.mocked(call).mock.calls.filter(([name]) => name === "initialise_project")).toHaveLength(1);
    expect(button).toHaveFocus();
  });

  it("overwrites and initialises when the developer confirms", async () => {
    const user = userEvent.setup();
    initialise = (force) => Promise.resolve({ conflicts: force ? [] : [".omp/WATCHDOG.yml"] });
    renderGetStarted();
    await user.click(await screen.findByRole("button", { name: "Initialise" }));
    await user.click(await screen.findByRole("button", { name: "Overwrite and initialise" }));
    expect(call).toHaveBeenCalledWith("initialise_project", { projectId: PROJECT, force: true });
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("shows `Could not initialise: <reason>.` under the button and keeps the button for a retry", async () => {
    const user = userEvent.setup();
    initialise = () => Promise.reject({ code: "refused", message: "the folder is read-only" });
    renderGetStarted();
    await user.click(await screen.findByRole("button", { name: "Initialise" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not initialise: the folder is read-only.");
    expect(screen.getByRole("button", { name: "Initialise" })).toBeEnabled();
    expect(initialiseFailure("no space left.")).toBe("Could not initialise: no space left.");
  });
});

describe("ProjectGate", () => {
  it("shows Get Started in place of a view until the project is initialised, then the view", async () => {
    render(
      <ProjectStatusProvider projectId={PROJECT}>
        <ProjectGate>
          <p>The Board</p>
        </ProjectGate>
      </ProjectStatusProvider>,
    );
    expect(await screen.findByRole("heading", { name: "Set up Weaver in this folder" })).toBeInTheDocument();
    expect(screen.queryByText("The Board")).not.toBeInTheDocument();
    // An older status than the one shown is dropped.
    act(() => push(status({ revision: 0, state: "has-model" })));
    expect(screen.queryByText("The Board")).not.toBeInTheDocument();
    act(() => push(status({ revision: 2, state: "empty-model" })));
    expect(screen.getByText("The Board")).toBeInTheDocument();
    act(() => push(status({ revision: 3, state: "not-initialised" })));
    expect(screen.getByRole("heading", { name: "Set up Weaver in this folder" })).toBeInTheDocument();
  });

  it("shows the view as it is when the status cannot be read", async () => {
    vi.mocked(call).mockImplementation((() => Promise.reject(new Error("closed"))) as unknown as typeof call);
    render(
      <ProjectStatusProvider projectId={PROJECT}>
        <ProjectGate>
          <p>The Board</p>
        </ProjectGate>
      </ProjectStatusProvider>,
    );
    expect(await screen.findByText("The Board")).toBeInTheDocument();
  });
});
