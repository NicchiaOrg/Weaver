import { ChevronDown, ChevronRight } from "lucide-react";
import { useId, useRef, useState } from "react";

import type { HistoryEntry } from "@/bindings/HistoryEntry";
import type { HistoryTask } from "@/bindings/HistoryTask";
import { AskMoreButton } from "@/components/ask/ask-button";
import { CardId } from "@/components/card-id";
import { FormDialog } from "@/components/dialogs";
import {
  DEFAULT_PERIOD,
  entryTime,
  groupByDay,
  PERIODS,
  plainSummary,
  type DayGroup,
  type Period,
} from "@/components/history/history-model";
import { useHistory } from "@/components/history/use-history";
import { Markdown } from "@/components/markdown";
import { useProject } from "@/components/project-context";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ViewHeader } from "@/components/view-header";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { formatDate, formatStarted, formatTime } from "@/lib/format";
import { labelTone } from "@/lib/label-tone";
import { TONE_BORDER, TONE_TEXT } from "@/lib/tones";
import { cn } from "@/lib/utils";

const tasksText = (count: number) => `${count} ${count === 1 ? "Task" : "Tasks"}`;

/** A label as a badge in its label colour; the text carries the meaning, the colour only tells labels apart. */
export function LabelBadge({ label }: { label: string }) {
  const tone = labelTone(label);
  return (
    <Badge variant="outline" className={cn(TONE_BORDER[tone], TONE_TEXT[tone])}>
      {label}
    </Badge>
  );
}

function EntryRow({ entry, showTime, onOpen }: { entry: HistoryEntry; showTime: boolean; onOpen: () => void }) {
  const time = entryTime(entry);
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          onClick={onOpen}
          className="flex w-full items-baseline gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <span className="max-w-[40%] shrink-0 truncate font-medium">{entry.title}</span>
          <span className="min-w-0 flex-1 truncate text-muted-foreground">{plainSummary(entry.summary)}</span>
          {showTime ? <time dateTime={entry.updated} className="shrink-0 font-mono text-xs text-muted-foreground">{formatTime(time)}</time> : null}
          <span className="shrink-0 text-xs text-muted-foreground">{tasksText(entry.tasks.length)}</span>
        </button>
      </TooltipTrigger>
      <TooltipContent>{entry.summary}</TooltipContent>
    </Tooltip>
  );
}

function DayList({ group, onOpen }: { group: DayGroup; onOpen: (entry: HistoryEntry) => void }) {
  const headingId = useId();
  return (
    <section aria-labelledby={headingId} className="flex flex-col gap-1">
      <h2 id={headingId} className="px-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        {group.header}
      </h2>
      <ul className="flex flex-col">
        {group.entries.map((entry) => (
          <li key={entry.id}>
            <EntryRow entry={entry} showTime={group.showTime} onOpen={() => onOpen(entry)} />
          </li>
        ))}
      </ul>
    </section>
  );
}

/** One Task of an entry, from its copy: a row that expands to its body and refs, read-only. */
function TaskRow({ task, now }: { task: HistoryTask; now: number }) {
  const [expanded, setExpanded] = useState(false);
  const panelId = useId();
  const finished = task.finished === null ? null : formatStarted(Date.parse(task.finished), now);
  const Chevron = expanded ? ChevronDown : ChevronRight;
  return (
    <li className="rounded-md border">
      <button
        type="button"
        aria-expanded={expanded}
        aria-controls={panelId}
        onClick={() => setExpanded((value) => !value)}
        className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <Chevron className="size-4 shrink-0" aria-hidden />
        <CardId permanentId={task.id} />
        <span className="min-w-0 flex-1 truncate">{task.title}</span>
        {task.label ? <LabelBadge label={task.label} /> : null}
        {finished ? <span className="shrink-0 text-xs text-muted-foreground">Finished {finished}</span> : null}
      </button>
      <div id={panelId} hidden={!expanded} className="flex flex-col gap-2 border-t px-3 py-2">
        {task.body ? <Markdown holderHeadingLevel={3}>{task.body}</Markdown> : <p className="text-sm text-muted-foreground">No text.</p>}
        {task.refs.length > 0 ? (
          <ul aria-label="References" className="flex flex-wrap gap-1">
            {task.refs.map((ref) => (
              <li key={ref}>
                <Badge variant="muted" className="font-mono font-normal">
                  {ref}
                </Badge>
              </li>
            ))}
          </ul>
        ) : null}
      </div>
    </li>
  );
}

