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
    fetch(`${import.meta.env.VITE_API_BASE_URL}/search/lang?name=${encodeURIComponent(name)}`)
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
// change when the Wiktionary data is refreshed; names are stable. Remembered
// languages (and the English fallback) are re-resolved by name on load.
async function resolveByName(lang: Lang): Promise<Lang | null> {
  try {
    const r = await fetch(
      `${import.meta.env.VITE_API_BASE_URL}/search/lang?name=${encodeURIComponent(lang.name)}`
    );
    const langs = (await r.json()) as Lang[];
    return langs.find((l) => l.name.toLowerCase() === lang.name.toLowerCase()) ?? null;
  } catch {
    return null;
  }
}
{
  const initial = selectedLang();
  resolveByName(initial).then((lang) => {
    // Only if nothing has set the language in the meantime.
    if (lang && lang.id !== initial.id && selectedLang().id === initial.id)
      setSelectedLang(lang);
  });
  const initialDesc = selectedDescLangs();
  if (initialDesc.length)
    Promise.all(initialDesc.map(resolveByName)).then((langs) => {
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

// Display settings (all applied client-side, never re-fetch)
export const [disabledEtyModes, setDisabledEtyModes] = createSignal<
  Set<string>
>(new Set());
export const [hiddenLangs, setHiddenLangs] = createSignal<Set<number>>(
  new Set()
);
export const [showGlosses, setShowGlosses] = createSignal(false);

// Tree cache
const treeCache = new Map<string, Etymology | Descendants[]>();

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

// Find an item by ID within a Descendants forest
function findItemById(nodes: Descendants[], itemId: number): Item | null {
  for (const node of nodes) {
    if (node.item.id === itemId) return node.item;
    const found = findItemById(node.children, itemId);
    if (found) return found;
  }
  return null;
}

// Collect all langs from a tree (for resolving descLang IDs to names)
function collectLangsFromEtymology(node: Etymology, out: Map<number, Lang>) {
  out.set(node.item.lang.id, node.item.lang);
  for (const p of node.parents) collectLangsFromEtymology(p, out);
}

function collectLangsFromDescendants(
  nodes: Descendants[],
  out: Map<number, Lang>
) {
  for (const node of nodes) {
    out.set(node.item.lang.id, node.item.lang);
    collectLangsFromDescendants(node.children, out);
  }
}

// Names for the URL's descendant-language ids: from the current selection
// first, then the tree, then the server for anything left
// (a cold-loaded link naming a language that doesn't occur in the tree).
async function resolveDescLangs(
  descLangIds: number[],
  treeResult: Etymology | Descendants[],
  kind: TreeKind
): Promise<Lang[]> {
  const langMap = new Map<number, Lang>();
  if (kind === TreeKind.Etymology) {
    collectLangsFromEtymology(treeResult as Etymology, langMap);
  } else {
    collectLangsFromDescendants(treeResult as Descendants[], langMap);
  }
  for (const l of [...selectedDescLangs(), selectedLang()])
    langMap.set(l.id, l);
  return Promise.all(
    descLangIds.map(
      async (id) =>
        langMap.get(id) ??
        (await fetch(`${import.meta.env.VITE_API_BASE_URL}/lang/${id}`)
          .then((r) => (r.ok ? (r.json() as Promise<Lang>) : null))
          .catch(() => null)) ??
        { id, name: `#${id}` }
    )
  );
}

// The URL is the truth: reflect the loaded request in the search box and
// view line. Etymology URLs carry no descendant languages, so those keep
// the current set.
function applySearchState(
  itemFromResult: Item,
  descLangs: Lang[],
  kind: TreeKind
) {
  setSelectedLang(itemFromResult.lang);
  setSelectedItem(itemFromResult);
  // Keep the array identity when nothing changed, so dependents don't churn.
  if (descLangs.length > 0 && !sameLangs(descLangs, selectedDescLangs()))
    setSelectedDescLangs(descLangs);
  setSelectedTreeKind(kind);
}

function sameLangs(a: Lang[], b: Lang[]): boolean {
  return a.length === b.length && a.every((l, i) => l.id === b[i].id);
}

function itemFromTree(
  parsed: { kind: TreeKind; itemId: number; distLangId: number },
  treeResult: Etymology | Descendants[]
): Item {
  const dummyItem = (): Item => ({
    id: parsed.itemId,
    etyNum: 0,
    lang: { id: parsed.distLangId, name: "" },
    term: "",
    imputed: false,
    reconstructed: false,
    url: null,
    pos: null,
    gloss: null,
    romanization: null,
  });
  switch (parsed.kind) {
    case TreeKind.Etymology:
      return (treeResult as Etymology).item;
    case TreeKind.Descendants:
      return (treeResult as Descendants[])[0]?.item ?? dummyItem();
    case TreeKind.Cognates:
      return (
        findItemById(treeResult as Descendants[], parsed.itemId) ?? dummyItem()
      );
  }
}

// Load tree data from a URL path
export async function loadFromPath(path: string) {
  const parsed = TreeRequest.parsePath(path);
  if (!parsed) return;
  // Never let a caller's tracking scope see the signal reads below.
  return untrack(() => loadFromPathInner(path, parsed));
}

async function loadFromPathInner(
  path: string,
  parsed: NonNullable<ReturnType<typeof TreeRequest.parsePath>>
) {

  let treeResult = treeCache.get(path);
  if (!treeResult) {
    try {
      const response = await fetch(
        `${import.meta.env.VITE_API_BASE_URL}${path}`
      );
      const data = await response.json();
      treeResult =
        parsed.kind === TreeKind.Etymology
          ? (data as Etymology)
          : parsed.kind === TreeKind.Descendants
            ? [data as Descendants]
            : (data as Descendants[]);
      treeCache.set(path, treeResult);
    } catch (error) {
      console.log(error);
      return;
    }
  }

  const itemFromResult = itemFromTree(parsed, treeResult);
  const descLangs = await resolveDescLangs(
    parsed.descLangIds,
    treeResult,
    parsed.kind
  );
  const result = treeResult;
  batch(() => {
    setTree(result);
    setLastRequest(
      new TreeRequest(
        itemFromResult.lang,
        itemFromResult,
        descLangs,
        parsed.kind
      )
    );
    applySearchState(itemFromResult, descLangs, parsed.kind);
  });
}

// Lazily fetched full items (for trees requested with `lean=true`).
const itemCache = new Map<number, Promise<Item>>();
export function fetchItem(id: number): Promise<Item> {
  let p = itemCache.get(id);
  if (!p) {
    p = fetch(`${import.meta.env.VITE_API_BASE_URL}/item/${id}`).then((r) =>
      r.json()
    );
    itemCache.set(id, p);
  }
  return p;
}
