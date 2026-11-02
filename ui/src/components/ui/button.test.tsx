import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Check } from "lucide-react";
import { describe, expect, it, vi } from "vitest";

import { Button } from "@/components/ui/button";

describe("Button", () => {
  it("is found by its accessible name and activates on click", async () => {
    const user = userEvent.setup();
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Approve</Button>);

    await user.click(screen.getByRole("button", { name: "Approve" }));

    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("is reached with Tab and activated with Enter and with Space", async () => {
    const user = userEvent.setup();
    const onClick = vi.fn();
    render(
      <>
        <Button variant="outline">Before</Button>
        <Button onClick={onClick}>Run</Button>
      </>,
    );

    await user.tab();
    await user.tab();
    expect(screen.getByRole("button", { name: "Run" })).toHaveFocus();

    await user.keyboard("{Enter}");
    await user.keyboard(" ");
    expect(onClick).toHaveBeenCalledTimes(2);
  });

  it("keeps the accessible name of an icon-only button", () => {
    render(
      <Button size="icon" aria-label="Close">
        <Check aria-hidden />
      </Button>,
    );

    expect(screen.getByRole("button", { name: "Close" })).toBeInTheDocument();
  });

  it("ignores clicks while disabled", async () => {
    const user = userEvent.setup();
    const onClick = vi.fn();
    render(
      <Button disabled onClick={onClick}>
        Delete
      </Button>,
    );

    await user.click(screen.getByRole("button", { name: "Delete" }));

    expect(onClick).not.toHaveBeenCalled();
  });
});
