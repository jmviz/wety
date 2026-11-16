// What the constellation draws around one word (a hub): the words of the
// reader's language that descend from it, grouped by the branch of the
// language tree they came down and, within a branch, by the first word of
// another language on the way (a head).
//
// Words of the hub's own language are passed through, so the Proto-Indo-
// European formations between a root and its Proto-Germanic reflexes never
// stand between the root and its branches.

import type { TreeNode } from "../ety/model";
import type { Lang } from "../search/types";
import { frequency, type Frequencies } from "./representatives.ts";

export interface LangTree {
  parents: Record<string, string>;
  names: Record<string, string>;
}

// A reconstructed stage may follow its own stage or an ancestor language,
// never a later attested language or a different proto family.
export function validProtoDescent(parent: TreeNode, child: TreeNode, langs: LangTree): boolean {
  const from = parent.item.lang.code, to = child.item.lang.code;
  if (!to.endsWith("-pro") || from === to) return true;
  if (!from.endsWith("-pro")) return false;
  const seen = new Set<string>();
  for (let at = langs.parents[to]; at && !seen.has(at); at = langs.parents[at]) {
    if (at === from) return true;
    seen.add(at);
  }
  return false;
}

// A head and the words under it. `own`: the words are the hub's own
// formations in the reader's language, with no head between.
export interface Group {
  head: TreeNode;
  words: TreeNode[];
  own: boolean;
  hasSelected: boolean;
}

export interface Branch {
  // The branch's language code: unique among a hub's branches.
  key: string;
  name: string;
  groups: Group[];
  // Every group's words in one list: the searched word, then the groups'
  // dealt out in proportion to their size, so the first few stand for the
  // branch (a root with one obscure word does not lead it).
  words: TreeNode[];
  wordCount: number;
  hasSelected: boolean;
}

// Reversible display experiment. The graph and placeholder construction
// stay intact, so switching this off restores the missing-root paths.
export const HIDE_UNDEFINED_ROOTS = true;

export function withoutUndefinedRoots(branches: Branch[]): Branch[] {
  return branches.flatMap((b) => {
    const groups = b.groups.filter((g) => g.own || !!g.head.item.term);
    if (groups.length === b.groups.length) return [b];
    if (!groups.length) return [];
    const allowed = new Set(groups.flatMap((g) => g.words.map((w) => w.item.term)));
    const words = b.words.filter((w) => allowed.has(w.item.term));
    return words.length ? [{ ...b, groups, words, wordCount: words.length, hasSelected: groups.some((g) => g.hasSelected) }] : [];
  });
}

// A row can share a historical source with another row. Keep its paths
// restricted to the displayed words that were actually clicked.
export function restrictBranches(branches: Branch[], terms: ReadonlySet<string>): Branch[] {
  return branches.flatMap((b) => {
    const groups = b.groups.flatMap((g) => {
      const words = g.words.filter((w) => terms.has(w.item.term));
      return words.length ? [{ ...g, words, hasSelected: words.some((w) => w.isSelected) }] : [];
    });
    const words = b.words.filter((w) => terms.has(w.item.term));
    return groups.length && words.length ? [{ ...b, groups, words, wordCount: words.length,
      hasSelected: words.some((w) => w.isSelected) }] : [];
  });
}

export function groupsForWords(branch: Branch, words: TreeNode[]): Group[] {
  return restrictBranches([branch], new Set(words.map((w) => w.item.term)))[0]?.groups ?? [];
}

