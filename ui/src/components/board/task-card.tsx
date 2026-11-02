import { ListChecks, Package, Play, Trash2 } from "lucide-react";
import { useRef, useState } from "react";

import type { TitledRef } from "@/bindings/TitledRef";
import { useBoard } from "@/components/board/board-state";
import { useBoardActions, type FocusView } from "@/components/board/board-ui";
import { registerCardRenderer, type BoardCardOf } from "@/components/board/card-registry";
import { CardShell } from "@/components/board/card-shell";
import { ConfirmDialog } from "@/components/dialogs";
import { cardReasons, GuardedButton } from "@/components/guarded-button";
import { Markdown } from "@/components/markdown";
import { Badge, badgeVariants } from "@/components/ui/badge";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { FocusSubject } from "@/components/views/focus-model";
import { labelTone, leftBorderClass } from "@/lib/label-colour";
import { TONE_BORDER, TONE_TEXT } from "@/lib/tones";
import { cn } from "@/lib/utils";

type TaskCardData = BoardCardOf<"task">;

/** How a card is named in a sentence: its short number, else its permanent id. */
function nameOf(ref: { id: string; shortNumber: string | null }): string {
  return ref.shortNumber ?? ref.id;
}

/** `To do`, `In progress`, or `Agent working` while a job holds the Task. */
export function taskStatus(card: Pick<TaskCardData, "doing" | "heldByAgent">): string {
  if (card.heldByAgent) return "Agent working";
  return card.doing ? "In progress" : "To do";
}

/** The 8 px status dot of a Task: a hollow ring in To do, a filled amber dot in progress. Never animated. */
function StatusDot({ card }: { card: TaskCardData }) {
  const label = taskStatus(card);
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span
          role="img"
          aria-label={label}
          className={cn(
            "mt-1.5 inline-block size-2 shrink-0 rounded-full",
            card.doing ? "bg-amber-600 dark:bg-amber-400" : "border border-muted-foreground",
          )}
        />
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

const blockedText = (refs: readonly TitledRef[]) => `Blocked by Drift ${refs.map(nameOf).join(", ")}`;
const waitingText = (refs: readonly TitledRef[]) => `Waiting for Task ${refs.map(nameOf).join(", ")}`;

/**
 * Scrolls the card (or its row in a resolution card) into view and focuses it. A Drift sits in
 * Backlog as a card of its own or as a row of a resolution; a fix Task sits in Active.
 */
export function revealCard(id: string): void {
  const target =
    document.querySelector<HTMLElement>(`[data-card-id="${id}"]`) ?? document.querySelector<HTMLElement>(`[data-drift-row="${id}"]`);
  if (!target) return;
  target.scrollIntoView?.({ block: "nearest" });
  (target.querySelector<HTMLElement>("[data-card-title]") ?? target).focus();
}

interface RefsPopoverProps {
  /** `blocked` or `waiting`. */
  kind: "blocked" | "waiting";
  refs: readonly TitledRef[];
}

/**
 * The `blocked` and `waiting <id>` badges: a button that opens a Popover listing what blocks the
 * Task (`Blocked by`: Drifts, `Waiting for`: the fix Task). A row closes the popover, scrolls its
 * card into view and focuses it.
 */
function RefsPopover({ kind, refs }: RefsPopoverProps) {
  const [open, setOpen] = useState(false);
  const chosen = useRef<string | null>(null);
  const blocked = kind === "blocked";
  const text = blocked ? blockedText(refs) : waitingText(refs);
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <Tooltip>
        <TooltipTrigger asChild>
          <PopoverTrigger asChild>
            <button
              type="button"
              aria-label={text}
              className={cn(
                badgeVariants({ variant: blocked ? "error" : "warning" }),
                "cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              )}
            >
              {blocked ? "blocked" : `waiting ${refs.map(nameOf).join(", ")}`}
            </button>
          </PopoverTrigger>
        </TooltipTrigger>
        <TooltipContent>{text}</TooltipContent>
      </Tooltip>
      <PopoverContent
        aria-label={blocked ? "Blocked by" : "Waiting for"}
        onCloseAutoFocus={(event) => {
          // The focus goes to the card the developer chose, not back to the badge.
          const id = chosen.current;
          chosen.current = null;
          if (id) {
            event.preventDefault();
            revealCard(id);
          }
        }}
      >
        <p className="mb-2 text-xs font-semibold">{blocked ? "Blocked by" : "Waiting for"}</p>
        <ul className="space-y-1">
          {refs.map((ref) => (
            <li key={ref.id}>
              <button
                type="button"
                className="flex w-full items-baseline gap-2 rounded-sm px-1 py-0.5 text-left text-xs hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                onClick={() => {
                  chosen.current = ref.id;
                  setOpen(false);
                }}
              >
                <span className="font-mono">{nameOf(ref)}</span>
                <span className="min-w-0 flex-1">{ref.title}</span>
              </button>
            </li>
          ))}
        </ul>
      </PopoverContent>
    </Popover>
  );
}

/** The badge of the Proposal's label, in the label's colour (and its text, never colour alone). */
export function LabelBadge({ label }: { label: string }) {
  const tone = labelTone(label);
  return (
    <Badge variant="outline" className={cn(TONE_TEXT[tone], TONE_BORDER[tone])} title={`Label ${label}`}>
      {label}
    </Badge>
  );
}

