import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import type { Extension } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { tags } from "@lezer/highlight";

import type { ResolvedTheme } from "@/components/theme-provider";

/**
 * The colours of the editor in each theme. Every text colour has at least 4.5:1 against the editor
 * background and against the selection (`requirements/quality#General`, WCAG 2.2 AA); a test checks
 * the pairs. Tailwind 800 on light, 400 on dark, so the syntax colours match the rest of the UI.
 */
export interface EditorPalette {
  background: string;
  selection: string;
  foreground: string;
  punctuation: string;
  comment: string;
  keyword: string;
  string: string;
  number: string;
  definition: string;
  type: string;
  link: string;
  invalid: string;
}

export const PALETTES: Record<ResolvedTheme, EditorPalette> = {
  light: {
    background: "#ffffff",
    selection: "#dbeafe",
    foreground: "#18181b",
    punctuation: "#3f3f46",
    comment: "#52525b",
    keyword: "#6b21a8",
    string: "#166534",
    number: "#1e40af",
    definition: "#155e75",
    type: "#92400e",
    link: "#1e40af",
    invalid: "#b91c1c",
  },
  dark: {
    background: "#09090b",
    selection: "#172554",
    foreground: "#fafafa",
    punctuation: "#d4d4d8",
    comment: "#a1a1aa",
    keyword: "#c084fc",
    string: "#4ade80",
    number: "#60a5fa",
    definition: "#22d3ee",
    type: "#fbbf24",
    link: "#60a5fa",
    invalid: "#f87171",
  },
};

function highlightStyle(palette: EditorPalette): HighlightStyle {
  return HighlightStyle.define([
    { tag: [tags.keyword, tags.operatorKeyword, tags.modifier, tags.controlKeyword], color: palette.keyword },
    { tag: [tags.string, tags.special(tags.string), tags.regexp, tags.character], color: palette.string },
    { tag: [tags.number, tags.bool, tags.null, tags.atom, tags.literal], color: palette.number },
    { tag: [tags.comment, tags.lineComment, tags.blockComment, tags.docComment], color: palette.comment, fontStyle: "italic" },
    {
      tag: [tags.function(tags.variableName), tags.function(tags.propertyName), tags.definition(tags.variableName), tags.definition(tags.propertyName), tags.labelName],
      color: palette.definition,
    },
    { tag: [tags.typeName, tags.className, tags.namespace, tags.tagName, tags.standard(tags.tagName)], color: palette.type },
    { tag: [tags.propertyName, tags.attributeName, tags.variableName], color: palette.foreground },
    { tag: [tags.operator, tags.punctuation, tags.separator, tags.bracket, tags.meta], color: palette.punctuation },
    { tag: [tags.heading, tags.strong], fontWeight: "bold", color: palette.foreground },
    { tag: tags.emphasis, fontStyle: "italic" },
    { tag: tags.strikethrough, textDecoration: "line-through" },
    { tag: [tags.link, tags.url], color: palette.link, textDecoration: "underline" },
    { tag: tags.invalid, color: palette.invalid },
  ]);
}

/** The editor's chrome and syntax colours for a resolved theme. */
export function editorTheme(theme: ResolvedTheme): Extension {
  const palette = PALETTES[theme];
  return [
    EditorView.theme(
      {
        "&": { color: palette.foreground, backgroundColor: palette.background, height: "100%" },
        ".cm-scroller": {
          fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
          fontSize: "13px",
          lineHeight: "1.5",
        },
        ".cm-content": { caretColor: palette.foreground },
        "&.cm-focused .cm-cursor": { borderLeftColor: palette.foreground },
        "&.cm-focused": { outline: "none" },
        "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
          backgroundColor: palette.selection,
        },
        ".cm-gutters": { backgroundColor: palette.background, color: palette.comment, border: "none" },
        ".cm-activeLineGutter": { backgroundColor: "transparent", color: palette.foreground },
        ".cm-matchingBracket": { backgroundColor: palette.selection, outline: `1px solid ${palette.punctuation}` },
        ".cm-panels": { backgroundColor: palette.background, color: palette.foreground },
        ".cm-panels.cm-panels-top": { borderBottom: `1px solid ${palette.comment}` },
        ".cm-textfield, .cm-button": { color: palette.foreground, backgroundColor: palette.background },
        ".cm-searchMatch": { backgroundColor: palette.selection, outline: `1px solid ${palette.keyword}` },
      },
      { dark: theme === "dark" },
    ),
    syntaxHighlighting(highlightStyle(palette)),
  ];
}
