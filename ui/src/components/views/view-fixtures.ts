import type { Detail } from "@/bindings/Detail";
import type { DiagramEdge } from "@/bindings/DiagramEdge";
import type { DiagramNode } from "@/bindings/DiagramNode";
import type { Level } from "@/bindings/Level";
import type { RefTargets } from "@/bindings/RefTargets";
import type { ViewData } from "@/bindings/ViewData";
import type { ViewSnapshot } from "@/bindings/ViewSnapshot";

// A small project for the tests of the views: an Actor `user`, a Feature `order` with the Machines
// `cart` and `payment`, and two Requirement Areas.

export const PROJECT = "0123456789abcdef";

export function detail(title: string, patch: Partial<Detail> = {}): Detail {
  return {
    kind: "feature",
    title,
    description: "",
    change: null,
    transitions: [],
    events: [],
    drifts: [],
    tasks: [],
    file: null,
    line: null,
    ...patch,
  };
}

export function node(id: string, title: string, patch: Partial<DiagramNode> = {}): DiagramNode {
  return {
    id,
    reference: id,
    kind: "feature",
    title,
    subtitle: null,
    opens: null,
    expandable: false,
    parent: null,
    change: null,
    drifts: 0,
    initial: false,
    final: false,
    compact: false,
    dashed: false,
    muted: false,
    ...patch,
  };
}

export function edge(id: string, source: string, target: string, label: string, patch: Partial<DiagramEdge> = {}): DiagramEdge {
  return { id, source, target, label, detail: detail(`${source} → ${target}`, { kind: "flow" }), events: [], change: null, inside: null, ...patch };
}

export function level(key: string, kind: Level["kind"], title: string, parent: string | null, nodes: DiagramNode[], edges: DiagramEdge[] = [], grid = false): Level {
  return { key, kind, title, parent, grid, nodes, edges };
}

export function levels(): Level[] {
  return [
    level(
      "",
      "root",
      "System",
      null,
      [
        node("whole", "Whole System", { kind: "whole-system", opens: "system", reference: null }),
        node("actor:user", "user", { kind: "actor", subtitle: "1 feature", opens: "actor:user" }),
      ],
      [],
      true,
    ),
    level(
      "system",
      "system",
      "Whole System",
      "",
      [
        node("actor:user", "user", { kind: "actor", subtitle: "1 feature", opens: "actor:user" }),
        node("feature:order", "order", { subtitle: "2 machines", opens: "feature:order", drifts: 2 }),
      ],
      [edge("e:actor:user>feature:order", "actor:user", "feature:order", "2")],
    ),
    level(
      "actor:user",
      "actor",
      "user",
      "",
      [
        node("actor:user", "user", { kind: "actor", subtitle: "1 feature" }),
        node("feature:order", "order", { subtitle: "2 machines", opens: "feature:order" }),
      ],
      [edge("e:actor:user>feature:order", "actor:user", "feature:order", "PAY, CANCEL")],
    ),
    level(
      "feature:order",
      "feature",
      "order",
      "",
      [
        node("machine:cart", "cart", { kind: "machine", opens: "machine:cart", expandable: true }),
        node("machine:payment", "payment", { kind: "machine", opens: "machine:payment" }),
        node("state:cart/Empty", "Empty", { kind: "state", reference: "cart/Empty", parent: "machine:cart", initial: true }),
        node("ext-actor:user", "user", { kind: "external-actor", compact: true, dashed: true, reference: "user" }),
      ],
      [
        edge("e:machine:cart>machine:payment", "machine:cart", "machine:payment", "CHARGE"),
        edge("e:ext-actor:user>machine:cart", "ext-actor:user", "machine:cart", "PAY"),
        edge("t:cart/Empty/0", "state:cart/Empty", "state:cart/Empty", "user.PAY", { inside: "machine:cart" }),
      ],
    ),
    level(
      "machine:cart",
      "machine",
      "cart",
      "feature:order",
      [
        node("state:cart/Empty", "Empty", { kind: "state", initial: true, opens: "state:cart/Empty", reference: "cart/Empty" }),
        node("state:cart/Paid", "Paid", { kind: "state", final: true, opens: "state:cart/Paid", reference: "cart/Paid" }),
      ],
      [edge("t:cart/Empty/1", "state:cart/Empty", "state:cart/Paid", "user.PAY")],
    ),
    level("state:cart/Empty", "state", "Empty", "machine:cart", [node("state:cart/Empty", "Empty", { kind: "state", reference: "cart/Empty" })]),
    level("state:cart/Paid", "state", "Paid", "machine:cart", [node("state:cart/Paid", "Paid", { kind: "state", reference: "cart/Paid" })]),
  ];
}

