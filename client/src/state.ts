import {
  createSignal,
  createMemo,
  createEffect,
  createRoot,
  batch,
  untrack,
} from "solid-js";
import {
  Cognates,
  Descendants,
  DescendantsView,
  Etymology,
  Item,
  Lang,
  TreeKind,
  TreeRequest,
  WordLink,
  kindPath,
  linkHref,
  parseWordHref,
  wordHref,
} from "./search/types";
import {
  Forest,
  LangCount,
  applyDefaultOpen,
  buildForest,
  snapshotOpen,
} from "./ety/model";
import { close as closePopover } from "./ety/popover";
import {
  ENGLISH,
  browserLangName,
  hasStoredLang,
  loadDescLangs,
  loadKind,
  loadLang,
  saveDescLangs,
  saveKind,
  saveLang,
} from "./prefs";

const API = import.meta.env.VITE_API_BASE_URL;

// Declared before anything below calls it at load time.
const langCache = new Map<string, Promise<Lang | null>>();
export function fetchLang(code: string): Promise<Lang | null> {
  let p = langCache.get(code);
  if (!p) {
    p = fetch(`${API}/lang/${encodeURIComponent(code)}`)
      .then((r) => (r.ok ? (r.json() as Promise<Lang>) : null))
      .catch(() => null);
    langCache.set(code, p);
  }
  return p;
}

// Global application state. The language chip, descendant-language set and
// view are remembered per browser.
export const [selectedLang, setSelectedLang] = createSignal<Lang>(loadLang());
export const [selectedItem, setSelectedItem] = createSignal<Item | null>(null);
export const [selectedDescLangs, setSelectedDescLangs] = createSignal<Lang[]>(
  loadDescLangs()
);
export const [selectedTreeKind, setSelectedTreeKind] = createSignal<TreeKind>(
  loadKind()
);
// First visit: pick the search language from the browser's UI language, if
// the API knows it by that name.
const firstVisit = !hasStoredLang();
if (firstVisit) {
  const name = browserLangName();
  if (name && name !== "English") {
    fetch(`${API}/search/lang?name=${encodeURIComponent(name)}`)
      .then((r) => r.json() as Promise<Lang[]>)
      .then((langs) => {
        const exact = langs.find((l) => l.name.toLowerCase() === name.toLowerCase());
        // Only if nothing has set the language in the meantime (a loaded
        // URL, or the user).
        if (exact && selectedItem() === null && selectedLang().name === ENGLISH.name)
          setSelectedLang(exact);
      })
      .catch(() => {});
  }
}
// Language ids are positions in the server's compiled language table, so they
// change when the Wiktionary data is refreshed; codes are stable. Remembered
// languages (and the English fallback) are re-resolved on load, by code (or by
// name, for languages remembered before codes were).
async function resolveStoredLang(lang: Lang): Promise<Lang | null> {
  try {
    if (lang.code) return await fetchLang(lang.code);
    const r = await fetch(`${API}/search/lang?name=${encodeURIComponent(lang.name)}`);
    const langs = (await r.json()) as Lang[];
    return langs.find((l) => l.name.toLowerCase() === lang.name.toLowerCase()) ?? null;
  } catch {
    return null;
  }
}
{
  const initial = selectedLang();
  resolveStoredLang(initial).then((lang) => {
    // Only if nothing has set the language in the meantime.
    if (lang && lang.id !== initial.id && selectedLang().id === initial.id)
      setSelectedLang(lang);
  });
  const initialDesc = selectedDescLangs();
  if (initialDesc.length)
    Promise.all(initialDesc.map(resolveStoredLang)).then((langs) => {
      if (selectedDescLangs() !== initialDesc) return;
      const resolved = langs.filter((l): l is Lang => l !== null);
      if (resolved.some((l, i) => l.id !== initialDesc[i]?.id) || resolved.length !== initialDesc.length)
        setSelectedDescLangs(resolved);
    });
}
createRoot(() => {
  createEffect(() => saveLang(selectedLang()));
  createEffect(() => saveDescLangs(selectedDescLangs()));
  createEffect(() => saveKind(selectedTreeKind()));
});
export const [tree, setTree] = createSignal<Etymology | Descendants[] | null>(
  null
);
export const [lastRequest, setLastRequest] = createSignal<TreeRequest | null>(
  null
);
// The cognates view's trail and sources (the tree itself is in `tree`).
export const [cognates, setCognates] = createSignal<
  Omit<Cognates, "tree" | "total" | "allLangs"> | null
