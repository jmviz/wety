import { Item, Lang } from "./types";

const base = () => import.meta.env.VITE_API_BASE_URL;

export async function searchLangs(name: string): Promise<Lang[]> {
  const r = await fetch(`${base()}/search/lang?name=${encodeURIComponent(name)}`);
  return r.json();
}

export async function searchItems(lang: Lang, term: string): Promise<Item[]> {
  const r = await fetch(
    `${base()}/search/item/${lang.id}?term=${encodeURIComponent(term)}`
  );
  return r.json();
}

export function cleanTerm(term: string): string {
  return term.trim().replace(/^\*/, "").toLowerCase();
}
