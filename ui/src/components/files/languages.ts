import { LanguageDescription, type LanguageSupport } from "@codemirror/language";
import { languages } from "@codemirror/language-data";

/** A language that colours a file, with the name `@codemirror/language-data` knows it by. */
export interface LoadedLanguage {
  name: string;
  support: LanguageSupport;
}

/**
 * The language of a file, chosen by its name and then by its extension (`@codemirror/language-data`
 * does both), loaded with a dynamic import the first time a file of that kind opens. `null` when the
 * kind is unknown (plain text) or the grammar cannot be loaded.
 */
export async function languageOf(fileName: string): Promise<LoadedLanguage | null> {
  const description = LanguageDescription.matchFilename(languages, fileName);
  if (!description) return null;
  try {
    return { name: description.name, support: await description.load() };
  } catch {
    return null;
  }
}
