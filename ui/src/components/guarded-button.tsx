import * as React from "react";

import { Button, type ButtonProps } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

/** A reason a control is disabled, or a falsy value when that reason does not apply. */
export type Reason = string | false | null | undefined;

/** `The agent is still finishing`: the job that handed the card over still works. Always first on a card's controls. */
export const AGENT_FINISHING = "The agent is still finishing";

/** The reasons that apply, in the order given (the fixed order of the control), without repeats. */
export function activeReasons(reasons: readonly Reason[]): string[] {
  const seen = new Set<string>();
  for (const reason of reasons) if (reason) seen.add(reason);
  return [...seen];
}

/**
 * The reasons of a control of a card: `The agent is still finishing` first when it applies, then
 * the control's own reasons in their fixed order.
 */
export function cardReasons(agentFinishing: boolean, own: readonly Reason[]): string[] {
  return activeReasons([agentFinishing && AGENT_FINISHING, ...own]);
}

/** One reason per line, as the tooltip and the accessible description show them. */
export function ReasonLines({ lines }: { lines: readonly string[] }) {
  return (
    <>
      {lines.map((line) => (
        <span key={line} className="block">
          {line}
        </span>
      ))}
    </>
  );
}

export interface GuardedButtonProps extends Omit<ButtonProps, "disabled"> {
  /**
   * Why the control cannot be used, in the fixed order of that control; falsy entries are
   * skipped, so each reason can be written as `condition && "Reason"`. Empty = enabled.
   */
  reasons?: readonly Reason[];
  /** Tooltip while enabled (at most two short sentences). */
  tooltip?: string;
  /**
   * Accessible name of an icon-only button; shown as the tooltip's first line. Every icon-only
   * button has one.
   */
  label?: string;
}

/**
 * The one button of Weaver that can be unavailable for a reason.
 *
 * While `reasons` has entries it is not `disabled` but `aria-disabled`: it stays in the tab
 * order, keeps the focus it has, ignores every activation (click, Enter, Space), and shows all
 * its reasons, one per line, in the tooltip (also at keyboard focus) and in its accessible
 * description. When the reasons go away it is the same element, so it works again at once and
 * the focus has not moved.
 */
export const GuardedButton = React.forwardRef<HTMLButtonElement, GuardedButtonProps>(function GuardedButton(
  { reasons = [], tooltip, label, onClick, children, ...props },
  ref,
) {
  const lines = activeReasons(reasons);
  const blocked = lines.length > 0;
  const descriptionId = React.useId();
  const tooltipLines = blocked ? (label ? [label, ...lines] : lines) : [label ?? tooltip].filter((t): t is string => !!t);
  const button = (
    <Button
      ref={ref}
      {...props}
      aria-label={label ?? props["aria-label"]}
      {...(blocked ? { "aria-disabled": true, "aria-describedby": descriptionId } : {})}
      onClick={(event) => {
        if (blocked) {
          event.preventDefault();
          event.stopPropagation();
          return;
        }
        onClick?.(event);
      }}
    >
      {children}
    </Button>
  );
  return (
    <>
      {/* Always the same tree, so the button keeps its focus when its reasons come and go. */}
      <Tooltip>
        <TooltipTrigger asChild>{button}</TooltipTrigger>
        {tooltipLines.length > 0 ? (
          <TooltipContent>
            <ReasonLines lines={tooltipLines} />
          </TooltipContent>
        ) : null}
      </Tooltip>
      {blocked ? (
        <span id={descriptionId} className="sr-only">
          <ReasonLines lines={lines} />
        </span>
      ) : null}
    </>
  );
});
