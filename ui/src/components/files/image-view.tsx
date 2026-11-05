import type { KeyboardEvent } from "react";

import { cn } from "@/lib/utils";

/** The squares under the image, from the theme's tokens, so that they hold in both themes. */
const CHECKERBOARD = {
  backgroundImage: "conic-gradient(var(--muted) 25%, var(--background) 0 50%, var(--muted) 0 75%, var(--background) 0)",
  backgroundSize: "16px 16px",
} as const;

export interface ImageViewProps {
  /** What the `<img>` draws: a `data:` URL, never a `blob:` or a file path. */
  src: string;
  /** The file's name. */
  name: string;
  /** One image pixel per CSS pixel (the panel scrolls both ways); otherwise fitted to the panel. */
  actualSize: boolean;
  onToggle: () => void;
  /** The image loaded: its natural size in pixels. */
  onLoad: (width: number, height: number) => void;
  /** The image could not be drawn. */
  onError: () => void;
}

/**
 * An image in the panel of a tab: centred, scaled down to fit and never enlarged, with the squares
 * of a checkerboard only under it. A click, or `Enter` or `Space` while it has focus, switches
 * between fitted and actual size; from actual size the panel scrolls in both directions.
 */
export function ImageView({ src, name, actualSize, onToggle, onLoad, onError }: ImageViewProps) {
  const onKeyDown = (event: KeyboardEvent<HTMLImageElement>) => {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    onToggle();
  };
  return (
    <div className="flex min-h-0 flex-1 overflow-auto p-4">
      {/* The image itself is the control: the accessible name says what a press does, the alt text is the file's name. */}
      <img
        src={src}
        alt={name}
        role="button"
        tabIndex={0}
        aria-label={actualSize ? "Fit to panel" : "Show actual size"}
        draggable={false}
        style={CHECKERBOARD}
        className={cn(
          "m-auto block shrink-0 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
          actualSize ? "max-w-none cursor-zoom-out" : "max-h-full max-w-full cursor-zoom-in object-contain",
        )}
        onClick={onToggle}
        onKeyDown={onKeyDown}
        onLoad={(event) => onLoad(event.currentTarget.naturalWidth, event.currentTarget.naturalHeight)}
        onError={onError}
      />
    </div>
  );
}
