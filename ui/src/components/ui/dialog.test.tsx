import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { AlertDialog, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogTitle } from "@/components/ui/alert-dialog";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";

afterEach(() => {
  vi.restoreAllMocks();
});

function expectCentredByLayout(panel: HTMLElement) {
  const overlay = panel.parentElement!;
  expect(overlay).toHaveClass("fixed", "inset-0", "grid", "place-items-center", "dialog-overlay");
  for (const element of [panel, overlay]) {
    expect(element.className).not.toMatch(/translate|zoom|scale|blur|animate-in|fade-in|top-1\/2|left-1\/2/);
  }
}

describe("Dialog", () => {
  it("is centred by the layout of a full-window container, without any shift or zoom", () => {
    render(
      <Dialog open>
        <DialogContent>
          <DialogTitle>Details</DialogTitle>
          <DialogDescription>Body</DialogDescription>
        </DialogContent>
      </Dialog>,
    );
    const panel = screen.getByRole("dialog");
    expectCentredByLayout(panel);
    expect(panel).toHaveClass("max-w-[720px]", "max-h-[85vh]");
  });

  it("moves its corner onto whole CSS pixels with offsets, never a transform", () => {
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      const left = 100.5 + (Number.parseFloat(this.style.left) || 0);
      const top = 40.25 + (Number.parseFloat(this.style.top) || 0);
      return { left, top, x: left, y: top, right: left + 10, bottom: top + 10, width: 10, height: 10, toJSON: () => ({}) };
    });
    render(
      <Dialog open>
        <DialogContent>
          <DialogTitle>Details</DialogTitle>
          <DialogDescription>Body</DialogDescription>
        </DialogContent>
      </Dialog>,
    );
    const panel = screen.getByRole("dialog");
    expect(panel.getBoundingClientRect().left).toBe(101);
    expect(panel.getBoundingClientRect().top).toBe(40);
    expect(panel.style.transform).toBe("");
  });

  it("closes with Esc", async () => {
    const onOpenChange = vi.fn();
    render(
      <Dialog open onOpenChange={onOpenChange}>
        <DialogContent>
          <DialogTitle>Details</DialogTitle>
          <DialogDescription>Body</DialogDescription>
        </DialogContent>
      </Dialog>,
    );
    await userEvent.keyboard("{Escape}");
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});

describe("AlertDialog", () => {
  it("is centred by the layout of a full-window container, without any shift or zoom", () => {
    render(
      <AlertDialog open>
        <AlertDialogContent>
          <AlertDialogTitle>Sure?</AlertDialogTitle>
          <AlertDialogDescription>Really.</AlertDialogDescription>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
        </AlertDialogContent>
      </AlertDialog>,
    );
    const panel = screen.getByRole("alertdialog");
    expectCentredByLayout(panel);
    expect(panel).toHaveClass("max-w-lg");
    expect(screen.getByRole("button", { name: "Cancel" })).toHaveFocus();
  });
});
