import * as React from "react";

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
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

/** Where the focus goes when a dialog closes, when the control that opened it will be gone. */
export type ReturnFocus = React.RefObject<HTMLElement | null>;

/**
 * Returns the focus to the element that had it when the dialog opened (the control that opened it),
 * or to `explicit` when that one is gone. Radix does it by itself only for a `Trigger`; the dialogs
 * of Weaver are opened from state, by buttons, menu items and cards.
 */
function useFocusReturn(open: boolean, explicit: ReturnFocus | undefined) {
  const opener = React.useRef<HTMLElement | null>(null);
  // A layout effect runs before Radix moves the focus into the dialog (its effects are later).
  React.useLayoutEffect(() => {
    if (open) opener.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  }, [open]);
  return (event: Event) => {
    const target = explicit?.current?.isConnected ? explicit.current : opener.current?.isConnected ? opener.current : null;
    if (target && target !== document.body) {
      event.preventDefault();
      target.focus();
    }
  };
}

export interface FormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: React.ReactNode;
  /** Short text under the title; a dialog always has a description for assistive technologies. */
  description?: React.ReactNode;
  /** The buttons; they stay in view while the body scrolls. */
  footer?: React.ReactNode;
  children: React.ReactNode;
  /** Set it when the dialog was opened from a menu item or a control that goes away. */
  returnFocus?: ReturnFocus;
  className?: string;
}

/**
 * A `Dialog` for details and forms: 720 px at most, the body scrolls, Esc, the close button and a
 * click outside close it, and the focus goes back to what opened it.
 */
export function FormDialog({
  open,
  onOpenChange,
  title,
  description,
  footer,
  children,
  returnFocus,
  className,
}: FormDialogProps) {
  const onCloseAutoFocus = useFocusReturn(open, returnFocus);
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={className} onCloseAutoFocus={onCloseAutoFocus}>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription className={description ? undefined : "sr-only"}>{description ?? title}</DialogDescription>
        </DialogHeader>
        <DialogBody>{children}</DialogBody>
        {footer ? <DialogFooter>{footer}</DialogFooter> : null}
      </DialogContent>
    </Dialog>
  );
}

export interface ConfirmDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: React.ReactNode;
  description: React.ReactNode;
  /** Label of the confirming button (`Delete`, `Dismiss`, `Scan System`...). */
  confirmLabel: string;
  /** `destructive` for an action that removes or discards something. */
  destructive?: boolean;
  cancelLabel?: string;
  /** Called when the developer confirms; the dialog closes after it. A cancelled one changes nothing. */
  onConfirm: () => void;
  returnFocus?: ReturnFocus;
  /** What the dialog lists under its text (paths, ...), before the buttons. */
  children?: React.ReactNode;
}

/**
 * A confirmation in an `AlertDialog`: it closes only with its two buttons or Esc, the initial
 * focus is on Cancel, the confirming button is `destructive` for destructive actions, and the
 * focus goes back to what opened it.
 */
export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel,
  destructive = false,
  cancelLabel = "Cancel",
  onConfirm,
  returnFocus,
  children,
}: ConfirmDialogProps) {
  const onCloseAutoFocus = useFocusReturn(open, returnFocus);
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent onCloseAutoFocus={onCloseAutoFocus}>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        {children}
        <AlertDialogFooter>
          <AlertDialogCancel>{cancelLabel}</AlertDialogCancel>
          <AlertDialogAction variant={destructive ? "destructive" : "default"} onClick={onConfirm}>
            {confirmLabel}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
