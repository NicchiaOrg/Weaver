import { SquareArrowOutUpRight } from "lucide-react";
import { useContext } from "react";
import { toast } from "sonner";

import { ProjectContext } from "@/components/project-context";
import { Button, type ButtonProps } from "@/components/ui/button";
import { call, isAppError } from "@/lib/backend";

export interface OpenInEditorProps extends Pick<ButtonProps, "className" | "variant" | "size"> {
  /** The file: absolute, or relative to the project root. */
  file: string;
  /** The 1-based line, or what gives it when the button is pressed; the file opens at its start without one. */
  line?: number | null | (() => number | null);
  /** The project a relative `file` belongs to; defaults to the project of the window. */
  projectId?: string;
}

/**
 * `Open in editor`: opens `file` at `line` in the editor chosen in the settings (preset or custom
 * command; the backend runs it without a shell). Shown wherever a file and a line are known. When
 * the editor cannot be started, a notice gives the reason.
 */
export function OpenInEditor({ file, line = null, projectId, variant = "ghost", size = "sm", className }: OpenInEditorProps) {
  const project = useContext(ProjectContext);
  const open = () => {
    const at = typeof line === "function" ? line() : line;
    call("open_in_editor", { file, line: at, projectId: projectId ?? project?.projectId ?? null }).catch((error: unknown) =>
      toast.error("Could not open the file in the editor.", {
        description: isAppError(error) ? error.message : "The editor could not be started.",
      }),
    );
  };
  return (
    <Button variant={variant} size={size} className={className} onClick={open}>
      <SquareArrowOutUpRight aria-hidden />
      Open in editor
    </Button>
  );
}
