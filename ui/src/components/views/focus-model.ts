import type { ViewData } from "@/bindings/ViewData";

/** The two views that hold a focus. */
export type ViewName = "requirements" | "specs";

/** What a focus is about: a Proposal's scope, a Task's references, or one reference (a chip). */
export type FocusSubject =
  | { kind: "proposal"; id: string; number: string | null; title: string }
  | { kind: "task"; id: string; number: string | null; title: string }
  | { kind: "ref"; ref: string };

/** `Proposal P3 – <title>`, `Task T4 – <title>`, or the reference. */
export function subjectText(subject: FocusSubject): string {
  if (subject.kind === "ref") return subject.ref;
  const label = subject.kind === "proposal" ? "Proposal" : "Task";
  return `${label} ${subject.number ?? subject.id} – ${subject.title}`;
}

/** One step of a focus: a section or Area, or a Specs field. */
export interface FocusStep {
  /** Stable across recomputations: `area#section`, or a field name. */
  key: string;
  /** The name of the section or Area, or of the field. */
  name: string;
}

export interface FocusPlan {
  steps: FocusStep[];
  /** The references this view concerns and finds nothing for. */
  notFound: string[];
}

export const EMPTY_PLAN: FocusPlan = { steps: [], notFound: [] };

/** The key of a Requirements step: an Area, or one of its sections. */
export function sectionKey(area: string, section: string | null): string {
  return section === null ? area : `${area}#${section}`;
}

function requirementsPlan(refs: readonly string[], data: ViewData): FocusPlan {
  // Page order: an Area, then its sections in fragment and file order.
  const order = new Map<string, { index: number; name: string }>();
  for (const area of data.requirements) {
    order.set(sectionKey(area.id, null), { index: order.size, name: area.title });
    for (const fragment of area.fragments) {
      for (const section of fragment.sections) {
        const key = sectionKey(area.id, section.title);
        if (!order.has(key)) order.set(key, { index: order.size, name: `${area.title} › ${section.title}` });
      }
    }
  }
  const found = new Set<string>();
  const notFound: string[] = [];
  for (const ref of refs) {
    const targets = data.targets[ref];
    if (targets && !targets.relevant.includes("requirements")) continue;
    const keys = (targets?.requirements ?? []).map((a) => sectionKey(a.area, a.section)).filter((k) => order.has(k));
    if (keys.length === 0) notFound.push(ref);
    keys.forEach((k) => found.add(k));
  }
  const steps = [...found]
    .sort((a, b) => order.get(a)!.index - order.get(b)!.index)
    .map((key) => ({ key, name: order.get(key)!.name }));
  return { steps, notFound };
}

function specsPlan(refs: readonly string[], data: ViewData): FocusPlan {
  const fields = data.specs?.fields.map((f) => f.name) ?? [];
  const found = new Set<string>();
  const notFound: string[] = [];
  for (const ref of refs) {
    const targets = data.targets[ref];
    if (targets && !targets.relevant.includes("specs")) continue;
    const names = (targets?.specs ?? []).filter((n) => fields.includes(n));
    if (names.length === 0) notFound.push(ref);
    names.forEach((n) => found.add(n));
  }
  const steps = fields.filter((f) => found.has(f)).map((key) => ({ key, name: key }));
  return { steps, notFound };
}

/** The steps of a focus on `refs` in `view`, and the references that view cannot find. */
export function planFocus(view: ViewName, refs: readonly string[], data: ViewData): FocusPlan {
  if (refs.length === 0) return EMPTY_PLAN;
  if (view === "requirements") return requirementsPlan(refs, data);
  return specsPlan(refs, data);
}

/** The views a reference can concern, from its text alone (for a Proposal's scope the snapshot may not know yet). */
function guessedViews(ref: string): ViewName[] {
  if (ref === "specs" || ref.startsWith("specs#")) return ["specs"];
  return ["requirements"];
}

/**
 * How many steps a focus on `refs` has in `view`. References the snapshot has no focus for (the
 * scope of a Proposal not drawn yet may add elements) count as one step when their text can name
 * something of that view: the overlay of the Proposal is drawn when the focus opens.
 */
export function stepCount(view: ViewName, refs: readonly string[], data: ViewData | null, assumeUnknown: boolean): number {
  if (!data || refs.length === 0) return 0;
  const known = refs.filter((r) => data.targets[r]);
  const unknown = assumeUnknown ? refs.filter((r) => !data.targets[r] && guessedViews(r).includes(view)) : [];
  return planFocus(view, known, data).steps.length + (unknown.length > 0 ? 1 : 0);
}

/** Where a step is after a recomputation: the same one when it still exists, else the nearest index. */
export function settleStep(steps: readonly FocusStep[], key: string | null, index: number): number {
  if (steps.length === 0) return 0;
  const same = key === null ? -1 : steps.findIndex((s) => s.key === key);
  return same >= 0 ? same : Math.min(Math.max(index, 0), steps.length - 1);
}
