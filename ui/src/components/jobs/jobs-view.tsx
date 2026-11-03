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
import { useEffect, useMemo, useState, type ReactNode } from "react";

import type { AgentNode } from "@/bindings/AgentNode";
import type { AskRow } from "@/bindings/AskRow";
import type { Entry } from "@/bindings/Entry";
import type { JobRow } from "@/bindings/JobRow";
import type { JobState } from "@/bindings/JobState";
import type { Step } from "@/bindings/Step";
import type { Tool } from "@/bindings/Tool";
import { ConfirmDialog } from "@/components/dialogs";
import { runningText, startedText } from "@/components/jobs/format";
import { useConversation, useJobs } from "@/components/jobs/use-jobs";
import { mergeRows, type ListRow } from "@/components/ask/ask-model";
import { AskConversation } from "@/components/ask/ask-conversation";
import { useOptionalAsks } from "@/components/ask/use-asks";
import { Markdown } from "@/components/markdown";
import { useProject } from "@/components/project-context";
import {
  jobSubAgentsApi,
  askSubAgentsApi,
  namesAlong,
  nodeAt,
  useSubAgentConversation,
  useSubAgentList,
  useSubAgentsView,
  type SubAgentsApi,
} from "@/components/sub-agents/api";
import { AgentState, SubAgentLine, SubAgentList, SubAgentNavProvider, type SubAgentNav } from "@/components/sub-agents/sub-agent-ui";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { VIEW_HEADER_CLASS, ViewHeader, viewTitleId } from "@/components/view-header";
import { useFollowScroll } from "@/hooks/use-follow-scroll";
import { call, isAppError } from "@/lib/backend";
import { cn } from "@/lib/utils";
import type { ViewId } from "@/views";

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

