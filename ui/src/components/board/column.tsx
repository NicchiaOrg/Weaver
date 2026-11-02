import { forwardRef, useId, type ReactNode } from "react";

import { ScrollArea } from "@/components/ui/scroll-area";

export interface ColumnProps {
  title: string;
  /** The number of real cards; ghosts never count and a resolution counts once. */
  count: number;
  /** What an empty column says, also shown above ghosts. */
  emptyText: string;
  /** The cards, already in the order the backend gave. */
  cards: ReactNode;
  /** The ghosts, after the cards. */
  ghosts?: ReactNode;
  /** Always visible at the bottom of the column. */
  footer: ReactNode;
}

/**
 * One of the two columns of the Board, of equal width. It scrolls on its own while its footer
 * stays at its bottom, and it takes the focus (it is a focus target, not a tab stop) when a
 * details dialog closes because its card left the Board.
 */
export const Column = forwardRef<HTMLElement, ColumnProps>(function Column(
  { title, count, emptyText, cards, ghosts, footer },
  ref,
) {
  const headingId = useId();
  return (
    <section
      ref={ref}
      tabIndex={-1}
      aria-labelledby={headingId}
      className="flex min-h-0 min-w-0 flex-col rounded-lg border bg-muted/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
    >
      <h2 className="flex items-baseline gap-2 px-3 pt-3 pb-2 text-sm font-semibold">
        <span id={headingId}>{title}</span>
        <span className="font-normal text-muted-foreground">{count}</span>
      </h2>
      <ScrollArea className="min-h-0 flex-1">
        <div className="flex flex-col gap-2 px-3 pb-3">
          {count === 0 ? <p className="py-4 text-center text-muted-foreground">{emptyText}</p> : null}
          {cards}
          {ghosts}
        </div>
      </ScrollArea>
      <div className="p-3 pt-0">{footer}</div>
    </section>
  );
});
