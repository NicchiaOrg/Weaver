import { File, FileArchive, FileAudio, FileCode, FileImage, FileSpreadsheet, FileText, FileType, FileVideo } from "lucide-react";
import { describe, expect, it } from "vitest";

import { emptyText, formatSize, iconOf, inGridOrder, isLarge, kindOf, SIZE_LIMIT, tooltipOf } from "@/components/assets/asset-kinds";

describe("the kind of an asset", () => {
  const table: [string, string[], unknown][] = [
    ["image", ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "tif", "tiff", "heic", "avif"], FileImage],
    ["video", ["mp4", "mov", "webm", "mkv", "avi", "m4v"], FileVideo],
    ["audio", ["mp3", "wav", "ogg", "flac", "m4a", "aac"], FileAudio],
    ["archive", ["zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar"], FileArchive],
    ["spreadsheet", ["csv", "tsv", "xls", "xlsx", "ods", "numbers"], FileSpreadsheet],
    ["document", ["txt", "md", "pdf", "doc", "docx", "rtf", "odt", "pages", "ppt", "pptx", "key"], FileText],
    ["code", ["json", "yaml", "yml", "toml", "xml", "html", "css", "js", "ts", "tsx", "jsx", "py", "rs", "go", "java", "c", "h", "cpp", "sh", "sql"], FileCode],
    ["font", ["ttf", "otf", "woff", "woff2"], FileType],
  ];

  it.each(table)("gives every %s extension its own icon", (kind, extensions, icon) => {
    for (const extension of extensions) {
      expect(kindOf(`file.${extension}`), extension).toBe(kind);
      expect(iconOf(`file.${extension}`), extension).toBe(icon);
    }
  });

  it("reads the extension without regard to case, from the last dot", () => {
    expect(kindOf("LOGO.PNG")).toBe("image");
    expect(kindOf("archive.tar.gz")).toBe("archive");
    expect(kindOf("Photo.JpG")).toBe("image");
  });

  it("gives anything else the plain file icon", () => {
    for (const name of ["README", "notes.unknown", "trailing.", "x.pngg", "png"]) {
      expect(kindOf(name), name).toBe("other");
      expect(iconOf(name), name).toBe(File);
    }
  });
});

describe("sizes", () => {
  it("writes B, KB, MB and GB with one decimal from 1 KB on, 1 KB being 1,024 B", () => {
    expect(formatSize(0)).toBe("0 B");
    expect(formatSize(1023)).toBe("1023 B");
    expect(formatSize(1024)).toBe("1.0 KB");
    expect(formatSize(1536)).toBe("1.5 KB");
    expect(formatSize(2.4 * 1024 * 1024)).toBe("2.4 MB");
    expect(formatSize(10 * 1024 * 1024)).toBe("10.0 MB");
    expect(formatSize(3 * 1024 ** 3)).toBe("3.0 GB");
    // A value that rounds up to 1024.0 of a unit is the next unit.
    expect(formatSize(1024 * 1024 - 1)).toBe("1.0 MB");
  });

  it("puts the name and the size in the tooltip", () => {
    expect(tooltipOf({ name: "logo.png", size: Math.round(2.4 * 1024 * 1024) })).toBe("logo.png — 2.4 MB");
  });

  it("calls an asset large above 10 MB, not at it", () => {
    expect(SIZE_LIMIT).toBe(10_485_760);
    expect(isLarge(SIZE_LIMIT)).toBe(false);
    expect(isLarge(SIZE_LIMIT + 1)).toBe(true);
  });
});

describe("the order of the grid", () => {
  it("is by name without regard to case, then by name", () => {
    const names = inGridOrder(["b.txt", "A.png", "a.png", "C.md"].map((name) => ({ name, size: 1 }))).map((asset) => asset.name);
    expect(names).toEqual(["A.png", "a.png", "b.txt", "C.md"]);
  });
});

describe("the text of the empty view", () => {
  it("names where the files come from on each system", () => {
    expect(emptyText("MacIntel")).toBe("No assets yet. Drag files here from the Finder or Explorer.");
    expect(emptyText("Win32")).toBe("No assets yet. Drag files here from File Explorer.");
    expect(emptyText("Linux x86_64")).toBe("No assets yet. Drag files here from your file manager.");
  });
});
