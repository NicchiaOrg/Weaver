import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { useFileTabs } from "@/components/files/file-tabs-context";
import { fileName } from "@/components/files/tab-model";

/**
 * The dialogs of the tabs: closing a tab with unsaved changes (`Save`, `Discard`, `Cancel`), and a
 * save that found the file changed on disk (`Reload from disk`, `Overwrite`, `Cancel`). Both are
 * `AlertDialog`s: they close only with their buttons or Esc, and `Cancel` has the initial focus.
 */
export function FileTabDialogs() {
  const { dialog, tabs, answerClose, answerChanged } = useFileTabs();
  const tab = dialog ? tabs.find((candidate) => candidate.id === dialog.id) : undefined;
  const closing = dialog?.kind === "close" && tab !== undefined;
  const changed = dialog?.kind === "changed" && tab !== undefined;
  return (
    <>
      <AlertDialog open={closing} onOpenChange={(open) => !open && answerClose("cancel")}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Save changes to {tab ? fileName(tab.path) : ""}?</AlertDialogTitle>
            <AlertDialogDescription>
              <span className="font-mono">{tab?.path}</span> has unsaved changes.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={() => answerClose("discard")}>
              Discard
            </AlertDialogAction>
            <AlertDialogAction onClick={() => answerClose("save")}>Save</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
      <AlertDialog open={changed} onOpenChange={(open) => !open && answerChanged("cancel")}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>This file changed on disk since it was opened.</AlertDialogTitle>
            <AlertDialogDescription>
              <span className="font-mono">{tab?.path}</span>: reloading loses your changes, overwriting replaces what is on disk.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={() => answerChanged("reload")}>
              Reload from disk
            </AlertDialogAction>
            <AlertDialogAction variant="destructive" onClick={() => answerChanged("overwrite")}>
              Overwrite
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
