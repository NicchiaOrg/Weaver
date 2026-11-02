import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ViewSnapshot } from "@/bindings/ViewSnapshot";
import type { FocusSubject } from "@/components/views/focus-model";
import { DiagramView } from "@/components/views/diagram-view";
import { useViews } from "@/components/views/views-state";
import { detail, levels, snapshot } from "@/components/views/view-fixtures";
import { fakeBackend, renderViews, type FakeBackend } from "@/components/views/view-test-utils";

vi.mock("@/lib/backend", () => ({
  call: vi.fn(),
  on: vi.fn(),
  isAppError: (value: unknown) => typeof value === "object" && value !== null && "code" in value && "message" in value,
}));

// The canvas draws with cytoscape, which needs a real canvas: a stand-in tells what it was given.
vi.mock("@/components/views/diagram-canvas", () => ({
  DiagramCanvas: (props: { level: { key: string }; selected: string | null; focus: unknown; expanded: ReadonlySet<string> }) => (
    <div
      data-testid="canvas"
      data-level={props.level.key}
      data-selected={props.selected ?? ""}
      data-focus={JSON.stringify(props.focus)}
      data-expanded={[...props.expanded].join(",")}
    />
  ),
}));

let current: ViewSnapshot;
let backend: FakeBackend;

beforeEach(() => {
  current = snapshot();
  backend = fakeBackend(() => current);
});

/** Opens the window state a card would: a focus on `refs`. */
function FocusOpener({ refs, subject }: { refs: string[]; subject: FocusSubject }) {
  const { focusOn } = useViews();
  return <button onClick={() => focusOn("diagram", refs, subject)}>open focus</button>;
}

const canvas = () => screen.getByTestId("canvas");
const listbox = () => screen.getByRole("listbox", { name: /Elements of/ });

async function show(extra?: React.ReactNode) {
  const result = renderViews(
    <>
      {extra}
      <DiagramView />
    </>,
  );
  await screen.findByTestId("canvas");
  return result;
}

