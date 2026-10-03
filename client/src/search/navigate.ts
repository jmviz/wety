import { Item, Lang, TreeKind } from "./types";
import {
  selectedDescLangs,
  setSelectedItem,
  setSelectedLang,
  setSelectedTreeKind,
} from "../state";

import { useNavigate } from "@tanstack/solid-router";

const kindPath: Record<TreeKind, string> = {
  [TreeKind.Etymology]: "etymology",
  [TreeKind.Descendants]: "descendants",
  [TreeKind.Cognates]: "cognates",
};

// The searched word's own language is always among the descendant languages.
export function withOwnLang(descLangs: Lang[], lang: Lang): Lang[] {
  return descLangs.some((l) => l.id === lang.id) ? descLangs : [...descLangs, lang];
}

// One place that turns (view, word, descendant languages) into a URL.
export function useGoTo() {
  const navigate = useNavigate();
  return (kind: TreeKind, item: Item, descLangs: Lang[] = selectedDescLangs()) => {
    setSelectedLang(item.lang);
    setSelectedItem(item);
    setSelectedTreeKind(kind);
    if (kind === TreeKind.Etymology) {
      navigate({ to: `/etymology/${item.id}`, search: {} });
      return;
    }
    navigate({
      to: `/${kindPath[kind]}/${item.id}`,
      search: {
        distLang: item.lang.id,
        descLang: withOwnLang(descLangs, item.lang).map((l) => l.id),
      },
    });
  };
}