function Row({
  job,
  selected,
  current,
  onSelect,
  children,
}: {
  job: JobRow;
  selected: boolean;
  /** The job's own conversation is shown (not one of its sub-agents'). */
  current: boolean;
  onSelect: () => void;
  /** The list of the job's sub-agents, when it is the selected one. */
  children?: ReactNode;
}) {
  const { remove } = useJobs();
  const running = useRunning(job);
  const finished = job.state === "done" || job.state === "failed" || job.state === "stopped";
  return (
    <li className={`border-b ${selected ? "bg-accent" : ""}`}>
      <div className="flex items-start gap-1">
        <button type="button" aria-current={current} className="min-w-0 flex-1 px-3 py-2 text-left" onClick={onSelect}>
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
      </div>
      {children}
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
        <SubAgentLine key={agent.id} call={tool.id} agent={agent} />
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
    case "assignment":
      return (
        <div className="rounded border p-2 text-sm">
          <div className="text-xs font-semibold text-muted-foreground">{entry.from}</div>
          <div className="whitespace-pre-wrap">{entry.text}</div>
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
        This job has ended. Continue in Chat.
        <Button variant="outline" size="sm" onClick={() => showView("general")}>
          Open Chat
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

/** The box of a sub-agent conversation: the layout of the job's message box, with nothing to send. */
function DisabledMessageBox() {
  return (
    <div className="flex items-end gap-2 border-t p-3">
      <Textarea aria-label="Message the agent" placeholder="Sub-agents take no messages." rows={1} className="max-h-60 min-h-9 flex-1" value="" readOnly disabled />
      <Button size="icon" aria-label="Send" disabled>
        <ArrowUp aria-hidden />
      </Button>
    </div>
  );
}

/** Follows new text unless the developer scrolled up; then `Jump to latest` shows. `version` changes with the content. */
export function FollowScroller({ version, children }: { version: unknown; children: ReactNode }) {
  const { scrollerRef, contentRef, following, follow, onScroll } = useFollowScroll(version);
  return (
    <div ref={scrollerRef} className="relative min-h-0 flex-1 overflow-y-auto p-4" onScroll={onScroll}>
      <div ref={contentRef} className="space-y-3">
        {children}
      </div>
      {!following && (
        <Button className="sticky bottom-2 ml-auto mt-3 flex" size="sm" variant="secondary" onClick={follow}>
          <ArrowDown aria-hidden />
          Jump to latest
        </Button>
      )}
    </div>
  );
}

function Conversation({ projectId, job, ompReady }: { projectId: string; job: JobRow; ompReady: boolean }) {
  const entries = useConversation(projectId, job.id);
  return (
    <div className="flex h-full min-h-0 flex-col">
      <FollowScroller version={entries}>
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
      </FollowScroller>
      <MessageBox projectId={projectId} job={job} />
    </div>
  );
}

const NOT_ASKED = () => {};

/**
 * The read-only conversation of the sub-agent at `path` (of a job, or of General): the path from
 * the owner with the state, then what it received and did, and a disabled message box at the
 * bottom. No `Stop`, no permission card, no notes, no end line.
 */
export function SubAgentConversationView({
  api,
  ownerName,
  nodes,
  path,
  back,
  actions,
  headerOf,
}: {
  api: SubAgentsApi;
  ownerName: string;
  nodes: readonly AgentNode[];
  path: readonly Step[];
  /** What leads the header, such as the `Back to Chat` button of the Chat view. */
  back?: ReactNode;
  /** What ends the header, such as the `Clear` button of the Chat view. */
  actions?: ReactNode;
  /**
   * The view whose header this one replaces (Chat): the header then has the view header's height
   * and its title is the heading the view's region is labelled by.
   */
  headerOf?: ViewId;
}) {
  const { entries, unavailable } = useSubAgentConversation(api, path);
  const node = nodeAt(nodes, path);
  const Title = headerOf ? "h1" : "h2";
  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className={cn(headerOf ? VIEW_HEADER_CLASS : "flex items-center gap-3 border-b px-4 py-3", "text-sm")}>
        {back}
        <Title id={headerOf && viewTitleId(headerOf)} className="min-w-0 truncate font-semibold">
          {[ownerName, ...namesAlong(nodes, path)].join(" › ")}
        </Title>
        {node && <AgentState state={node.state} />}
        {actions && <div className="ml-auto">{actions}</div>}
      </header>
      <FollowScroller version={entries}>
        {unavailable !== null ? (
          <p className="text-sm text-muted-foreground">Conversation not available: {unavailable.replace(/\.$/, "")}.</p>
        ) : (
          entries.map((entry, index) => <EntryView key={index} entry={entry} answer={NOT_ASKED} />)
        )}
      </FollowScroller>
      <DisabledMessageBox />
    </div>
  );
}

const NO_ASKS: readonly AskRow[] = [];

/** A row of the list that is an Ask more conversation: name, kind `ask`, badge, opening time and `X`. */
function AskListRow({
  ask,
  selected,
  current,
  onSelect,
  children,
}: {
  ask: AskRow;
  selected: boolean;
  /** The conversation itself is shown (not one of its sub-agents'). */
  current: boolean;
  onSelect: () => void;
  children?: ReactNode;
}) {
  const asks = useOptionalAsks();
  const [confirming, setConfirming] = useState(false);
  const working = ask.status !== "idle";
  return (
    <li className={`border-b ${selected ? "bg-accent" : ""}`}>
      <div className="flex items-start gap-1">
        <button type="button" aria-current={current} className="min-w-0 flex-1 px-3 py-2 text-left" onClick={onSelect}>
          <div className="flex items-center gap-2">
            <span className="truncate font-semibold">{ask.name}</span>
            <span className="text-xs text-muted-foreground">ask</span>
            {ask.status === "idle" ? <Badge variant="outline">idle</Badge> : <StateBadge state={ask.status} />}
          </div>
          <div className="text-xs text-muted-foreground">{startedText(ask.opened)}</div>
        </button>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon" aria-label={`Close ${ask.name}`} onClick={() => (working ? setConfirming(true) : asks?.close(ask.id))}>
              <X aria-hidden />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Close</TooltipContent>
        </Tooltip>
      </div>
      {children}
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={`Close ${ask.name}?`}
        description="The agent's turn stops and the conversation ends."
        confirmLabel="Close"
        destructive
        onConfirm={() => asks?.close(ask.id)}
      />
    </li>
  );
}

/** Which row of the list is chosen: a job or a conversation. */
interface Chosen {
  type: "job" | "ask";
  id: string;
}

/**
 * The Jobs view: the project's jobs and Ask more conversations on the left, the selected one's
 * conversation, or its selected sub-agent's, on the right.
 */
export function JobsView() {
  const { projectId, openedJob } = useProject();
  const { jobs, ompReady } = useJobs();
  const asksState = useOptionalAsks();
  const asks = asksState?.asks ?? NO_ASKS;
  const opened = asksState?.opened ?? null;
  const [chosen, setChosen] = useState<Chosen | null>(null);
  const [agent, setAgent] = useState<{ owner: string; path: Step[] } | null>(null);
  const [focus, setFocus] = useState({ id: "", nonce: 0 });
  useEffect(() => {
    if (openedJob) {
      setChosen({ type: "job", id: openedJob });
      setAgent(null);
    }
  }, [openedJob]);
  // A conversation just opened: it is the one shown, and its message box takes the focus.
  useEffect(() => {
    if (opened) {
      setChosen({ type: "ask", id: opened.id });
      setAgent(null);
      setFocus({ id: opened.id, nonce: opened.nonce });
    }
  }, [opened]);
  const rows = useMemo(() => mergeRows(jobs, asks), [jobs, asks]);
  const identity = (row: ListRow) => (row.type === "job" ? { type: row.type, id: row.job.id } : { type: row.type, id: row.ask.id });
  const selected = rows.find((row) => chosen && identity(row).type === chosen.type && identity(row).id === chosen.id) ?? rows[0] ?? null;
  const selectedKey = selected ? `${identity(selected).type}:${identity(selected).id}` : null;
  const agentPath = agent && agent.owner === selectedKey ? agent.path : null;
  const api = useMemo(() => {
    if (!selected) return null;
    return selected.type === "job" ? jobSubAgentsApi(projectId, selected.job.id) : askSubAgentsApi(projectId, selected.ask.id);
    // The API follows the identity of the selected row, not its changing fields.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, selectedKey]);
  const nodes = useSubAgentList(api);
  useSubAgentsView(api, agentPath);
  const nav = useMemo<SubAgentNav>(
    () => ({
      base: agentPath ?? [],
      open: (path) => {
        if (selectedKey) setAgent({ owner: selectedKey, path });
      },
    }),
    [agentPath, selectedKey],
  );
  const select = (row: ListRow) => {
    setChosen(identity(row));
    setAgent(null);
  };
  const panels = (
    <div className="flex min-h-0 flex-1">
      <nav aria-label="Jobs" className="w-[300px] shrink-0 overflow-y-auto border-r">
        {rows.length === 0 ? (
          <p className="p-4 text-sm text-muted-foreground">No jobs yet</p>
        ) : (
          <ul className="pt-3">
            {rows.map((row) => {
              const key = `${identity(row).type}:${identity(row).id}`;
              const here = key === selectedKey;
              const list = here && nodes.length > 0 && (
                <SubAgentList nodes={nodes} selected={agentPath} onSelect={(path) => setAgent({ owner: key, path })} />
              );
              return row.type === "job" ? (
                <Row key={key} job={row.job} selected={here} current={here && !agentPath} onSelect={() => select(row)}>
                  {list}
                </Row>
              ) : (
                <AskListRow key={key} ask={row.ask} selected={here} current={here && !agentPath} onSelect={() => select(row)}>
                  {list}
                </AskListRow>
              );
            })}
          </ul>
        )}
      </nav>
      <section aria-label="Conversation" className="min-w-0 flex-1">
        <SubAgentNavProvider value={nav}>
          {selected &&
            (agentPath && api ? (
              <SubAgentConversationView
                api={api}
                ownerName={selected.type === "job" ? selected.job.name : selected.ask.name}
                nodes={nodes}
                path={agentPath}
              />
            ) : selected.type === "job" ? (
              <Conversation key={selected.job.id} projectId={projectId} job={selected.job} ompReady={ompReady} />
            ) : (
              <AskConversation key={selected.ask.id} ask={selected.ask} focus={focus.id === selected.ask.id ? focus.nonce : 0} />
            ))}
        </SubAgentNavProvider>
      </section>
    </div>
  );
  return (
    <div className="flex h-full min-h-0 flex-col">
      <ViewHeader view="jobs" />
      {panels}
    </div>
  );
}