describe("the levels of the Diagram", () => {
  it("starts at the root, `System`, with the nodes of the root in an accessible list", async () => {
    await show();
    expect(canvas()).toHaveAttribute("data-level", "");
    expect(within(screen.getByRole("navigation", { name: "Breadcrumb" })).getByRole("button", { name: "System" })).toHaveAttribute("aria-current", "page");
    expect(within(listbox()).getAllByRole("option").map((o) => o.getAttribute("aria-label"))).toEqual([
      "Whole System, Whole System",
      "user, Actor, 1 feature",
    ]);
  });

  it("moves the selection with the arrow keys, opens the level with Enter and goes up with Backspace", async () => {
    const user = userEvent.setup();
    await show();
    listbox().focus();
    await user.keyboard("{ArrowRight}");
    expect(canvas()).toHaveAttribute("data-selected", "whole");
    await user.keyboard("{ArrowRight}{ArrowLeft}{Enter}");
    expect(canvas()).toHaveAttribute("data-level", "system");
    const crumbs = screen.getByRole("navigation", { name: "Breadcrumb" });
    expect(within(crumbs).getAllByRole("button").map((b) => b.textContent)).toEqual(["System", "Whole System"]);

    await user.keyboard("{ArrowRight}{ArrowRight}{Enter}");
    expect(canvas()).toHaveAttribute("data-level", "feature:order");
    expect(within(crumbs).getAllByRole("button").map((b) => b.textContent)).toEqual(["System", "Whole System", "order"]);

    await user.keyboard("{Backspace}");
    expect(canvas()).toHaveAttribute("data-level", "system");
  });

  it("does not open a level where a node has none", async () => {
    const user = userEvent.setup();
    await show();
    listbox().focus();
    await user.keyboard("{ArrowRight}{Enter}{Enter}");
    expect(canvas()).toHaveAttribute("data-level", "system");
    await user.keyboard("{ArrowRight}{ArrowRight}{ArrowRight}{Enter}");
    expect(canvas()).toHaveAttribute("data-level", "system");
  });

  it("goes to any level by its breadcrumb segment and selects the node that opened the next one", async () => {
    const user = userEvent.setup();
    await show();
    listbox().focus();
    await user.keyboard("{ArrowRight}{Enter}{ArrowRight}{ArrowRight}{Enter}");
    expect(canvas()).toHaveAttribute("data-level", "feature:order");
    await user.click(screen.getByRole("button", { name: "Whole System" }));
    expect(canvas()).toHaveAttribute("data-level", "system");
    expect(canvas()).toHaveAttribute("data-selected", "feature:order");
    await user.click(screen.getByRole("button", { name: "System" }));
    expect(canvas()).toHaveAttribute("data-level", "");
  });

  it("expands a Machine in place with Space and collapses it again", async () => {
    const user = userEvent.setup();
    await show();
    listbox().focus();
    await user.keyboard("{ArrowRight}{Enter}{ArrowRight}{ArrowRight}{Enter}");
    expect(canvas()).toHaveAttribute("data-level", "feature:order");
    await user.keyboard("{ArrowRight}");
    expect(canvas()).toHaveAttribute("data-selected", "machine:cart");
    await user.keyboard(" ");
    expect(canvas()).toHaveAttribute("data-expanded", "machine:cart");
    expect(within(listbox()).getByRole("option", { name: /^Empty, State/ })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Collapse States" }));
    expect(canvas()).toHaveAttribute("data-expanded", "");
    expect(within(listbox()).queryByRole("option", { name: /^Empty, State/ })).not.toBeInTheDocument();
  });
});

describe("the details panel", () => {
  async function select(path: string) {
    const user = userEvent.setup();
    await show();
    listbox().focus();
    await user.keyboard(path);
    return user;
  }

  it("shows the title, the rendered description, the open Drifts and the Tasks of the selected element", async () => {
    await select("{ArrowRight}{Enter}{ArrowRight}{ArrowRight}");
    const panel = await screen.findByRole("complementary", { name: "Details" });
    expect(within(panel).getByRole("heading", { name: "order" })).toBeInTheDocument();
    expect(within(panel).getByText("Everything about one order. It starts in a cart.")).toBeInTheDocument();
    expect(within(panel).getByText("Cart total is wrong")).toBeInTheDocument();
    expect(within(panel).getByText("Fix the total")).toBeInTheDocument();
    expect(within(panel).getByRole("img", { name: "Short number (temporary) D1" })).toBeInTheDocument();
  });

  it("has no panel while nothing is selected, and Esc clears the selection", async () => {
    const user = await select("{ArrowRight}{Enter}{ArrowRight}{ArrowRight}");
    expect(screen.getByRole("complementary", { name: "Details" })).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("complementary", { name: "Details" })).not.toBeInTheDocument();
    expect(canvas()).toHaveAttribute("data-selected", "");
  });

  it("lists the Transitions of a Machine", async () => {
    await select("{ArrowRight}{Enter}{ArrowRight}{ArrowRight}{Enter}{ArrowRight}");
    const panel = screen.getByRole("complementary", { name: "Details" });
    expect(within(panel).getByRole("heading", { name: "cart" })).toBeInTheDocument();
    expect(within(panel).getByText("user.PAY")).toBeInTheDocument();
    expect(within(panel).getByText("Empty → Paid")).toBeInTheDocument();
    expect(within(panel).getByText("Takes the payment.")).toBeInTheDocument();
  });

  it("opens the file of a Transition at its line", async () => {
    const user = await select("{ArrowRight}{Enter}{ArrowRight}{ArrowRight}{Enter}{ArrowRight}");
    const panel = screen.getByRole("complementary", { name: "Details" });
    const lines = within(panel).getAllByRole("button", { name: "Open in editor" });
    await user.click(lines[lines.length - 1]!);
    await waitFor(() => expect(backend.opened).toEqual([{ file: ".trama/model/order/cart.md", line: 12 }]));
  });

  it("opens the file of the element at its line in the editor", async () => {
    const user = await select("{ArrowRight}{Enter}{ArrowRight}{ArrowRight}");
    const panel = await screen.findByRole("complementary", { name: "Details" });
    await user.click(within(panel).getByRole("button", { name: "Open in editor" }));
    await waitFor(() => expect(backend.opened).toEqual([{ file: ".trama/model/order/feature.md", line: 3 }]));
  });

  it("names the overlay change of an element in words", async () => {
    current = snapshot({ overlay: null }, {});
    const changed = current.data.model.details["feature:order"]!;
    current.data.model.details["feature:order"] = { ...changed, change: "changed" };
    await select("{ArrowRight}{Enter}{ArrowRight}{ArrowRight}");
    const panel = await screen.findByRole("complementary", { name: "Details" });
    expect(within(panel).getByText("changed")).toBeInTheDocument();
  });
});

