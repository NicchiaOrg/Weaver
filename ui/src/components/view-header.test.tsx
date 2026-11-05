import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { ViewHeader } from "@/components/view-header";
import { VIEWS } from "@/views";

describe("the view header", () => {
  it("shows the name of the sidebar entry as the text-only title, 14 px semibold", () => {
    for (const view of VIEWS) {
      const { unmount } = render(<ViewHeader view={view.id} />);
      const title = screen.getByRole("heading", { level: 1, name: view.label });
      expect(title).toHaveClass("text-sm", "font-semibold");
      expect(title.querySelector("svg")).toBeNull();
      expect(title).not.toHaveClass("sr-only");
      unmount();
    }
  });

  it("has the same fixed height of 56 px plus its 1 px bottom border, with and without controls", () => {
    const { container } = render(
      <>
        <ViewHeader view="board" />
        <ViewHeader view="history">
          <button type="button">Control</button>
        </ViewHeader>
      </>,
    );
    const headers = container.querySelectorAll("header");
    expect(headers).toHaveLength(2);
    for (const header of headers) {
      expect(header).toHaveClass("box-content", "h-14", "border-b", "px-4", "items-center");
    }
  });

  it("puts the controls on the right of the title", () => {
    render(
      <ViewHeader view="history">
        <button type="button">Control</button>
      </ViewHeader>,
    );
    const controls = screen.getByRole("button", { name: "Control" }).parentElement!;
    expect(controls).toHaveClass("ml-auto");
    expect(screen.getByRole("heading", { name: "History" }).nextElementSibling).toBe(controls);
  });
});
