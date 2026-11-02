import type { MouseEvent, ReactNode } from "react";

import { useBoardUi } from "@/components/board/board-ui";
import { CardId } from "@/components/card-id";
import { cn } from "@/lib/utils";

export interface CardShellProps {
  /** The permanent identity: names the card to the Board (details, focus). */
  cardId: string;
  /** `T4`, when the card has a number. */
  shortNumber: string | null;
  title: string;
  /** The kind mark, at the right of the title. */
  mark?: ReactNode;
  /** Class of the card's own accent (a coloured left border, a tint). */
  accent?: string;
  /** The badges, in their order. */
  badges?: ReactNode;
  /** What sits between the badges and the controls. */
  body?: ReactNode;
  /** The controls. A click on them never opens the details. */
  controls?: ReactNode;
  /** Appears before the id (a status dot). */
  lead?: ReactNode;
}

const OTHER_CONTROLS = "button, a, input, select, textarea, [role='button'], [role='menuitem']";

/**
 * The frame every card of the Board shares: the id, the title (at most 3 lines, then `…`) and the
 * kind mark, a slot for badges, one for the body and one for the controls. A click on the card
 * outside its controls, or Enter on its title, opens the details dialog; no card can be dragged,
 * moved or removed.
 */
export function CardShell({ cardId, shortNumber, title, mark, accent, badges, body, controls, lead }: CardShellProps) {
  const { openDetails } = useBoardUi();
  const open = (event: MouseEvent<HTMLDivElement>) => {
    const target = event.target instanceof Element ? event.target : null;
    const control = target?.closest(OTHER_CONTROLS);
    // The title is the one button that opens the details; every other control keeps its click.
    if (control && !control.hasAttribute("data-card-title")) return;
    if (target?.closest("[data-card-controls]")) return;
    openDetails(cardId);
  };
  return (
    // Keyboard users open the details with Enter on the title (a button); the click handler of
    // the frame serves the pointer, and the title's click reaches it too.
    <div
      data-card-id={cardId}
      onClick={open}
      className={cn(
        "cursor-pointer rounded-lg border bg-card p-3 text-card-foreground shadow-sm transition-colors duration-150 hover:bg-accent/40",
        accent,
      )}
    >
      <div className="flex items-start gap-2">
        {lead}
        <CardId shortNumber={shortNumber} permanentId={cardId} className="mt-0.5 shrink-0" />
        <h3 className="min-w-0 flex-1 text-sm font-medium">
          <button
            type="button"
            data-card-title
            className="line-clamp-3 w-full rounded-sm text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            {title}
          </button>
        </h3>
        {mark ? <div className="shrink-0">{mark}</div> : null}
      </div>
      {badges ? <div className="mt-2 flex flex-wrap items-center gap-1">{badges}</div> : null}
      {body ? <div className="mt-2">{body}</div> : null}
      {controls ? (
        <div data-card-controls className="mt-3 flex flex-wrap items-center gap-2">
          {controls}
        </div>
      ) : null}
    </div>
  );
}