describe("the focus of the Diagram", () => {
  const taskSubject = { kind: "task", id: "tttttttt", number: "T4", title: "Fix the total" } as const;

  async function focused(refs: string[], subject = taskSubject) {
    const user = userEvent.setup();
    await show(<FocusOpener refs={refs} subject={subject} />);
    await user.click(screen.getByRole("button", { name: "open focus" }));
    return user;
  }

  it("shows what it is about, the step, its position, and goes to the first step's level", async () => {
    await focused(["cart", "order"]);
    const bar = await screen.findByRole("region", { name: "Focus" });
    expect(within(bar).getByText("Task T4 – Fix the total")).toBeInTheDocument();
    expect(within(bar).getByText("1 of 3")).toBeInTheDocument();
    expect(within(bar).getByText("System › Whole System")).toBeInTheDocument();
    await waitFor(() => expect(canvas()).toHaveAttribute("data-level", "system"));
    expect(JSON.parse(canvas().getAttribute("data-focus")!)).toEqual({ nodes: ["feature:order"], edges: [] });
  });

  it("moves between steps with Previous and Next, disabled at the ends, announcing each step", async () => {
    const user = await focused(["cart", "order"]);
    const bar = await screen.findByRole("region", { name: "Focus" });
    expect(within(bar).getByRole("button", { name: "Previous" })).toHaveAttribute("aria-disabled", "true");
    await user.click(within(bar).getByRole("button", { name: "Next" }));
    expect(within(bar).getByText("2 of 3")).toBeInTheDocument();
    expect(canvas()).toHaveAttribute("data-level", "actor:user");
    await user.click(within(bar).getByRole("button", { name: "Next" }));
    expect(within(bar).getByText("3 of 3")).toBeInTheDocument();
    expect(canvas()).toHaveAttribute("data-level", "feature:order");
    expect(within(bar).getByRole("button", { name: "Next" })).toHaveAttribute("aria-disabled", "true");
    expect(within(bar).getByRole("status")).toHaveTextContent("System › order, 3 of 3");
    await user.click(within(bar).getByRole("button", { name: "Previous" }));
    expect(within(bar).getByText("2 of 3")).toBeInTheDocument();
  });

  it("has neither counter nor arrows with one step", async () => {
    await focused(["cart/Paid"], { kind: "ref", ref: "cart/Paid" } as never);
    const bar = await screen.findByRole("region", { name: "Focus" });
    expect(within(bar).getByText("cart/Paid")).toBeInTheDocument();
    expect(within(bar).queryByRole("button", { name: "Next" })).not.toBeInTheDocument();
    expect(within(bar).queryByText(/ of /)).not.toBeInTheDocument();
    await waitFor(() => expect(canvas()).toHaveAttribute("data-level", "machine:cart"));
  });

  it("counts the references it cannot find, and lists them in a tooltip", async () => {
    const user = await focused(["cart", "order/Paid", "gone"]);
    const bar = await screen.findByRole("region", { name: "Focus" });
    const missing = within(bar).getByText("2 not found");
    await user.hover(missing);
    expect(await screen.findByRole("tooltip")).toHaveTextContent("order/Paid gone");
  });

  it("is cleared by Esc after the selection, and `Clear` keeps the level", async () => {
    const user = await focused(["cart"]);
    await screen.findByRole("region", { name: "Focus" });
    await waitFor(() => expect(canvas()).toHaveAttribute("data-level", "feature:order"));
    listbox().focus();
    await user.keyboard("{ArrowRight}");
    expect(canvas()).toHaveAttribute("data-selected", "machine:cart");
    await user.keyboard("{Escape}");
    expect(canvas()).toHaveAttribute("data-selected", "");
    expect(screen.getByRole("region", { name: "Focus" })).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("region", { name: "Focus" })).not.toBeInTheDocument();
    expect(canvas()).toHaveAttribute("data-level", "feature:order");
    expect(canvas()).toHaveAttribute("data-focus", "null");
  });

  it("stays on the same step, and goes to the nearest one, when the files change", async () => {
    const user = await focused(["cart", "order"]);
    const bar = await screen.findByRole("region", { name: "Focus" });
    await user.click(within(bar).getByRole("button", { name: "Next" }));
    expect(within(bar).getByText("2 of 3")).toBeInTheDocument();

    // The Actor level goes: the step of `actor:user` is gone, so the nearest index is kept.
    const smaller = snapshot({ revision: 2 });
    smaller.data.model.levels = smaller.data.model.levels.filter((l) => l.key !== "actor:user");
    smaller.data.targets.order = { ...smaller.data.targets.order!, diagram: smaller.data.targets.order!.diagram.filter((a) => a.level !== "actor:user") };
    current = smaller;
    await act(async () => backend.emit(["model"], 2));
    await waitFor(() => expect(within(screen.getByRole("region", { name: "Focus" })).getByText("2 of 2")).toBeInTheDocument());

    // A change that leaves the step alone keeps it.
    const same = snapshot({ revision: 3 });
    same.data.model.levels = smaller.data.model.levels;
    same.data.targets = smaller.data.targets;
    current = same;
    await act(async () => backend.emit(["model"], 3));
    await waitFor(() => expect(backend.asked.length).toBeGreaterThan(2));
    expect(within(screen.getByRole("region", { name: "Focus" })).getByText("2 of 2")).toBeInTheDocument();
  });
});

