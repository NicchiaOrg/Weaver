import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { act } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { UpdateStatus } from "@/bindings/UpdateStatus";
import { UpdateBanner } from "@/components/update-banner";
import { call, on } from "@/lib/backend";

vi.mock("@/lib/backend", () => ({
  call: vi.fn(),
  on: vi.fn(),
  isAppError: (value: unknown) => typeof value === "object" && value !== null && "code" in value && "message" in value,
}));

const offered: UpdateStatus = { available: { version: "0.2.0", notes: "Faster start.\nFewer crashes." }, downloading: false };

let push: (status: UpdateStatus) => void;

function backendAnswers(status: UpdateStatus, others: Record<string, () => Promise<unknown>> = {}) {
  vi.mocked(call).mockImplementation(((name: string) =>
    name === "update_status" ? Promise.resolve(status) : (others[name]?.() ?? Promise.resolve(null))) as never);
}

beforeEach(() => {
  vi.mocked(call).mockReset();
  vi.mocked(on).mockImplementation(((_name: string, handler: (s: UpdateStatus) => void) => {
    push = handler;
    return Promise.resolve(() => {});
  }) as unknown as typeof on);
});

describe("update banner", () => {
  it("shows the version and the notes of an update found, with Install and restart and Later", async () => {
    backendAnswers(offered);
    render(<UpdateBanner />);
    expect(await screen.findByText("Weaver 0.2.0 is available.")).toBeInTheDocument();
    expect(screen.getByText(/Faster start\.\s+Fewer crashes\./)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Install and restart" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Later" })).toBeInTheDocument();
  });

  it("shows nothing without an update, and an update that arrives later appears", async () => {
    backendAnswers({ available: null, downloading: false });
    const { container } = render(<UpdateBanner />);
    await act(async () => {});
    expect(container).toBeEmptyDOMElement();
    act(() => push(offered));
    expect(await screen.findByText("Weaver 0.2.0 is available.")).toBeInTheDocument();
    act(() => push({ available: null, downloading: false }));
    expect(container).toBeEmptyDOMElement();
  });

  it("installs only when Install and restart is chosen", async () => {
    const user = userEvent.setup();
    backendAnswers(offered);
    render(<UpdateBanner />);
    await screen.findByText("Weaver 0.2.0 is available.");
    expect(call).not.toHaveBeenCalledWith("update_install");
    await user.click(screen.getByRole("button", { name: "Install and restart" }));
    expect(call).toHaveBeenCalledWith("update_install");
  });

  it("Later asks the backend to hide the update", async () => {
    const user = userEvent.setup();
    backendAnswers(offered);
    render(<UpdateBanner />);
    await user.click(await screen.findByRole("button", { name: "Later" }));
    expect(call).toHaveBeenCalledWith("update_later");
    expect(call).not.toHaveBeenCalledWith("update_install");
  });

  it("says why an update was refused", async () => {
    const user = userEvent.setup();
    backendAnswers(offered, {
      update_install: () =>
        Promise.reject({ code: "internal", message: "The update was refused: its signature does not verify." }),
    });
    render(<UpdateBanner />);
    await user.click(await screen.findByRole("button", { name: "Install and restart" }));
    expect(await screen.findByText("The update was refused: its signature does not verify.")).toBeInTheDocument();
  });

  it("does not act while the update downloads", async () => {
    const user = userEvent.setup();
    backendAnswers({ ...offered, downloading: true });
    render(<UpdateBanner />);
    const button = await screen.findByRole("button", { name: "Downloading…" });
    expect(button).toHaveAttribute("aria-disabled", "true");
    await user.click(button);
    await user.click(screen.getByRole("button", { name: "Later" }));
    expect(call).not.toHaveBeenCalledWith("update_install");
    expect(call).not.toHaveBeenCalledWith("update_later");
  });
});