>(null);
// A descendants or cognates tree's unfiltered size, and whether it is drawn in
// every language.
export const [treeSize, setTreeSize] = createSignal<{ total: number; allLangs: boolean } | null>(null);
export const [loading, setLoading] = createSignal(false);
export const [loadError, setLoadError] = createSignal<string | null>(null);
// Every item sharing the shown word's language and term (Wiktionary's
// numbered etymologies), for switching between them.
export const [homographs, setHomographs] = createSignal<Item[]>([]);
// A URL that names no single word: several homographs and no number to pick
// one ("ambiguous"), or nothing at all ("missing").
export const [unresolved, setUnresolved] = createSignal<{
  link: WordLink;
  reason: "ambiguous" | "missing";
} | null>(null);

// Display settings (all applied client-side, never re-fetch)
export const [disabledEtyModes, setDisabledEtyModes] = createSignal<
  Set<string>
>(new Set());
export const [hiddenLangs, setHiddenLangs] = createSignal<Set<number>>(
  new Set()
);
// On where there is room for them.
export const [showGlosses, setShowGlosses] = createSignal(
  window.matchMedia("(min-width: 700px)").matches
);

// The display forest: the fetched tree with filters applied. When only a
// filter changes, open/closed state is carried over; a new tree gets the
// default collapse policy.
let prevForest: Forest | null = null;
let prevTree: Etymology | Descendants[] | null = null;
export const forest = createMemo<Forest | null>(() => {
  const t = tree();
  const request = lastRequest();
  if (t === null || request === null) return null;
  const filters = {
    disabledEtyModes: disabledEtyModes(),
    hiddenLangs: hiddenLangs(),
  };
  const sameTree = t === prevTree && prevForest !== null;
  const f = untrack(() =>
    buildForest(
      request.kind,
      t,
      request.item.id,
      filters,
      sameTree ? snapshotOpen(prevForest!) : undefined
    )
  );
  if (!sameTree) applyDefaultOpen(f);
  prevForest?.dispose();
  prevForest = f;
  prevTree = t;
  return f;
});

// Languages of the fetched (unfiltered) tree with counts, for the settings
// sidebar; stable across filter changes so hidden languages stay in place.
export const treeLangs = createMemo<LangCount[]>(() => {
  const t = tree();
  if (t === null) return [];
  const counts = new Map<number, LangCount>();
  const visit = (item: Item, kids: () => void) => {
    const lc = counts.get(item.lang.id);
    if (lc) lc.count++;
    else counts.set(item.lang.id, { lang: item.lang, count: 1 });
    kids();
  };
  const visitD = (d: Descendants) => visit(d.item, () => d.children.forEach(visitD));
  const visitE = (e: Etymology) => visit(e.item, () => e.parents.forEach(visitE));
  if (Array.isArray(t)) t.forEach(visitD);
  else visitE(t);
  return [...counts.values()].sort(
    (a, b) => b.count - a.count || a.lang.name.localeCompare(b.lang.name)
  );
});

// Connection modes in the fetched (unfiltered) tree with counts, for the
// filters; like `treeLangs`, stable across filter changes.
export const treeModes = createMemo<Map<string, number>>(() => {
  const t = tree();
  const counts = new Map<string, number>();
  if (t === null) return counts;
  const count = (mode: string | null) => mode && counts.set(mode, (counts.get(mode) ?? 0) + 1);
  const visitD = (d: Descendants) => {
    count(d.etyMode);
    d.children.forEach(visitD);
  };
  const visitE = (e: Etymology) => {
    if (e.parents.length) count(e.etyMode);
    e.parents.forEach(visitE);
  };
  if (Array.isArray(t)) t.forEach(visitD);
  else visitE(t);
  return counts;
});

// Debounce utility
export function debounce<T extends (...args: any[]) => any>(
  fn: T,
  delay: number
): T {
  let timer: number;
  return ((...args: any[]) => {
    clearTimeout(timer);
    timer = window.setTimeout(() => fn(...args), delay);
  }) as unknown as T;
}

