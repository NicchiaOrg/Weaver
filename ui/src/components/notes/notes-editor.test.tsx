import type { Editor } from "@tiptap/core";
import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createNotesEditor, NotesSheet } from "@/components/notes/notes-editor";
import { ImageStore, type EditorEvents, type NotesPort } from "@/components/notes/notes-session";

const { backend, toast } = vi.hoisted(() => ({
  backend: { call: vi.fn(), on: vi.fn() },
  toast: { error: vi.fn(), warning: vi.fn() },
}));
vi.mock("sonner", () => ({ toast }));
vi.mock("@/lib/backend", () => ({
  call: backend.call,
  on: backend.on,
  isAppError: (value: unknown) => typeof value === "object" && value !== null && "code" in value && "message" in value,
}));

const events = (): EditorEvents => ({ edited: vi.fn(), ticked: vi.fn(), blurred: vi.fn(), imagesRemoved: vi.fn(), pasted: vi.fn() });

interface Harness {
  port: NotesPort;
  editor: Editor;
  events: EditorEvents;
  images: ImageStore;
}

let made: NotesPort[] = [];

function open(markdown: string, given: EditorEvents = events()): Harness {
  const images = new ImageStore("p");
  const port = createNotesEditor({ markdown, images, events: given });
  made.push(port);
  render(<NotesSheet port={port} />);
  return { port, editor: (port as unknown as { editor: Editor }).editor, events: given, images };
}

/** Types text the way the keyboard does: ProseMirror's text input handlers first (input rules), the plain insertion otherwise. */
function type(editor: Editor, text: string) {
  for (const char of text) {
    const { from, to } = editor.state.selection;
    const handled = editor.view.someProp("handleTextInput", (handler) => handler(editor.view, from, to, char, () => editor.state.tr.insertText(char, from, to)));
    if (!handled) editor.view.dispatch(editor.state.tr.insertText(char, from, to));
  }
}

/** A key pressed in the editor; `Mod` is `Ctrl` here (jsdom is not a Mac). */
function press(editor: Editor, key: string, modifiers: { mod?: boolean; shift?: boolean } = {}) {
  const event = new KeyboardEvent("keydown", { key, ctrlKey: modifiers.mod === true, shiftKey: modifiers.shift === true, bubbles: true, cancelable: true });
  return editor.view.someProp("handleKeyDown", (handler) => handler(editor.view, event)) === true;
}

/** Puts the cursor at the end of the first block that has `text`. */
function cursorAfter(editor: Editor, text: string) {
  let found = -1;
  editor.state.doc.descendants((node, pos) => {
    if (found < 0 && node.isText && node.text?.includes(text)) found = pos + (node.text.indexOf(text) + text.length);
  });
  expect(found).toBeGreaterThan(-1);
  editor.commands.setTextSelection(found);
}

beforeEach(() => {
  backend.on.mockImplementation(() => Promise.resolve(() => {}));
  backend.call.mockReset().mockImplementation(() => Promise.reject(new Error("no such image")));
});

afterEach(() => {
  for (const port of made) port.destroy();
  made = [];
});

/** Every supported element, written as the editor writes it. */
const SUPPORTED: Record<string, string> = {
  paragraphs: "First paragraph\n\nSecond paragraph\n",
  headings: "# One\n\n## Two\n\n### Three\n",
  bullets: "- a\n- b\n  - c\n    - d\n",
  numbered: "1. a\n2. b\n   1. c\n",
  checklist: "- [ ] a\n- [x] b\n  - [ ] c\n",
  mixedNesting: "- a\n  - [ ] b\n  1. c\n",
  marks: "**bold** *italic* ~~strike~~ `code` and ***both***\n",
  fenced: "```js\nconst a = 1;\n\nconst b = 2;\n```\n",
  quote: "> quoted\n> text\n",
  link: "[Weaver](https://example.com/a?b=c)\n",
  divider: "above\n\n---\n\nbelow\n",
  image: "![](<picture.png>)\n\n![](<with space.png>)\n\n![](<https://example.com/remote.png>)\n",
  empty: "",
};

