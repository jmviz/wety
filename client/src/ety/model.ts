// A display model shared by the descendants outline and the etymology chart.
//
// The API returns plain JSON trees (`Descendants` / `Etymology`); this module
// turns them into `TreeNode`s that carry everything the renderers need:
// stable keys, parent links, per-subtree counts and language summaries, a
// stable display order, the path to the searched item, and a per-node
// open/closed signal. Client-side filters (etymology modes, hidden languages)
// are applied here, so switching a filter never re-fetches.

import {
  Accessor,
  Setter,
  batch,
  createMemo,
  createRoot,
  createSignal,
} from "solid-js";
import {
  Descendants,
  Etymology,
  Item,
  Lang,
  OtherParent,
  TreeKind,
} from "../search/types";

export interface LangCount {
  lang: Lang;
  count: number;
}

export interface TreeNode {
  key: number;
  item: Item;
  langDistance: number;
  // How this node derives from its parent (outline) / from its `children`,
  // which are its etymological parents (chart).
  etyMode: string | null;
  otherParents: OtherParent[];
  // Position of the tree parent among all parents of this item (outline).
  parentEtyOrder: number | null;
  // Position of this item among its child's parents (chart).
  etyOrder: number | null;
  parent: TreeNode | null;
  children: TreeNode[];
  depth: number;
  // Nodes strictly below this one, after filtering.
  descendantCount: number;
  // Languages among the descendants, most frequent first.
  langCounts: LangCount[];
  sameLangAsParent: boolean;
  // True when one or more hidden-language nodes were removed between this
  // node and its displayed parent.
  hoisted: boolean;
  onSelectedPath: boolean;
  isSelected: boolean;
  open: Accessor<boolean>;
  setOpen: Setter<boolean>;
  // Rows this node currently occupies on screen (itself plus its visible
  // descendants); drives the `contain-intrinsic-size` hint.
  visibleRows: Accessor<number>;
}

export interface Forest {
  kind: TreeKind;
  roots: TreeNode[];
  byKey: Map<number, TreeNode>;
  nodeCount: number;
  langs: LangCount[];
  selected: TreeNode | null;
  // Releases the per-node memos; call when the forest is replaced.
  dispose: () => void;
}

export interface TreeFilters {
  disabledEtyModes: Set<string>;
  hiddenLangs: Set<number>;
}

interface RawNode {
  item: Item;
  langDistance: number;
  etyMode: string | null;
  otherParents: OtherParent[];
  parentEtyOrder: number | null;
  etyOrder: number | null;
  children: RawNode[];
  hoisted: boolean;
}

function rawFromDescendants(d: Descendants): RawNode {
  return {
    item: d.item,
    langDistance: d.langDistance,
    etyMode: d.etyMode,
    otherParents: d.otherParents ?? [],
    parentEtyOrder: d.parentEtyOrder,
    etyOrder: null,
    children: d.children.map(rawFromDescendants),
    hoisted: false,
  };
}

function rawFromEtymology(e: Etymology): RawNode {
  return {
    item: e.item,
    langDistance: e.langDistance,
    etyMode: e.etyMode,
    otherParents: [],
    parentEtyOrder: null,
    etyOrder: e.etyOrder,
    children: e.parents.map(rawFromEtymology),
    hoisted: false,
  };
}

// Drop edges whose mode is disabled (the whole subtree goes with them, as
// before) and splice out nodes in hidden languages, hoisting their children.
function applyFilters(nodes: RawNode[], filters: TreeFilters): RawNode[] {
  const out: RawNode[] = [];
  for (const n of nodes) {
    if (n.etyMode && filters.disabledEtyModes.has(n.etyMode)) continue;
    if (filters.hiddenLangs.has(n.item.lang.id)) {
      for (const c of applyFilters(n.children, filters)) {
        c.hoisted = true;
        out.push(c);
      }
    } else {
      n.children = applyFilters(n.children, filters);
      out.push(n);
    }
  }
  return out;
}

// Identity that survives re-filtering, so open/closed state can be carried
// over when the forest is rebuilt for a new filter.
export function nodeIdent(node: TreeNode): string {
  return node.parent ? `${node.parent.item.id}>${node.item.id}` : `${node.item.id}`;
}

