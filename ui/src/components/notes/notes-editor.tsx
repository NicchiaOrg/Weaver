// The Notes view's editor: TipTap 3 on ProseMirror, reading the sheet as GFM Markdown and writing it
// back (`requirements/tech#notes-view`, "Editor"). This is the module the view loads with one
// `import()` the first time it opens in a window; nothing else in the UI imports TipTap.
import { createDocument, Editor } from "@tiptap/core";
import { TaskList } from "@tiptap/extension-list";
import { Placeholder } from "@tiptap/extension-placeholder";
import { Markdown } from "@tiptap/markdown";
import { Fragment, Slice } from "@tiptap/pm/model";
import type { Transaction } from "@tiptap/pm/state";
import { insertPoint } from "@tiptap/pm/transform";
import { EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { marked, Tokenizer, type Tokens } from "marked";
import type { JSONContent } from "@tiptap/core";
import type { EditorView } from "@tiptap/pm/view";

import { openInBrowser, isWebUrl } from "@/components/markdown";
import { EMPTY_TEXT, isModifier, linkTooltip } from "@/components/notes/notes-model";
import {
  BlockKeys,
  EXTERNAL_META,
  NotesHorizontalRule,
  NotesImage,
  NotesTaskItem,
  RawInline,
  RawMarkdown,
  rawInlineToken,
  rawToken,
  TICK_META,
} from "@/components/notes/notes-extensions";
import type { DropPoint, EditorEvents, ImageStore, InsertAnchor, NotesPort } from "@/components/notes/notes-session";

let configured = false;

/**
 * Makes marked hand the blocks no extension maps to `rawMarkdown` (raw HTML, tables, link and
 * footnote definitions, headings deeper than level 3), and leaves inline HTML as the text it is (`rawInline`).
 */
function configureMarked(): void {
  if (configured) return;
  configured = true;
  const proto = Tokenizer.prototype;
  marked.use({
    tokenizer: {
      html(src) {
        const token = proto.html.call(this, src);
        return token && (rawToken(token.raw) as unknown as Tokens.HTML);
      },
      table(src) {
        const token = proto.table.call(this, src);
        return token && ({ ...token, ...rawToken(token.raw) } as unknown as Tokens.Table);
      },
      def(src) {
        const token = proto.def.call(this, src);
        return token && ({ ...token, ...rawToken(token.raw) } as unknown as Tokens.Def);
      },
      heading(src) {
        const token = proto.heading.call(this, src);
        return token && token.depth > 3 ? (rawToken(token.raw) as unknown as Tokens.Heading) : token;
      },
      tag(src) {
        const token = proto.tag.call(this, src);
        return token && (rawInlineToken(token.raw) as unknown as Tokens.Tag);
      },
    },
  });
}

/** The editor and what the view needs of it beyond the port. */
interface EditorPort extends NotesPort {
  editor: Editor;
}

const EMPTY_DOC: JSONContent = { type: "doc", content: [{ type: "paragraph" }] };

/** The sheet as the editor writes it: Markdown with one final newline, nothing for an empty sheet. */
export function writeMarkdown(editor: Editor): string {
  const markdown = editor.getMarkdown().replace(/\n+$/, "");
  return markdown === "" ? "" : `${markdown}\n`;
}

/** The document a text reads as, never without a block. */
function readMarkdown(editor: Editor, markdown: string): JSONContent {
  const parsed = editor.markdown?.parse(markdown) ?? EMPTY_DOC;
  return parsed.content?.length ? parsed : EMPTY_DOC;
}

/** The images that the steps of a transaction took out of the document. */
function removedImages(transaction: Transaction): string[] {
  const names: string[] = [];
  transaction.steps.forEach((step, index) => {
    const before = transaction.docs[index];
    if (!before) return;
    step.getMap().forEach((from, to) => {
      before.nodesBetween(from, to, (node) => {
        if (node.type.name === "image") names.push(String(node.attrs.src));
      });
    });
  });
  return names;
}

/** Whether pasted text is a single plain line of text: the editor's own paste handles it (and makes a link of an address). */
function isPlainParagraph(doc: JSONContent): boolean {
  const [only] = doc.content ?? [];
  return doc.content?.length === 1 && only?.type === "paragraph" && (only.content ?? []).every((node) => node.type === "text" && !node.marks);
}

export function createNotesEditor({ markdown, images, events }: { markdown: string; images: ImageStore; events: EditorEvents }): NotesPort {
  configureMarked();
  const anchors = new Set<{ pos: number }>();

  const editor: Editor = new Editor({
    extensions: [
      StarterKit.configure({
        underline: false,
        trailingNode: false,
        horizontalRule: false,
        heading: { levels: [1, 2, 3] },
        dropcursor: { color: "var(--primary)", width: 2 },
        link: { openOnClick: false, autolink: true, linkOnPaste: true, protocols: ["http", "https"], isAllowedUri: (url) => isWebUrl(url) },
      }),
      NotesHorizontalRule,
      TaskList,
      NotesTaskItem.configure({ nested: true }),
      NotesImage.configure({ images }),
      RawMarkdown,
      RawInline,
      BlockKeys,
      Placeholder.configure({ placeholder: ({ editor: current }) => (current.isEmpty ? EMPTY_TEXT : ""), showOnlyCurrent: false }),
      Markdown,
    ],
    content: markdown,
    contentType: "markdown",
    editorProps: {
      attributes: { role: "textbox", "aria-multiline": "true", "aria-label": "Notes", class: "notes-editor" },
      handlePaste: (view, event) => paste(view, event),
      handleDOMEvents: {
        click: (_view, event) => {
          const link = event.target instanceof Element ? event.target.closest("a") : null;
          if (!link) return false;
          // A click puts the cursor in the link; `Cmd`/`Ctrl` + click opens it, as `http` or `https` only.
          event.preventDefault();
          const href = link.getAttribute("href") ?? "";
          if (isModifier(event) && isWebUrl(href)) openInBrowser(href);
          return false;
        },
        mouseover: (_view, event) => {
          const link = event.target instanceof Element ? event.target.closest("a") : null;
          const href = link?.getAttribute("href");
          if (link && href) link.title = linkTooltip(href);
          return false;
        },
      },
    },
  });

  function paste(view: EditorView, event: ClipboardEvent): boolean {
    const data = event.clipboardData;
    if (!data) return false;
    const files = [...data.files];
    if (files.length > 0) {
      event.preventDefault();
      events.pasted(files, track());
      return true;
    }
    // Formatted text keeps the elements of the sheet and drops the rest (the schema decides); text alone is Markdown.
    const text = data.getData("text/plain");
    if (data.getData("text/html") !== "" || text === "") return false;
    const parsed = readMarkdown(editor, text);
    if (isPlainParagraph(parsed)) return false;
    const doc = createDocument(parsed, editor.schema);
    view.dispatch(view.state.tr.replaceSelection(Slice.maxOpen(doc.content)).scrollIntoView().setMeta("paste", true).setMeta("uiEvent", "paste"));
    event.preventDefault();
    return true;
  }

  editor.on("transaction", ({ transaction }) => {
    if (!transaction.docChanged) return;
    for (const anchor of anchors) anchor.pos = transaction.mapping.map(anchor.pos);
    if (transaction.getMeta(EXTERNAL_META)) return;
    const removed = removedImages(transaction);
    if (removed.length > 0) events.imagesRemoved(removed);
    if (transaction.getMeta(TICK_META)) events.ticked();
    else events.edited();
  });
  editor.on("blur", () => events.blurred());

  function track(pos?: number): InsertAnchor {
    const anchor = { pos: pos ?? editor.state.selection.to };
    anchors.add(anchor);
    return {
      insert(names) {
        const type = editor.schema.nodes.image;
        if (!type) return;
        const nodes = names.map((src) => type.create({ src }));
        const { doc } = editor.state;
        const at = Math.min(anchor.pos, doc.content.size);
        const $at = doc.resolve(at);
        const tr = editor.state.tr;
        // A cursor in an empty line: the images take the line's place; otherwise they go beside the block that holds it.
        if ($at.parent.isTextblock && $at.parent.content.size === 0) tr.replaceWith($at.before(), $at.after(), nodes);
        else tr.insert(insertPoint(doc, at, type) ?? doc.content.size, Fragment.from(nodes));
        editor.view.dispatch(tr.scrollIntoView());
        editor.commands.focus(null, { scrollIntoView: false });
      },
      release() {
        anchors.delete(anchor);
      },
    };
  }

  const port: EditorPort = {
    editor,
    getMarkdown: () => writeMarkdown(editor),
    replaceFromDisk(text) {
      const next = createDocument(readMarkdown(editor, text), editor.schema);
      const current = editor.state.doc;
      const start = current.content.findDiffStart(next.content);
      const ends = current.content.findDiffEnd(next.content);
      if (start === null || !ends) return;
      // The one transaction replaces only what differs, so the cursor stays where the text around it still is.
      let { a: endCurrent, b: endNext } = ends;
      const overlap = start - Math.min(endCurrent, endNext);
      if (overlap > 0) {
        endCurrent += overlap;
        endNext += overlap;
      }
      editor.view.dispatch(editor.state.tr.replace(start, endCurrent, next.slice(start, endNext)).setMeta(EXTERNAL_META, true).setMeta("addToHistory", false));
    },
    focus: () => void editor.commands.focus(null, { scrollIntoView: false }),
    track,
    dropPoint(x, y): DropPoint | null {
      const type = editor.schema.nodes.image;
      const hit = editor.view.posAtCoords({ left: x, top: y });
      if (!hit || !type) return null;
      const { doc } = editor.state;
      const pos = insertPoint(doc, hit.pos, type) ?? doc.content.size;
      const $pos = doc.resolve(pos);
      const sheet = editor.view.dom.getBoundingClientRect();
      const next = $pos.nodeAfter && editor.view.nodeDOM(pos);
      const previous = $pos.nodeBefore && editor.view.nodeDOM(pos - $pos.nodeBefore.nodeSize);
      const top = next instanceof Element ? next.getBoundingClientRect().top : previous instanceof Element ? previous.getBoundingClientRect().bottom : sheet.top;
      return { pos, line: { left: sheet.left, top, width: sheet.width } };
    },
    destroy: () => editor.destroy(),
  };
  return port;
}

/** The editor's page: the column that takes every click, with the editor in it. */
export function NotesSheet({ port }: { port: NotesPort }) {
  const { editor } = port as EditorPort;
  return (
    <div
      className="min-h-full cursor-text"
      onMouseDown={(event) => {
        // A click anywhere in the column but on the text puts the cursor at the end.
        if (event.target instanceof Element && event.target.closest(".ProseMirror")) return;
        event.preventDefault();
        // After an image or a divider there is no line to put the cursor in: one is made.
        const { doc } = editor.state;
        if (doc.lastChild?.isAtom) editor.commands.insertContentAt(doc.content.size, { type: "paragraph" });
        editor.commands.focus("end");
      }}
    >
      <div className="mx-auto min-h-full w-full max-w-[760px] px-6 pt-12 pb-24">
        <EditorContent editor={editor} />
      </div>
    </div>
  );
}
