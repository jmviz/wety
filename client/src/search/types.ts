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

// One step of a word's trail back to its oldest form that is the same word.
export interface TrailStep {
  item: Item;
  etyMode: string | null;
}

// The descendants of one ancestor (`root`, an item id) of the requested
// word, with the word's trail and the sources of the trail's last step.
export interface Cognates {
  trail: TrailStep[];
  sources: Item[];
  // The word's ultimate ancestors not already on the trail or among its
  // sources, in etymology order: root entries, then affixes and the like.
  ultimateRoots: Item[];
  ultimateOthers: Item[];
  root: number;
  // Where the tree starts without a `root` in the link: the trail's last step,
  // or the root entry it is formed from.
  defaultRoot: number;
  // Words in the root's unfiltered tree (the server stops counting at
  // 100,000), and whether `tree` covers every language.
  total: number;
  allLangs: boolean;
  tree: Descendants;
}

export interface DescendantsView {
  total: number;
  allLangs: boolean;
  tree: Descendants;
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

// What a URL names: `/<view>/<lang code>/<term>?ety=…&lang=…`. Trees are
// drawn in every language when small and otherwise in the reader's
// languages, unless `lang` names the languages to draw (codes) or `all=1`
// asks for every one; `within=1` also draws words formed from a word of the
// same language. Cognates may add `root=<lang code>:<term>` (and `rootEty`),
// the ancestor to start from, when it is not the default.
export interface WordLink {
  kind: TreeKind;
  lang: string;
  term: string;
  ety: string | null;
  descLangs: string[];
  allLangs: boolean;
  within: boolean;
  root: string | null;
  rootEty: string | null;
}

export function linkHref(link: WordLink): string {
  const params = new URLSearchParams();
  if (link.ety) params.set("ety", link.ety);
  if (link.kind !== TreeKind.Etymology) {
    if (link.allLangs) params.set("all", "1");
    else for (const c of link.descLangs) params.append("lang", c);
    if (link.within) params.set("within", "1");
  }
  if (link.kind === TreeKind.Cognates && link.root) {
    params.set("root", link.root);
    if (link.rootEty) params.set("rootEty", link.rootEty);
  }
  const search = params.toString();
  return `/${kindPath[link.kind]}/${link.lang}/${encodeTerm(link.term)}${search ? `?${search}` : ""}`;
}

// Which languages a tree is drawn in: the default (none named), exactly
// these, or all of them; and whether words formed within one are drawn.
export interface LangChoice {
  langs?: Lang[];
  all?: boolean;
  within?: boolean;
}

// `root`: the ancestor a cognates tree starts from, when not the default.
export function wordHref(
  kind: TreeKind,
  item: Item,
  choice: LangChoice = {},
  root: Item | null = null
): string {
  return linkHref({
    kind,
    lang: item.lang.code,
    term: item.term,
    ety: item.ety ?? null,
    descLangs: choice.all ? [] : (choice.langs ?? []).map((l) => l.code),
    allLangs: !!choice.all,
    within: !!choice.within,
    root: root && `${root.lang.code}:${root.term}`,
    rootEty: root?.ety ?? null,
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
    descLangs: kind === TreeKind.Etymology || params.get("all") ? [] : params.getAll("lang"),
    allLangs: kind !== TreeKind.Etymology && params.get("all") === "1",
    within: kind !== TreeKind.Etymology && params.get("within") === "1",
    root: kind === TreeKind.Cognates ? params.get("root") : null,
    rootEty: kind === TreeKind.Cognates ? params.get("rootEty") : null,
  };
}

// A loaded view: the word, the languages its link names (none for the
// default), the cognates root when not the default, and the graph build its
// numeric ids belong to.
export class TreeRequest {
  lang: Lang;
  item: Item;
  descLangs: Lang[];
  allLangs: boolean;
  within: boolean;
  kind: TreeKind;
  root: Item | null;

  constructor(
    lang: Lang,
    item: Item,
    descLangs: Lang[],
    kind: TreeKind,
    root: Item | null = null,
    allLangs = false,
    within = false
  ) {
    this.lang = lang;
    this.item = item;
    this.descLangs = descLangs;
    this.descLangs.sort((a, b) => a.id - b.id);
    this.allLangs = allLangs;
    this.within = within;
    this.kind = kind;
    this.root = root;
  }

  // The same languages, for another view of this word.
  get choice(): LangChoice {
    const within = this.within || undefined;
    if (this.allLangs) return { all: true, within };
    return this.descLangs.length ? { langs: this.descLangs, within } : { within };
  }
}
