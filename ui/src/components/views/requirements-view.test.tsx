import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ViewSnapshot } from "@/bindings/ViewSnapshot";
import type { FocusSubject } from "@/components/views/focus-model";
import { RequirementsView } from "@/components/views/requirements-view";
import { useViews } from "@/components/views/views-state";
import { snapshot } from "@/components/views/view-fixtures";
import { fakeBackend, renderViews, type FakeBackend } from "@/components/views/view-test-utils";

vi.mock("@/lib/backend", () => ({
  call: vi.fn(),
  on: vi.fn(),
  isAppError: (value: unknown) => typeof value === "object" && value !== null && "code" in value && "message" in value,
}));

let current: ViewSnapshot;
let backend: FakeBackend;
const showView = vi.fn();

beforeEach(() => {
  current = snapshot();
  backend = fakeBackend(() => current);
  showView.mockReset();
});

function FocusOpener({ refs, subject }: { refs: string[]; subject: FocusSubject }) {
  const { focusOn } = useViews();
  return <button onClick={() => focusOn("requirements", refs, subject)}>open focus</button>;
}

async function show(extra?: React.ReactNode) {
  renderViews(
    <>
      {extra}
      <RequirementsView />
    </>,
    showView,
  );
  await screen.findByRole("heading", { name: "Orders" });
}

describe("the Requirements page", () => {
  it("shows Areas in folder order, fragments with their file name in muted text, and sections with General first", async () => {
    await show();
    expect(screen.getAllByRole("heading", { level: 2 }).map((h) => h.textContent)).toEqual(["Orders", "Interface"]);
    expect(screen.getByText("How orders work.")).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 3, name: "Checkout" })).toBeInTheDocument();
    expect(screen.getByText("checkout.md")).toHaveClass("text-muted-foreground");
    const orders = screen.getByRole("region", { name: "Orders, General" }).parentElement!;
    expect(within(orders).getAllByRole("heading", { level: 4 }).map((h) => h.textContent)).toEqual(["General", "order"]);
  });

  it("shows a section's title as plain text with `Open in editor` as its only control, and no reference chip", async () => {
    await show();
    const section = screen.getByRole("region", { name: "Orders, order" });
    expect(within(section).getByRole("heading", { level: 4, name: "order" })).toBeInTheDocument();
    expect(within(section).getAllByRole("button").map((b) => b.getAttribute("aria-label") ?? b.textContent)).toEqual(["Open in editor"]);
    expect(screen.queryByRole("button", { name: /View on diagram/ })).not.toBeInTheDocument();
  });

  it("has `Ask more` for the Requirements as the only control in its header, and none on any section, fragment or Area heading", async () => {
    await show();
    const buttons = screen.getAllByRole("button", { name: /Ask more/ });
    expect(buttons.map((b) => b.getAttribute("aria-label"))).toEqual(["Ask more about the Requirements"]);
    const button = buttons[0]!;
    expect(button).toHaveTextContent("Ask more");
    expect(button).toBeEnabled();
    const header = screen.getByRole("heading", { level: 1, name: "Requirements" }).closest("header")!;
    expect(within(header).getAllByRole("button")).toEqual([button]);
    expect(within(header).queryByRole("combobox")).toBeNull();
    expect(screen.queryByRole("combobox", { name: "Overlay" })).not.toBeInTheDocument();
    for (const heading of screen.getAllByRole("heading", { level: 2 })) {
      expect(within(heading.parentElement!).queryByRole("button", { name: /Ask more/ })).toBeNull();
    }
  });

  it("tells what it is about with its tooltip", async () => {
    const user = userEvent.setup();
    await show();
    await user.hover(screen.getByRole("button", { name: "Ask more about the Requirements" }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Ask an agent about the Requirements");
  });

  it("sends only a null overlay when none is drawn, and Ask more never changes the view", async () => {
    const user = userEvent.setup();
    const askMore = vi.fn().mockResolvedValue("aaaaaaaa");
    renderViews(<RequirementsView />, showView, { askMore });
    await screen.findByRole("heading", { name: "Orders" });
    await user.click(screen.getByRole("button", { name: "Ask more about the Requirements" }));
    expect(askMore).toHaveBeenCalledTimes(1);
    expect(askMore).toHaveBeenCalledWith({ kind: "requirements", overlay: null });
    expect(showView).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Ask more about the Requirements" })).toBeEnabled();
  });

  it("sends the Proposal drawn over the Requirements and nothing else", async () => {
    const user = userEvent.setup();
    current = snapshot({ overlays: [{ id: "pppppppp", shortNumber: "P3", title: "Pay" }], overlay: "pppppppp" });
    const askMore = vi.fn().mockResolvedValue("aaaaaaaa");
    renderViews(<RequirementsView />, showView, { askMore });
    await screen.findByRole("heading", { name: "Orders" });
    await user.click(screen.getByRole("button", { name: "Ask more about the Requirements" }));
    expect(askMore).toHaveBeenCalledWith({ kind: "requirements", overlay: "pppppppp" });
  });

  it("opens the file at the line of a section's heading", async () => {
    const user = userEvent.setup();
    await show();
    const section = screen.getByRole("region", { name: "Orders, order" });
    await user.click(within(section).getByRole("button", { name: "Open in editor" }));
    await waitFor(() => expect(backend.opened).toEqual([{ file: ".weaver/requirements/orders/checkout.md", line: 9 }]));
  });

  it("shows the errors of an invalid Requirement at the top", async () => {
    current = snapshot({}, { errors: [{ message: "Two sections are titled `order`.", file: ".weaver/requirements/orders/checkout.md", line: 20 }] });
    await show();
    expect(await screen.findByText("The Model has 1 error")).toBeInTheDocument();
  });
});