// Keep each preview row attached to the source that produced it. Matching
// only by spelling incorrectly opens every unrelated source of e.g. port.
export function previewRows(branch: Branch, target: Lang, children: (head: TreeNode) => Branch[], common: Common | null = null, scores?: Frequencies, splitSources = false): { words: TreeNode[]; lines: TreeNode[][]; groups: Group[] }[] {
  const proto = branch.key.endsWith("-pro");
  if (proto && !splitSources) return previewLines(branch, target, children, common, scores)
    .map((words) => ({ words, lines: [words], groups: groupsForWords(branch, words) }));
  return branch.groups.flatMap((g) => {
    const scoped = { ...branch, groups: [g], words: branch.words.filter((w) => g.words.some((v) => v.item.term === w.item.term)) };
    const lines = previewLines(scoped, target, children, common, scores);
    const words = lines.flat();
    return words.length ? [{ words, lines, groups: [g] }] : [];
  });
}

// Keep one-word paths in the middle, with larger lanes above and below.
// The collapsed preview uses this same order as the expanded rack.
export function rackOrder<T>(entries: T[], single: (entry: T) => boolean): T[] {
  const many = entries.filter((entry) => !single(entry));
  return [
    ...many.filter((_, i) => i % 2 === 0),
    ...entries.filter(single),
    ...many.filter((_, i) => i % 2 === 1).reverse(),
  ];
}

// Each line previews one child branch of the expanded rack. Keep the
// existing word filters and show a spelling once, in its first subgroup.
export function previewLines(branch: Branch, target: Lang, children: (head: TreeNode) => Branch[], common: Common | null = null, scores?: Frequencies, combineSingles = true): TreeNode[][] {
  const cache = new Map<Branch, TreeNode[][]>();
  const lines = (branch: Branch): TreeNode[][] => {
    const had = cache.get(branch);
    if (had) return had;
    const open = branch.groups.filter((g) => canOpen(g, target)).map((g) => ({ g, branches: children(g.head) }));
    const ordered = rackOrder(open, ({ g, branches }) => !!g.head.item.term && singleEndpoint(branches));
    const candidates = [
      ...ordered.flatMap(({ g, branches }) => branches.length ? branches.map((b) => lines(b).flat()) : [g.words]),
      ...branch.groups.filter((g) => !canOpen(g, target)).map((g) => g.words),
    ];
    const allowed = new Set(branch.words.map((w) => w.item.term));
    const seen = new Set<string>();
    const result = candidates.map((words) => words.filter((w) => {
      const term = w.item.term;
      if (!allowed.has(term) || seen.has(term)) return false;
      seen.add(term);
      return true;
    }).sort(byUsage(scores))).filter((words) => words.length > 0);
    cache.set(branch, result);
    return result;
  };
  const groups = lines(branch);
  if (!combineSingles) return groups;
  const singles = groups.filter((words) => words.length === 1).flat();
  if (singles.length < 2) return groups;
  singles.sort((a, b) => byUsage(scores)(a, b) ||
    (common ? (level(common, a.item.term) ?? Infinity) - (level(common, b.item.term) ?? Infinity) : 0));
  // Compact solos only when sharing the preview with larger families.
  // An all-solo preview has room to give each choice its own row.
  if (singles.length === groups.length) return singles.map((word) => [word]);
  let inserted = false;
  return groups.flatMap((words) => {
    if (words.length !== 1) return [words];
    if (inserted) return [];
    inserted = true;
    return [singles];
  });
}

function dealt(groups: Group[]): TreeNode[] {
  const all = groups.flatMap((g) =>
    g.words.map((node, i) => ({ node, at: (i + 0.5) / g.words.length, of: g.words.length }))
  );
  all.sort(
    (a, b) => Number(b.node.isSelected) - Number(a.node.isSelected) || a.at - b.at || b.of - a.of
  );
  const seen = new Set<string>();
  return all.flatMap(({ node }) => {
    const { term } = node.item;
    if (seen.has(term)) return [];
    seen.add(term);
    return [node];
  });
}

// A language's ancestors, oldest first, ending with itself.
function chain(code: string, langs: LangTree): string[] {
  const out = [code];
  const seen = new Set(out);
  for (let p = langs.parents[code]; p && !seen.has(p); p = langs.parents[p]) {
    out.unshift(p);
    seen.add(p);
  }
  return out;
}

