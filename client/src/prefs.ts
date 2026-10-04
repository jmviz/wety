// Small things remembered per browser: the language chip, the descendant
// language set and the view. All best-effort; storage may be unavailable.

import { Lang, TreeKind } from "./search/types";

const KEY = "wety.prefs";

interface Prefs {
  lang?: Lang;
  descLangs?: Lang[];
  kind?: TreeKind;
}

function read(): Prefs {
  try {
    return JSON.parse(window.localStorage.getItem(KEY) ?? "{}") as Prefs;
  } catch {
    return {};
  }
}

function write(patch: Prefs) {
  try {
    window.localStorage.setItem(KEY, JSON.stringify({ ...read(), ...patch }));
  } catch {
    // ignore
  }
}

// The search chip has to start in *some* language. Rather than assume one,
// take it from the browser's UI language (e.g. "de-CH" → "German") and let
// the caller resolve that name against the API; English is only the last
// resort.
// Its id is a placeholder until state.ts resolves "en" against the server.
export const ENGLISH: Lang = { id: -1, code: "en", name: "English" };

export function browserLangName(): string | null {
  try {
    const tag = navigator.language;
    if (!tag) return null;
    const code = new Intl.Locale(tag).language;
    const name = new Intl.DisplayNames(["en"], { type: "language" }).of(code);
    return name && name !== code ? name : null;
  } catch {
    return null;
  }
}

export const hasStoredLang = (): boolean => read().lang !== undefined;

export const loadLang = (): Lang => read().lang ?? ENGLISH;
export const saveLang = (lang: Lang) => write({ lang });
// No built-in language list: the set starts empty and the searched word's own
// language is always added on top (see `withOwnLang`).
export const loadDescLangs = (): Lang[] => read().descLangs ?? [];
export const saveDescLangs = (descLangs: Lang[]) => write({ descLangs });
export const loadKind = (): TreeKind => read().kind ?? TreeKind.Cognates;
export const saveKind = (kind: TreeKind) => write({ kind });
