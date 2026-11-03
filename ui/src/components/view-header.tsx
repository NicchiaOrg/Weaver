import type { ReactNode } from "react";

import { VIEWS, type ViewId } from "@/views";

/**
 * The row every view starts with (`requirements/ui#General`, "View header"): full width, a fixed
 * 56 px plus its 1 px bottom border whether or not it holds controls, side padding 16 px, content
 * centred in the row. A view that replaces the header (Chat in front of a sub-agent) takes the
 * same class so the border does not move.
 */
export const VIEW_HEADER_CLASS = "box-content flex h-14 shrink-0 items-center gap-3 border-b px-4";

/** The id of the heading the region of a view is labelled by. */
export const viewTitleId = (view: ViewId) => `view-${view}`;

/**
 * The header of a view: its name (the sidebar entry's) as the title on the left, the view's own
 * controls (`children`) on the right. The title is the view's main heading for assistive
 * technologies.
 */
export function ViewHeader({ view, children }: { view: ViewId; children?: ReactNode }) {
  const label = VIEWS.find((entry) => entry.id === view)?.label ?? view;
  return (
    <header className={VIEW_HEADER_CLASS}>
      <h1 id={viewTitleId(view)} className="min-w-0 truncate text-sm font-semibold">
        {label}
      </h1>
      {children ? <div className="ml-auto flex shrink-0 items-center gap-3">{children}</div> : null}
    </header>
  );
}