interface Found {
  node: TreeNode;
  depth: number;
}

// English formations inherit a direct jump until a recorded non-English
// stage intervenes. Filtering only the first English word would let its
// derivatives reappear farther down the same path.
export function directEarlyEnglish(root: TreeNode, early: (code: string) => boolean): Set<number> {
  const omitted = new Set<number>();
  const visit = (n: TreeNode, source: string | undefined, blocked: boolean) => {
    const english = n.item.lang.code === "en";
    const skip = english && (blocked || (source !== undefined && early(source)));
    if (skip && !n.isSelected) omitted.add(n.key);
    for (const child of n.children) visit(child, english ? source : n.item.lang.code, english && skip);
  };
  visit(root, undefined, false);
  return omitted;
}

// How common each word of the reader's language is: the smallest SCOWL list
// it is in (10, the commonest, to 70, the broadest), and nothing for a
// word in none. The default stops at 50; higher tiers include obscure and
// dialect words such as "fremd". `virtual:common-words` has English alone.
export type Common = Map<string, number>;
const level = (common: Common, term: string) => common.get(term.toLowerCase());
// `scripts/build-word-frequencies.py` scores words up to this tier, and the
// build refuses a frequency asset that stops short of it (`vite.config.ts`).
export const COMMON_LIMIT = 50;
const LATIN_COMMON_LIMIT = 35;

// The searched word first, then the commoner by corpus usage where known.
const byUsage = (scores: Frequencies | undefined) => (a: TreeNode, b: TreeNode) =>
  Number(b.isSelected) - Number(a.isSelected) ||
  (scores ? frequency(scores, b.item.term) - frequency(scores, a.item.term) : 0);

// Follow the recorded word path, not the language family: a Germanic word
// borrowed through Latin qualifies, while an unrelated Germanic path does
// not. Retain this ancestry after opening a French or English intermediate.
export function uncommonLatinEnglish(root: TreeNode, common: Common | null): Set<number> {
  const omitted = new Set<number>();
  if (!common) return omitted;
  const isLatin = (n: TreeNode) => n.item.lang.code === "la" || n.item.lang.code.startsWith("la-") || /\bLatin$/.test(n.item.lang.name);
  let inherited = false;
  for (let p = root.parent; p; p = p.parent) inherited ||= isLatin(p);
  const visit = (n: TreeNode, latin: boolean) => {
    latin ||= isLatin(n);
    // Affixes must qualify too. SCOWL stores ordinary spellings, so try the
    // form without boundary hyphens; "après-" gets no blanket exemption.
    const frequency = level(common, n.item.term) ??
      (isAffix(n) ? level(common, n.item.term.replace(/^-+|-+$/g, "")) : undefined);
    if (latin && n.item.lang.code === "en" && !n.isSelected &&
        (frequency ?? Infinity) > LATIN_COMMON_LIMIT) omitted.add(n.key);
    for (const child of n.children) visit(child, latin);
  };
  visit(root, inherited);
  return omitted;
}

// An affix or combining form as a dictionary enters one ("-logy", "un-"). A
// reconstructed root is written the same way ("*leǵ-") and is not one.
const isAffix = (n: TreeNode) => !n.item.reconstructed && /^-|-$/.test(n.item.term);

// Whether the reader's language has an affix below a node. Where a hub has
// one, the words formed with an affix on the way down from it are left to
// the affixes listed: cosmology, theology and etymology, each formed with
// Ancient Greek -λογία or -λογος, are left to "-logy", while apology and
// trilogy, formed from λόγος itself, are words in their own right.
const standIns = new WeakMap<TreeNode, boolean>();
function hasStandIn(n: TreeNode, target: number): boolean {
  let has = standIns.get(n);
  if (has === undefined) {
    has = n.children.some((c) => (c.item.lang.id === target && isAffix(c)) || hasStandIn(c, target));
    standIns.set(n, has);
  }
  return has;
}

