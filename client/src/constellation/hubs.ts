// The constellation's open hubs: the root, drawn as a disc in the middle,
// and each head the reader opened, which is a row in the blob it was opened
// in (`via`) and has no shape of its own. Pure model: the component keeps the
// list in a signal and draws it.

import type { TreeNode } from "../ety/model";
import type { Lang } from "../search/types";
import {
  canOpen,
  isUndefinedRoot,
  rackOrder,
  restrictBranches,
  singleEndpoint,
  type Branch,
  type Group,
} from "./branches.ts";
import { canFinishTogether, linearGroups, unbranchedLineage } from "./expansion.ts";

export interface DisplayBranch extends Branch {
  // For a preview column shared by sibling sources: which hub each group came
  // from, and every hub feeding each head.
  origins?: Map<Group, Hub>;
  parents?: Map<number, Set<string>>;
}

// A preview row's breakout: its own identity and the modern words it was
// opened for, kept through every later language stage.
export interface Scope {
  id: string;
  words: ReadonlySet<string>;
}

export interface Hub {
  id: string;
  node: TreeNode;
  parent: Hub | null;
  // The blob it is a row of (`blobId`); null for the root.
  via: string | null;
  branches: DisplayBranch[];
  // Sibling sources whose language preview this hub's words were merged into.
  shared: { host: Hub; branch: DisplayBranch }[];
  // Which click opened it; later clicks sit nearer the remaining preview.
  revealed: number;
  // The colour of the branch it was opened from, which its own branches keep.
  hue: string | null;
  hues: Map<string, string>;
  // A head with one word under it: the word stands alone beside the head's
  // row, named as the row is, in a blob as wide as the rack and as tall as
  // the row.
  single: boolean;
  scope?: Scope;
}

export type BranchReader = (node: TreeNode) => Branch[];

export const blobId = (hub: Hub, branch: Branch) => `${hub.id}/${branch.key}`;

// One colour per branch of the root, as "r,g,b".
export const PALETTE = [
  "125,211,252",
  "251,191,119",
  "196,181,253",
  "134,239,172",
  "249,168,212",
  "94,234,212",
  "253,164,145",
  "190,242,100",
];

// The branches met most often keep one colour whatever word was searched.
const FIXED: Record<string, number> = {
  "gem-pro": 0,
  "itc-pro": 1,
  "grk-pro": 2,
  "cel-pro": 3,
  "iir-pro": 4,
  "ine-bsl-pro": 5,
};

// A colour for each of a hub's branches: its fixed one, else whichever is
// still free, starting from one that depends only on the branch.
export function huesOf(keys: string[]): Map<string, string> {
  const taken = new Set(keys.map((k) => FIXED[k]).filter((s) => s !== undefined));
  const hues = new Map<string, string>();
  for (const key of keys) {
    let slot = FIXED[key];
    if (slot === undefined) {
      let h = 0;
      for (const ch of key) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
      slot = h % PALETTE.length;
      for (let i = 0; i < PALETTE.length && taken.has(slot); i++) slot = (slot + 1) % PALETTE.length;
      taken.add(slot);
    }
    hues.set(key, PALETTE[slot]);
  }
  return hues;
}

// Not a head with no name (branches.ts `unnamed`): the one word under it is
// the only name its line has, and stays in its own branch's blob.
const isSingle = (node: TreeNode, parent: Hub | null, branches: Branch[]) =>
  !!parent && !isUndefinedRoot(node) && singleEndpoint(branches);

// A branch's colour. Every hub has one for each of its branches (`makeHub`,
// `extendHub`); a missing one is a bug, not something to draw uncoloured.
export function hueOf(hub: Hub, branch: Branch): string {
  const hue = hub.hues.get(branch.key);
  if (!hue) throw new Error(`constellation: hub ${hub.id} has no colour for branch ${branch.key}`);
  return hue;
}

export function makeHub(
  branchesFor: BranchReader,
  node: TreeNode,
  parent: Hub | null,
  via: string | null,
  hue: string | null,
  scope = parent?.scope
): Hub {
  const available = scope ? restrictBranches(branchesFor(node), scope.words) : branchesFor(node);
  const branches = available.map((b) => ({ ...b, groups: [...b.groups], words: [...b.words] }));
  const keys = branches.map((b) => b.key);
  return {
    id: scope ? `${node.item.id}@${scope.id}` : String(node.item.id),
    scope,
    node,
    parent,
    via,
    branches,
    shared: [],
    revealed: 0,
    hue,
    hues: hue ? new Map(keys.map((k) => [k, hue])) : huesOf(keys),
    single: isSingle(node, parent, branches),
  };
}