describe("the level and the selection when the Model changes", () => {
  async function selectMachine() {
    const user = userEvent.setup();
    await show();
    listbox().focus();
    await user.keyboard("{ArrowRight}{Enter}{ArrowRight}{ArrowRight}{Enter}");
    await user.keyboard("{ArrowRight}{Enter}{ArrowRight}");
    return user;
  }

  it("keeps the level and the selection when they still exist", async () => {
    await selectMachine();
    expect(canvas()).toHaveAttribute("data-level", "machine:cart");
    expect(canvas()).toHaveAttribute("data-selected", "state:cart/Empty");
    current = snapshot({ revision: 2 });
    await act(async () => backend.emit(["model"], 2));
    await waitFor(() => expect(backend.asked).toHaveLength(2));
    expect(canvas()).toHaveAttribute("data-level", "machine:cart");
    expect(canvas()).toHaveAttribute("data-selected", "state:cart/Empty");
  });

  it("falls back to the nearest container that exists, selecting the node that held the missing level", async () => {
    await selectMachine();
    const smaller = snapshot({ revision: 2 });
    smaller.data.model.levels = smaller.data.model.levels.filter((l) => !l.key.startsWith("machine:") && !l.key.startsWith("state:"));
    current = smaller;
    await act(async () => backend.emit(["model"], 2));
    await waitFor(() => expect(canvas()).toHaveAttribute("data-level", "feature:order"));
    expect(canvas()).toHaveAttribute("data-selected", "machine:cart");
  });

  it("falls back up to the root when everything below it is gone", async () => {
    await selectMachine();
    const empty = snapshot({ revision: 2 });
    empty.data.model.levels = levels().slice(0, 1);
    current = empty;
    await act(async () => backend.emit(["model"], 2));
    await waitFor(() => expect(canvas()).toHaveAttribute("data-level", ""));
  });

  it("drops a selection that no longer exists on a level that does", async () => {
    await selectMachine();
    const changed = snapshot({ revision: 2 });
    const machine = changed.data.model.levels.find((l) => l.key === "machine:cart")!;
    machine.nodes = machine.nodes.filter((n) => n.id !== "state:cart/Empty");
    current = changed;
    await act(async () => backend.emit(["model"], 2));
    await waitFor(() => expect(canvas()).toHaveAttribute("data-selected", ""));
    expect(canvas()).toHaveAttribute("data-level", "machine:cart");
  });

  it("asks again only for slices the views draw", async () => {
    await show();
    await act(async () => backend.emit(["history"], 2));
    expect(backend.asked).toHaveLength(1);
    await act(async () => backend.emit(["board"], 3));
    await waitFor(() => expect(backend.asked).toHaveLength(2));
  });
});

