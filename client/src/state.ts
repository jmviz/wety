import { createSignal, createMemo, batch } from "solid-js";
import {
  Descendants,
  Etymology,
  InterLangDescendants,
  Item,
  Lang,
  TreeKind,
  TreeRequest,
} from "./search/types";
import {
  filterEtymologyTree,
  filterDescendantsTree,
} from "./settings/filterTree";
import { interLangDescendants } from "./ety/DescendantsTree";

// Global application state
export const [selectedLang, setSelectedLang] = createSignal<Lang | null>(null);
export const [selectedItem, setSelectedItem] = createSignal<Item | null>(null);
export const [selectedDescLangs, setSelectedDescLangs] = createSignal<Lang[]>(
  []
);
export const [selectedTreeKind, setSelectedTreeKind] = createSignal<TreeKind>(
  TreeKind.Cognates
);
export const [tree, setTree] = createSignal<
  Etymology | InterLangDescendants[] | null
>(null);
export const [lastRequest, setLastRequest] = createSignal<TreeRequest | null>(
  null
);
export const [disabledEtyModes, setDisabledEtyModes] = createSignal<
  Set<string>
>(new Set());

// Tree cache
const treeCache = new Map<string, Etymology | InterLangDescendants[]>();

// Filtered tree (derived from tree + disabledEtyModes)
export const filteredTree = createMemo(() => {
  const t = tree();
  const disabled = disabledEtyModes();
  if (t === null || disabled.size === 0) return t;
  if (lastRequest()?.kind === TreeKind.Etymology) {
    return filterEtymologyTree(t as Etymology, disabled);
  }
  return (t as InterLangDescendants[]).map((x) =>
    filterDescendantsTree(x, disabled)
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

// Find an item by ID within an InterLangDescendants tree
function findItemById(
  nodes: InterLangDescendants[],
  itemId: number
): Item | null {
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
  nodes: InterLangDescendants[],
  out: Map<number, Lang>
) {
  for (const node of nodes) {
    out.set(node.item.lang.id, node.item.lang);
    collectLangsFromDescendants(node.children, out);
  }
}

function resolveDescLangs(
  descLangIds: number[],
  treeResult: Etymology | InterLangDescendants[],
  kind: TreeKind
): Lang[] {
  const langMap = new Map<number, Lang>();
  if (kind === TreeKind.Etymology) {
    collectLangsFromEtymology(treeResult as Etymology, langMap);
  } else {
    collectLangsFromDescendants(
      treeResult as InterLangDescendants[],
      langMap
    );
  }
  return descLangIds.map(
    (id) => langMap.get(id) ?? { id, name: `#${id}` }
  );
}

// Apply search field state from loaded tree data (only fills empty fields)
function applySearchState(
  rootItem: Item,
  descLangs: Lang[],
  kind: TreeKind
) {
  if (!selectedLang()) setSelectedLang(rootItem.lang);
  if (!selectedItem()) setSelectedItem(rootItem);
  if (selectedDescLangs().length === 0) setSelectedDescLangs(descLangs);
  setSelectedTreeKind(kind);
}

// Load tree data from a URL path
export async function loadFromPath(path: string) {
  const parsed = TreeRequest.parsePath(path);
  if (!parsed) return;

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

  const cached = treeCache.get(path);
  if (cached) {
    const rootItem =
      parsed.kind === TreeKind.Etymology
        ? (cached as Etymology).item
        : parsed.kind === TreeKind.Cognates
          ? (findItemById(cached as InterLangDescendants[], parsed.itemId) ??
            dummyItem())
          : (cached as InterLangDescendants[])[0]?.item ?? dummyItem();
    const descLangs = resolveDescLangs(parsed.descLangIds, cached, parsed.kind);
    batch(() => {
      setTree(cached);
      setLastRequest(
        new TreeRequest(rootItem.lang, rootItem, descLangs, parsed.kind)
      );
      applySearchState(rootItem, descLangs, parsed.kind);
    });
    return;
  }

  try {
    const response = await fetch(
      `${import.meta.env.VITE_API_BASE_URL}${path}`
    );
    const data = await response.json();

    let treeResult: Etymology | InterLangDescendants[];
    let rootItem: Item;

    switch (parsed.kind) {
      case TreeKind.Etymology: {
        treeResult = data as Etymology;
        rootItem = (treeResult as Etymology).item;
        break;
      }
      case TreeKind.Descendants: {
        treeResult = [interLangDescendants(data as Descendants)];
        rootItem = (treeResult as InterLangDescendants[])[0].item;
        break;
      }
      case TreeKind.Cognates: {
        treeResult = (data as Descendants[]).map((t) =>
          interLangDescendants(t)
        );
        rootItem =
          findItemById(treeResult as InterLangDescendants[], parsed.itemId) ??
          dummyItem();
        break;
      }
    }

    const descLangs = resolveDescLangs(
      parsed.descLangIds,
      treeResult,
      parsed.kind
    );
    treeCache.set(path, treeResult);
    batch(() => {
      setTree(treeResult);
      setLastRequest(
        new TreeRequest(rootItem.lang, rootItem, descLangs, parsed.kind)
      );
      applySearchState(rootItem, descLangs, parsed.kind);
    });
  } catch (error) {
    console.log(error);
  }
}