// Sibling sources opened by one click that feed the same language share one
// preview column: the first keeps it, the others' groups and words join it,
// and they point to it from `shared`. Endpoints in the reader's language stay
// separate, and only siblings combine, never unrelated family branches.
// Changes the given hubs, which are new and not yet drawn.
export function shareColumns(made: Hub[]): void {
  type Column = { host: Hub; branch: DisplayBranch; origins: Map<Group, Hub>; parents: Map<number, Set<string>> };
  const columns = new Map<string, Column>();
  for (const child of made) child.branches = child.branches.filter((b) => {
    if (b.groups.every((g) => g.own)) return true;
    const previous = columns.get(b.key);
    if (!previous) {
      const origins = new Map(b.groups.map((g) => [g, child]));
      const parents = new Map(b.groups.map((g) => [g.head.item.id, new Set([child.id])]));
      Object.assign(b, { origins, parents });
      columns.set(b.key, { host: child, branch: b, origins, parents });
      return true;
    }
    const combined = previous.branch;
    for (const g of b.groups) {
      const parents = previous.parents.get(g.head.item.id) ?? new Set<string>();
      parents.add(child.id);
      previous.parents.set(g.head.item.id, parents);
      previous.origins.set(g, child);
    }
    combined.groups.push(...b.groups);
    const terms = new Set(combined.words.map((w) => w.item.term));
    for (const word of b.words) if (!terms.has(word.item.term)) {
      terms.add(word.item.term);
      combined.words.push(word);
    }
    combined.wordCount = combined.words.length;
    combined.hasSelected ||= b.hasSelected;
    child.shared.push({ host: previous.host, branch: combined });
    return false;
  });
}

// Another preview row reached an entry that is already drawn under the same
// parent branch: widen that hub's word scope and add the branches the new
// words bring, instead of drawing the entry and its lineage twice. This is
// the one place an on-screen hub changes in place; its identity is what its
// rows, child hubs (`parent`) and shared columns refer to. `all` is the list
// being built for the next render, whose shared references are repointed.
function extendHub(hub: Hub, wider: Hub, all: Hub[]): void {
  for (const b of wider.branches) {
    const prior = hub.branches.find((old) => old.key === b.key);
    if (prior) {
      const replacement = { ...prior, ...b };
      hub.branches = hub.branches.map((old) => old === prior ? replacement : old);
      for (const h of all) for (const shared of h.shared) {
        if (shared.branch === prior) shared.branch = replacement;
      }
    } else hub.branches = [...hub.branches, b];
    hub.hues.set(b.key, hueOf(wider, b));
  }
  hub.scope = wider.scope;
  hub.single = isSingle(hub.node, hub.parent, hub.branches);
}

export interface Expansion {
  target: Lang;
  branchesFor: BranchReader;
  // The clicked branch is a descendant tray: its linear paths finish
  // independently of neighbouring forks.
  fromTray: boolean;
  // This click's number (`Hub.revealed`).
  revealed: number;
}

// Open `groups` of `branch` in `owner`, each head a new row with its own
// branches, then keep opening on their own what one click should finish: a
// lineage with no forks, a small subtree with at most one split, or (from a
// tray) each uninterrupted path. Returns the next hub list and the hubs this
// click made or widened.
export function expand(
  current: Hub[],
  owner: Hub,
  branch: Branch,
  groups: Group[],
  scope: Scope | undefined,
  { target, branchesFor, fromTray, revealed }: Expansion
): { all: Hub[]; touched: Set<Hub> } {
  const all = [...current];
  const taken = new Set(all.map((h) => h.id));
  const touched = new Set<Hub>();
  const read = (head: TreeNode) => {
    const branches = branchesFor(head);
    return scope ? restrictBranches(branches, scope.words) : branches;
  };
  const selected = { ...branch, groups, words: groups.flatMap((g) => g.words) };
  const lineage = unbranchedLineage(selected, read);
  const finish = canFinishTogether(scope ? restrictBranches([selected], scope.words) : [selected], read);
  const queue: { owner: Hub; branch: Branch; groups: Group[] }[] = [{ owner, branch, groups }];
  for (const step of queue) {
    const via = blobId(step.owner, step.branch);
    const existing = all.filter((h) => h.via === via);
    const made: Hub[] = [];
    const extended: Hub[] = [];
    for (const g of step.groups) {
      if (!canOpen(g, target)) continue;
      const origins = (step.branch as DisplayBranch).origins;
      const source = origins && [...origins].find(([group]) => group.head === g.head)?.[1];
      const repeated = existing.find((h) => h.node.item.id === g.head.item.id);
      if (repeated) {
        if (!repeated.scope || (scope && [...scope.words].every((word) => repeated.scope!.words.has(word)))) continue;
        const words = new Set([...repeated.scope.words, ...(scope?.words ?? g.words.map((w) => w.item.term))]);
        const wider = makeHub(branchesFor, g.head, source ?? step.owner, via, repeated.hue, { id: repeated.scope.id, words });
        extendHub(repeated, wider, all);
        touched.add(repeated);
        extended.push(repeated);
        continue;
      }
      const child = makeHub(branchesFor, g.head, source ?? step.owner, via, hueOf(step.owner, step.branch), scope);
      child.revealed = revealed;
      if (taken.has(child.id)) continue;
      taken.add(child.id);
      touched.add(child);
      made.push(child);
    }
    shareColumns(made);
    all.push(...rackOrder(made, (h) => h.single));
    // Preserve choices at the initial overview. Once a tray is opened,
    // linear continuations finish independently of neighbouring forks.
    queue.push(...[...extended, ...made].flatMap((child) => child.branches.flatMap((next) => {
      const scoped = scope ? restrictBranches([next], scope.words)[0] : next;
      if (!scoped) return [];
      const more = lineage || finish ? scoped.groups : fromTray ? linearGroups(scoped, read) : [];
      return more.length ? [{ owner: child, branch: next, groups: more }] : [];
    })));
  }
  return { all, touched };
}
