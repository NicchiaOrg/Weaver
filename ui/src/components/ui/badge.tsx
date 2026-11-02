import { cva, type VariantProps } from "class-variance-authority";
import * as React from "react";

import { cn } from "@/lib/utils";

// Coloured variants use the palette at 600 (light) / 400 (dark) for the text, on the theme's
// background with a coloured border: the meaning is in the text and icon the badge carries too.
const badgeVariants = cva(
  "inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs font-semibold whitespace-nowrap transition-colors [&_svg]:size-3 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        default: "border-transparent bg-primary text-primary-foreground",
        secondary: "border-transparent bg-secondary text-secondary-foreground",
        outline: "text-foreground",
        success: "border-green-600 text-green-600 dark:border-green-400 dark:text-green-400",
        warning: "border-amber-600 text-amber-600 dark:border-amber-400 dark:text-amber-400",
        error: "border-red-600 text-red-600 dark:border-red-400 dark:text-red-400",
        info: "border-blue-600 text-blue-600 dark:border-blue-400 dark:text-blue-400",
        muted: "border-transparent bg-muted text-muted-foreground",
      },
    },
    defaultVariants: { variant: "default" },
  },
);

export interface BadgeProps extends React.HTMLAttributes<HTMLDivElement>, VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return <div className={cn(badgeVariants({ variant }), className)} {...props} />;
}

export { Badge, badgeVariants };
