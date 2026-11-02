import { ArrowDown, ArrowUp, Eraser, Square } from "lucide-react";
import { Fragment, useEffect, useRef, useState } from "react";

import type { Entry } from "@/bindings/Entry";
import { ConfirmDialog } from "@/components/dialogs";
import { useGeneral } from "@/components/general/use-general";
import { GuardedButton } from "@/components/guarded-button";
import { EntryView } from "@/components/jobs/jobs-view";
import { useProject } from "@/components/project-context";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { call } from "@/lib/backend";
import { useOmp } from "@/hooks/use-omp";

const EMPTY = "Talk with omp in this project. Ask it to start a job of Weaver when you want one.";

/** The index of the last message of the developer that omp has not received yet, or -1. */
function lastQueued(entries: readonly Entry[]): number {
  for (let index = entries.length - 1; index >= 0; index -= 1) {
    const entry = entries[index];
    if (entry?.kind === "developer" && entry.delivery.status === "queued") return index;
  }
  return -1;
}

/** General: the free conversation with omp of the project, always there in its window. */
export function GeneralView() {
  const { projectId } = useProject();
  const { status } = useOmp();
  const { state, entries, problem, send, stop, answer, clear } = useGeneral(projectId);
  const [text, setText] = useState("");
  const [confirming, setConfirming] = useState(false);
  const scroller = useRef<HTMLDivElement>(null);
  const [following, setFollowing] = useState(true);
  useEffect(() => {
    const element = scroller.current;
    if (following && element) element.scrollTop = element.scrollHeight;
  }, [entries, following]);

  const working = state?.working ?? false;
  const waiting = state?.starting ? lastQueued(entries) : -1;
  const empty = entries.length === 0 && !state?.kept;
  const missing = status?.state === "missing";

  const submit = () => {
    const message = text.trim();
    if (!message) return;
    setText("");
    setFollowing(true);
    send(message);
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="flex items-center justify-between gap-2 border-b px-3 py-2">
        <h2 className="text-sm font-semibold">General</h2>
        <GuardedButton variant="outline" size="sm" reasons={[empty && "The conversation is empty"]} onClick={() => setConfirming(true)}>
          <Eraser aria-hidden />
          Clear
        </GuardedButton>
      </header>
      <div
        ref={scroller}
        className="relative min-h-0 flex-1 space-y-3 overflow-y-auto p-3"
        onScroll={(event) => {
          const element = event.currentTarget;
          setFollowing(element.scrollHeight - element.scrollTop - element.clientHeight < 24);
        }}
      >
        {entries.length === 0 && <p className="py-8 text-center text-sm text-muted-foreground">{EMPTY}</p>}
        {entries.map((entry, index) => (
          <Fragment key={index}>
            <EntryView entry={entry} answer={answer} />
            {index === waiting && <p className="text-right text-xs text-muted-foreground">Starting omp…</p>}
          </Fragment>
        ))}
        {!following && (
          <Button className="sticky bottom-2 ml-auto flex" size="sm" variant="secondary" onClick={() => setFollowing(true)}>
            <ArrowDown aria-hidden />
            Jump to latest
          </Button>
        )}
      </div>
      {missing && (
        <div role="status" className="flex items-center gap-2 border-t p-3 text-sm text-red-600 dark:text-red-400">
          omp is not installed.
          <Button variant="outline" size="sm" onClick={() => void call("omp_install").catch(() => {})}>
            Install omp
          </Button>
        </div>
      )}
      {problem && (
        <p role="alert" className="border-t px-3 pt-2 text-xs text-red-600 dark:text-red-400">
          {problem}
        </p>
      )}
      <div className="flex items-end gap-2 border-t p-3">
        <Textarea
          aria-label="Message omp"
          placeholder="Message omp…"
          rows={1}
          className="max-h-60 min-h-9 flex-1"
          value={text}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              submit();
            }
          }}
        />
        <Button size="icon" aria-label="Send" aria-disabled={text.trim() === ""} onClick={submit}>
          <ArrowUp aria-hidden />
        </Button>
        {working && (
          <Button variant="outline" size="sm" onClick={stop}>
            <Square aria-hidden />
            Stop
          </Button>
        )}
      </div>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title="Clear General?"
        description={
          working
            ? "The agent's turn stops, the conversation is deleted and a new one starts."
            : "The conversation is deleted and a new one starts."
        }
        confirmLabel="Clear"
        destructive
        onConfirm={clear}
      />
    </div>
  );
}