// The words a blob lists: single words that are not proper nouns. Phrases
// ("video chat") and names ("Winterbottom") crowd out the cognates a reader
// came for. Given `common`, so do words beyond the default commonness limit
// (Wiktionary has "logodaedaly" and "theologaster" beside "logic") and those
// formed with an affix that is listed itself (`grafted`): both are left out,
// and an affix of the reader's language is listed. The searched word is
// listed whatever it is.
function listed(n: TreeNode, common: Common | null, grafted: boolean): boolean {
  if (n.isSelected) return true;
  const { term, pos } = n.item;
  if (/\s/.test(term) || pos?.includes("name")) return false;
  if (common) {
    if (isAffix(n)) return true;
    if (grafted || (level(common, term) ?? Infinity) > COMMON_LIMIT) return false;
  }
  // A capitalised word with no part of speech recorded is nearly always a name.
  return !(!pos?.length && /^\p{Lu}/u.test(term));
}

// One entry per spelling (the searched word, else the nearest), in the order
// a reader is likeliest to know them: corpus frequency and SCOWL tier where
// known, then what the graph has: a plain lower-case word before affixes and
// the rest; then words with several entries or parts of speech, which the
// everyday ones tend to have; then the nearest and the shortest. Which plain
// derivatives are left out is decided from recorded derivations, never from
// spelling (`representatives.ts`).
const PLAIN = /^\p{Ll}+$/u;
function rank(found: Found[], common: Common | null, scores?: Frequencies): TreeNode[] {
  const byTerm = new Map<string, Found & { uses: number }>();
  for (const f of found) {
    const uses = f.node.item.pos?.length ?? 0;
    const had = byTerm.get(f.node.item.term);
    if (!had) byTerm.set(f.node.item.term, { ...f, uses });
    else if (!had.node.isSelected && (f.node.isSelected || f.depth < had.depth))
      byTerm.set(f.node.item.term, { ...f, uses: had.uses + uses });
    else had.uses += uses;
  }
  const rarity = (f: Found) => (common && level(common, f.node.item.term)) || Infinity;
  return [...byTerm.values()]
    .sort(
      (a, b) =>
        byUsage(scores)(a.node, b.node) ||
        rarity(a) - rarity(b) ||
        Number(!PLAIN.test(a.node.item.term)) - Number(!PLAIN.test(b.node.item.term)) ||
        Math.min(b.uses, 3) - Math.min(a.uses, 3) ||
        a.depth - b.depth ||
        a.node.item.term.length - b.node.item.term.length ||
        a.node.item.term.localeCompare(b.node.item.term)
    )
    .map((f) => f.node);
}

// The words of the reader's language at and below a head.
// `folds`: words formed with an affix are left to the affixes listed.
// `grafted`: the head itself was formed with one.
function wordsUnder(
  head: TreeNode,
  target: number,
  common: Common | null,
  folds: boolean,
  grafted: boolean,
  omitted: ReadonlySet<number>,
  scores: Frequencies | undefined,
  langs: LangTree
): TreeNode[] {
  const found: Found[] = [];
  const visit = (n: TreeNode, depth: number, grafted: boolean) => {
    if (n.item.lang.id === target && !omitted.has(n.key) && listed(n, common, grafted)) found.push({ node: n, depth });
    const below = grafted || (folds && isAffix(n));
    for (const c of n.children) if (validProtoDescent(n, c, langs)) visit(c, depth + 1, below);
  };
  visit(head, 0, grafted);
  return rank(found, common, scores);
}

interface Trie {
  code: string;
  heads: TreeNode[];
  kids: Map<string, Trie>;
}

const group = (head: TreeNode, own: boolean, words: TreeNode[]): Group => ({
  head,
  words,
  own,
  hasSelected: words.some((w) => w.isSelected),
});