describe("the overlay labels of the Requirements", () => {
  function overlaid(): ViewSnapshot {
    const drawn = snapshot({ overlays: [{ id: "pppppppp", shortNumber: "P3", title: "Pay" }], overlay: "pppppppp" });
    const [orders, ui] = drawn.data.requirements;
    const checkout = orders!.fragments[0]!;
    checkout.sections = [
      { ...checkout.sections[0]!, change: "changed", before: "Old general rules.", body: "New general rules." },
      { ...checkout.sections[1]!, change: "removed" },
      { title: "payment", body: "Pays.", before: null, change: "added", movedFrom: null, file: null, line: null },
      { title: "cart", body: "Carts.", before: null, change: null, movedFrom: "legacy", file: null, line: null },
    ];
    orders!.fragments.push({ name: "refunds", title: "Refunds", description: "", change: "new", file: null, sections: [] });
    ui!.fragments[0]!.change = "removed";
    return drawn;
  }

  it("uses text for every change: added, removed, changed, moved from, new fragment, fragment removed", async () => {
    current = overlaid();
    await show();
    expect(screen.getByRole("region", { name: "Orders, General, changed" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Orders, order, removed" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Orders, payment, added" })).toBeInTheDocument();
    expect(screen.getByText("moved from legacy")).toBeInTheDocument();
    expect(screen.getByText("new fragment")).toBeInTheDocument();
    expect(screen.getByText("fragment removed")).toBeInTheDocument();
  });

  it("strikes a removed section through in muted text", async () => {
    current = overlaid();
    await show();
    const removed = within(screen.getByRole("region", { name: "Orders, order, removed" })).getByText("An order has a cart.");
    expect(removed.closest("div.line-through")).toHaveClass("text-muted-foreground");
  });

  it("shows a changed section with its new text and `Show before` toggles the old one", async () => {
    current = overlaid();
    const user = userEvent.setup();
    await show();
    const section = screen.getByRole("region", { name: "Orders, General, changed" });
    expect(within(section).getByText("New general rules.")).toBeInTheDocument();
    await user.click(within(section).getByRole("button", { name: "Show before" }));
    expect(within(section).getByText("Old general rules.")).toBeInTheDocument();
    expect(within(section).queryByText("New general rules.")).not.toBeInTheDocument();
    await user.click(within(section).getByRole("button", { name: "Show before" }));
    expect(within(section).getByText("New general rules.")).toBeInTheDocument();
  });
});

