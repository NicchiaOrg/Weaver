import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { bracketMatching, foldGutter, foldKeymap, indentOnInput } from "@codemirror/language";
import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
import { Annotation, Compartment, EditorState, type Extension } from "@codemirror/state";
import {
  crosshairCursor,
  drawSelection,
  EditorView,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
  rectangularSelection,
  type ViewUpdate,
} from "@codemirror/view";

import { editorTheme } from "@/components/files/editor-theme";
import type { ResolvedTheme } from "@/components/theme-provider";

/** Marks a change the code made (a text read from the disk): it is not an edit of the developer. */
export const programmatic = Annotation.define<boolean>();

export const themeCompartment = new Compartment();
export const languageCompartment = new Compartment();
export const readOnlyCompartment = new Compartment();

/** The editor's `readOnly` facet for a tab that can or cannot be changed. */
export function readOnlyExtension(readOnly: boolean): Extension {
  return EditorState.readOnly.of(readOnly);
}

/** Whether an update holds a change of the developer, not a text read from the disk. */
export function isEdit(update: ViewUpdate): boolean {
  return update.docChanged && !update.transactions.some((transaction) => transaction.annotation(programmatic) === true);
}

/**
 * The state of one tab's editor: line numbers, undo and redo, search and replace (`Cmd+F` /
 * `Ctrl+F`), matching brackets, code folding and multiple cursors (`Alt`-click, `Alt`-drag for a
 * rectangle); no completion, no diagnostics and no language server. `onUpdate` hears of every
 * update of the view that shows it. `Tab` is not bound: it moves the focus.
 */
export function createEditorState(doc: string, theme: ResolvedTheme, readOnly: boolean, onUpdate: (update: ViewUpdate) => void): EditorState {
  return EditorState.create({
    doc,
    extensions: [
      lineNumbers(),
      highlightActiveLineGutter(),
      foldGutter(),
      history(),
      drawSelection(),
      indentOnInput(),
      bracketMatching(),
      highlightSelectionMatches(),
      EditorState.allowMultipleSelections.of(true),
      rectangularSelection(),
      crosshairCursor(),
      keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, ...foldKeymap]),
      themeCompartment.of(editorTheme(theme)),
      languageCompartment.of([]),
      readOnlyCompartment.of(readOnlyExtension(readOnly)),
      EditorView.contentAttributes.of({ "aria-label": "File text" }),
      EditorView.updateListener.of(onUpdate),
    ],
  });
}