/** The entry in a Dialog: title, date, summary, text, then its Tasks from the copies it keeps; `Ask more` on the whole entry in its footer. */
function EntryDialog({
  entry,
  now,
  onClose,
}: {
  entry: HistoryEntry | null;
  now: number;
  onClose: (open: boolean) => void;
}) {
  // The last entry shown keeps the dialog's content while it closes.
  const last = useRef<HistoryEntry | null>(null);
  if (entry) last.current = entry;
  const shown = entry ?? last.current;
  if (!shown) return null;
  const time = entryTime(shown);
  return (
    <FormDialog
      open={entry !== null}
      onOpenChange={onClose}
      title={shown.title}
      description={`${formatDate(time)}, ${formatTime(time)}`}
      footer={<AskMoreButton target={{ kind: "history", entry: shown.id }} noun="History entry" name={shown.id} />}
    >
      <div className="flex flex-col gap-4">
        <div className="font-medium">
          <Markdown holderHeadingLevel={2}>{shown.summary}</Markdown>
        </div>
        {shown.text ? <Markdown holderHeadingLevel={2}>{shown.text}</Markdown> : null}
        <section aria-label="Tasks" className="flex flex-col gap-2">
          <h3 className="text-sm font-semibold">{tasksText(shown.tasks.length)}</h3>
          <ul className="flex flex-col gap-2">
            {shown.tasks.map((task) => (
              <TaskRow key={task.id} task={task} now={now} />
            ))}
          </ul>
        </section>
      </div>
    </FormDialog>
  );
}

export interface HistoryViewProps {
  /** The current time in milliseconds; tests pass a fixed one. */
  now?: () => number;
}

/**
 * The History of the project window: the entries newest first by their date, grouped by local
 * day, filtered by a period (`Last 7 days` at each opening of the window). What is in an entry
 * and its order come from the backend; this view only groups by day and filters by period,
 * because only the interface knows the local time zone.
 */
export function HistoryView({ now = Date.now }: HistoryViewProps) {
  const { projectId } = useProject();
  const { snapshot, loadError, reload } = useHistory(projectId);
  const [period, setPeriod] = useState<Period>(DEFAULT_PERIOD);
  const [openedId, setOpenedId] = useState<string | null>(null);
  const instant = now();

  const periodSelect = (
    <Select value={period} onValueChange={(value) => setPeriod(value as Period)}>
      <SelectTrigger aria-label="Period" size="sm" className="w-44">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {PERIODS.map((choice) => (
          <SelectItem key={choice.id} value={choice.id}>
            {choice.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );

  if (!snapshot) {
    return (
      <div className="flex h-full min-h-0 flex-col">
        <ViewHeader view="history">{periodSelect}</ViewHeader>
        {loadError ? (
          <div className="flex flex-col items-start gap-3 p-4">
            <Alert variant="error" role="alert">
              <p className="min-w-0 flex-1">{loadError}</p>
            </Alert>
            <Button variant="outline" onClick={reload}>
              Try again
            </Button>
          </div>
        ) : (
          <p role="status" className="p-4 text-muted-foreground">
            Loading the History…
          </p>
        )}
      </div>
    );
  }

  const groups = groupByDay(snapshot.entries, period, instant);
  const opened = snapshot.entries.find((entry) => entry.id === openedId) ?? null;
  return (
    <div className="flex h-full min-h-0 flex-col">
      <ViewHeader view="history">{periodSelect}</ViewHeader>
      <div className="min-h-0 flex-1 overflow-y-auto px-3 py-4">
        {groups.length === 0 ? (
          <p className="p-3 text-muted-foreground">Nothing done in this period</p>
        ) : (
          <div className="flex flex-col gap-4">
            {groups.map((group) => (
              <DayList
                key={group.day}
                group={group}
                onOpen={(entry) => setOpenedId(entry.id)}
              />
            ))}
          </div>
        )}
      </div>
      <EntryDialog entry={opened} now={instant} onClose={(open) => !open && setOpenedId(null)} />
    </div>
  );
}