describe("loading then saving", () => {
  for (const [name, markdown] of Object.entries(SUPPORTED)) {
    it(`gives back the same text for ${name}`, () => {
      const { port } = open(markdown);
      expect(port.getMarkdown()).toBe(markdown);
    });
  }
});

/** Blocks the sheet cannot show: kept as plain text, written back byte for byte. */
const UNSUPPORTED: Record<string, string> = {
  "raw HTML": "before\n\n<div class=\"x\">\n  <b>raw</b>\n</div>\n\nafter\n",
  "an HTML comment": "<!-- a note\n\nacross blank lines -->\n\ntext\n",
  table: "| a | b |\n| --- | --- |\n| 1 | 2 |\n",
  footnote: "text[^1]\n\n[^1]: the note\n",
  "front matter": "---\ntitle: Notes\ntags: [a, b]\n---\n\n# Heading\n",
  "conflict markers": "<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> branch\n\nafter\n",
  "a heading deeper than level 3": "#### Deep\n\ntext\n",
};

describe("content the sheet cannot show", () => {
  for (const [name, markdown] of Object.entries(UNSUPPORTED)) {
    it(`writes ${name} back byte for byte`, () => {
      const { port, editor } = open(markdown);
      expect(port.getMarkdown()).toBe(markdown);
      expect(editor.getJSON().content?.some((node) => node.type === "rawMarkdown")).toBe(true);
    });
  }

  it("holds the source as plain, editable text", () => {
    const { port, editor } = open("<div>raw</div>\n");
    cursorAfter(editor, "<div>raw");
    type(editor, "!");
    expect(port.getMarkdown()).toBe("<div>raw!</div>\n");
  });

  it("leaves unknown inline syntax as text, written back as it is", () => {
    const markdown = "a <kbd>K</kbd> b<br>c and a note[^1] here\n\n[^1]: the note\n";
    const { port, editor } = open(markdown);
    expect(editor.getText()).toContain("a <kbd>K</kbd> b<br>c and a note[^1] here");
    expect(port.getMarkdown()).toBe(markdown);
  });
});