// The graph build the server is serving, as of the last resolve. Numeric ids
// are only sent along with it; if the server has moved on (a deploy while the
// tab was open), reloading re-resolves the URL, which names words, not ids.
let graph = "";
function staleGraph(): never {
  window.location.reload();
  throw new Error("graph changed; reloading");
}

async function fetchJson(url: string, allowMissing = false): Promise<{ status: number; body: any }> {
  const r = await fetch(url);
  if (r.status === 409) staleGraph();
  if (!r.ok && !(allowMissing && r.status === 404))
    throw new Error(`Request failed (${r.status})`);
  return { status: r.status, body: await r.json().catch(() => null) };
}

interface Resolved {
  graph: string;
  item: Item | null;
  homographs: Item[];
}

const resolveCache = new Map<string, Promise<Resolved>>();
function resolveLink(link: WordLink): Promise<Resolved> {
  const params = new URLSearchParams({ lang: link.lang, term: link.term });
  if (link.ety) params.set("ety", link.ety);
  const key = params.toString();
  let p = resolveCache.get(key);
  if (!p) {
    // A missing word is a valid resolution result, sent with HTTP 404.
    p = fetchJson(`${API}/resolve?${key}`, true).then(({ body }) => body as Resolved);
    p.catch(() => resolveCache.delete(key));
    resolveCache.set(key, p);
  }
  return p;
}

function langsOfTree(t: Etymology | Descendants[], out: Map<string, Lang>) {
  const visitE = (e: Etymology) => {
    out.set(e.item.lang.code, e.item.lang);
    e.parents.forEach(visitE);
  };
  const visitD = (d: Descendants) => {
    out.set(d.item.lang.code, d.item.lang);
    d.children.forEach(visitD);
  };
  if (Array.isArray(t)) t.forEach(visitD);
  else visitE(t);
}

// The URL's descendant-language codes as languages: from the tree and the
// current selection where possible, else from the server. Unknown codes drop.
async function resolveDescLangs(
  codes: string[],
  treeResult: Etymology | Descendants[]
): Promise<Lang[]> {
  const known = new Map<string, Lang>();
  langsOfTree(treeResult, known);
  for (const l of [...selectedDescLangs(), selectedLang()]) if (l.code) known.set(l.code, l);
  const langs = await Promise.all(codes.map(async (c) => known.get(c) ?? (await fetchLang(c))));
  return langs.filter((l): l is Lang => l !== null);
}

// The URL is the truth: reflect the loaded request in the search box and
// view line. Etymology URLs carry no descendant languages, so those keep
// the current set.
function applySearchState(item: Item, descLangs: Lang[], kind: TreeKind) {
  setSelectedLang(item.lang);
  setSelectedItem(item);
  // Keep the array identity when nothing changed, so dependents don't churn.
  if (descLangs.length > 0 && !sameLangs(descLangs, selectedDescLangs()))
    setSelectedDescLangs(descLangs);
  setSelectedTreeKind(kind);
}

function sameLangs(a: Lang[], b: Lang[]): boolean {
  return a.length === b.length && a.every((l, i) => l.id === b[i].id);
}

const treeCache = new Map<string, Etymology | DescendantsView | Cognates>();

// Each load (and each return home) supersedes the ones before it, so a slow
// response for an earlier URL never replaces what the current one shows.
let loadSeq = 0;

// The home page: no word, no tree.
export function clearView() {
  loadSeq++;
  batch(() => {
    closePopover();
    setTree(null);
    setCognates(null);
    setTreeSize(null);
    setLastRequest(null);
    setUnresolved(null);
    setHomographs([]);
    setSelectedItem(null);
    setLoading(false);
    setLoadError(null);
  });
}

// Load the view a URL names. Returns the word's canonical URL (current code,
// displayed term, ordered languages) when `path` names it differently, for
// the caller to adopt.
export async function loadFromPath(path: string): Promise<string | null> {
  const link = parseWordHref(path);
  if (!link) return null;
  const seq = ++loadSeq;
  batch(() => {
    closePopover();
    setTree(null);
    setCognates(null);
    setTreeSize(null);
    setLastRequest(null);
    setUnresolved(null);
    setHomographs([]);
    setLoadError(null);
    setLoading(true);
  });
  // Never let a caller's tracking scope see the signal reads below.
  try {
    return await untrack(() => loadLink(link, seq));
  } catch {
    if (seq === loadSeq)
      setLoadError("Could not load this word. Check your connection and try again.");
    return null;
  } finally {
    if (seq === loadSeq) setLoading(false);
  }
}