describe("the overlay on the Diagram", () => {
  const withOverlay = (): ViewSnapshot => {
    const drawn = snapshot({ overlays: [{ id: "pppppppp", shortNumber: "P3", title: "Add payments" }], overlay: "pppppppp", scope: ["order"] });
    drawn.data.model.details["feature:order"] = detail("order", { change: "added", description: "New." });
    drawn.data.model.levels[1]!.nodes[1]!.change = "added";
    return drawn;
  };

  it("offers `Current Model` and the open Proposals, and asks the backend for the one chosen", async () => {
    current = snapshot({ overlays: [{ id: "pppppppp", shortNumber: "P3", title: "Add payments" }] });
    backend = fakeBackend((overlay) => (overlay === "pppppppp" ? withOverlay() : current));
    const user = userEvent.setup();
    await show();
    await user.click(screen.getByRole("combobox", { name: "Overlay" }));
    const options = within(await screen.findByRole("listbox")).getAllByRole("option");
    expect(options.map((o) => o.textContent)).toEqual(["Current Model", "P3 – Add payments"]);
    await user.click(options[1]!);
    await waitFor(() => expect(backend.asked).toEqual([null, "pppppppp"]));
    expect(screen.getByRole("combobox", { name: "Overlay" })).toHaveTextContent("P3 – Add payments");
  });

  it("names a change in words in the accessible name of the element and in the details", async () => {
    current = withOverlay();
    backend = fakeBackend(() => current);
    const user = userEvent.setup();
    await show();
    listbox().focus();
    await user.keyboard("{ArrowRight}{Enter}");
    expect(within(listbox()).getByRole("option", { name: "order, Feature, 2 machines, added, 2 open Drifts" })).toBeInTheDocument();
    await user.keyboard("{ArrowRight}{ArrowRight}");
    expect(within(await screen.findByRole("complementary", { name: "Details" })).getByText("added")).toBeInTheDocument();
  });

  it("goes back to `Current Model` with a notice when the Proposal stops being open", async () => {
    let open = true;
    backend = fakeBackend((overlay) => {
      if (!open) return snapshot({ revision: 2 });
      return overlay === "pppppppp" ? withOverlay() : snapshot({ overlays: [{ id: "pppppppp", shortNumber: "P3", title: "Add payments" }] });
    });
    const user = userEvent.setup();
    await show();
    await user.click(screen.getByRole("combobox", { name: "Overlay" }));
    await user.click(within(await screen.findByRole("listbox")).getByRole("option", { name: "P3 – Add payments" }));
    await waitFor(() => expect(screen.getByRole("combobox", { name: "Overlay" })).toHaveTextContent("P3 – Add payments"));

    // It was approved: the backend no longer lists it, and answers with the current Model.
    open = false;
    await act(async () => backend.emit(["board"], 2));
    await waitFor(() => expect(screen.getByRole("combobox", { name: "Overlay" })).toHaveTextContent("Current Model"));
    expect(screen.getByText("Proposal P3 is no longer open, so the views show the Current Model.")).toBeInTheDocument();
  });
});

describe("the errors of an invalid Model", () => {
  it("show `The Model has <n> errors` at the top, with an expandable list that opens each file at its line", async () => {
    current = snapshot(
      {},
      {
        errors: [
          { message: "Transition target `Gone` is not a State of cart.", file: ".trama/model/order/cart.md", line: 14 },
          { message: "The file could not be read: permission denied", file: ".trama/model/order/payment.md", line: null },
        ],
      },
    );
    const user = userEvent.setup();
    await show();
    const alert = await screen.findByRole("alert");
    expect(within(alert).getByText("The Model has 2 errors")).toBeInTheDocument();
    expect(within(alert).queryByText(/Transition target/)).not.toBeInTheDocument();
    await user.click(within(alert).getByRole("button", { name: "Show errors" }));
    expect(within(alert).getByText("Transition target `Gone` is not a State of cart.")).toBeInTheDocument();
    expect(within(alert).getByText(".trama/model/order/cart.md, line 14")).toBeInTheDocument();
    expect(within(alert).getByText(".trama/model/order/payment.md")).toBeInTheDocument();
    await user.click(within(alert).getAllByRole("button", { name: "Open in editor" })[0]!);
    await waitFor(() => expect(backend.opened).toEqual([{ file: ".trama/model/order/cart.md", line: 14 }]));
  });

  it("say `1 error` in the singular and show nothing for a valid Model", async () => {
    current = snapshot({}, { errors: [{ message: "Bad.", file: ".trama/model/system.md", line: 1 }] });
    await show();
    expect(await screen.findByText("The Model has 1 error")).toBeInTheDocument();
  });

  it("are not shown for a valid Model", async () => {
    await show();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
