import { Item, Lang, TreeKind, wordHref } from "./types";
import {
  selectedDescLangs,
  setSelectedItem,
  setSelectedLang,
  setSelectedTreeKind,
} from "../state";

import { useRouter } from "@tanstack/solid-router";

// The searched word's own language is always among the descendant languages.
export function withOwnLang(descLangs: Lang[], lang: Lang): Lang[] {
  return descLangs.some((l) => l.id === lang.id) ? descLangs : [...descLangs, lang];
}

// One place that turns (view, word, descendant languages) into a URL. The
// href is pushed as is: it is already encoded (see `wordHref`).
export function useGoTo() {
  const router = useRouter();
  return (kind: TreeKind, item: Item, descLangs: Lang[] = selectedDescLangs()) => {
    setSelectedLang(item.lang);
    setSelectedItem(item);
    setSelectedTreeKind(kind);
    const href = wordHref(kind, item, withOwnLang(descLangs, item.lang));
    // Already there (e.g. the set lost only the word's own language, which
    // comes straight back): no duplicate history entry.
    const here = router.history.location;
    if (here.pathname + here.search === href) return;
    router.history.push(href);
  };
}
