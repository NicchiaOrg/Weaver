// The TipTap pieces that are the Notes view's own: the raw Markdown block that keeps what the sheet
// cannot show, the image block with its placeholder, the checklist item with its clickable
// checkbox, and the keys the Markdown shortcuts of `requirements/ui#notes-view` need beyond
// StarterKit. Only `notes-editor.tsx` imports this module, and the view loads that one lazily.
import { Extension, InputRule, Mark, mergeAttributes, Node, nodeInputRule, type Editor, type MarkdownToken } from "@tiptap/core";
import { HorizontalRule } from "@tiptap/extension-horizontal-rule";
import Image, { type ImageOptions } from "@tiptap/extension-image";
import { TaskItem } from "@tiptap/extension-list";
import { NodeViewContent, NodeViewWrapper, ReactNodeViewRenderer, type NodeViewProps } from "@tiptap/react";
import { ImageOff } from "lucide-react";
import { useEffect, useState, useSyncExternalStore } from "react";

import { Checkbox } from "@/components/ui/checkbox";
import { notesImageName } from "@/components/notes/notes-model";
import type { ImageStore } from "@/components/notes/notes-session";
import { cn } from "@/lib/utils";

/** Marks a transaction that is not the developer's: the sheet replaced from disk. */
export const EXTERNAL_META = "notesExternal";
/** Marks a transaction that is a tick: the sheet is written at once. */
export const TICK_META = "notesTick";

// --- unsupported blocks -----------------------------------------------------------------------

const CONFLICT_REGION = /^<{7}(?: [^\n]*)?\n(?:[\s\S]*?\n)?>{7}(?: [^\n]*)?(?:\n|$)/;
const CONFLICT_MARKER = /^(?:<{7}|={7}|>{7}|\|{7})(?: [^\n]*)?(?:\n|$)/;
const FOOTNOTE_DEFINITION = /^\[\^[^\]\s]+\]:[^\n]*(?:\n(?: {2,}|\t)[^\n]*|\n\n(?: {4}|\t)[^\n]*)*(?:\n|$)/;
const FRONT_MATTER = /^---[ \t]*\n(?:[\s\S]*?\n)?(?:---|\.\.\.)[ \t]*(?:\n|$)/;

/** The token of a block that no extension maps: its source, kept as it is. */
export function rawToken(raw: string): MarkdownToken {
  return { type: "rawMarkdown", raw, text: raw.replace(/\n+$/, "") } as MarkdownToken;
}

/**
 * A block of the file that nothing else maps (raw HTML, a table, a footnote, front matter,
 * conflict markers, a heading deeper than level 3): one block that holds its source as plain text
 * and writes it back as it is.
 */
export const RawMarkdown = Node.create({
  name: "rawMarkdown",
  group: "block",
  content: "text*",
  marks: "",
  code: true,
  defining: true,
  isolating: true,

  parseHTML() {
    return [{ tag: "pre[data-raw-markdown]", preserveWhitespace: "full" }];
  },

  renderHTML({ HTMLAttributes }) {
    return ["pre", mergeAttributes(HTMLAttributes, { "data-raw-markdown": "" }), ["code", 0]];
  },

  markdownTokenName: "rawMarkdown",

  // What marked cannot know: front matter at the very start, conflict markers and footnote
  // definitions. HTML, tables, link definitions and deep headings are turned into this token by
  // `notes-editor.tsx`.
  markdownTokenizer: {
    name: "rawMarkdown",
    level: "block",
    start: (src: string) => src.search(/^(?:<{7}|\[\^[^\]\s]+\]:)/m),
    tokenize: (src, tokens) => {
      const region = CONFLICT_REGION.exec(src) ?? CONFLICT_MARKER.exec(src) ?? FOOTNOTE_DEFINITION.exec(src) ?? (tokens.length === 0 ? FRONT_MATTER.exec(src) : null);
      return region ? rawToken(region[0]) : undefined;
    },
  },

  parseMarkdown: (token, helpers) => helpers.createNode("rawMarkdown", {}, token.text ? [helpers.createTextNode(token.text)] : []),

  renderMarkdown: (node, helpers) => (node.content ? helpers.renderChildren(node.content) : ""),
});