// A missing word, rather than a recorded word of a different language.
// Odd negative IDs stand for missing words within a branch; even ones for
// the branch's missing root. The anchor is always a real graph node.
function undefinedHead(word: TreeNode, lang: Lang, children: TreeNode[], branch = false): TreeNode {
  const anchor = word.item.id < 0 ? word.children[0] : word;
  const offset = branch ? 2 : 1;
  return {
    ...word,
    key: -anchor.key * 2 - offset,
    item: {
      ...word.item,
      id: -anchor.item.id * 2 - offset,
      lang,
      term: "",
      imputed: true,
      reconstructed: false,
      url: null,
      pos: null,
      gloss: null,
      romanization: null,
    },
    etyMode: null,
    isSelected: false,
    children,
  };
}

// A word of a later language that Wiktionary only tags with the root (Old
// French intellectuel under *leǵ-) did not come straight from the root, nor
// straight from the family's branch (no Old French word comes straight from
// Proto-Italic): it came through a word that is not recorded. That word is
// stood in for by a head with no name, in the first language on the way down
// that is not a stage of a proto-language or family (Latin, for Old French),
// with the tagged word under it. Nothing for a word whose own language is
// that first one (Latin legenda): it is a head itself.
const lasting = (code: string) => !code.includes("-");
const langsBelow = new WeakMap<TreeNode, Map<string, Lang>>();
function unnamed(word: TreeNode, hub: TreeNode, langs: LangTree): TreeNode | null {
  const line = chain(word.item.lang.code, langs);
  const code = line.slice(line.indexOf(hub.item.lang.code) + 1, -1).find(lasting);
  if (!code) return null;
  // The language as the tree has it, where a word of it is in the tree.
  let known = langsBelow.get(hub);
  if (!known) {
    known = new Map();
    const visit = (n: TreeNode) => {
      known!.set(n.item.lang.code, n.item.lang);
      n.children.forEach(visit);
    };
    visit(hub);
    langsBelow.set(hub, known);
  }
  const lang = known.get(code) ?? { id: -1, code, name: langs.names[code] ?? code };
  return undefinedHead(word, lang, [word]);
}

