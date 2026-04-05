import { signal, computed } from "@preact/signals";
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
export const selectedLang = signal<Lang | null>(null);
export const selectedItem = signal<Item | null>(null);
export const selectedDescLangs = signal<Lang[]>([]);
export const selectedTreeKind = signal<TreeKind>(TreeKind.Cognates);
export const tree = signal<Etymology | InterLangDescendants[] | null>(null);
export const lastRequest = signal<TreeRequest | null>(null);
export const disabledEtyModes = signal<Set<string>>(new Set());

// Tree cache
const treeCache = new Map<string, Etymology | InterLangDescendants[]>();

// Filtered tree (computed from tree + disabledEtyModes)
export const filteredTree = computed(() => {
  const t = tree.value;
  const disabled = disabledEtyModes.value;
  if (t === null || disabled.size === 0) return t;
  if (lastRequest.value?.kind === TreeKind.Etymology) {
    return filterEtymologyTree(t as Etymology, disabled);
  }
  return (t as InterLangDescendants[]).map((x) =>
    filterDescendantsTree(x, disabled)
  );
});

// Location / routing
export const locationPath = signal(
  window.location.pathname + window.location.search
);

export function navigate(path: string) {
  window.history.pushState(null, "", path);
  locationPath.value = path;
}

window.addEventListener("popstate", () => {
  locationPath.value = window.location.pathname + window.location.search;
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

// Load tree data from a URL path
export async function loadFromPath(path: string) {
  const parsed = TreeRequest.parsePath(path);
  if (!parsed) return;

  const makeRequest = (rootItem: Item) => {
    const descLangs: Lang[] = parsed.descLangIds.map((id) => ({
      id,
      name: "",
    }));
    return new TreeRequest(rootItem.lang, rootItem, descLangs, parsed.kind);
  };

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
        : (cached as InterLangDescendants[])[0]?.item ?? dummyItem();
    tree.value = cached;
    lastRequest.value = makeRequest(rootItem);
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
        treeResult = (data as Descendants[]).map((t) => interLangDescendants(t));
        rootItem =
          (treeResult as InterLangDescendants[])[0]?.item ?? dummyItem();
        break;
      }
    }

    treeCache.set(path, treeResult);
    tree.value = treeResult;
    lastRequest.value = makeRequest(rootItem);
  } catch (error) {
    console.log(error);
  }
}
