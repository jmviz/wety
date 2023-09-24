export interface LangOption {
  id: number;
  code: string;
  name: string;
  similarity: number;
  items: number;
}

export interface Item {
  id: number;
  etyNum: number;
  lang: string;
  term: string;
  imputed: boolean;
  reconstructed: boolean;
  url: string | null;
  pos: string[] | null;
  gloss: string[] | null;
  romanization: string | null;
}

export function term(item: Item): string {
  return item.reconstructed ? `*${item.term}` : item.term;
}

export interface ItemOption {
  distance: number;
  item: Item;
}

export interface Descendants {
  item: Item;
  children: Descendants[] | null;
  langDistance: number;
  etyMode: string | null;
  otherParents: Item[] | null;
  parentEtyOrder: number | null;
}

export interface Etymology {
  item: Item;
  etyMode: string | null;
  parents: Etymology[] | null;
  langDistance: number;
}
