import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ViewSnapshot } from "@/bindings/ViewSnapshot";
import { FocusButton } from "@/components/board/task-card";
import { useBoardActions } from "@/components/board/board-ui";
import { RequirementsView } from "@/components/views/requirements-view";
import { settleNav, useViews } from "@/components/views/views-state";
import { snapshot } from "@/components/views/view-fixtures";
import { call } from "@/lib/backend";
import { fakeBackend, renderViews, type FakeBackend } from "@/components/views/view-test-utils";

vi.mock("@/lib/backend", () => ({
  call: vi.fn(),
  on: vi.fn(),
  isAppError: (value: unknown) => typeof value === "object" && value !== null && "code" in value && "message" in value,
}));

let current: ViewSnapshot;
let backend: FakeBackend;

beforeEach(() => {
  current = snapshot();
  backend = fakeBackend((overlay) => (overlay ? { ...current, overlay } : current));
});

/** What the views of a window hold, written out. */
function Probe() {
  const { snapshot: shown, focus, overlay } = useViews();
  const steps = focus.diagram?.plan.steps.length ?? 0;
  return (
    <p data-testid="probe">
      {shown ? `revision ${shown.revision}` : "loading"}; overlay {overlay ?? "none"}; diagram steps {steps}
    </p>
  );
}

function Steps({ view, refs }: { view: "diagram" | "requirements" | "specs"; refs: string[] }) {
  const { focusSteps } = useBoardActions();
  return <p data-testid={`steps-${view}`}>{focusSteps?.(view, refs)}</p>;
}

describe("the Board's `View on diagram` and `View requirements`", () => {
  it("exist for a Task only when the project has a step for its references, and open the view with the Task as subject", async () => {
    const showView = vi.fn();
    const user = userEvent.setup();
    renderViews(
      <>
        <Probe />
        <FocusButton view="diagram" refs={["cart", "order"]} subject={{ kind: "task", id: "tttttttt", number: "T4", title: "Fix" }} />
        <FocusButton view="requirements" refs={["cart"]} subject={{ kind: "task", id: "tttttttt", number: "T4", title: "Fix" }} />
      </>,
      showView,
    );
    await screen.findByText(/revision 1/);
    expect(screen.queryByRole("button", { name: "View requirements" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "View on diagram" }));
    expect(showView).toHaveBeenCalledWith("diagram");
    await waitFor(() => expect(screen.getByTestId("probe")).toHaveTextContent("diagram steps 3"));
  });

  it("answers the number of steps in each view for the references it is asked about", async () => {
    renderViews(
      <>
        <Steps view="diagram" refs={["cart", "order"]} />
        <Steps view="requirements" refs={["order", "requirements/ui"]} />
        <Steps view="specs" refs={["specs#languages"]} />
      </>,
    );
    await waitFor(() => expect(screen.getByTestId("steps-diagram")).toHaveTextContent("3"));
    expect(screen.getByTestId("steps-requirements")).toHaveTextContent("2");
    expect(screen.getByTestId("steps-specs")).toHaveTextContent("1");
  });

  it("draws a Proposal over the Model when the focus is about it, and counts its new elements as steps", async () => {
    current = snapshot({ overlays: [{ id: "pppppppp", shortNumber: "P3", title: "Pay" }] });
    const user = userEvent.setup();
    renderViews(
      <>
        <Probe />
        <FocusButton view="diagram" refs={["fresh"]} subject={{ kind: "proposal", id: "pppppppp", number: "P3", title: "Pay" }} />
      </>,
    );
    await screen.findByRole("button", { name: "View on diagram" });
    await user.click(screen.getByRole("button", { name: "View on diagram" }));
    await waitFor(() => expect(backend.asked).toEqual([null, "pppppppp"]));
    await waitFor(() => expect(screen.getByTestId("probe")).toHaveTextContent("overlay pppppppp"));
  });

  it("is not offered for a Proposal's reference that only a Proposal adds when the card has no subject", async () => {
    renderViews(<Steps view="diagram" refs={["fresh"]} />);
    await waitFor(() => expect(screen.getByTestId("steps-diagram")).toHaveTextContent("0"));
  });
});

describe("what the views ask the backend", () => {
  it("drops an answer that is no longer the latest asked for", async () => {
    const resolvers: ((value: ViewSnapshot) => void)[] = [];
    vi.mocked(call).mockImplementation((() => new Promise<ViewSnapshot>((resolve) => resolvers.push(resolve))) as unknown as typeof call);
    renderViews(<Probe />);
    await waitFor(() => expect(resolvers).toHaveLength(1));
    await act(async () => backend.emit(["model"], 2));
    await waitFor(() => expect(resolvers).toHaveLength(2));
    await act(async () => {
      resolvers[1]!(snapshot({ revision: 2 }));
      resolvers[0]!(snapshot({ revision: 1 }));
    });
    expect(screen.getByTestId("probe")).toHaveTextContent("revision 2");
  });

  it("shows why the views could not be read, with a way to try again", async () => {
    vi.mocked(call).mockRejectedValueOnce({ code: "not-found", message: "The project is not open." });
    vi.mocked(call).mockResolvedValue(snapshot());
    const user = userEvent.setup();
    renderViews(<RequirementsView />);
    expect(await screen.findByText("The project is not open.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByRole("heading", { name: "Orders" })).toBeInTheDocument();
  });
});

describe("the navigation after a snapshot", () => {
  it("keeps the navigation object when nothing changed, so nothing is drawn again", () => {
    const nav = { path: ["", "system"], selected: "feature:order", expanded: new Set<string>() };
    expect(settleNav(nav, snapshot())).toBe(nav);
  });

  it("drops an expanded Machine that no longer exists", () => {
    const nav = { path: ["", "feature:order"], selected: null, expanded: new Set(["machine:cart", "machine:gone"]) };
    expect([...settleNav(nav, snapshot()).expanded]).toEqual(["machine:cart"]);
  });
});
