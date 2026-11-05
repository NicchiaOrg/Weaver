import { describe, expect, it } from "vitest";

import { planFocus, settleStep, stepCount, subjectText } from "@/components/views/focus-model";
import { viewData } from "@/components/views/view-fixtures";

describe("the name of a focus subject", () => {
  it("is `Proposal <number> – <title>`, `Task <number> – <title>` or the reference", () => {
    expect(subjectText({ kind: "proposal", id: "pppppppp", number: "P3", title: "Add a cart" })).toBe("Proposal P3 – Add a cart");
    expect(subjectText({ kind: "task", id: "tttttttt", number: "T4", title: "Fix" })).toBe("Task T4 – Fix");
    expect(subjectText({ kind: "ref", ref: "order/Paid" })).toBe("order/Paid");
  });

  it("falls back to the permanent id when the card has no number", () => {
    expect(subjectText({ kind: "task", id: "tttttttt", number: null, title: "Fix" })).toBe("Task tttttttt – Fix");
  });
});

describe("the steps of a Requirements focus", () => {
  it("are Areas and sections in page order, whatever the order of the references", () => {
    const plan = planFocus("requirements", ["requirements/ui", "requirements/orders#order"], viewData());
    expect(plan.steps.map((s) => s.key)).toEqual(["orders#order", "ui"]);
    expect(plan.steps.map((s) => s.name)).toEqual(["Orders › order", "Interface"]);
  });

  it("reports a section the page lacks as not found", () => {
    expect(planFocus("requirements", ["order/Paid"], viewData()).notFound).toEqual(["order/Paid"]);
  });
});

describe("the steps of a Specs focus", () => {
  it("are the fields in page order; a field the Specs lack is not found", () => {
    const plan = planFocus("specs", ["specs#notes", "specs#languages", "specs#cloud"], viewData());
    expect(plan.steps.map((s) => s.key)).toEqual(["languages", "notes"]);
    expect(plan.notFound).toEqual(["specs#cloud"]);
  });
});

describe("how many steps a card's focus has", () => {
  it("counts the steps of the references the snapshot knows", () => {
    expect(stepCount("requirements", ["cart", "order"], viewData(), false)).toBe(1);
    expect(stepCount("requirements", ["cart"], viewData(), false)).toBe(0);
    expect(stepCount("requirements", [], viewData(), false)).toBe(0);
    expect(stepCount("requirements", ["order"], null, false)).toBe(0);
  });

  it("counts a reference only a Proposal adds as one step when the text can name something of the view", () => {
    expect(stepCount("requirements", ["fresh"], viewData(), true)).toBe(1);
    expect(stepCount("requirements", ["fresh"], viewData(), false)).toBe(0);
    expect(stepCount("specs", ["fresh"], viewData(), true)).toBe(0);
    expect(stepCount("specs", ["specs#frameworks"], viewData(), true)).toBe(1);
  });
});

describe("staying on a step when the files change", () => {
  const steps = planFocus("requirements", ["requirements/ui", "requirements/orders#order"], viewData()).steps;

  it("keeps the same step when it still exists", () => {
    expect(settleStep(steps, "ui", 0)).toBe(1);
  });

  it("falls back to the nearest index when it does not", () => {
    expect(settleStep(steps, "gone", 7)).toBe(1);
    expect(settleStep([], "gone", 1)).toBe(0);
  });
});
