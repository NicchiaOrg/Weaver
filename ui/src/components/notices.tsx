import { TriangleAlert, X } from "lucide-react";

import { Alert } from "@/components/ui/alert";
import { GuardedButton } from "@/components/guarded-button";

export interface Notice {
  id: string;
  /** What failed and why (`Could not delete T4: the agent that filed it is still working.`). */
  text: string;
}

/**
 * The notices at the top of a view, newest first, each an `Alert` with a close button, until
 * closed or the window closes. A decision the backend refused when applying it shows here and
 * changes nothing. `notices` are in arrival order; the newest is shown on top.
 */
export function NoticeStack({ notices, onClose }: { notices: readonly Notice[]; onClose: (id: string) => void }) {
  if (notices.length === 0) return null;
  return (
    <div className="flex flex-col gap-2 p-3">
      {[...notices].reverse().map((notice) => (
        <Alert key={notice.id} variant="warning">
          <TriangleAlert aria-hidden />
          <p className="min-w-0 flex-1">{notice.text}</p>
          <GuardedButton variant="ghost" size="icon" className="-my-1.5 size-7" label="Close notice" onClick={() => onClose(notice.id)}>
            <X aria-hidden />
          </GuardedButton>
        </Alert>
      ))}
    </div>
  );
}