describe("Markdown shortcuts", () => {
  const typed = (keys: string) => {
    const { port, editor } = open("");
    editor.commands.focus();
    type(editor, keys);
    return { port, editor };
  };

  it.each([
    ["# ", "# Title\n"],
    ["## ", "## Title\n"],
    ["### ", "### Title\n"],
    ["- ", "- Title\n"],
    ["* ", "- Title\n"],
    ["+ ", "- Title\n"],
    ["1. ", "1. Title\n"],
    ["[] ", "- [ ] Title\n"],
    ["[ ] ", "- [ ] Title\n"],
    ["[x] ", "- [x] Title\n"],
    ["> ", "> Title\n"],
  ])("turns `%s` at the start of an empty line into its element and removes the marker", (marker, written) => {
    const { port } = typed(`${marker}Title`);
    expect(port.getMarkdown()).toBe(written);
  });

  it.each([
    ["- [ ] ", "- [ ] Title\n"],
    ["- [x] ", "- [x] Title\n"],
    ["- [] ", "- [ ] Title\n"],
  ])("turns `%s` into a checklist item, not a bullet with a list inside", (marker, written) => {
    const { port } = typed(`${marker}Title`);
    expect(port.getMarkdown()).toBe(written);
  });

  it("does not turn a marker typed after text", () => {
    const { editor } = typed("text # ");
    expect(editor.getJSON().content?.map((node) => node.type)).toEqual(["paragraph"]);
    expect(editor.getText()).toBe("text # ");
  });

  it("starts a code block with three backticks and a space", () => {
    const { port } = typed("``` code");
    expect(port.getMarkdown()).toBe("```\ncode\n```\n");
  });

  it("starts a code block with three backticks and Enter", () => {
    const { port, editor } = typed("```");
    press(editor, "Enter");
    type(editor, "code");
    expect(port.getMarkdown()).toBe("```\ncode\n```\n");
  });

  it("makes a divider with three dashes and Enter, and not before", () => {
    const { port, editor } = typed("---");
    expect(editor.getJSON().content?.map((node) => node.type)).toEqual(["paragraph"]);
    press(editor, "Enter");
    expect(editor.getJSON().content?.some((node) => node.type === "horizontalRule")).toBe(true);
    type(editor, "after");
    expect(port.getMarkdown()).toBe("---\n\nafter\n");
  });

  it.each([
    ["**bold**", "**bold**"],
    ["*italic*", "*italic*"],
    ["_italic_", "*italic*"],
    ["~~strike~~", "~~strike~~"],
    ["`code`", "`code`"],
  ])("turns %s into formatting as soon as the closing marker is typed", (keys, written) => {
    const { port } = typed(`before ${keys}`);
    expect(port.getMarkdown()).toBe(`before ${written}\n`);
  });

  it.each([
    ["b", { mod: true }, "**word**"],
    ["i", { mod: true }, "*word*"],
    ["s", { mod: true, shift: true }, "~~word~~"],
    ["e", { mod: true }, "`word`"],
  ])("formats the selection with Ctrl+%s", (key, modifiers, written) => {
    const { port, editor } = open("word\n");
    editor.commands.selectAll();
    expect(press(editor, key, modifiers)).toBe(true);
    expect(port.getMarkdown()).toBe(`${written}\n`);
  });

  it("has no underline", () => {
    const { editor } = open("word\n");
    editor.commands.selectAll();
    press(editor, "u", { mod: true });
    expect(editor.getHTML()).not.toContain("<u>");
  });

  it("turns a heading back into a paragraph with Backspace at its start", () => {
    const { port, editor } = open("## Title\n");
    editor.commands.setTextSelection(1);
    expect(press(editor, "Backspace")).toBe(true);
    expect(port.getMarkdown()).toBe("Title\n");
  });

  it("turns a heading after a paragraph back into a paragraph, not into joined text", () => {
    const { port, editor } = open("above\n\n# Title\n");
    cursorAfter(editor, "above");
    editor.commands.setTextSelection(editor.state.doc.child(0).nodeSize + 1);
    press(editor, "Backspace");
    expect(port.getMarkdown()).toBe("above\n\nTitle\n");
  });

  it("turns a quote back into a paragraph with Backspace at its start", () => {
    const { port, editor } = open("> quoted\n");
    editor.commands.setTextSelection(2);
    press(editor, "Backspace");
    expect(port.getMarkdown()).toBe("quoted\n");
  });

  it.each([
    ["- item\n", "item\n"],
    ["1. item\n", "item\n"],
    ["- [ ] item\n", "item\n"],
  ])("turns an item of %j back into a paragraph with Backspace at its start", (markdown, written) => {
    const { port, editor } = open(markdown);
    editor.commands.focus("start");
    press(editor, "Backspace");
    expect(port.getMarkdown()).toBe(written);
  });
});

describe("lists", () => {
  it("continues a checklist with an unticked item on Enter", () => {
    const { port, editor } = open("- [x] done\n");
    cursorAfter(editor, "done");
    press(editor, "Enter");
    type(editor, "next");
    expect(port.getMarkdown()).toBe("- [x] done\n- [ ] next\n");
  });

  it.each([
    ["- a\n", "- a\n- b\n"],
    ["1. a\n", "1. a\n2. b\n"],
  ])("starts an item of the same kind below on Enter in %j", (markdown, written) => {
    const { port, editor } = open(markdown);
    cursorAfter(editor, "a");
    press(editor, "Enter");
    type(editor, "b");
    expect(port.getMarkdown()).toBe(written);
  });

  it.each([
    ["- [ ] a\n", "- [ ] a\n\n"],
    ["- a\n", "- a\n\n"],
  ])("ends the list on Enter in an empty item of %j", (markdown, before) => {
    const { port, editor } = open(markdown);
    cursorAfter(editor, "a");
    press(editor, "Enter");
    press(editor, "Enter");
    type(editor, "after");
    expect(port.getMarkdown()).toBe(`${before.trimEnd()}\n\nafter\n`);
  });

  it("nests an item under the one above with Tab, and takes it out with Shift+Tab", () => {
    const { port, editor } = open("- [ ] a\n- [ ] b\n");
    cursorAfter(editor, "b");
    expect(press(editor, "Tab")).toBe(true);
    expect(port.getMarkdown()).toBe("- [ ] a\n  - [ ] b\n");
    expect(press(editor, "Tab", { shift: true })).toBe(true);
    expect(port.getMarkdown()).toBe("- [ ] a\n- [ ] b\n");
  });

  it("nests bulleted and numbered items the same way", () => {
    const { port, editor } = open("- a\n- b\n\n1. c\n2. d\n");
    cursorAfter(editor, "b");
    press(editor, "Tab");
    cursorAfter(editor, "d");
    press(editor, "Tab");
    expect(port.getMarkdown()).toBe("- a\n  - b\n\n1. c\n   1. d\n");
  });
});

