import { TriangleAlert } from "lucide-react";
import type { KeyboardEvent, ReactNode } from "react";

import { NoticeStack } from "@/components/notices";
import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { ErrorBanner } from "@/components/views/error-banner";
import type { ViewName } from "@/components/views/focus-model";
import { FocusBar } from "@/components/views/focus-bar";
import { OverlayPicker } from "@/components/views/overlay-picker";
import { useViews } from "@/components/views/views-state";

/**
 * What Diagram, Requirements and Specs have in common: the toolbar with the overlay picker, the
 * notice of an overlay that stopped being open, the error banner, the focus bar, and a message
 * while the snapshot is loading or could not be read. Esc clears the Diagram's selection first
 * and then the focus; on the other views it clears the focus.
 */
export function ViewFrame({ view, toolbar, children }: { view: ViewName; toolbar?: ReactNode; children: ReactNode }) {
  const { snapshot, loadError, reload, notices, closeNotice, focus, clearFocus, nav, select } = useViews();

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "Escape" || event.defaultPrevented) return;
    if (view === "diagram" && nav.selected !== null) select(null);
    else if (focus[view]) clearFocus(view);
    else return;
    event.preventDefault();
  };

  return (
    <div className="flex h-full min-h-0 flex-col" onKeyDown={onKeyDown}>
      <div className="flex shrink-0 items-center gap-3 border-b px-4 py-2">
        <OverlayPicker />
        {toolbar}
      </div>
      <NoticeStack notices={notices} onClose={closeNotice} />
      {snapshot ? <ErrorBanner errors={snapshot.data.errors} /> : null}
      <FocusBar view={view} />
      <div className="min-h-0 flex-1">
        {snapshot ? (
          children
        ) : loadError ? (
          <Alert variant="error" className="m-4 w-auto">
            <TriangleAlert aria-hidden />
            <p className="min-w-0 flex-1">{loadError}</p>
            <Button variant="outline" size="sm" onClick={reload}>
              Try again
            </Button>
          </Alert>
        ) : (
          <p className="p-4 text-sm text-muted-foreground" role="status">
            Loading…
          </p>
        )}
      </div>
    </div>
  );
}