describe("the focus of the Requirements", () => {
  const subject: FocusSubject = { kind: "task", id: "tttttttt", number: "T4", title: "Fix" };
  const scrolled: HTMLElement[] = [];

  beforeEach(() => {
    scrolled.length = 0;
    Element.prototype.scrollIntoView = function (this: HTMLElement) {
      scrolled.push(this);
    };
  });

  async function focused(refs: string[]) {
    const user = userEvent.setup();
    await show(<FocusOpener refs={refs} subject={subject} />);
    await user.click(screen.getByRole("button", { name: "open focus" }));
    await screen.findByRole("region", { name: "Focus" });
    return user;
  }

  it("tints focused sections and Areas, scrolls to the step and counts the references not found", async () => {
    await focused(["requirements/ui", "order", "order/Paid"]);
    const bar = screen.getByRole("region", { name: "Focus" });
    expect(within(bar).getByText("Task T4 – Fix")).toBeInTheDocument();
    expect(within(bar).getByText("Orders › order")).toBeInTheDocument();
    expect(within(bar).getByText("1 of 2")).toBeInTheDocument();
    expect(within(bar).getByText("1 not found")).toBeInTheDocument();
    await waitFor(() => expect(new Set(scrolled.map((el) => el.dataset.stepKey))).toEqual(new Set(["orders#order"])));
    expect(screen.getByRole("region", { name: "Orders, order" })).toHaveClass("border-cyan-600");
    expect(screen.getByRole("region", { name: "Orders, General" })).not.toHaveClass("border-cyan-600");
  });

  it("goes to the next step and scrolls to it", async () => {
    const user = await focused(["requirements/ui", "order"]);
    await user.click(within(screen.getByRole("region", { name: "Focus" })).getByRole("button", { name: "Next" }));
    await waitFor(() => expect(scrolled[scrolled.length - 1]?.dataset.stepKey).toBe("ui"));
    expect(within(screen.getByRole("region", { name: "Focus" })).getByText("2 of 2")).toBeInTheDocument();
  });

  it("keeps `Ask more` in the header with a focus and with `Only these` on, and sends nothing of them", async () => {
    const askMore = vi.fn().mockResolvedValue("aaaaaaaa");
    const user = userEvent.setup();
    renderViews(
      <>
        <FocusOpener refs={["order"]} subject={subject} />
        <RequirementsView />
      </>,
      showView,
      { askMore },
    );
    await screen.findByRole("heading", { name: "Orders" });
    await user.click(screen.getByRole("button", { name: "open focus" }));
    await screen.findByRole("region", { name: "Focus" });
    await user.click(screen.getByRole("checkbox", { name: "Only these" }));
    expect(screen.queryByRole("region", { name: "Orders, General" })).not.toBeInTheDocument();
    const button = screen.getByRole("button", { name: "Ask more about the Requirements" });
    expect(button).toBeEnabled();
    await user.click(button);
    expect(askMore).toHaveBeenCalledWith({ kind: "requirements", overlay: null });
    // The focus and `Only these` stay as they were.
    expect(screen.getByRole("region", { name: "Focus" })).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "Only these" })).toBeChecked();
  });

  it("hides what is not focused with `Only these`, off at the start", async () => {
    const user = await focused(["order"]);
    const toggle = screen.getByRole("checkbox", { name: "Only these" });
    expect(toggle).not.toBeChecked();
    expect(screen.getByRole("heading", { name: "Interface" })).toBeInTheDocument();
    await user.click(toggle);
    expect(screen.queryByRole("heading", { name: "Interface" })).not.toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Orders, General" })).not.toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Orders, order" })).toBeInTheDocument();
    await user.click(toggle);
    expect(screen.getByRole("heading", { name: "Interface" })).toBeInTheDocument();
  });

  it("`Clear` and Esc remove the focus and keep the page", async () => {
    const user = await focused(["order"]);
    await user.click(screen.getByRole("button", { name: "Clear" }));
    expect(screen.queryByRole("region", { name: "Focus" })).not.toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Orders, order" })).not.toHaveClass("border-cyan-600");
    await user.click(screen.getByRole("button", { name: "open focus" }));
    await screen.findByRole("region", { name: "Focus" });
    screen.getByRole("checkbox", { name: "Only these" }).focus();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("region", { name: "Focus" })).not.toBeInTheDocument();
  });

  it("uses the change colour of a focused section under an overlay", async () => {
    current = snapshot({ overlay: "pppppppp", overlays: [{ id: "pppppppp", shortNumber: "P3", title: "Pay" }] });
    current.data.requirements[0]!.fragments[0]!.sections[1]!.change = "added";
    await focused(["order"]);
    expect(screen.getByRole("region", { name: "Orders, order, added" })).toHaveClass("border-green-600");
  });

  it("stays on the same step when the files change", async () => {
    const user = await focused(["requirements/ui", "order"]);
    await user.click(within(screen.getByRole("region", { name: "Focus" })).getByRole("button", { name: "Next" }));
    current = snapshot({ revision: 2 });
    await act(async () => backend.emit(["requirements"], 2));
    await waitFor(() => expect(backend.asked).toHaveLength(2));
    expect(within(screen.getByRole("region", { name: "Focus" })).getByText("2 of 2")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "Focus" })).getByText("Interface")).toBeInTheDocument();
  });
});