async function loadLink(link: WordLink, seq: number): Promise<string | null> {
  const resolved = await resolveLink(link);
  if (seq !== loadSeq) return null;
  graph = resolved.graph;
  const item = resolved.item;
  if (!item) {
    const lang = resolved.homographs[0]?.lang ?? await fetchLang(link.lang);
    if (seq !== loadSeq) return null;
    batch(() => {
      setTree(null);
      setLastRequest(null);
      setHomographs(resolved.homographs);
      setUnresolved({ link, reason: resolved.homographs.length ? "ambiguous" : "missing" });
      setSelectedItem(null);
      if (lang) setSelectedLang(lang);
      setSelectedTreeKind(link.kind);
    });
    return null;
  }

  // Languages the link names, plus the word's own; by default the reader's
  // languages (remembered from the last choice).
  const named = link.kind !== TreeKind.Etymology && !link.allLangs && link.descLangs.length > 0;
  const codes =
    link.kind === TreeKind.Etymology || link.allLangs
      ? []
      : [...new Set([...(named ? link.descLangs : selectedDescLangs().map((l) => l.code)), item.lang.code])].sort();
  const params = new URLSearchParams({ graph });
  for (const c of codes) params.append("descLang", c);
  if (link.within) params.set("within", "true");
  if (link.kind === TreeKind.Cognates && link.root) {
    params.set("root", link.root);
    if (link.rootEty) params.set("rootEty", link.rootEty);
  }
  const apiPath = `/${kindPath[link.kind]}/${item.id}?${params}`;
  let body = treeCache.get(apiPath);
  if (!body) {
    body = (await fetchJson(`${API}${apiPath}`)).body as Etymology | DescendantsView | Cognates;
    treeCache.set(apiPath, body);
  }
  const cog = link.kind === TreeKind.Cognates ? (body as Cognates) : null;
  const view = link.kind === TreeKind.Etymology ? null : (body as DescendantsView | Cognates);
  const treeResult: Etymology | Descendants[] = view ? [view.tree] : (body as Etymology);

  const descLangs = named ? await resolveDescLangs(codes, treeResult) : [];
  if (seq !== loadSeq) return null;
  // The server falls back to the default root when the link names none it
  // knows; only a root other than the trail's last step goes in the URL.
  const root = cog && cog.root !== cog.defaultRoot ? cog.tree.item : null;
  const request = new TreeRequest(item.lang, item, descLangs, link.kind, root, link.allLangs, link.within);
  const previous = lastRequest();
  batch(() => {
    setUnresolved(null);
    setHomographs(resolved.homographs);
    // Hidden languages belong to the tree they were hidden in.
    if (previous?.item.id !== item.id) setHiddenLangs(new Set<number>());
    setTree(treeResult);
    setCognates(
      cog && {
        trail: cog.trail,
        sources: cog.sources,
        ultimateRoots: cog.ultimateRoots,
        ultimateOthers: cog.ultimateOthers,
        root: cog.root,
        defaultRoot: cog.defaultRoot,
      }
    );
    setTreeSize(view && { total: view.total, allLangs: view.allLangs });
    setLastRequest(request);
    applySearchState(item, descLangs, link.kind);
  });
  // Compared as links, not strings: the router may hand us `path` decoded.
  const canonical = parseWordHref(wordHref(link.kind, item, request.choice, root))!;
  return sameLink(canonical, link) ? null : linkHref(canonical);
}

function sameLink(a: WordLink, b: WordLink): boolean {
  return (
    a.kind === b.kind &&
    a.lang === b.lang &&
    a.term === b.term &&
    a.ety === b.ety &&
    a.allLangs === b.allLangs &&
    a.within === b.within &&
    a.root === b.root &&
    a.rootEty === b.rootEty &&
    a.descLangs.join(" ") === b.descLangs.join(" ")
  );
}

// Lazily fetched full items (for nodes whose glosses were left out).
const itemCache = new Map<number, Promise<Item>>();
export function fetchItem(id: number): Promise<Item> {
  let p = itemCache.get(id);
  if (!p) {
    p = fetchJson(`${API}/item/${id}?graph=${graph}`).then(({ body }) => body as Item);
    itemCache.set(id, p);
  }
  return p;
}
