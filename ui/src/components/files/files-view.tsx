import { TriangleAlert } from "lucide-react";
import { useRef, useState } from "react";

import { DeleteDialog, LeaveDialog, MoveDialog } from "@/components/files/file-dialogs";
import { FileTabs } from "@/components/files/file-tabs";
import { FileTabsProvider, useFileTabs } from "@/components/files/file-tabs-context";
import { FileTree, type FileTreeHandle } from "@/components/files/file-tree";
import { useLeaveGuard } from "@/components/files/leave-guard";
import { useFileTree } from "@/components/files/use-file-tree";
import { useProject } from "@/components/project-context";
import { ViewHeader } from "@/components/view-header";
import { Alert } from "@/components/ui/alert";

/** What the developer is deciding when he leaves the view with unsaved changes. */
interface Leaving {
  paths: readonly string[];
  proceed: () => void;
}

function FilesLayout() {
  const { projectId } = useProject();
  const tabs = useFileTabs();
  const tree = useFileTree(projectId);
  const treeHandle = useRef<FileTreeHandle>(null);
  const [moving, setMoving] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [leaving, setLeaving] = useState<Leaving | null>(null);

  const focusTree = () => setTimeout(() => treeHandle.current?.focus(), 0);

  // Another view takes the place of this one: the tabs close, after the developer chose what to do
  // with the unsaved changes. Cancel, and a file that could not be saved, keep the view.
  const closeTabsThen = (proceed: () => void) => {
    tabs.closeAll();
    proceed();
  };
  useLeaveGuard((proceed) => {
    if (tabs.unsavedPaths.length === 0) closeTabsThen(proceed);
    else setLeaving({ paths: [...tabs.unsavedPaths], proceed });
  });

  const saveAll = (proceed: () => void) => {
    tabs
      .saveAll()
      .then((failures) => failures.length === 0 && closeTabsThen(proceed))
      .catch(() => {});
  };

  const moveSource = moving === null ? null : tree.entryOf(moving);
  const deleteEntry = deleting === null ? null : tree.entryOf(deleting);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <ViewHeader view="files" />
      <div className="flex min-h-0 flex-1">
        <div className="flex w-72 shrink-0 flex-col overflow-auto border-r">
          {tree.failure ? (
            <Alert variant="error" className="m-2">
              <TriangleAlert aria-hidden />
              <p className="min-w-0 flex-1">{tree.failure}</p>
            </Alert>
          ) : null}
          <FileTree ref={treeHandle} tree={tree} onRequestMove={setMoving} onRequestDelete={setDeleting} />
        </div>
        <div className="min-w-0 flex-1">
          <FileTabs />
        </div>
      </div>
      <MoveDialog
        source={moveSource}
        onMove={tree.move}
        onClose={() => {
          setMoving(null);
          focusTree();
        }}
      />
      <DeleteDialog
        path={deleteEntry ? deleting : null}
        folder={deleteEntry?.kind === "folder"}
        unsaved={deleting === null ? 0 : tabs.unsavedUnder(deleting).length}
        onConfirm={(path) => void tree.remove(path)}
        onClose={() => {
          setDeleting(null);
          focusTree();
        }}
      />
      <LeaveDialog
        paths={leaving?.paths ?? null}
        onSaveAll={() => leaving && saveAll(leaving.proceed)}
        onDiscardAll={() => leaving && closeTabsThen(leaving.proceed)}
        onCancel={() => setLeaving(null)}
      />
    </div>
  );
}

/**
 * The File view (`Files` in the sidebar): the tree of the project's files on the left, the tabs and
 * the editor on the right. It exists only while it is shown: choosing another view closes it.
 */
export function FilesView() {
  return (
    <FileTabsProvider>
      <FilesLayout />
    </FileTabsProvider>
  );
}