describe("ticking", () => {
  const box = (name: string) => screen.findByRole("checkbox", { name });

  it("shows a checkbox with the item's text as its name and its state", async () => {
    open("- [ ] open\n- [x] shut\n");
    expect(await box("open")).toHaveAttribute("aria-checked", "false");
    expect(await box("shut")).toHaveAttribute("aria-checked", "true");
  });

  it("ticks with a click: one transaction, only that marker changes, the cursor stays and the sheet is written at once", async () => {
    const { port, editor, events: seen } = open("- [ ] a\n- [ ] b\n\ntext\n");
    cursorAfter(editor, "text");
    const selection = editor.state.selection.from;
    const transactions: number[] = [];
    editor.on("transaction", ({ transaction }) => transactions.push(transaction.steps.length));
    fireEvent.click(await box("b"));
    expect(port.getMarkdown()).toBe("- [ ] a\n- [x] b\n\ntext\n");
    expect(transactions).toHaveLength(1);
    expect(editor.state.selection.from).toBe(selection);
    expect(seen.ticked).toHaveBeenCalledTimes(1);
    expect(seen.edited).not.toHaveBeenCalled();
    fireEvent.click(await box("b"));
    expect(port.getMarkdown()).toBe("- [ ] a\n- [ ] b\n\ntext\n");
  });

  it("ticks the item that holds the cursor with Ctrl+Enter, and unticks it again", () => {
    const { port, editor, events: seen } = open("- [ ] a\n  - [ ] nested\n");
    cursorAfter(editor, "nested");
    expect(press(editor, "Enter", { mod: true })).toBe(true);
    expect(port.getMarkdown()).toBe("- [ ] a\n  - [x] nested\n");
    expect(seen.ticked).toHaveBeenCalledTimes(1);
    press(editor, "Enter", { mod: true });
    expect(port.getMarkdown()).toBe("- [ ] a\n  - [ ] nested\n");
  });

  it("keeps a ticked item in its place, struck through by its data attribute", async () => {
    open("- [ ] a\n- [x] b\n- [ ] c\n");
    await box("a");
    const items = screen.getAllByRole("listitem");
    expect(items.map((item) => item.getAttribute("data-checked"))).toEqual(["false", "true", "false"]);
    fireEvent.click(await box("a"));
    expect(screen.getAllByRole("listitem").map((item) => item.getAttribute("data-checked"))).toEqual(["true", "true", "false"]);
  });
});

describe("undo and redo", () => {
  it("undo and redo with Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y", () => {
    const { port, editor } = open("a\n");
    cursorAfter(editor, "a");
    type(editor, "b");
    expect(port.getMarkdown()).toBe("ab\n");
    press(editor, "z", { mod: true });
    expect(port.getMarkdown()).toBe("a\n");
    press(editor, "z", { mod: true, shift: true });
    expect(port.getMarkdown()).toBe("ab\n");
    press(editor, "z", { mod: true });
    press(editor, "y", { mod: true });
    expect(port.getMarkdown()).toBe("ab\n");
  });
});

