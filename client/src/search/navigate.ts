import { Item, Lang, LangChoice, TreeKind, wordHref } from "./types";
import {
  setSelectedItem,
  setSelectedLang,
  setSelectedTreeKind,
} from "../state";

import { useRouter } from "@tanstack/solid-router";

// The searched word's own language is always among the descendant languages.
export function withOwnLang(descLangs: Lang[], lang: Lang): Lang[] {
  return descLangs.some((l) => l.id === lang.id) ? descLangs : [...descLangs, lang];
}

// One place that turns (view, word, languages, cognates root) into a URL.
// A new word starts from the default languages. The href is pushed as is: it
// is already encoded (see `wordHref`).
export function useGoTo() {
  const router = useRouter();
  return (kind: TreeKind, item: Item, choice: LangChoice = {}, root: Item | null = null) => {
    setSelectedLang(item.lang);
    setSelectedItem(item);
    setSelectedTreeKind(kind);
    const langs = choice.langs && withOwnLang(choice.langs, item.lang);
    const href = wordHref(kind, item, { ...choice, langs }, root);
    // Already there (e.g. the set lost only the word's own language, which
    // comes straight back): no duplicate history entry.
    const here = router.history.location;
    if (here.pathname + here.search === href) return;
    router.history.push(href);
  };
}
