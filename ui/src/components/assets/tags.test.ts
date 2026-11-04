import { describe, expect, it } from "vitest";

import { findTags, insertTag, suggestionsFor, typingAt } from "@/components/assets/tags";

const found = (text: string, names: string[]) => findTags(text, names).map((tag) => text.slice(tag.start, tag.end));

describe("tag recognition", () => {
  it("takes `@` at the start or after whitespace, then the exact name of an asset", () => {
    const names = ["logo.png", "spec.pdf"];
    expect(found("@logo.png", names)).toEqual(["@logo.png"]);
    expect(found("look at @logo.png now", names)).toEqual(["@logo.png"]);
    expect(found("a\n@logo.png", names)).toEqual(["@logo.png"]);
    expect(found("mail me@logo.png", names)).toEqual([]);
    expect(found("see @other.png", names)).toEqual([]);
    expect(found("@", names)).toEqual([]);
    expect(found("@ logo.png", names)).toEqual([]);
  });

  it("compares the name with its case", () => {
    expect(found("@Logo.png", ["logo.png"])).toEqual([]);
    expect(found("@logo.PNG", ["logo.png"])).toEqual([]);
  });

  it("needs the end of the text, whitespace or one of the closing characters after the name", () => {
    for (const ok of ["@a", "@a ", "@a\n", "@a.", "@a,", "@a;", "@a:", "@a!", "@a?", "@a)", "@a]", "@a}", '@a"', "@a'"]) {
      expect(found(ok, ["a"])).toEqual(["@a"]);
    }
    for (const bad of ["@ab", "@a-", "@a_b", "@a/", "@a(", "@a@", "@a`", "@a*", "@a>"]) {
      expect(found(bad, ["a"])).toEqual([]);
    }
  });

  it("takes the longest name, so names with spaces need no quotes", () => {
    const names = ["notes", "notes.txt", "my notes.txt", "my"];
    expect(found("@notes.txt", names)).toEqual(["@notes.txt"]);
    expect(found("@notes.", names)).toEqual(["@notes"]);
    expect(found("read @my notes.txt please", names)).toEqual(["@my notes.txt"]);
    expect(found("read @my notes please", names)).toEqual(["@my"]);
  });

  it("counts a tag inside code as any other, and two tags apart", () => {
    const names = ["a.png", "b.png"];
    expect(found("`@a.png`", names)).toEqual([]);
    expect(found("```\n@a.png\n```", names)).toEqual(["@a.png"]);
    expect(found("@a.png @b.png", names)).toEqual(["@a.png", "@b.png"]);
    expect(found("@a.png,@b.png", names)).toEqual(["@a.png"]);
  });

  it("never takes a name that cannot be quoted for the agent", () => {
    expect(found("@a`b @tab\tname @x", ["a`b", "tab\tname", "x"])).toEqual(["@x"]);
  });

  it("gives the range of each tag in the text", () => {
    expect(findTags("è @a.png ☕ @b.png", ["a.png", "b.png"])).toEqual([
      { start: 2, end: 8, name: "a.png" },
      { start: 11, end: 17, name: "b.png" },
    ]);
  });
});

describe("the tag being typed", () => {
  it("is the nearest `@` before the caret that starts the text or follows whitespace", () => {
    expect(typingAt("@", 1)).toEqual({ at: 0, query: "" });
    expect(typingAt("see @lo", 7)).toEqual({ at: 4, query: "lo" });
    expect(typingAt("see @my no", 10)).toEqual({ at: 4, query: "my no" });
    expect(typingAt("see @lo and more", 7)).toEqual({ at: 4, query: "lo" });
    expect(typingAt("mail me@lo", 10)).toBeNull();
    expect(typingAt("no tag here", 5)).toBeNull();
    expect(typingAt("@lo\nnext", 8)).toBeNull();
    expect(typingAt("@lo", 0)).toBeNull();
  });

  it("is replaced by `@<name>` and a space, the caret after them", () => {
    const typing = typingAt("see @lo here", 7);
    expect(typing).not.toBeNull();
    expect(insertTag("see @lo here", 7, typing as NonNullable<typeof typing>, "logo.png")).toEqual({ text: "see @logo.png  here", caret: 14 });
  });
});

describe("the suggestions", () => {
  const names = ["spec.pdf", "Logo.png", "my logo.svg", "notes.txt", "banner.png"];

  it("hold the names that contain what was typed, without regard to case", () => {
    expect(suggestionsFor(names, "log")).toEqual(["Logo.png", "my logo.svg"]);
    expect(suggestionsFor(names, "LOG")).toEqual(["Logo.png", "my logo.svg"]);
    expect(suggestionsFor(names, "zzz")).toEqual([]);
  });

  it("put the names that start with it first, then the others, each group in name order", () => {
    expect(suggestionsFor(["b-a.txt", "a.txt", "c.txt", "ba.txt", "xa.txt"], "a")).toEqual(["a.txt", "b-a.txt", "ba.txt", "xa.txt"]);
    expect(suggestionsFor(names, "").slice(0, 2)).toEqual(["banner.png", "Logo.png"]);
  });

  it("leave out a name that cannot be quoted", () => {
    expect(suggestionsFor(["a`b", "ab"], "a")).toEqual(["ab"]);
  });
});
