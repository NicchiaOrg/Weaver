import { useEffect, useRef } from "react";

import { selection } from "@/components/files/tree-model";

/**
 * The text field that takes the place of a tile's name while the asset is renamed
 * (`requirements/ui#asset-view`, **Actions**): it holds the name with the part before the
 * extension selected. Enter confirms, Esc or leaving the field cancels. A refused name stays in
 * the field, for the developer to correct.
 */
export function AssetNameField({
  initial,
  onCommit,
  onCancel,
}: {
  initial: string;
  /** Resolves `true` when the rename was done; a refusal shows its reason in a toast and resolves `false`. */
  onCommit: (name: string) => Promise<boolean>;
  /** The field ended without a rename: `refocus` is whether the tile gets the focus back (Esc), not when the developer went elsewhere. */
  onCancel: (refocus: boolean) => void;
}) {
  const field = useRef<HTMLInputElement>(null);
  const finished = useRef(false);

  useEffect(() => {
    const input = field.current;
    if (!input) return;
    input.focus();
    const [from, to] = selection(initial, false);
    input.setSelectionRange(from, to);
  }, [initial]);

  const confirm = () => {
    if (finished.current) return;
    finished.current = true;
    onCommit(field.current?.value ?? "")
      .then((done) => {
        if (!done) finished.current = false;
      })
      .catch(() => {
        finished.current = false;
      });
  };

  return (
    <input
      ref={field}
      type="text"
      aria-label={`New name for ${initial}`}
      defaultValue={initial}
      spellCheck={false}
      autoComplete="off"
      onClick={(event) => event.stopPropagation()}
      onPointerDown={(event) => event.stopPropagation()}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === "Enter") {
          event.preventDefault();
          confirm();
        } else if (event.key === "Escape") {
          event.preventDefault();
          finished.current = true;
          onCancel(true);
        }
      }}
      onBlur={() => {
        if (finished.current) return;
        finished.current = true;
        onCancel(false);
      }}
      className="h-6 w-full min-w-0 rounded-sm border bg-background px-1 text-center text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
    />
  );
}
