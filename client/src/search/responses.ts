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

export interface Etymology {
  item: Item;
  etyMode: string | null;
  etyOrder: number;
  parents: Etymology[] | null;
  langDistance: number;
}

export interface OtherParent {
  item: Item;
  langDistance: number;
}

export interface Descendants {
  item: Item;
  children: Descendants[] | null;
  langDistance: number;
  etyMode: string | null;
  otherParents: OtherParent[];
  parentEtyOrder: number | null;
}
