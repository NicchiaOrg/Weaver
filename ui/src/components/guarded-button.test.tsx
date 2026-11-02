import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import {
  activeReasons,
  AGENT_FINISHING,
  cardReasons,
  GuardedButton,
  type GuardedButtonProps,
} from "@/components/guarded-button";
import { TooltipProvider } from "@/components/ui/tooltip";

function ui(props: GuardedButtonProps) {
  return (
    <TooltipProvider>
      <GuardedButton {...props}>Run</GuardedButton>
    </TooltipProvider>
  );
}

const REASONS = ["Drifts are being resolved", "A run is in progress", "The agent is working on it"];

describe("reasons", () => {
  it("keeps the order given, drops falsy entries and repeats", () => {
    expect(activeReasons([false, "B", null, "A", undefined, "B"])).toEqual(["B", "A"]);
  });

  it("puts `The agent is still finishing` first on the controls of a card", () => {
    expect(cardReasons(true, ["A run is in progress", "Blocked by Drift D3"])).toEqual([
      AGENT_FINISHING,
      "A run is in progress",
      "Blocked by Drift D3",
    ]);
    expect(cardReasons(false, ["A run is in progress"])).toEqual(["A run is in progress"]);
    expect(cardReasons(true, [AGENT_FINISHING, "x"])).toEqual([AGENT_FINISHING, "x"]);
  });
});

describe("GuardedButton", () => {
  it("uses aria-disabled, not disabled, and stays focusable", async () => {
    const user = userEvent.setup();
    render(ui({ reasons: REASONS }));
    const button = screen.getByRole("button", { name: "Run" });
    expect(button).toHaveAttribute("aria-disabled", "true");
    expect(button).not.toBeDisabled();
    await user.tab();
    expect(button).toHaveFocus();
  });

  it("ignores click, Enter and Space", async () => {
    const onClick = vi.fn();
    const user = userEvent.setup();
    render(ui({ reasons: REASONS, onClick }));
    const button = screen.getByRole("button", { name: "Run" });
    await user.click(button);
    button.focus();
    await user.keyboard("{Enter}");
    await user.keyboard(" ");
    expect(onClick).not.toHaveBeenCalled();
  });

  it("does not submit a form while it is blocked", async () => {
    const onSubmit = vi.fn((e: React.FormEvent) => e.preventDefault());
    const user = userEvent.setup();
    render(
      <TooltipProvider>
        <form onSubmit={onSubmit}>
          <GuardedButton type="submit" reasons={["No Task is in To do"]}>
            Run all
          </GuardedButton>
        </form>
      </TooltipProvider>,
    );
    await user.click(screen.getByRole("button", { name: "Run all" }));
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("lists every reason, one per line and in the order given, in the accessible description", () => {
    render(ui({ reasons: [false, ...REASONS, null] }));
    const button = screen.getByRole("button", { name: "Run" });
    const description = document.getElementById(button.getAttribute("aria-describedby") ?? "");
    expect(description).not.toBeNull();
    const lines = [...(description?.children ?? [])];
    expect(lines.map((line) => line.textContent)).toEqual(REASONS);
    for (const line of lines) expect(line).toHaveClass("block");
  });

  it("shows every reason, in order, in the tooltip at keyboard focus", async () => {
    const user = userEvent.setup();
    render(ui({ reasons: [AGENT_FINISHING, ...REASONS], tooltip: "Run this Task" }));
    await user.tab();
    const tooltip = await screen.findByRole("tooltip");
    const lines = [...tooltip.children].map((line) => line.textContent);
    expect(lines).toEqual([AGENT_FINISHING, ...REASONS]);
  });

  it("shows the tooltip of an enabled control on focus and closes it with Esc", async () => {
    const user = userEvent.setup();
    render(ui({ tooltip: "Run the Task" }));
    const button = screen.getByRole("button", { name: "Run" });
    expect(button).not.toHaveAttribute("aria-disabled");
    expect(button).not.toHaveAttribute("aria-describedby");
    await user.tab();
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Run the Task");
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("opens its tooltip after hovering, not before 400 ms", async () => {
    const user = userEvent.setup();
    render(ui({ reasons: REASONS }));
    const button = screen.getByRole("button", { name: "Run" });
    await user.hover(button);
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    expect(await screen.findByRole("tooltip", undefined, { timeout: 2000 })).toBeInTheDocument();
    // Esc closes it (pointer leave and blur are Radix's own, driven by real pointer geometry).
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("re-enables itself in place: same element, same focus, and it works at once", async () => {
    const onClick = vi.fn();
    const user = userEvent.setup();
    const { rerender } = render(ui({ reasons: ["A run is in progress"], onClick }));
    await user.tab();
    const button = screen.getByRole("button", { name: "Run" });
    const reasonsId = (button.getAttribute("aria-describedby") ?? "").split(" ")[0] ?? "";
    expect(reasonsId).not.toBe("");
    expect(button).toHaveFocus();

    rerender(ui({ reasons: [false], onClick }));
    expect(screen.getByRole("button", { name: "Run" })).toBe(button);
    expect(button).toHaveFocus();
    expect(button).not.toHaveAttribute("aria-disabled");
    // The description of the reasons is gone with them.
    expect(document.getElementById(reasonsId)).toBeNull();
    expect(button.getAttribute("aria-describedby") ?? "").not.toContain(reasonsId);

    await user.keyboard("{Enter}");
    expect(onClick).toHaveBeenCalledTimes(1);

    rerender(ui({ reasons: ["A run is in progress"], onClick }));
    expect(button).toHaveFocus();
    expect(button).toHaveAttribute("aria-disabled", "true");
  });

  it("gives an icon-only button its accessible name and shows it first in the tooltip", async () => {
    const user = userEvent.setup();
    render(
      <TooltipProvider>
        <GuardedButton size="icon" label="Close" reasons={["Nothing to close"]}>
          x
        </GuardedButton>
      </TooltipProvider>,
    );
    expect(screen.getByRole("button", { name: "Close" })).toBeInTheDocument();
    await user.tab();
    const tooltip = await screen.findByRole("tooltip");
    expect([...tooltip.children].map((c) => c.textContent)).toEqual(["Close", "Nothing to close"]);
  });
});