/** The token of inline syntax that nothing maps (an HTML tag, a footnote mark): text, written back as it is. */
export function rawInlineToken(raw: string): MarkdownToken {
  return { type: "rawInline", raw, text: raw } as MarkdownToken;
}

/**
 * Inline syntax the sheet does not know stays as the text it is. The mark only keeps that text
 * from being escaped on writing; it shows as plain text.
 */
export const RawInline = Mark.create({
  name: "rawInline",
  code: true,
  excludes: "_",
  inclusive: false,

  parseHTML() {
    return [{ tag: "span[data-raw-inline]" }];
  },

  renderHTML({ HTMLAttributes }) {
    return ["span", mergeAttributes(HTMLAttributes, { "data-raw-inline": "" }), 0];
  },

  markdownTokenName: "rawInline",

  markdownTokenizer: {
    name: "rawInline",
    level: "inline",
    start: (src: string) => src.indexOf("[^"),
    tokenize: (src) => {
      const mark = /^\[\^[^\]\s]+\]/.exec(src);
      return mark ? rawInlineToken(mark[0]) : undefined;
    },
  },

  parseMarkdown: (token, helpers) => helpers.applyMark("rawInline", [helpers.createTextNode(token.text ?? token.raw ?? "")]),

  renderMarkdown: (node, helpers) => (node.content ? helpers.renderChildren(node.content) : ""),
});

// --- images -----------------------------------------------------------------------------------

export interface NotesImageOptions extends ImageOptions {
  images: ImageStore;
}

function ImageView({ node, selected, extension }: NodeViewProps) {
  const store = (extension.options as NotesImageOptions).images;
  const src = node.attrs.src as string;
  const alt = (node.attrs.alt as string | null) ?? "";
  const name = notesImageName(src);
  const entry = useSyncExternalStore(store.subscribe, () => (name === null ? undefined : store.peek(name)));
  const [failed, setFailed] = useState<string | null>(null);
  useEffect(() => {
    if (name !== null) store.ensure(name);
  }, [name, store, entry]);

  const url = entry?.state === "ready" && failed !== entry.url ? entry.url : null;
  return (
    <NodeViewWrapper className={cn("my-3", url !== null && "w-fit max-w-full")} data-selected={selected ? "" : undefined}>
      {url !== null ? (
        <img
          src={url}
          alt={alt}
          draggable={false}
          onError={() => setFailed(url)}
          className={cn("block h-auto max-w-full rounded-sm", selected && "outline-2 outline-offset-0 outline-ring")}
        />
      ) : (
        <div
          data-image-placeholder=""
          className={cn(
            "flex h-12 w-full items-center gap-2 rounded-md border border-dashed bg-muted px-3 text-muted-foreground",
            selected && "outline-2 outline-offset-0 outline-ring",
          )}
        >
          <ImageOff className="size-4 shrink-0" aria-hidden />
          <span className="min-w-0 truncate">{name ?? src}</span>
        </div>
      )}
    </NodeViewWrapper>
  );
}

/** The destination is written as it is, between angle brackets so that spaces need no escaping. */
export function imageMarkdown(src: string, alt: string, title: string): string {
  const destination = /[<>\n]/.test(src) ? src : `<${src}>`;
  return title ? `![${alt}](${destination} "${title}")` : `![${alt}](${destination})`;
}

/** A block image that keeps its Markdown destination and shows it only when it names a file of the notes folder. */
export const NotesImage = Image.extend<NotesImageOptions>({
  draggable: false,

  addNodeView() {
    return ReactNodeViewRenderer(ImageView);
  },

  renderMarkdown: (node) => imageMarkdown(String(node.attrs?.src ?? ""), String(node.attrs?.alt ?? ""), String(node.attrs?.title ?? "")),
});

// --- checklists -------------------------------------------------------------------------------

