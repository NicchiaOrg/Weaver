/**
 * The `@` tags of assets in the message boxes (`requirements/rules#assets`, `requirements/ui#assets`):
 * the same rules as `weaver_core::assets::tags`, applied to the names the window already holds, so
 * that typing starts no process and no disk work.
 */

/** A tag found in a text: the range of `@<name>` (UTF-16 offsets, as the DOM counts) and the asset it names. */
export interface Tag {
  start: number;
  end: number;
  name: string;
}

/** What may follow the name of an asset in a tag, besides the end of the text and whitespace. */
const AFTER_NAME = new Set([".", ",", ";", ":", "!", "?", ")", "]", "}", '"', "'"]);

const WHITESPACE = /\s/;

/**
 * A name that cannot be written between backticks in a line for the agent (it holds a backtick or a
 * control character) is never a tag, in the core as here.
 */
export function isQuotable(name: string): boolean {
  if (name === "" || name.includes("`")) return false;
  for (const char of name) {
    const code = char.codePointAt(0) ?? 0;
    if (code < 0x20 || (code >= 0x7f && code <= 0x9f)) return false;
  }
  return true;
}

function mayFollow(next: string | undefined): boolean {
  return next === undefined || WHITESPACE.test(next) || AFTER_NAME.has(next);
}

/**
 * Every tag of `text` against the assets `names`: `@` at the start of the text or after whitespace,
 * then the name of an asset compared exactly (case included), then the end of the text, whitespace
 * or one of `. , ; : ! ? ) ] } " '`. When several names fit, the longest wins. Anything else after
 * `@` is plain text.
 */
export function findTags(text: string, names: readonly string[]): Tag[] {
  const usable = names.filter(isQuotable);
  const tags: Tag[] = [];
  let from = 0;
  for (let at = text.indexOf("@"); at !== -1; at = text.indexOf("@", from)) {
    from = at + 1;
    if (at > 0 && !WHITESPACE.test(text.charAt(at - 1))) continue;
    const rest = text.slice(at + 1);
    let longest: string | null = null;
    for (const name of usable) {
      if (rest.startsWith(name) && mayFollow(rest[name.length]) && (longest === null || name.length > longest.length)) longest = name;
    }
    if (longest !== null) {
      const end = at + 1 + longest.length;
      tags.push({ start: at, end, name: longest });
      from = end;
    }
  }
  return tags;
}

/** The `@` the developer is typing at the caret: where it is and what follows it up to the caret. */
export interface Typing {
  /** The offset of the `@`. */
  at: number;
  /** The text between the `@` and the caret. */
  query: string;
}

/**
 * The tag being typed at `caret`: the nearest `@` before it that starts the text or follows
 * whitespace, with no line break between the two. `null` when the caret is not in such a tag.
 */
export function typingAt(text: string, caret: number): Typing | null {
  const end = Math.max(0, Math.min(caret, text.length));
  for (let at = end - 1; at >= 0; at--) {
    const char = text.charAt(at);
    if (char === "\n" || char === "\r") return null;
    if (char !== "@") continue;
    if (at > 0 && !WHITESPACE.test(text.charAt(at - 1))) return null;
    return { at, query: text.slice(at + 1, end) };
  }
  return null;
}

const byName = (a: string, b: string): number => {
  const x = a.toLowerCase();
  const y = b.toLowerCase();
  if (x !== y) return x < y ? -1 : 1;
  return a < b ? -1 : a > b ? 1 : 0;
};

/**
 * The assets the list offers for what was typed after `@`: the names that contain it, without
 * regard to case; the names that start with it first, then the others, each group in name order.
 */
export function suggestionsFor(names: readonly string[], query: string): string[] {
  const needle = query.toLowerCase();
  const matching = names.filter((name) => isQuotable(name) && name.toLowerCase().includes(needle));
  const starting = matching.filter((name) => name.toLowerCase().startsWith(needle)).sort(byName);
  const others = matching.filter((name) => !name.toLowerCase().startsWith(needle)).sort(byName);
  return [...starting, ...others];
}

/** The text and the caret once the asset `name` is chosen in place of what was typed after the `@` at `typing`. */
export function insertTag(text: string, caret: number, typing: Typing, name: string): { text: string; caret: number } {
  const inserted = `@${name} `;
  return { text: text.slice(0, typing.at) + inserted + text.slice(Math.max(caret, typing.at)), caret: typing.at + inserted.length };
}
