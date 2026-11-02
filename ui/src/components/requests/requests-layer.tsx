import { FilePen } from "lucide-react";
import { useMemo, type ReactNode } from "react";

import { BoardActionsContext, useBoardActions } from "@/components/board/board-ui";
import { RequestEditor } from "@/components/requests/request-editor";
import { useRequests } from "@/components/requests/use-requests";
import { Button } from "@/components/ui/button";

/**
 * Adds `+ Work Item` (a new design request) to the actions the Board already has, keeping the ones
 * other parts of the window provide.
 */
export function RequestsBoardActions({ children }: { children: ReactNode }) {
  const parent = useBoardActions();
  const { newRequest } = useRequests();
  const actions = useMemo(() => ({ ...parent, newRequest }), [parent, newRequest]);
  return <BoardActionsContext.Provider value={actions}>{children}</BoardActionsContext.Provider>;
}

/**
 * Where the request editor opens: in place of what `children` show (the Board stays mounted, so
 * it keeps its state), with a way back to the requests that are still open.
 */
export function RequestsLayer({ projectId, children }: { projectId: string; children: ReactNode }) {
  const { snapshot, editing, openEditor } = useRequests();
  const open = snapshot?.requests.length ?? 0;
  return (
    <div className="flex h-full min-h-0 flex-col">
      {editing ? (
        <div className="min-h-0 flex-1">
          <RequestEditor projectId={projectId} />
        </div>
      ) : open > 0 ? (
        <div className="flex items-center gap-2 border-b px-3 py-2 text-sm">
          <FilePen aria-hidden className="size-4 text-muted-foreground" />
          <span>{open === 1 ? "1 request is open" : `${open} requests are open`}</span>
          <Button type="button" variant="outline" size="sm" className="ml-auto" onClick={() => openEditor()}>
            Open requests
          </Button>
        </div>
      ) : null}
      <div className="min-h-0 flex-1" hidden={editing}>
        {children}
      </div>
    </div>
  );
}
