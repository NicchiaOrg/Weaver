import { Toaster as Sonner, type ToasterProps } from "sonner";

import { useResolvedTheme } from "@/components/theme-provider";

/** Toasts (`toast(...)` from `sonner`), themed with the tokens of the current theme. */
const Toaster = (props: ToasterProps) => {
  const theme = useResolvedTheme();
  return (
    <Sonner
      theme={theme}
      className="toaster group"
      toastOptions={{
        classNames: {
          toast: "group toast !bg-popover !text-popover-foreground !border-border !shadow-lg",
          description: "!text-muted-foreground",
        },
      }}
      {...props}
    />
  );
};

export { Toaster };
