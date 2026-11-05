import type { ProposalView } from "@/bindings/ProposalView";

/** The view of a Proposal that carries no Task, changes no file and has nothing to say, for the tests. */
export function proposalView(patch: Partial<ProposalView> = {}): ProposalView {
  return {
    label: null,
    stale: false,
    blocked: [],
    model: null,
    files: { added: [], changed: [], removed: [] },
    carriesTasks: false,
    creates: 0,
    tasks: [],
    removalTasks: [],
    requires: [],
    approve: { reasons: [], tooltip: null, refines: false },
    rejectReasons: [],
    canRework: true,
    scopeRefs: [],
    ...patch,
  };
}