export function snapshotOpen(forest: Forest): Map<string, boolean> {
  const snap = new Map<string, boolean>();
  for (const n of walk(forest.roots)) if (n.children.length) snap.set(nodeIdent(n), n.open());
  return snap;
}

export function buildForest(
  kind: TreeKind,
  tree: Etymology | Descendants[],
  selectedItemId: number | null,
  filters: TreeFilters,
  prevOpen?: Map<string, boolean>
): Forest {
  const rawRoots = applyFilters(
    Array.isArray(tree)
      ? tree.map(rawFromDescendants)
      : [rawFromEtymology(tree)],
    filters
  );

  const byKey = new Map<number, TreeNode>();
  const langTotals = new Map<number, LangCount>();
  let nextKey = 0;
  let selected: TreeNode | null = null;

  // Node memos need an owner so they can be disposed with the forest.
  return createRoot((dispose) => {
  function build(raw: RawNode, parent: TreeNode | null, depth: number): TreeNode {
    const [open, setOpen] = createSignal(false);
    const node: TreeNode = {
      key: nextKey++,
      item: raw.item,
      langDistance: raw.langDistance,
      etyMode: raw.etyMode,
      otherParents: raw.otherParents,
      parentEtyOrder: raw.parentEtyOrder,
      etyOrder: raw.etyOrder,
      parent,
      children: [],
      depth,
      descendantCount: 0,
      langCounts: [],
      sameLangAsParent:
        parent !== null && parent.item.lang.id === raw.item.lang.id,
      hoisted: raw.hoisted,
      onSelectedPath: false,
      isSelected: false,
      open,
      setOpen,
      visibleRows: () => 1,
    };
    byKey.set(node.key, node);

    const total = langTotals.get(raw.item.lang.id);
    if (total) total.count++;
    else langTotals.set(raw.item.lang.id, { lang: raw.item.lang, count: 1 });

    if (selected === null && raw.item.id === selectedItemId) {
      selected = node;
      node.isSelected = true;
      for (let a: TreeNode | null = node; a; a = a.parent) a.onSelectedPath = true;
    }

    const langCounts = new Map<number, LangCount>();
    for (const rawChild of raw.children) {
      const child = build(rawChild, node, depth + 1);
      node.children.push(child);
      node.descendantCount += 1 + child.descendantCount;
      bump(langCounts, child.item.lang, 1);
      for (const lc of child.langCounts) bump(langCounts, lc.lang, lc.count);
    }
    node.langCounts = [...langCounts.values()].sort(
      (a, b) => b.count - a.count || a.lang.name.localeCompare(b.lang.name)
    );

    node.children.sort(kind === TreeKind.Etymology ? byEtyOrder : byDisplay);
    if (node.children.length) {
      node.visibleRows = createMemo(() =>
        open() ? 1 + node.children.reduce((sum, c) => sum + c.visibleRows(), 0) : 1
      );
      if (prevOpen) {
        const was = prevOpen.get(nodeIdent(node));
        if (was !== undefined) setOpen(was);
      }
    }
    return node;
  }

  const roots = rawRoots.map((r) => build(r, null, 0));
  roots.sort(kind === TreeKind.Etymology ? byEtyOrder : byDisplay);

  return {
    kind,
    roots,
    byKey,
    nodeCount: byKey.size,
    langs: [...langTotals.values()].sort(
      (a, b) => b.count - a.count || a.lang.name.localeCompare(b.lang.name)
    ),
    selected,
    dispose,
  };
  });
}

function bump(map: Map<number, LangCount>, lang: Lang, n: number) {
  const lc = map.get(lang.id);
  if (lc) lc.count += n;
  else map.set(lang.id, { lang, count: n });
}

// Stable display order for descendants: the searched item's ancestry first,
// then closest languages, then the biggest subtrees, then alphabetical.
function byDisplay(a: TreeNode, b: TreeNode): number {
  return (
    +b.onSelectedPath - +a.onSelectedPath ||
    a.langDistance - b.langDistance ||
    b.descendantCount - a.descendantCount ||
    a.item.lang.name.localeCompare(b.item.lang.name) ||
    a.item.term.localeCompare(b.item.term)
  );
}

function byEtyOrder(a: TreeNode, b: TreeNode): number {
  return (a.etyOrder ?? 0) - (b.etyOrder ?? 0);
}