function TaskCard({ card }: { card: TaskCardData }) {
  const { head } = card;
  const held = card.heldByAgent;
  return (
    <CardShell
      cardId={head.id}
      shortNumber={head.shortNumber}
      title={head.title}
      lead={<StatusDot card={card} />}
      accent={cn(leftBorderClass(card.label), held && "task-held")}
      badges={
        <>
          {card.technical ? <Badge variant="outline">technical</Badge> : null}
          {card.blockedBy.length > 0 ? <RefsPopover kind="blocked" refs={card.blockedBy} /> : null}
          {card.waitingFor.length > 0 ? <RefsPopover kind="waiting" refs={card.waitingFor} /> : null}
          {card.label ? <LabelBadge label={card.label} /> : null}
        </>
      }
      controls={<TaskControls card={card} />}
    />
  );
}

/** `View requirements` (only when its focus has at least one step) and `Run`. */
function TaskControls({ card }: { card: TaskCardData }) {
  const actions = useBoardActions();
  const subject: FocusSubject = { kind: "task", id: card.head.id, number: card.head.shortNumber, title: card.head.title };
  return (
    <>
      <FocusButton view="requirements" refs={card.refs} subject={subject} />
      <GuardedButton variant="outline" size="sm" reasons={card.runReasons} onClick={() => actions.runTask?.(card.head.id)}>
        <Play aria-hidden />
        Run
      </GuardedButton>
    </>
  );
}

const FOCUS_BUTTONS: Record<FocusView, { label: string; icon: typeof ListChecks }> = {
  requirements: { label: "View requirements", icon: ListChecks },
  specs: { label: "View specs", icon: Package },
};

/** Only when the focus on `refs` has at least one step in `view`; `subject` names the Task or Proposal it is about. */
export function FocusButton({ view, refs, subject }: { view: FocusView; refs: readonly string[]; subject?: FocusSubject }) {
  const { focusSteps, viewOn } = useBoardActions();
  if (refs.length === 0 || !viewOn || !focusSteps || focusSteps(view, refs, subject) < 1) return null;
  const { label, icon: Icon } = FOCUS_BUTTONS[view];
  return (
    <GuardedButton variant="outline" size="sm" onClick={() => viewOn(view, refs, subject)}>
      <Icon aria-hidden />
      {label}
    </GuardedButton>
  );
}

/** A reference as a chip: it focuses the first of Requirements and Specs where that reference has a step. */
export function RefChip({ reference }: { reference: string }) {
  const { focusSteps, viewOn } = useBoardActions();
  const single = [reference];
  const view: FocusView | null =
    viewOn && focusSteps ? (["requirements", "specs"] as const).find((v) => focusSteps(v, single) >= 1) ?? null : null;
  const chip = cn(badgeVariants({ variant: "outline" }), "font-mono font-normal");
  if (!view) return <span className={chip}>{reference}</span>;
  return (
    <button
      type="button"
      className={cn(chip, "cursor-pointer hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring")}
      aria-label={`${FOCUS_BUTTONS[view].label}: ${reference}`}
      onClick={() => viewOn?.(view, single)}
    >
      {reference}
    </button>
  );
}

/** The body of the details dialog: status, badges, label, references, and the text of the Task. */
function TaskDetails({ card }: { card: TaskCardData }) {
  const { head } = card;
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <Badge variant="secondary">{taskStatus(card)}</Badge>
        {card.technical ? <Badge variant="outline">technical</Badge> : null}
        {card.blockedBy.length > 0 ? <Badge variant="error">blocked</Badge> : null}
        {card.waitingFor.length > 0 ? <Badge variant="warning">waiting {card.waitingFor.map(nameOf).join(", ")}</Badge> : null}
        {card.label ? <LabelBadge label={card.label} /> : null}
      </div>
      {card.blockedBy.length > 0 ? (
        <p className="text-xs text-muted-foreground">
          {blockedText(card.blockedBy)}: {card.blockedBy.map((ref) => `${nameOf(ref)} ${ref.title}`).join("; ")}
        </p>
      ) : null}
      {card.waitingFor.length > 0 ? (
        <p className="text-xs text-muted-foreground">
          {waitingText(card.waitingFor)}: {card.waitingFor.map((ref) => `${nameOf(ref)} ${ref.title}`).join("; ")}
        </p>
      ) : null}
      {card.refs.length > 0 ? (
        <ul aria-label="References" className="flex flex-wrap gap-1">
          {card.refs.map((reference) => (
            <li key={reference}>
              <RefChip reference={reference} />
            </li>
          ))}
        </ul>
      ) : null}
      {head.body.trim() ? <Markdown holderHeadingLevel={2}>{head.body}</Markdown> : <p className="text-muted-foreground">No description.</p>}
    </div>
  );
}

/** `View requirements`, `Run` and, only in To do, `Delete` (confirmed). */
function TaskFooter({ card }: { card: TaskCardData }) {
  const { deleteTask } = useBoard();
  const [confirming, setConfirming] = useState(false);
  const name = nameOf(card.head);
  return (
    <>
      <TaskControls card={card} />
      {card.doing ? null : (
        <>
          <GuardedButton
            variant="destructive"
            size="sm"
            reasons={cardReasons(card.head.agentFinishing, card.deleteReasons)}
            onClick={() => setConfirming(true)}
          >
            <Trash2 aria-hidden />
            Delete
          </GuardedButton>
          <ConfirmDialog
            open={confirming}
            onOpenChange={setConfirming}
            title={`Delete ${name} for good?`}
            description="The Task is removed and nothing remains of it."
            confirmLabel="Delete"
            destructive
            onConfirm={() => void deleteTask(card.head.id, name)}
          />
        </>
      )}
    </>
  );
}

registerCardRenderer("task", {
  Card: TaskCard,
  Details: TaskDetails,
  Footer: TaskFooter,
  description: (card) => taskStatus(card),
});
