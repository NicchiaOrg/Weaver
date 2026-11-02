import type { RefTargets } from "@/bindings/RefTargets";
import type { ViewData } from "@/bindings/ViewData";
import type { ViewSnapshot } from "@/bindings/ViewSnapshot";

// A small project for the tests of the views: two Requirement Areas and a Specs field list.

export const PROJECT = "0123456789abcdef";

const target = (patch: Partial<RefTargets>): RefTargets => ({ relevant: ["requirements"], requirements: [], specs: [], ...patch });

export function viewData(patch: Partial<ViewData> = {}): ViewData {
  return {
    errors: [],
    requirements: [
      {
        id: "orders",
        title: "Orders",
        description: "",
        intro: "How orders work.",
        file: ".weaver/requirements/orders/area.md",
        fragments: [
          {
            name: "checkout",
            title: "Checkout",
            description: "",
            change: null,
            file: ".weaver/requirements/orders/checkout.md",
            sections: [
              { title: "General", body: "General rules.", before: null, change: null, movedFrom: null, file: ".weaver/requirements/orders/checkout.md", line: 3 },
              { title: "order", body: "An order has a cart.", before: null, change: null, movedFrom: null, file: ".weaver/requirements/orders/checkout.md", line: 9 },
            ],
          },
        ],
      },
      {
        id: "ui",
        title: "Interface",
        description: "",
        intro: "",
        file: ".weaver/requirements/ui/area.md",
        fragments: [
          {
            name: "app",
            title: "App",
            description: "",
            change: null,
            file: null,
            sections: [{ title: "General", body: "Looks.", before: null, change: null, movedFrom: null, file: null, line: null }],
          },
        ],
      },
    ],
    specs: {
      fields: [
        {
          name: "languages",
          change: null,
          entries: [
            { name: "TypeScript", version: "5.9", note: "Strict", chosenReason: null, change: null, before: null },
            { name: "Rust", version: null, note: null, chosenReason: "Fast and safe", change: null, before: null },
          ],
          notes: null,
          beforeNotes: null,
          file: ".weaver/specs.yaml",
          line: 2,
        },
        { name: "notes", change: null, entries: [], notes: "Plain *notes*.", beforeNotes: null, file: ".weaver/specs.yaml", line: 9 },
      ],
    },
    targets: {
      "order/Paid": target({}),
      "cart/Paid": target({}),
      order: target({ requirements: [{ area: "orders", section: "order" }] }),
      cart: target({}),
      "requirements/ui": target({ requirements: [{ area: "ui", section: null }] }),
      "requirements/orders#order": target({ requirements: [{ area: "orders", section: "order" }] }),
      "specs#languages": target({ relevant: ["specs"], specs: ["languages"] }),
      "specs#notes": target({ relevant: ["specs"], specs: ["notes"] }),
      "specs#cloud": target({ relevant: ["specs"] }),
    },
    ...patch,
  };
}

export function snapshot(patch: Partial<ViewSnapshot> = {}, data: Partial<ViewData> = {}): ViewSnapshot {
  return {
    projectId: PROJECT,
    revision: 1,
    overlays: [],
    overlay: null,
    scope: [],
    data: viewData(data),
    ...patch,
  };
}
