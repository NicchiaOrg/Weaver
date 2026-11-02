import {
  ArrowDown,
  ArrowUp,
  Bot,
  Check,
  ChevronDown,
  ChevronRight,
  Clock,
  FilePen,
  FileText,
  Info,
  Loader,
  MessageCircleQuestion,
  Pause,
  Search,
  Square,
  SquareTerminal,
  StickyNote,
  TriangleAlert,
  Wrench,
  X,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";

import type { Entry } from "@/bindings/Entry";
import type { JobRow } from "@/bindings/JobRow";
import type { JobState } from "@/bindings/JobState";
import type { Tool } from "@/bindings/Tool";
import { ConfirmDialog } from "@/components/dialogs";
import { runningText, startedText } from "@/components/jobs/format";
import { useConversation, useJobs } from "@/components/jobs/use-jobs";
import { Markdown } from "@/components/markdown";
import { useProject } from "@/components/project-context";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { call, isAppError } from "@/lib/backend";

const BADGES: Record<JobState, { label: string; icon: LucideIcon | null; className: string }> = {
  queued: { label: "queued", icon: null, className: "border-border text-foreground" },
  running: { label: "running", icon: Loader, className: "bg-blue-600/15 text-blue-600 dark:text-blue-400" },
  waiting: { label: "waiting", icon: MessageCircleQuestion, className: "bg-amber-600/15 text-amber-600 dark:text-amber-400" },
  suspended: { label: "suspended", icon: Pause, className: "border-border text-foreground" },
  done: { label: "done", icon: Check, className: "bg-green-600/15 text-green-600 dark:text-green-400" },
  failed: { label: "failed", icon: X, className: "bg-red-600/15 text-red-600 dark:text-red-400" },
  stopped: { label: "stopped", icon: Square, className: "bg-muted text-muted-foreground" },
};

export function StateBadge({ state }: { state: JobState }) {
  const { label, icon: Icon, className } = BADGES[state];
  return (
    <Badge variant="outline" className={className}>
      {Icon && <Icon aria-hidden className={state === "running" ? "size-3 animate-spin motion-reduce:animate-none" : "size-3"} />}
      {label}
    </Badge>
  );
}

/** Seconds since the session started, ticking while the job works and frozen at its end. */
function useRunning(job: JobRow): string | null {
  const [now, setNow] = useState(() => Date.now());
  const working = job.state === "running" || job.state === "waiting" || job.state === "suspended";
  useEffect(() => {
    if (!working) return;
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [working]);
  if (!job.started) return null;
  const end = job.ended ? new Date(job.ended).getTime() : now;
  return runningText(end - new Date(job.started).getTime());
}

function Row({ job, selected, onSelect }: { job: JobRow; selected: boolean; onSelect: () => void }) {
  const { remove } = useJobs();
  const running = useRunning(job);
  const finished = job.state === "done" || job.state === "failed" || job.state === "stopped";
  return (
    <li className={`flex items-start gap-1 border-b ${selected ? "bg-accent" : ""}`}>
      <button type="button" aria-current={selected} className="min-w-0 flex-1 px-3 py-2 text-left" onClick={onSelect}>
        <div className="flex items-center gap-2">
          <span className="truncate font-semibold">{job.name}</span>
          <span className="text-xs text-muted-foreground">{job.kind}</span>
          <StateBadge state={job.state} />
        </div>
        <div className="text-xs text-muted-foreground">
          {startedText(job.created)}
          {running ? ` · ${running}` : ""}
        </div>
        {job.note && (
          <div className="truncate text-xs text-muted-foreground" title={job.note}>
            {job.note}
          </div>
        )}
      </button>
      {finished && (
        <Button variant="ghost" size="icon" aria-label={`Remove ${job.name}`} title="Remove from the list" onClick={() => remove(job.id)}>
          <X aria-hidden />
        </Button>
      )}
    </li>
  );
}

const TOOL_ICONS: Record<Tool["icon"], LucideIcon> = {
  read: FileText,
  search: Search,
  command: SquareTerminal,
  edit: FilePen,
  "sub-agent": Bot,
  other: Wrench,
};

function ToolRow({ tool }: { tool: Tool }) {
  const [open, setOpen] = useState(false);
  const Icon = TOOL_ICONS[tool.icon];
  const text = tool.command ?? tool.title;
  const Status = tool.status === "done" ? Check : tool.status === "failed" ? X : Loader;
  return (
    <div className="text-sm">
      <button
        type="button"
        className="flex w-full items-center gap-2 text-left"
        aria-expanded={open}
        aria-label={tool.title}
        onClick={() => setOpen((value) => !value)}
      >
        {open ? <ChevronDown aria-hidden className="size-4" /> : <ChevronRight aria-hidden className="size-4" />}
        <Icon aria-hidden className="size-4" />
        <span className="truncate">{tool.title}</span>
        <Status aria-hidden className={`ml-auto size-4 ${tool.status === "failed" ? "text-red-600 dark:text-red-400" : ""} ${tool.status === "working" ? "animate-spin motion-reduce:animate-none" : ""}`} />
      </button>
      {open && <pre className="mt-1 max-h-48 overflow-auto rounded bg-muted p-2 font-mono text-xs whitespace-pre-wrap">{text}</pre>}
      {tool.subAgents.map((agent) => (
        <div key={agent.id} className="ml-8 flex items-center gap-2 text-xs">
          <Bot aria-hidden className="size-3" />
          <span>{agent.name}</span>
          <span className="text-muted-foreground">{agent.state}</span>
        </div>
      ))}
    </div>
  );
}

/** The conversation of a job and of General render alike; the permission card answers through `answer`. */
export function EntryView({ entry, answer }: { entry: Entry; answer: (request: number, option: string) => void }) {
  switch (entry.kind) {
    case "instruction":
      return (
        <div className="rounded border p-2 text-sm">
          <div className="text-xs font-semibold text-muted-foreground">Weaver</div>
          {entry.text}
        </div>
      );
    case "developer":
      return (
        <div className="ml-auto max-w-[80%] rounded bg-secondary p-2 text-sm whitespace-pre-wrap">
          {entry.text}
          {entry.delivery.status === "queued" && (
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <Clock aria-hidden className="size-3" />
              Queued
            </div>
          )}
          {entry.delivery.status === "not-delivered" && (
            <div className="text-xs text-red-600 dark:text-red-400">Not delivered: {entry.delivery.reason}.</div>
          )}
        </div>
      );
    case "agent":
      return <Markdown>{entry.text}</Markdown>;
    case "thinking":
      return (
        <details className="text-sm text-muted-foreground">
          <summary>Thinking</summary>
          <div className="whitespace-pre-wrap">{entry.text}</div>
        </details>
      );
    case "tool":
      return <ToolRow tool={entry} />;
    case "note":
      return (
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <StickyNote aria-hidden className="size-3" />
          <span>{startedText(entry.at).slice(-5)}</span>
          <span>{entry.text}</span>
        </div>
      );
    case "permission":
      return (
        <div role="group" aria-label="omp asks permission" className="rounded border border-amber-600 p-2 text-sm dark:border-amber-400">
          <div className="font-semibold">omp asks permission</div>
          {entry.tool && <ToolRow tool={entry.tool} />}
          {entry.state.status === "asking" ? (
            <div className="mt-2 flex flex-wrap gap-2">
              {entry.options.map((option) => (
                <Button
                  key={option.id}
                  size="sm"
                  variant={option.rejects ? "outline" : "default"}
                  onClick={() => answer(entry.request, option.id)}
                >
                  {option.name}
                </Button>
              ))}
            </div>
          ) : (
            <div className="mt-1 text-xs text-muted-foreground">
              {entry.state.status === "chosen" ? entry.state.name : entry.state.status === "void" ? "No longer needed." : "Cancelled."}
            </div>
          )}
        </div>
      );
    case "reloaded":
      return <div className="text-xs text-muted-foreground">Session reloaded; the agent continues its work.</div>;
    case "end":
      return entry.outcome === "failed" ? (
        <FailedLine reason={entry.reason} details={entry.details} />
      ) : (
        <div className="text-xs text-muted-foreground">{entry.outcome === "finished" ? "Finished." : "Stopped."}</div>
      );
    case "job-started":
      return <JobStartedLine job={entry.job} name={entry.name} />;
    case "job-refused":
      return (
        <div className="flex items-center gap-2 text-sm text-amber-600 dark:text-amber-400">
          <TriangleAlert aria-hidden className="size-4 shrink-0" />
          Weaver did not start {entry.jobKind}: {entry.reason.replace(/\.$/, "")}.
        </div>
      );
    case "notice":
      return (
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Info aria-hidden className="size-3 shrink-0" />
          {entry.text}
        </div>
      );
    case "turn-failed":
      return <ErrorLine text={`The turn failed: ${entry.reason.replace(/\.$/, "")}.`} details={null} />;
    case "omp-stopped":
      return (
        <div className="space-y-1">
          <ErrorLine text={`omp stopped: ${entry.reason.replace(/\.$/, "")}.`} details={entry.details} />
          <div className="text-xs text-muted-foreground">Your next message starts omp again.</div>
        </div>
      );
  }
}

/** `Started <job name>` with the button that selects that job in the Jobs view. */
function JobStartedLine({ job, name }: { job: string; name: string }) {
  const { openJob } = useProject();
  return (
    <div className="flex items-center gap-2 text-sm">
      <Bot aria-hidden className="size-4 shrink-0" />
      <span>Started {name}</span>
      <Button variant="outline" size="sm" onClick={() => openJob(job)}>
        Open conversation
      </Button>
    </div>
  );
}

function FailedLine({ reason, details }: { reason: string; details: string | null }) {
  return <ErrorLine text={`Failed: ${reason.replace(/\.$/, "")}.`} details={details} />;
}

/** A failure in the error colour, with an icon, and its details on request. */
function ErrorLine({ text, details }: { text: string; details: string | null }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="text-sm text-red-600 dark:text-red-400">
      <div className="flex items-center gap-2">
        <TriangleAlert aria-hidden className="size-4" />
        {text}
        {details && (
          <Button variant="ghost" size="sm" onClick={() => setOpen((value) => !value)}>
            Show details
          </Button>
        )}
      </div>
      {open && details && <pre className="mt-1 overflow-auto rounded bg-muted p-2 font-mono text-xs whitespace-pre-wrap">{details}</pre>}
    </div>
  );
}

function MessageBox({ projectId, job }: { projectId: string; job: JobRow }) {
  const [text, setText] = useState("");
  const [confirming, setConfirming] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const finished = job.state === "done" || job.state === "failed" || job.state === "stopped";
  const { showView } = useProject();
  if (finished) {
    return (
      <div className="flex items-center gap-2 border-t p-3 text-sm text-muted-foreground">
        This job has ended. Continue in General.
        <Button variant="outline" size="sm" onClick={() => showView("general")}>
          Open General
        </Button>
      </div>
    );
  }
  const canSend = job.state === "running" || job.state === "waiting";
  const send = () => {
    const message = text.trim();
    if (!message || !canSend) return;
    setText("");
    call("job_send_message", { projectId, jobId: job.id, text: message }).catch((error: unknown) =>
      setProblem(isAppError(error) ? error.message : "The message was not sent."),
    );
  };
  return (
    <div className="flex items-end gap-2 border-t p-3">
      <Textarea
        aria-label="Message the agent"
        placeholder="Message the agent…"
        rows={1}
        className="max-h-60 min-h-9 flex-1"
        value={text}
        disabled={!canSend}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault();
            send();
          }
        }}
      />
      <Button size="icon" aria-label="Send" aria-disabled={!canSend || text.trim() === ""} onClick={send}>
        <ArrowUp aria-hidden />
      </Button>
      <Button variant="outline" size="icon" aria-label="Stop the agent" title="Stop the agent" onClick={() => setConfirming(true)}>
        <Square aria-hidden />
      </Button>
      {problem && <span className="text-xs text-red-600 dark:text-red-400">{problem}</span>}
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={`Stop ${job.name}?`}
        description="The agent stops at once and every process it started ends."
        confirmLabel="Stop"
        destructive
        onConfirm={() => void call("job_stop", { projectId, jobId: job.id })}
      />
    </div>
  );
}

