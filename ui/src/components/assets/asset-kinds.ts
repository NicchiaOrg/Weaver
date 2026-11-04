import { File, FileArchive, FileAudio, FileCode, FileImage, FileSpreadsheet, FileText, FileType, FileVideo, type LucideIcon } from "lucide-react";

import type { Asset } from "@/bindings/Asset";

/** The kinds of file the Assets view tells apart, each with its own icon (`requirements/ui#asset-view`). */
export type AssetKind = "image" | "video" | "audio" | "archive" | "spreadsheet" | "document" | "code" | "font" | "other";

const EXTENSIONS: Record<Exclude<AssetKind, "other">, readonly string[]> = {
  image: ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "tif", "tiff", "heic", "avif"],
  video: ["mp4", "mov", "webm", "mkv", "avi", "m4v"],
  audio: ["mp3", "wav", "ogg", "flac", "m4a", "aac"],
  archive: ["zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar"],
  spreadsheet: ["csv", "tsv", "xls", "xlsx", "ods", "numbers"],
  document: ["txt", "md", "pdf", "doc", "docx", "rtf", "odt", "pages", "ppt", "pptx", "key"],
  code: ["json", "yaml", "yml", "toml", "xml", "html", "css", "js", "ts", "tsx", "jsx", "py", "rs", "go", "java", "c", "h", "cpp", "sh", "sql"],
  font: ["ttf", "otf", "woff", "woff2"],
};

const KIND_OF_EXTENSION: ReadonlyMap<string, AssetKind> = new Map(
  (Object.entries(EXTENSIONS) as [AssetKind, readonly string[]][]).flatMap(([kind, extensions]) => extensions.map((extension) => [extension, kind] as const)),
);

/** The icon of each kind (lucide-react). */
export const KIND_ICONS: Record<AssetKind, LucideIcon> = {
  image: FileImage,
  video: FileVideo,
  audio: FileAudio,
  archive: FileArchive,
  spreadsheet: FileSpreadsheet,
  document: FileText,
  code: FileCode,
  font: FileType,
  other: File,
};

/** The kind of the file `name`, by its extension without regard to case. */
export function kindOf(name: string): AssetKind {
  const dot = name.lastIndexOf(".");
  if (dot < 0) return "other";
  return KIND_OF_EXTENSION.get(name.slice(dot + 1).toLowerCase()) ?? "other";
}

/** The icon of the file `name`. */
export function iconOf(name: string): LucideIcon {
  return KIND_ICONS[kindOf(name)];
}

/** An asset is larger than the limit above 10 MB (10 × 1024 × 1024 bytes). It is only a warning and a mark. */
export const SIZE_LIMIT = 10 * 1024 * 1024;

export const isLarge = (size: number): boolean => size > SIZE_LIMIT;

/** The accessible name and tooltip of the mark on a large asset. */
export const LARGE_TEXT = "Larger than 10 MB: it goes into the repository.";

/** A size in `B`, `KB`, `MB` or `GB` (1 KB = 1,024 B), with one decimal from 1 KB on: `2.4 MB`. */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = 0;
  // A value that rounds up to 1024.0 belongs to the next unit.
  while (unit < units.length - 1 && Number(value.toFixed(1)) >= 1024) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(1)} ${units[unit]}`;
}

/** The tooltip of a tile: the full name and the size, `logo.png — 2.4 MB`. */
export const tooltipOf = (asset: Asset): string => `${asset.name} — ${formatSize(asset.size)}`;

/** The assets in the order of the grid: by name, without regard to case (as the backend lists them). */
export function inGridOrder(assets: readonly Asset[]): Asset[] {
  return [...assets].sort((a, b) => {
    const x = a.name.toLowerCase();
    const y = b.name.toLowerCase();
    if (x !== y) return x < y ? -1 : 1;
    return a.name < b.name ? -1 : a.name > b.name ? 1 : 0;
  });
}

/** The system's name as the window reports it (`navigator.platform`); empty outside a browser. */
const platformName = (): string => (typeof navigator === "undefined" ? "" : navigator.platform);

/** Whether the system is macOS, where `Cmd` takes the place of `Ctrl` (and `Backspace` the place of `Delete`). */
export const isMacPlatform = (platform: string = platformName()): boolean => /Mac|iPhone|iPad/.test(platform);

/**
 * What the empty view says (`requirements/ui#asset-view`): `the Finder or Explorer`, which the
 * system's own file manager takes the place of on Windows (`File Explorer`) and Linux (`your file
 * manager`).
 */
export function emptyText(platform: string = platformName()): string {
  const source = isMacPlatform(platform) ? "the Finder or Explorer" : /^Win/i.test(platform) ? "File Explorer" : "your file manager";
  return `No assets yet. Drag files here from ${source}.`;
}