// `common`: how common the reader's language's words are, to leave the rare
// ones out and list the commonest first; null lists every word.
export function branchesOf(hub: TreeNode, target: Lang, langs: LangTree, common: Common | null, omitted: ReadonlySet<number> = new Set(), scores?: Frequencies): Branch[] {
  // A hub at the top of its family: a reconstructed word of a language with
  // no ancestor of its own (Proto-Indo-European). Its branches are the
  // family's: the languages that come straight from it, a fixed set, under
  // which a later language's word stands when nothing is recorded in between
  // (Ancient Greek under Proto-Hellenic). And no word of the reader's
  // language comes straight from it: one recorded so (tagged with the root,
  // its line of descent not given) is left out.
  const top = hub.item.reconstructed && !langs.parents[hub.item.lang.code];
  // Heads: the first words below the hub that are not in its language. A hub
  // of the reader's language also has formations of its own.
  const heads = new Map<number, TreeNode>();
  // The heads formed with an affix of the hub's language that is passed
  // through on the way to them.
  const graftedHeads = new Set<number>();
  const own: Found[] = [];
  const folds = !!common && hasStandIn(hub, target.id);
  const collect = (n: TreeNode, depth: number, grafted: boolean) => {
    const below = grafted || (folds && isAffix(n));
    for (const c of n.children) {
      if (!validProtoDescent(n, c, langs)) continue;
      if (c.item.lang.code !== hub.item.lang.code) {
        if (top && c.item.lang.id === target.id) continue;
        const head = (top && c.etyMode === "root" && unnamed(c, hub, langs)) || c;
        if (!heads.has(head.item.id)) {
          heads.set(head.item.id, head);
          if (below) graftedHeads.add(head.item.id);
        }
        continue;
      }
      if (c.item.lang.id === target.id && !omitted.has(c.key) && listed(c, common, below)) own.push({ node: c, depth });
      collect(c, depth + 1, below);
    }
  };
  collect(hub, 1, false);
  const under = (h: TreeNode) => wordsUnder(h, target.id, common, folds, graftedHeads.has(h.item.id), omitted, scores, langs);

  // Only a family's root uses ancestry to choose its fixed branches.
  // Everywhere else the heading is the exact language of every head in it:
  // a common ancestor is not a license to call later words Old Latin.
  const root: Trie = { code: hub.item.lang.code, heads: [], kids: new Map() };
  const names = new Map<string, string>();
  for (const head of heads.values()) {
    const lang = head.item.lang;
    names.set(lang.code, lang.name);
    const line = chain(lang.code, langs);
    const at = top && lang.id !== target.id ? line.indexOf(root.code) : -1;
    let node = root;
    for (const code of at < 0 ? [lang.code] : line.slice(at + 1)) {
      let kid = node.kids.get(code);
      if (!kid) node.kids.set(code, (kid = { code, heads: [], kids: new Map() }));
      node = kid;
    }
    node.heads.push(head);
  }
  const below = (t: Trie): TreeNode[] => [...t.heads, ...[...t.kids.values()].flatMap(below)];

  const branches: Branch[] = [];
  const add = (key: string, name: string, groups: Group[]) => {
    if (!groups.length) return;
    groups.sort(
      (a, b) =>
        Number(b.hasSelected) - Number(a.hasSelected) ||
        b.words.length - a.words.length ||
        a.head.item.term.localeCompare(b.head.item.term)
    );
    const words = dealt(groups);
    branches.push({
      key,
      name,
      groups,
      words,
      wordCount: words.length,
      hasSelected: groups.some((g) => g.hasSelected),
    });
  };

  for (const t of root.kids.values()) {
    // At the family root, a branch names a direct daughter language and
    // missing forms go under its Undefined Root. Otherwise it names the
    // heads' actual language, never an inferred common ancestor.
    let members = below(t);
    const name = langs.names[t.code] ?? names.get(t.code) ?? t.code;
    if (top && t.code !== target.code) {
      // A fixed family branch is a language of its own. Later words with
      // no recorded word in that language share one undefined root, whose
      // branches then group those words by their actual languages.
      const missing = members.filter((h) => h.item.lang.code !== t.code);
      if (missing.length) {
        const lang = members.find((h) => h.item.lang.code === t.code)?.item.lang ??
          { id: -1, code: t.code, name };
        members = [
          ...members.filter((h) => h.item.lang.code === t.code),
          undefinedHead(missing[0], lang, missing, true),
        ];
      }
    }
    if (t.code === target.code) {
      // The reader's own language: one list, no heads.
      const words = rank(
        members.flatMap((h) => under(h).map((node, depth) => ({ node, depth }))),
        common, scores
      );
      add(t.code, name, words.length ? [group(hub, true, words)] : []);
    } else {
      add(
        t.code,
        name,
        members.map((h) => group(h, false, under(h))).filter((g) => g.words.length > 0)
      );
    }
  }
  if (own.length) add(target.code, target.name, [group(hub, true, rank(own, common, scores))]);

  return branches.sort(
    (a, b) =>
      Number(b.hasSelected) - Number(a.hasSelected) ||
      b.wordCount - a.wordCount ||
      a.name.localeCompare(b.name)
  );
}

// A head opens into a hub of its own unless everything under it is already
// listed: a word of the reader's language has only its own formations below.
export function canOpen(g: Group, target: Lang): boolean {
  return !g.own && g.head.item.lang.id !== target.id && g.head.children.length > 0;
}

// A one-word endpoint is compact only when it is the next language step.
// Counting final words alone hides Latin/French/etc. between a source and
// English, making a retained later-language path look like a direct jump.
export function singleEndpoint(branches: Branch[]): boolean {
  return branches.length === 1 && branches[0].words.length === 1 && branches[0].groups.every((g) => g.own);
}
