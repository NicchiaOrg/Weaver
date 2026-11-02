import type { DriftView } from "@/bindings/DriftView";

/** The view of a Drift with nothing to say and every control enabled, for the tests. */
export function driftView(patch: Partial<DriftView> = {}): DriftView {
  return {
    refs: [],
    happens: "",
    should: "",
    options: [],
    whereInCode: "",
    places: [],
    previous: null,
    resolving: false,
    prepared: false,
    resolveReasons: [],
    dismissReasons: [],
    attached: null,
    ...patch,
  };
}