describe("what the developer's edits report", () => {
  it("reports an edit for typing, and nothing for a replacement from disk", () => {
    const { port, editor, events: seen } = open("a\n");
    cursorAfter(editor, "a");
    type(editor, "b");
    expect(seen.edited).toHaveBeenCalledTimes(1);
    port.replaceFromDisk("changed\n");
    expect(seen.edited).toHaveBeenCalledTimes(1);
    expect(port.getMarkdown()).toBe("changed\n");
  });

  it("reports the editor losing the focus", () => {
    const { events: seen } = open("a\n");
    const text = screen.getByRole("textbox", { name: "Notes" });
    fireEvent.focus(text);
    fireEvent.blur(text);
    expect(seen.blurred).toHaveBeenCalled();
  });

  it("collects the images that leave the document through undo and redo, not through a reload", () => {
    const { port, editor, events: seen } = open("![](<a.png>)\n\n![](<b.png>)\n");
    // The developer removes the second image.
    editor.view.dispatch(editor.state.tr.delete(editor.state.doc.child(0).nodeSize, editor.state.doc.content.size));
    expect(seen.imagesRemoved).toHaveBeenLastCalledWith(["b.png"]);
    // Undo brings it back; redo removes it again: a second removal is reported.
    press(editor, "z", { mod: true });
    expect(port.getMarkdown()).toContain("b.png");
    press(editor, "z", { mod: true, shift: true });
    expect(seen.imagesRemoved).toHaveBeenCalledTimes(2);
    // A reload from disk that takes the images away is not the developer's.
    port.replaceFromDisk("text only\n");
    expect(seen.imagesRemoved).toHaveBeenCalledTimes(2);
    // The same for an undo of the developer's earlier edit that removes an image he added.
    expect(port.getMarkdown()).toBe("text only\n");
  });
});

describe("an outside change", () => {
  it("replaces the content in one transaction that keeps the cursor where the text around it exists", () => {
    const { port, editor } = open("first\n\nsecond\n");
    cursorAfter(editor, "second");
    const transactions: number[] = [];
    editor.on("transaction", () => transactions.push(1));
    port.replaceFromDisk("new line\n\nfirst\n\nsecond\n");
    expect(transactions).toHaveLength(1);
    expect(port.getMarkdown()).toBe("new line\n\nfirst\n\nsecond\n");
    const { $from } = editor.state.selection;
    expect($from.parent.textContent).toBe("second");
    expect($from.parentOffset).toBe("second".length);
  });

  it("shows an empty sheet when the file is gone", () => {
    const { port, editor } = open("text\n");
    port.replaceFromDisk("");
    expect(port.getMarkdown()).toBe("");
    expect(editor.isEmpty).toBe(true);
  });

  it("does nothing when the text is the same", () => {
    const { port, editor } = open("text\n");
    const transactions: number[] = [];
    editor.on("transaction", () => transactions.push(1));
    port.replaceFromDisk("text\n");
    expect(transactions).toHaveLength(0);
  });
});

describe("the empty sheet", () => {
  it("shows the first line in muted text that is not part of the text, and drops it with the first character", () => {
    const { port, editor } = open("");
    const first = document.querySelector(".ProseMirror p");
    expect(first).toHaveAttribute("data-placeholder", "Write notes for this project. Start a line with [ ] for a checklist.");
    expect(editor.getText()).toBe("");
    editor.commands.focus();
    type(editor, "x");
    expect(document.querySelector(".ProseMirror p")).not.toHaveAttribute("data-placeholder", expect.stringContaining("Write notes"));
    expect(port.getMarkdown()).toBe("x\n");
  });
});

describe("the editor", () => {
  it("makes a line to write in when a click below the last block finds an image", () => {
    const { port, editor } = open("![](<pic.png>)\n");
    const column = document.querySelector(".ProseMirror")?.parentElement?.parentElement?.parentElement as Element;
    fireEvent.mouseDown(column);
    type(editor, "after");
    expect(port.getMarkdown()).toBe("![](<pic.png>)\n\nafter\n");
  });

  it("is a textbox with aria-multiline and the name Notes", () => {
    open("a\n");
    const box = screen.getByRole("textbox", { name: "Notes" });
    expect(box).toHaveAttribute("aria-multiline", "true");
    expect(box).toHaveAttribute("contenteditable", "true");
  });

  it("puts the cursor at the end with a click below the last block", () => {
    const { editor } = open("one\n\ntwo\n");
    editor.commands.focus("start");
    const column = document.querySelector(".ProseMirror")?.parentElement?.parentElement?.parentElement;
    expect(column).toBeTruthy();
    fireEvent.mouseDown(column as Element);
    expect(editor.state.selection.from).toBe(editor.state.doc.content.size - 1);
  });
});

