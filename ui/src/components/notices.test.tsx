import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { NoticeStack } from "@/components/notices";
import { TooltipProvider } from "@/components/ui/tooltip";

describe("NoticeStack", () => {
  it("shows the newest notice first, each with a close button", async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();
    render(
      <TooltipProvider>
        <NoticeStack
          notices={[
            { id: "a", text: "Could not delete T4: the agent that filed it is still working." },
            { id: "b", text: "P3 went back to Draft while it was being decided." },
          ]}
          onClose={onClose}
        />
      </TooltipProvider>,
    );
    const alerts = screen.getAllByRole("alert");
    expect(alerts.map((a) => a.textContent)).toEqual([
      "P3 went back to Draft while it was being decided.",
      "Could not delete T4: the agent that filed it is still working.",
    ]);
    await user.click(screen.getAllByRole("button", { name: "Close notice" })[1] as HTMLElement);
    expect(onClose).toHaveBeenCalledWith("a");
  });

  it("renders nothing without notices", () => {
    const { container } = render(<NoticeStack notices={[]} onClose={() => {}} />);
    expect(container).toBeEmptyDOMElement();
  });
});
