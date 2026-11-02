import { CircleX, TriangleAlert, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Toaster as Sonner, type ToasterProps } from "sonner";

import { useResolvedTheme } from "@/components/theme-provider";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

/** Every toast leaves by itself this long after it appears. */
export const TOAST_DURATION_MS = 4000;
/** At most this many toasts are visible at once. */
export const VISIBLE_TOASTS = 3;

/** The `Alt+T` hotkey (sonner's own default, kept explicit): it moves the focus to the toasts. */
const HOTKEY = ["altKey", "KeyT"];

/** The part of a toast's text and icon that carries the colour of its kind (600 light, 400 dark). */
function kindClasses(tone: "amber" | "red"): string {
  return tone === "amber"
    ? "!border-amber-600 dark:!border-amber-400 [&_[data-icon]]:text-amber-600 dark:[&_[data-icon]]:text-amber-400 [&_[data-title]]:!text-amber-600 dark:[&_[data-title]]:!text-amber-400"
    : "!border-red-600 dark:!border-red-400 [&_[data-icon]]:text-red-600 dark:[&_[data-icon]]:text-red-400 [&_[data-title]]:!text-red-600 dark:[&_[data-title]]:!text-red-400";
}

/**
 * The `X` inside the close button of a toast, with the `Close` tooltip. Sonner draws the button
 * itself and gives it no tooltip, so this icon fills the button and carries the tooltip: it opens on
 * hover (after the usual delay) and at once when the button gets the focus.
 */
function ToastCloseIcon() {
  const icon = useRef<HTMLSpanElement>(null);
  const [open, setOpen] = useState(false);
  useEffect(() => {
    const button = icon.current?.closest("button");
    if (!button) return;
    const show = () => setOpen(true);
    const hide = () => setOpen(false);
    button.addEventListener("focus", show);
    button.addEventListener("blur", hide);
    return () => {
      button.removeEventListener("focus", show);
      button.removeEventListener("blur", hide);
    };
  }, []);
  return (
    <Tooltip open={open} onOpenChange={setOpen}>
      <TooltipTrigger asChild>
        <span ref={icon} className="flex size-full items-center justify-center">
          <X className="size-3.5" aria-hidden />
        </span>
      </TooltipTrigger>
      {/* Above the toaster, which sits at the top of the stacking order. */}
      <TooltipContent className="z-[1000000000]">Close</TooltipContent>
    </Tooltip>
  );
}

/**
 * Toasts (`toast(...)` from `sonner`), themed with the tokens of the current theme: bottom right, at
 * most 3 visible, gone after 4 s, each with an icon-only close button reachable with `Alt+T` then
 * `Tab`. Info is the neutral popover look without icon; a warning is amber with `TriangleAlert`, an
 * error red with `CircleX`. Esc never closes a toast (sonner only collapses the stack on it).
 */
const Toaster = (props: ToasterProps) => {
  const theme = useResolvedTheme();
  return (
    <Sonner
      theme={theme}
      className="toaster group"
      position="bottom-right"
      closeButton
      duration={TOAST_DURATION_MS}
      visibleToasts={VISIBLE_TOASTS}
      hotkey={HOTKEY}
      icons={{
        info: null,
        warning: <TriangleAlert className="size-4" aria-hidden />,
        error: <CircleX className="size-4" aria-hidden />,
        close: <ToastCloseIcon />,
      }}
      toastOptions={{
        closeButtonAriaLabel: "Close",
        classNames: {
          toast: "group toast !bg-popover !text-popover-foreground !border-border !shadow-lg",
          description: "!text-muted-foreground",
          closeButton:
            "!size-6 !border-border !bg-popover !text-foreground hover:!bg-accent focus-visible:!outline-none focus-visible:!shadow-none focus-visible:!ring-2 focus-visible:!ring-ring",
          warning: kindClasses("amber"),
          error: kindClasses("red"),
        },
      }}
      {...props}
    />
  );
};

export { Toaster };
