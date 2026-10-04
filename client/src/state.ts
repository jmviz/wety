import {
  createSignal,
  createMemo,
  createEffect,
  createRoot,
  batch,
  untrack,
} from "solid-js";
import {
  Descendants,
  Etymology,
  Item,
  Lang,
  TreeKind,
  TreeRequest,
  WordLink,
  kindPath,
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
export const [showGlosses, setShowGlosses] = createSignal(false);

// The display forest: the fetched tree with filters applied. When only a
// filter changes, open/closed state is carried over; a new tree gets the
// default collapse policy.
let prevForest: Forest | null = null;
let prevTree: Etymology | Descendants[] | null = null;
export const forest = createMemo<Forest | null>(() => {
  const t = tree();
  const request = lastRequest();
  const filters = {
    disabledEtyModes: disabledEtyModes(),
    hiddenLangs: hiddenLangs(),
  };
  if (t === null || request === null) return null;
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

async function fetchJson(url: string): Promise<{ status: number; body: any }> {
  const r = await fetch(url);
  if (r.status === 409) staleGraph();
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
    p = fetchJson(`${API}/resolve?${key}`).then(({ body }) => body as Resolved);
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

const treeCache = new Map<string, Etymology | Descendants[]>();

// Load the view a URL names. Returns the word's canonical URL (current code
// and displayed term) when it differs from `path`, for the caller to adopt.
export async function loadFromPath(path: string): Promise<string | null> {
  const link = parseWordHref(path);
  if (!link) return null;
  // Never let a caller's tracking scope see the signal reads below.
  return untrack(() => loadLink(path, link));
}

async function loadLink(path: string, link: WordLink): Promise<string | null> {
  let resolved: Resolved;
  try {
    resolved = await resolveLink(link);
  } catch (error) {
    console.log(error);
    return null;
  }
  graph = resolved.graph;
  const item = resolved.item;
  if (!item) {
    batch(() => {
      setTree(null);
      setLastRequest(null);
      setHomographs(resolved.homographs);
      setUnresolved({ link, reason: resolved.homographs.length ? "ambiguous" : "missing" });
      if (resolved.homographs[0]) setSelectedLang(resolved.homographs[0].lang);
      setSelectedTreeKind(link.kind);
    });
    return null;
  }

  const codes = [...new Set(link.descLangs)].sort();
  const params = new URLSearchParams({ graph });
  for (const c of codes) params.append("descLang", c);
  const apiPath = `/${kindPath[link.kind]}/${item.id}?${params}`;
  let treeResult = treeCache.get(apiPath);
  if (!treeResult) {
    try {
      const { body } = await fetchJson(`${API}${apiPath}`);
      treeResult =
        link.kind === TreeKind.Etymology
          ? (body as Etymology)
          : link.kind === TreeKind.Descendants
            ? [body as Descendants]
            : (body as Descendants[]);
      treeCache.set(apiPath, treeResult);
    } catch (error) {
      console.log(error);
      return null;
    }
  }

  const descLangs = await resolveDescLangs(codes, treeResult);
  const result = treeResult;
  const request = new TreeRequest(item.lang, item, descLangs, link.kind);
  batch(() => {
    setUnresolved(null);
    setHomographs(resolved.homographs);
    setTree(result);
    setLastRequest(request);
    applySearchState(item, descLangs, link.kind);
  });
  const canonical = wordHref(link.kind, item, request.descLangs);
  return canonical !== path ? canonical : null;
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