function Conversation({ projectId, job, ompReady }: { projectId: string; job: JobRow; ompReady: boolean }) {
  const entries = useConversation(projectId, job.id);
  const scroller = useRef<HTMLDivElement>(null);
  const [following, setFollowing] = useState(true);
  useEffect(() => {
    const element = scroller.current;
    if (following && element) element.scrollTop = element.scrollHeight;
  }, [entries, following]);
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div
        ref={scroller}
        className="relative min-h-0 flex-1 space-y-3 overflow-y-auto p-3"
        onScroll={(event) => {
          const element = event.currentTarget;
          setFollowing(element.scrollHeight - element.scrollTop - element.clientHeight < 24);
        }}
      >
        {job.state === "queued" && (
          <p className="py-8 text-center text-sm text-muted-foreground">{ompReady ? "Starting omp…" : "Waiting for omp to be ready."}</p>
        )}
        {entries.map((entry, index) => (
          <EntryView
            key={index}
            entry={entry}
            answer={(request, option) => void call("job_answer_permission", { projectId, jobId: job.id, request, option })}
          />
        ))}
        {job.state === "suspended" && (
          <p className="text-sm text-muted-foreground">Weaver stopped; the job continues when its session is reloaded.</p>
        )}
        {!following && (
          <Button className="sticky bottom-2 ml-auto flex" size="sm" variant="secondary" onClick={() => setFollowing(true)}>
            <ArrowDown aria-hidden />
            Jump to latest
          </Button>
        )}
      </div>
      <MessageBox projectId={projectId} job={job} />
    </div>
  );
}

/** The Jobs view: the project's jobs on the left, the selected job's conversation on the right. */
export function JobsView() {
  const { projectId, openedJob } = useProject();
  const { jobs, ompReady } = useJobs();
  const [chosen, setChosen] = useState<string | null>(null);
  useEffect(() => {
    if (openedJob) setChosen(openedJob);
  }, [openedJob]);
  const selected = jobs.find((job) => job.id === chosen) ?? jobs[0] ?? null;
  return (
    <div className="flex h-full min-h-0">
      <nav aria-label="Jobs" className="w-[300px] shrink-0 overflow-y-auto border-r">
        {jobs.length === 0 ? (
          <p className="p-4 text-sm text-muted-foreground">No jobs yet</p>
        ) : (
          <ul>
            {jobs.map((job) => (
              <Row key={job.id} job={job} selected={job.id === selected?.id} onSelect={() => setChosen(job.id)} />
            ))}
          </ul>
        )}
      </nav>
      <section aria-label="Conversation" className="min-w-0 flex-1">
        {selected && <Conversation key={selected.id} projectId={projectId} job={selected} ompReady={ompReady} />}
      </section>
    </div>
  );
}
