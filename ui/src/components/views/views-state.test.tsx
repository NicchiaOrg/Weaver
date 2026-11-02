import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ViewSnapshot } from "@/bindings/ViewSnapshot";
import { FocusButton } from "@/components/board/task-card";
import { useBoardActions } from "@/components/board/board-ui";
import { RequirementsView } from "@/components/views/requirements-view";
import { useViews } from "@/components/views/views-state";
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
  const steps = focus.requirements?.plan.steps.length ?? 0;
  return (
    <p data-testid="probe">
      {shown ? `revision ${shown.revision}` : "loading"}; overlay {overlay ?? "none"}; requirements steps {steps}
    </p>
  );
}

function Steps({ view, refs }: { view: "requirements" | "specs"; refs: string[] }) {
  const { focusSteps } = useBoardActions();
  return <p data-testid={`steps-${view}`}>{focusSteps?.(view, refs)}</p>;
}

describe("the Board's `View requirements`", () => {
  it("exists for a Task only when the project has a step for its references, and opens the view with the Task as subject", async () => {
    const showView = vi.fn();
    renderViews(
      <>
        <Probe />
        <FocusButton view="requirements" refs={["cart"]} subject={{ kind: "task", id: "tttttttt", number: "T4", title: "Fix" }} />
      </>,
      showView,
    );
    await screen.findByText(/revision 1/);
    expect(screen.queryByRole("button", { name: "View requirements" })).not.toBeInTheDocument();
  });

  it("opens Requirements on the steps of a Task's references", async () => {
    const showView = vi.fn();
    const user = userEvent.setup();
    renderViews(
      <>
        <Probe />
        <FocusButton view="requirements" refs={["cart", "order"]} subject={{ kind: "task", id: "tttttttt", number: "T4", title: "Fix" }} />
      </>,
      showView,
    );
    await screen.findByRole("button", { name: "View requirements" });
    await user.click(screen.getByRole("button", { name: "View requirements" }));
    expect(showView).toHaveBeenCalledWith("requirements");
    await waitFor(() => expect(screen.getByTestId("probe")).toHaveTextContent("requirements steps 1"));
  });

  it("answers the number of steps in each view for the references it is asked about", async () => {
    renderViews(
      <>
        <Steps view="requirements" refs={["order", "requirements/ui"]} />
        <Steps view="specs" refs={["specs#languages"]} />
      </>,
    );
    await waitFor(() => expect(screen.getByTestId("steps-requirements")).toHaveTextContent("2"));
    expect(screen.getByTestId("steps-specs")).toHaveTextContent("1");
  });

  it("draws a Proposal over the Model when the focus is about it, and counts its new elements as steps", async () => {
    current = snapshot({ overlays: [{ id: "pppppppp", shortNumber: "P3", title: "Pay" }] });
    const user = userEvent.setup();
    renderViews(
      <>
        <Probe />
        <FocusButton view="requirements" refs={["fresh"]} subject={{ kind: "proposal", id: "pppppppp", number: "P3", title: "Pay" }} />
      </>,
    );
    await screen.findByRole("button", { name: "View requirements" });
    await user.click(screen.getByRole("button", { name: "View requirements" }));
    await waitFor(() => expect(backend.asked).toEqual([null, "pppppppp"]));
    await waitFor(() => expect(screen.getByTestId("probe")).toHaveTextContent("overlay pppppppp"));
  });

  it("is not offered for a Proposal's reference that only a Proposal adds when the card has no subject", async () => {
    renderViews(<Steps view="requirements" refs={["fresh"]} />);
    await waitFor(() => expect(screen.getByTestId("steps-requirements")).toHaveTextContent("0"));
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