export function details(): Record<string, Detail> {
  return {
    "actor:user": detail("user", { kind: "actor", description: "The person who orders. Anyone can." }),
    "feature:order": detail("order", {
      description: "Everything about one order. It starts in a cart.",
      drifts: [{ id: "dddddddd", shortNumber: "D1", title: "Cart total is wrong" }],
      tasks: [{ id: "tttttttt", shortNumber: "T4", title: "Fix the total" }],
      file: ".trama/model/order/feature.md",
      line: 3,
    }),
    "machine:cart": detail("cart", {
      kind: "machine",
      description: "A cart.",
      transitions: [
        { event: "user.PAY", machine: null, from: "Empty", target: "Paid", prose: "Takes the payment.", change: null, file: ".trama/model/order/cart.md", line: 12 },
      ],
    }),
    "cart/Empty": detail("Empty", { kind: "state" }),
    "cart/Paid": detail("Paid", { kind: "state" }),
    user: detail("user", { kind: "actor" }),
  };
}

const target = (patch: Partial<RefTargets>): RefTargets => ({ relevant: ["diagram", "requirements"], diagram: [], requirements: [], specs: [], ...patch });

export function viewData(patch: Partial<ViewData> = {}): ViewData {
  return {
    errors: [],
    model: { levels: levels(), details: details() },
    requirements: [
      {
        id: "orders",
        title: "Orders",
        description: "",
        intro: "How orders work.",
        file: ".trama/requirements/orders/area.md",
        fragments: [
          {
            name: "checkout",
            title: "Checkout",
            description: "",
            change: null,
            file: ".trama/requirements/orders/checkout.md",
            sections: [
              { title: "General", body: "General rules.", before: null, change: null, movedFrom: null, file: ".trama/requirements/orders/checkout.md", line: 3 },
              { title: "order", body: "An order has a cart.", before: null, change: null, movedFrom: null, file: ".trama/requirements/orders/checkout.md", line: 9 },
            ],
          },
        ],
      },
      {
        id: "ui",
        title: "Interface",
        description: "",
        intro: "",
        file: ".trama/requirements/ui/area.md",
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
          file: ".trama/specs.yaml",
          line: 2,
        },
        { name: "notes", change: null, entries: [], notes: "Plain *notes*.", beforeNotes: null, file: ".trama/specs.yaml", line: 9 },
      ],
    },
    targets: {
      "order/Paid": target({}),
      "cart/Paid": target({ diagram: [{ level: "machine:cart", nodes: ["state:cart/Paid"], edges: [] }] }),
      order: target({
        diagram: [
          { level: "system", nodes: ["feature:order"], edges: [] },
          { level: "actor:user", nodes: ["feature:order"], edges: [] },
        ],
        requirements: [{ area: "orders", section: "order" }],
      }),
      cart: target({ diagram: [{ level: "feature:order", nodes: ["machine:cart"], edges: [] }] }),
      "requirements/ui": target({ relevant: ["requirements"], requirements: [{ area: "ui", section: null }] }),
      "requirements/orders#order": target({ relevant: ["requirements", "diagram"], requirements: [{ area: "orders", section: "order" }] }),
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