/** Ticks or unticks the item at `pos`: one transaction that changes only its `checked` attribute. */
export function tickItem(editor: Editor, pos: number, checked?: boolean): boolean {
  const node = editor.state.doc.nodeAt(pos);
  if (node?.type.name !== "taskItem") return false;
  const next = checked ?? !node.attrs.checked;
  editor.view.dispatch(editor.state.tr.setNodeMarkup(pos, undefined, { ...node.attrs, checked: next }).setMeta(TICK_META, true));
  return true;
}

function TaskItemView({ node, editor, getPos }: NodeViewProps) {
  const checked = node.attrs.checked === true;
  const text = node.firstChild?.textContent ?? "";
  return (
    <NodeViewWrapper className="flex items-start gap-2">
      <span contentEditable={false} className="mt-[3px] shrink-0 select-none">
        <Checkbox
          checked={checked}
          aria-label={text === "" ? "Empty item" : text}
          tabIndex={-1}
          // The click ticks the item and leaves the cursor and the scroll where they are.
          onMouseDown={(event) => event.preventDefault()}
          onCheckedChange={(value) => {
            const pos = getPos();
            if (pos !== undefined) tickItem(editor, pos, value === true);
          }}
        />
      </span>
      <NodeViewContent className="task-body min-w-0 flex-1" />
    </NodeViewWrapper>
  );
}

/** A checklist item with a checkbox of the shadcn look; `Cmd+Enter` / `Ctrl+Enter` ticks the item that holds the cursor. */
export const NotesTaskItem = TaskItem.extend({
  addNodeView() {
    // The renderer's own element is the `li`, so the list stays a list of items.
    return ReactNodeViewRenderer(TaskItemView, { as: "li", attrs: ({ node }) => ({ "data-type": "taskItem", "data-checked": node.attrs.checked ? "true" : "false" }) });
  },

  addInputRules() {
    return [
      // `- ` makes a bullet; `[ ] ` or `[x] ` typed in its empty first line makes it a checklist item instead.
      new InputRule({
        find: /^\[([ xX]?)\]\s$/,
        handler: ({ state, range, match, chain }) => {
          const { $from } = state.selection;
          if ($from.depth < 3 || $from.node($from.depth - 1).type.name !== "listItem") return null;
          const checked = match[1]?.toLowerCase() === "x";
          chain().deleteRange(range).toggleList("taskList", "taskItem").updateAttributes("taskItem", { checked }).run();
          return undefined;
        },
      }),
      ...(this.parent?.() ?? []),
    ];
  },

  addKeyboardShortcuts() {
    return {
      ...this.parent?.(),
      "Mod-Enter": ({ editor }) => {
        const { $from } = editor.state.selection;
        if ($from.parent.type.name === "codeBlock") return false;
        for (let depth = $from.depth; depth > 0; depth--) {
          if ($from.node(depth).type.name === "taskItem") return tickItem(editor, $from.before(depth));
        }
        return false;
      },
    };
  },
});

// --- shortcuts beyond StarterKit --------------------------------------------------------------

/** A divider is made by `---` followed by Enter (StarterKit's rule makes it at the third dash). */
export const NotesHorizontalRule = HorizontalRule.extend({
  addInputRules() {
    return [nodeInputRule({ find: /^---\n$/, type: this.type })];
  },
});

/** `Backspace` at the start of a heading or a quote turns it back into a paragraph. */
export const BlockKeys = Extension.create({
  name: "notesBlockKeys",
  priority: 200,

  addKeyboardShortcuts() {
    return {
      Backspace: ({ editor }) => {
        const { selection } = editor.state;
        const { $from } = selection;
        if (!selection.empty || $from.parentOffset !== 0) return false;
        if ($from.parent.type.name === "heading") return editor.commands.setNode("paragraph");
        const quote = $from.depth >= 2 ? $from.node($from.depth - 1) : null;
        if (quote?.type.name === "blockquote" && $from.index($from.depth - 1) === 0) return editor.commands.lift("blockquote");
        return false;
      },
    };
  },
});