describe("images in the sheet", () => {
  const pictures = async (count: number) => {
    await vi.waitFor(() => expect(document.querySelectorAll("img")).toHaveLength(count));
    return [...document.querySelectorAll("img")];
  };
  const read = (name: string) => backend.call.mock.calls.filter(([command, args]) => command === "notes_image" && (args as { name: string }).name === name);

  it("shows an image of the notes folder at its natural size, read once", async () => {
    backend.call.mockImplementation((command: string) => (command === "notes_image" ? Promise.resolve("data:image/png;base64,AAAA") : Promise.reject(new Error("unexpected"))));
    const { images } = open("![](<pic.png>)\n\n![](<pic.png>)\n");
    const shown = await pictures(2);
    expect(shown[0]).toHaveAttribute("src", "data:image/png;base64,AAAA");
    expect(read("pic.png")).toHaveLength(1);
    expect(images.peek("pic.png")).toEqual({ state: "ready", url: "data:image/png;base64,AAAA" });
  });

  it("shows the placeholder, with the name, for an image that is missing", async () => {
    open("![](<gone.png>)\n");
    expect(await screen.findByText("gone.png")).toBeInTheDocument();
    expect(document.querySelector("[data-image-placeholder]")).toHaveClass("h-12", "w-full", "border-dashed", "bg-muted");
    expect(document.querySelector("[data-image-placeholder] svg")).toHaveClass("lucide-image-off");
    expect(document.querySelector("img")).toBeNull();
  });

  it.each(["https://example.com/a.png", "http://example.com/a.png", "data:image/png;base64,AAAA", "sub/pic.png", "../pic.png", "/abs/pic.png", ".hidden.png", "notes.md"])(
    "never reads nor fetches %s: it shows the placeholder with its address",
    async (destination) => {
      const { editor } = open(`![](<${destination}>)\n`);
      expect(await screen.findByText(destination)).toBeInTheDocument();
      expect(backend.call).not.toHaveBeenCalledWith("notes_image", expect.anything());
      expect(document.querySelector("img")).toBeNull();
      expect(editor.getJSON().content?.[0]?.attrs?.src).toBe(destination);
    },
  );

  it("shows the placeholder when the image fails to load", async () => {
    backend.call.mockImplementation(() => Promise.resolve("data:image/png;base64,AAAA"));
    open("![](<broken.png>)\n");
    fireEvent.error((await pictures(1))[0] as Element);
    expect(await screen.findByText("broken.png")).toBeInTheDocument();
  });

  it("reads an image again only after it is invalidated", async () => {
    backend.call.mockImplementation(() => Promise.resolve("data:image/png;base64,AAAA"));
    const { images } = open("![](<pic.png>)\n");
    await pictures(1);
    images.invalidate(["pic.png"]);
    await vi.waitFor(() => expect(read("pic.png")).toHaveLength(2));
    await pictures(1);
    expect(read("pic.png")).toHaveLength(2);
  });

  it("outlines a selected image with the ring and removes it with Backspace", async () => {
    backend.call.mockImplementation(() => Promise.resolve("data:image/png;base64,AAAA"));
    const { port, editor, events: seen } = open("![](<pic.png>)\n\ntext\n");
    const [image] = await pictures(1);
    editor.commands.setNodeSelection(0);
    await vi.waitFor(() => expect(image).toHaveClass("outline-ring", "outline-2"));
    expect(image?.closest("[data-selected]")).not.toBeNull();
    expect(press(editor, "Backspace")).toBe(true);
    expect(port.getMarkdown()).toBe("text\n");
    expect(seen.imagesRemoved).toHaveBeenCalledWith(["pic.png"]);
  });

  it("writes an image as ![](<name>) at the cursor, in the order given, in place of an empty line", () => {
    const { port, editor } = open("above\n\n");
    editor.commands.focus("end");
    const anchor = port.track();
    anchor.insert(["one.png", "two words.png"]);
    anchor.release();
    expect(port.getMarkdown()).toBe("above\n\n![](<one.png>)\n\n![](<two words.png>)\n");
  });

  it("inserts beside the block that holds the cursor, and follows the text when it changes meanwhile", () => {
    const { port, editor } = open("first\n\nsecond\n");
    cursorAfter(editor, "first");
    const anchor = port.track();
    editor.commands.setTextSelection(editor.state.doc.content.size - 1);
    type(editor, "!");
    cursorAfter(editor, "fir");
    editor.commands.insertContentAt(1, "xx");
    anchor.insert(["pic.png"]);
    anchor.release();
    expect(port.getMarkdown()).toBe("xxfirst\n\n![](<pic.png>)\n\nsecond!\n");
  });
});

