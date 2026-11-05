import type { AskKind } from "@/bindings/AskKind";
import type { AskRow } from "@/bindings/AskRow";
import type { JobRow } from "@/bindings/JobRow";
import type { JobState } from "@/bindings/JobState";

/** A row of the Jobs list: a job, or an Ask more conversation. */
export type ListRow = { type: "job"; job: JobRow } | { type: "ask"; ask: AskRow };

/** The text of the empty message box, by what the conversation is about. */
export const PLACEHOLDERS: Record<AskKind, string> = {
  proposal: "Ask about the Proposal…",
  task: "Ask about the Task…",
  drift: "Ask about the Drift…",
  history: "Ask about the History entry…",
  requirements: "Ask about the Requirements…",
  question: "Ask about the question…",
};

/** After the first message the placeholder is the one of a job. */
export const MESSAGE_PLACEHOLDER = "Message the agent…";

const WORKING_ORDER: readonly JobState[] = ["waiting", "running", "queued", "suspended"];

/**
 * The Jobs list with the Ask more conversations in it: a `waiting` or `running` conversation sits
 * with the jobs in the same state (the most recent opening or start first); the `idle` ones come
 * after the suspended jobs and before the finished ones, the most recent first. The jobs keep the
 * order the backend gave them.
 */
export function mergeRows(jobs: readonly JobRow[], asks: readonly AskRow[]): ListRow[] {
  const rows: ListRow[] = [];
  const newest = (a: ListRow, b: ListRow) => at(b).localeCompare(at(a));
  const at = (row: ListRow) => (row.type === "job" ? (row.job.started ?? row.job.created) : row.ask.opened);
  for (const state of WORKING_ORDER) {
    const group: ListRow[] = jobs.filter((job) => job.state === state).map((job) => ({ type: "job", job }));
    if (state === "waiting" || state === "running") {
      group.push(...asks.filter((ask) => ask.status === state).map((ask) => ({ type: "ask" as const, ask })));
      group.sort(newest);
    }
    rows.push(...group);
  }
  const idle = asks.filter((ask) => ask.status === "idle").sort((a, b) => b.opened.localeCompare(a.opened));
  rows.push(...idle.map((ask) => ({ type: "ask" as const, ask })));
  rows.push(
    ...jobs
      .filter((job) => !WORKING_ORDER.includes(job.state))
      .map((job) => ({ type: "job" as const, job })),
  );
  return rows;
}