// ---------------------------------------------------------------------------
// Open/closed state

export function* walk(nodes: TreeNode[]): Generator<TreeNode> {
  for (const n of nodes) {
    yield n;
    yield* walk(n.children);
  }
}

// Bulk changes are applied in batched slices, one slice per frame, so a
// very large tree never blocks the main thread for more than a frame or two
// (Firefox in particular is slow to toggle thousands of <details> at once).
const SLICE = 300;
let bulkToken = 0;
function applyInSlices(nodes: TreeNode[], open: boolean) {
  const token = ++bulkToken;
  let i = 0;
  const step = () => {
    if (token !== bulkToken) return;
    batch(() => {
      for (const end = Math.min(i + SLICE, nodes.length); i < end; i++)
        nodes[i].setOpen(open);
    });
    if (i < nodes.length) requestAnimationFrame(step);
  };
  step();
}

export function setAllOpen(forest: Forest, open: boolean) {
  const branches: TreeNode[] = [];
  for (const n of walk(forest.roots)) if (n.children.length) branches.push(n);
  // Opening top-down shows progress from the roots; closing bottom-up keeps
  // the visible part of the page stable until the end.
  applyInSlices(open ? branches : branches.reverse(), open);
}

// Open every node on the way to every item in `langId`.
export function openLang(forest: Forest, langId: number) {
  const toOpen = new Set<TreeNode>();
  for (const n of walk(forest.roots)) {
    if (n.item.lang.id !== langId) continue;
    for (let a = n.parent; a && !toOpen.has(a); a = a.parent) toOpen.add(a);
  }
  applyInSlices([...toOpen], true);
}

export function openPathTo(node: TreeNode) {
  batch(() => {
    for (let a = node.parent; a; a = a.parent) a.setOpen(true);
  });
}

// Default collapse policy: always show the path to the searched item and its
// direct descendants; then open the rest level by level (nearest the roots
// first) while the number of visible rows stays within `budget`.
export function applyDefaultOpen(forest: Forest, budget = 200) {
  let visible = forest.roots.length;
  const opened = new Set<number>();
  const open = (n: TreeNode) => {
    n.setOpen(true);
    opened.add(n.key);
    visible += n.children.length;
  };

  if (forest.selected) {
    const path: TreeNode[] = [];
    for (let a: TreeNode | null = forest.selected; a; a = a.parent) path.push(a);
    for (const n of path.reverse()) if (n.children.length) open(n);
  }

  const queue = [...forest.roots];
  while (queue.length) {
    const n = queue.shift()!;
    if (!n.children.length) continue;
    if (!opened.has(n.key)) {
      if (visible + n.children.length > budget) {
        n.setOpen(false);
        continue;
      }
      open(n);
    }
    queue.push(...n.children);
  }
}

export function visibleRowCount(forest: Forest): number {
  let count = 0;
  const visit = (n: TreeNode) => {
    count++;
    if (n.open()) n.children.forEach(visit);
  };
  forest.roots.forEach(visit);
  return count;
}

// ---------------------------------------------------------------------------
// Etymology line: "borrowed from Latin status + Latin -ālis"

export interface EtySource {
  item: Item;
  langDistance: number;
}

// All etymological parents of an outline node, in wiktionary order: the tree
// parent spliced in among `otherParents` at `parentEtyOrder`.
export function outlineSources(node: TreeNode): EtySource[] {
  const sources: EtySource[] = node.otherParents
    .slice()
    .sort((a, b) => a.etyOrder - b.etyOrder)
    .map((p) => ({ item: p.item, langDistance: p.langDistance }));
  if (node.parent && node.parentEtyOrder !== null) {
    sources.splice(node.parentEtyOrder, 0, {
      item: node.parent.item,
      langDistance: node.parent.langDistance,
    });
  }
  return sources;
}

// Parents of a chart (etymology) node are its `children`.
export function chartSources(node: TreeNode): EtySource[] {
  return node.children.map((c) => ({ item: c.item, langDistance: c.langDistance }));
}

export function sourcesOf(forest: Forest, node: TreeNode): EtySource[] {
  return forest.kind === TreeKind.Etymology
    ? chartSources(node)
    : outlineSources(node);
}
