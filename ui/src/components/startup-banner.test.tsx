import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { act } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { StartupFailure } from "@/bindings/StartupFailure";
import { startupMessage, StartupBanner } from "@/components/startup-banner";
import { call, on } from "@/lib/backend";

vi.mock("@/lib/backend", () => ({ call: vi.fn(), on: vi.fn() }));

const app: StartupFailure = { projectId: null, projectName: null, step: "local socket", reason: "address already in use" };
const acme: StartupFailure = { projectId: "aaaa", projectName: "Acme", step: "skills", reason: "cannot write agent files" };
const other: StartupFailure = { projectId: "bbbb", projectName: "Other", step: "skills", reason: "disk full" };

let push: (failure: StartupFailure) => void;

beforeEach(() => {
  vi.mocked(on).mockImplementation(((_name: string, handler: (f: StartupFailure) => void) => {
    push = handler;
    return Promise.resolve(() => {});
  }) as unknown as typeof on);
});

describe("startup banner", () => {
  it("words the failures as `Weaver could not start: <reason>` and `<project> could not open: <reason>`", () => {
    expect(startupMessage(app)).toBe("Weaver could not start: address already in use");
    expect(startupMessage(acme)).toBe("Acme could not open: cannot write agent files");
  });

  it("shows the app's failures and its own project's, with Open log, and nobody else's", async () => {
    vi.mocked(call).mockImplementation((name: string) => Promise.resolve(name === "startup_failures" ? [app, acme, other] : null) as never);
    render(<StartupBanner projectId="aaaa" />);
    expect(await screen.findByText("Weaver could not start: address already in use")).toBeInTheDocument();
    expect(screen.getByText("Acme could not open: cannot write agent files")).toBeInTheDocument();
    expect(screen.queryByText(/Other could not open/)).not.toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Open log" })).toHaveLength(2);
  });

  it("reveals the log with Open log", async () => {
    const user = userEvent.setup();
    vi.mocked(call).mockImplementation((name: string) => Promise.resolve(name === "startup_failures" ? [app] : null) as never);
    render(<StartupBanner />);
    await user.click(await screen.findByRole("button", { name: "Open log" }));
    expect(call).toHaveBeenCalledWith("open_log");
  });

  it("shows a failure announced after the window loaded, once", async () => {
    vi.mocked(call).mockImplementation(() => Promise.resolve([]) as never);
    render(<StartupBanner />);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    await act(async () => {});
    act(() => push(app));
    act(() => push(app));
    expect(await screen.findAllByRole("alert")).toHaveLength(1);
  });

  it("shows nothing when every step worked", async () => {
    vi.mocked(call).mockImplementation(() => Promise.resolve([]) as never);
    const { container } = render(<StartupBanner />);
    await act(async () => {});
    expect(container).toBeEmptyDOMElement();
  });
});
