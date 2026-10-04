// `id` is a position in the server's current graph build: fine within a
// session, never in a URL. Links use `code`.
export interface Lang {
  id: number;
  code: string;
  name: string;
}

export interface Item {
  id: number;
  etyNum: number;
  // Tells this item apart from others of the same lang and term in a link:
  // Wiktionary's etymology number (absent when nothing needs telling apart).
  ety?: string;
  lang: Lang;
  term: string;
  imputed: boolean;
  reconstructed: boolean;
  url: string | null;
  pos: string[] | null;
  // `undefined` when the tree was requested with `lean=true`; fetch lazily
  // from `/item/:id` in that case.
  gloss?: string[] | null;
  romanization: string | null;
}

export function term(item: Item): string {
  return item.reconstructed ? `*${item.term}` : item.term;
}

export interface Etymology {
  item: Item;
  etyMode: string | null;
  etyOrder: number;
  parents: Etymology[];
  langDistance: number;
}

export interface OtherParent {
  item: Item;
  etyOrder: number;
  langDistance: number;
}

export interface Descendants {
  item: Item;
  children: Descendants[];
  // Sent by newer servers; the client fills it in when absent.
  descendantCount?: number;
  langDistance: number;
  etyMode: string | null;
  otherParents: OtherParent[];
  parentEtyOrder: number | null;
}

export enum TreeKind {
  Cognates = "Cognates",
  Etymology = "Etymology",
  Descendants = "Descendants",
}

export const kindPath: Record<TreeKind, string> = {
  [TreeKind.Etymology]: "etymology",
  [TreeKind.Descendants]: "descendants",
  [TreeKind.Cognates]: "cognates",
};

// Spaces are written as underscores, as Wiktionary does (its titles cannot
// contain "_").
export const encodeTerm = (term: string) => encodeURIComponent(term.replace(/ /g, "_"));
export const decodeTerm = (s: string) => decodeURIComponent(s).replace(/_/g, " ");

// What a URL names: `/<view>/<lang code>/<term>?ety=…&lang=…`, where `lang`
// lists the descendant languages (codes).
export interface WordLink {
  kind: TreeKind;
  lang: string;
  term: string;
  ety: string | null;
  descLangs: string[];
}

export function linkHref(link: WordLink): string {
  const params = new URLSearchParams();
  if (link.ety) params.set("ety", link.ety);
  if (link.kind !== TreeKind.Etymology) for (const c of link.descLangs) params.append("lang", c);
  const search = params.toString();
  return `/${kindPath[link.kind]}/${link.lang}/${encodeTerm(link.term)}${search ? `?${search}` : ""}`;
}

export function wordHref(kind: TreeKind, item: Item, descLangs: Lang[] = []): string {
  return linkHref({
    kind,
    lang: item.lang.code,
    term: item.term,
    ety: item.ety ?? null,
    descLangs: descLangs.map((l) => l.code),
  });
}

export function parseWordHref(href: string): WordLink | null {
  const qIdx = href.indexOf("?");
  const pathname = qIdx >= 0 ? href.slice(0, qIdx) : href;
  const params = new URLSearchParams(qIdx >= 0 ? href.slice(qIdx) : "");
  const [kindStr, lang, ...rest] = pathname.split("/").filter(Boolean);
  const kind = (Object.keys(kindPath) as TreeKind[]).find((k) => kindPath[k] === kindStr);
  if (!kind || !lang || rest.length === 0) return null;
  let term: string;
  try {
    term = decodeTerm(rest.join("/"));
  } catch {
    return null;
  }
  return {
    kind,
    lang,
    term,
    ety: params.get("ety"),
    descLangs: kind === TreeKind.Etymology ? [] : params.getAll("lang"),
  };
}

// A loaded view: the word, its descendant languages, and the graph build
// its numeric ids belong to.
export class TreeRequest {
  lang: Lang;
  item: Item;
  descLangs: Lang[];
  kind: TreeKind;

  constructor(lang: Lang, item: Item, descLangs: Lang[], kind: TreeKind) {
    this.lang = lang;
    this.item = item;
    this.descLangs = descLangs;
    this.descLangs.sort((a, b) => a.id - b.id);
    this.kind = kind;
  }

  apiPath(graph: string): string {
    const params = new URLSearchParams({ graph });
    if (this.kind !== TreeKind.Etymology)
      for (const l of this.descLangs) params.append("descLang", l.code);
    return `/${kindPath[this.kind]}/${this.item.id}?${params}`;
  }
}
