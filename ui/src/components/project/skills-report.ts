import type { SkillsReport } from "@/bindings/SkillsReport";

/** `Could not update the agent files: <reason>.` (the reason without its own full stop). */
export function skillsFailure(reason: string): string {
  return `Could not update the agent files: ${reason.trim().replace(/\.+$/, "")}.`;
}

/** What `Update skills` shows under its button: the same structured result the command line prints. */
export type ReportView =
  | { kind: "unchanged"; text: string }
  | { kind: "changed"; text: string; files: { path: string; action: string }[] };

export function reportView(report: SkillsReport): ReportView {
  if (report.changes.length === 0) return { kind: "unchanged", text: "Everything was already up to date." };
  const n = report.changes.length;
  return {
    kind: "changed",
    text: `Updated ${n} ${n === 1 ? "file" : "files"}:`,
    files: report.changes.map((change) => ({ path: change.path, action: change.action })),
  };
}