describe("links", () => {
  const link = () => document.querySelector("a") as HTMLAnchorElement;

  it("turns an address typed and followed by a space into a link, in the primary blue style", () => {
    const { port, editor } = open("");
    editor.commands.focus();
    type(editor, "see https://example.com now");
    expect(port.getMarkdown()).toBe("see [https://example.com](https://example.com) now\n");
  });

  it("does not link a scheme other than http and https", () => {
    const { editor } = open("");
    editor.commands.focus();
    type(editor, "see ftp://example.com now");
    expect(editor.getHTML()).not.toContain("<a");
  });

  it("shows the address and how to open it as the tooltip", () => {
    open("[Weaver](https://example.com)\n");
    fireEvent.mouseOver(link());
    expect(link().title).toBe("https://example.com\nCtrl+click to open");
  });

  it("puts the cursor in the link on a click, and opens it only with Ctrl+click, in the system browser", () => {
    backend.call.mockImplementation(() => Promise.resolve(null));
    open("[Weaver](https://example.com)\n");
    fireEvent.click(link());
    expect(backend.call).not.toHaveBeenCalledWith("open_url", expect.anything());
    fireEvent.click(link(), { ctrlKey: true });
    expect(backend.call).toHaveBeenCalledWith("open_url", { url: "https://example.com" });
  });

  it("opens nothing for a link that is not http or https", () => {
    open("[Mail](mailto:a@b.c)\n");
    fireEvent.click(link(), { ctrlKey: true });
    expect(backend.call).not.toHaveBeenCalled();
  });
});

describe("pasting", () => {
  const clipboard = (data: Record<string, string>, files: File[] = []) =>
    ({ clipboardData: { files, getData: (type: string) => data[type] ?? "" }, preventDefault: vi.fn() }) as unknown as ClipboardEvent;
  const paste = (editor: Editor, event: ClipboardEvent) => editor.view.someProp("handlePaste", (handler) => handler(editor.view, event, editor.state.doc.slice(0, 0))) === true;

  it("reads pasted text as Markdown", () => {
    const { port, editor } = open("");
    editor.commands.focus();
    expect(paste(editor, clipboard({ "text/plain": "# Title\n\n- [ ] one\n- [x] two\n\n**bold** text" }))).toBe(true);
    expect(port.getMarkdown()).toBe("# Title\n\n- [ ] one\n- [x] two\n\n**bold** text\n");
  });

  it("leaves a single plain line to the editor", () => {
    const { editor } = open("");
    editor.commands.focus();
    expect(paste(editor, clipboard({ "text/plain": "just words" }))).toBe(false);
  });

  it("turns an http or https address pasted alone into a link", () => {
    const { port, editor } = open("");
    editor.commands.focus();
    expect(paste(editor, clipboard({ "text/plain": "https://example.com/x" }))).toBe(true);
    expect(port.getMarkdown()).toBe("[https://example.com/x](https://example.com/x)\n");
  });

  it("leaves formatted text to the editor's own paste, which drops what the sheet cannot show", () => {
    const { editor } = open("");
    editor.commands.focus();
    expect(paste(editor, clipboard({ "text/html": "<p>a</p>", "text/plain": "a" }))).toBe(false);
  });

  it("hands pasted image files over with an anchor at the cursor, and keeps the text unchanged", () => {
    const { port, editor, events: seen } = open("text\n");
    editor.commands.focus("end");
    const file = new File([new Uint8Array([1])], "shot.png", { type: "image/png" });
    const event = clipboard({}, [file]);
    expect(paste(editor, event)).toBe(true);
    expect(event.preventDefault).toHaveBeenCalled();
    expect(seen.pasted).toHaveBeenCalledWith([file], expect.objectContaining({ insert: expect.any(Function) }));
    expect(port.getMarkdown()).toBe("text\n");
  });
});
