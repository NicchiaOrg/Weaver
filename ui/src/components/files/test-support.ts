import { EditorView } from "@codemirror/view";
import { act } from "@testing-library/react";

/**
 * What CodeMirror measures and jsdom does not have: the client rectangles of a range. Call it once
 * in a suite that mounts an editor.
 */
export function installEditorShims(): void {
  const none = Object.assign([], { item: () => null });
  const rect = { x: 0, y: 0, top: 0, left: 0, bottom: 0, right: 0, width: 0, height: 0, toJSON: () => ({}) } as DOMRect;
  Range.prototype.getClientRects ??= () => none as unknown as DOMRectList;
  Range.prototype.getBoundingClientRect ??= () => rect;
}

/** The view of the editor that is on screen. */
export function editorView(container: HTMLElement): EditorView {
  const dom = container.querySelector<HTMLElement>(".cm-editor");
  const view = dom && EditorView.findFromDOM(dom);
  if (!view) throw new Error("no editor on screen");
  return view;
}

/** The text the editor on screen holds. */
export function editorText(container: HTMLElement): string {
  return editorView(container).state.doc.toString();
}

/** Types `text` at the end of the document, as the developer does. */
export function typeAtEnd(container: HTMLElement, text: string): void {
  const view = editorView(container);
  act(() => view.dispatch({ changes: { from: view.state.doc.length, insert: text }, userEvent: "input.type" }));
}
