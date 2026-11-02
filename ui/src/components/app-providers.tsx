import type { ReactNode } from "react";

import { ThemeProvider, type ThemePreference } from "@/components/theme-provider";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";

/**
 * What every window of Weaver is wrapped in: the theme (from the Theme setting), the tooltip
 * timing (400 ms on hover, at once on focus) and the toaster.
 */
export function AppProviders({ preference, children }: { preference: ThemePreference; children: ReactNode }) {
  return (
    <ThemeProvider preference={preference}>
      <TooltipProvider>
        {children}
        <Toaster />
      </TooltipProvider>
    </ThemeProvider>
  );
}
