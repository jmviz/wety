// Which words of the reader's language a constellation lists, settled once
// per drawing from the graph, the English word lists and the language
// periods. Pure: the component only renders what this decides.

import type { TreeNode } from "../ety/model";
import type { Lang } from "../search/types";
import {
  ENGLISH,
  HIDE_UNDEFINED_ROOTS,
  branchesOf,
  directEarlyEnglish,
  isProto,
  previewLines,
  uncommonLatinEnglish,
  withoutUndefinedRoots,
  type Branch,
  type Common,
  type LangTree,
} from "./branches.ts";
import { representatives, type Frequencies } from "./representatives.ts";

export interface Selection {
  // How common the reader's language's words are; null when every word is
  // listed (a language without a word list, or rare words switched on).
  common: Common | null;
  // Corpus frequencies, English only.
  scores: Frequencies | undefined;
  // The branches a node opens into, every filter applied; cached per node.
  branchesFor: (node: TreeNode) => Branch[];
}

function cachedPerNode<T>(f: (node: TreeNode) => T): (node: TreeNode) => T {
  const cache = new WeakMap<TreeNode, T>();
  return (node) => {
    if (!cache.has(node)) cache.set(node, f(node));
    return cache.get(node)!;
  };
}

// `wordList`: SCOWL tiers, or null to list rare words too. `times`: each
// language's approximate year (`languageTimes`).
export function selectWords(
  root: TreeNode,
  target: Lang,
  langs: LangTree,
  wordList: Common | null,
  frequencies: Frequencies,
  times: ReadonlyMap<string, number>
): Selection {
  const english = target.code === ENGLISH;
  const common = english ? wordList : null;
  const scores = english ? frequencies : undefined;
  // Classical Latin ends around 200 CE in the shipped period registry.
  // Broad Ancient Greek and proto labels also count as early sources even
  // where their registry dates describe a later boundary or breakup.
  const omitted = english
    ? directEarlyEnglish(root, (code) =>
        code === "la" || code === "grc" || isProto(code) || (times.get(code) ?? Infinity) <= 200)
    : new Set<number>();
  for (const key of uncommonLatinEnglish(root, common)) omitted.add(key);
  const display = (node: TreeNode) => {
    const branches = branchesOf(node, target, langs, common, omitted, scores);
    return HIDE_UNDEFINED_ROOTS ? withoutUndefinedRoots(branches) : branches;
  };
  // The representatives pass reads the previews as they would be without
  // it, then leaves out the English words it does not keep.
  if (common && scores) {
    const preliminary = cachedPerNode(display);
    const reading = { target, children: preliminary, common, scores };
    const groups = preliminary(root).flatMap((b) => previewLines(b, reading, { combineSingles: false }));
    const keep = representatives(groups, scores);
    const visit = (n: TreeNode) => {
      if (n.item.lang.code === ENGLISH && !n.isSelected && !keep.has(n.item.term)) omitted.add(n.key);
      n.children.forEach(visit);
    };
    visit(root);
  }
  return { common, scores, branchesFor: cachedPerNode(display) };
}
