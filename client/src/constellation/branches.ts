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

export interface LangTree {
  parents: Record<string, string>;
  names: Record<string, string>;
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
  // Unique among a hub's branches.
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

// `folded`: a plain derivative of a word of another group is left out too
// (analogous, from one head, beside analogy, from another).
function dealt(groups: Group[], folded: boolean): TreeNode[] {
  const all = groups.flatMap((g) =>
    g.words.map((node, i) => ({ node, at: (i + 0.5) / g.words.length, of: g.words.length }))
  );
  all.sort(
    (a, b) => Number(b.node.isSelected) - Number(a.node.isSelected) || a.at - b.at || b.of - a.of
  );
  const terms = new Set(all.map(({ node }) => node.item.term));
  const seen = new Set<string>();
  return all.flatMap(({ node }) => {
    const { term } = node.item;
    if (seen.has(term) || (folded && !node.isSelected && derived(term, terms))) return [];
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

// How common each word of the reader's language is: the smallest SCOWL list
// it is in (10, the commonest, to 70, a large desk dictionary's), and nothing
// for a word in none. `virtual:common-words` has English alone.
export type Common = Map<string, number>;
const level = (common: Common, term: string) => common.get(term.toLowerCase());

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
// came for. Given `common`, so do the words no desk dictionary has
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
    if (grafted || level(common, term) === undefined) return false;
  }
  // A capitalised word with no part of speech recorded is nearly always a name.
  return !(!pos?.length && /^\p{Lu}/u.test(term));
}

// The words an English word may be a plain derivative of: itself without
// un- or non-, or without an inflection or one of the suffixes that keep a
// word's sense (forgiven, forgiving, forgiveness, forgivable and unforgiven
// all lead back to forgive, lexicologist to lexicology, "-logical" to
// "-logic"). Spelling is allowed for: a dropped e or y, y become i, a
// doubled consonant.
const PREFIXES = ["un", "non"];
const SUFFIXES = [
  ...["ness", "less", "able", "ible", "ical", "ful", "ing", "ism", "ist", "ize", "ise"],
  ...["ics", "ly", "ed", "en", "es", "al", "ic", "s"],
];
function bases(term: string): string[] {
  const out: string[] = [];
  for (const p of PREFIXES) if (term.startsWith(p) && term.length >= p.length + 3) out.push(term.slice(p.length));
  for (const s of SUFFIXES) {
    if (!term.endsWith(s) || term.length < s.length + 3) continue;
    const stem = term.slice(0, -s.length);
    out.push(stem, stem + "e", stem + "y");
    if (stem.endsWith("i")) out.push(stem.slice(0, -1) + "y");
    if (stem.at(-1) === stem.at(-2)) out.push(stem.slice(0, -1));
  }
  return out;
}
const derived = (term: string, terms: { has(term: string): boolean }) =>
  bases(term.toLowerCase()).some((b) => terms.has(b));

// One entry per spelling (the searched word, else the nearest), in the order
// a reader is likeliest to know them. Given `common`, that is how common the
// word is, and a plain derivative of another word here is left out (the
// searched word excepted): both come from the same root, so "business" goes
// with "busy" as "forgiveness" does with "forgive". Without it, the order
// goes by what the graph has: a plain lower-case word before affixes and the
// rest; then words with several entries or parts of speech, which the
// everyday ones tend to have; then the nearest and the shortest.
const PLAIN = /^\p{Ll}+$/u;
function rank(found: Found[], common: Common | null): TreeNode[] {
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
    .filter((f) => !common || f.node.isSelected || !derived(f.node.item.term, byTerm))
    .sort(
      (a, b) =>
        Number(b.node.isSelected) - Number(a.node.isSelected) ||
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
  grafted: boolean
): TreeNode[] {
  const found: Found[] = [];
  const visit = (n: TreeNode, depth: number, grafted: boolean) => {
    if (n.item.lang.id === target && listed(n, common, grafted)) found.push({ node: n, depth });
    const below = grafted || (folds && isAffix(n));
    for (const c of n.children) visit(c, depth + 1, below);
  };
  visit(head, 0, grafted);
  return rank(found, common);
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
// ones and plain derivatives out and list the commonest first; null lists
// every word.
export function branchesOf(hub: TreeNode, target: Lang, langs: LangTree, common: Common | null): Branch[] {
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
      if (c.item.lang.code !== hub.item.lang.code) {
        if (top && c.item.lang.id === target.id) continue;
        const head = (top && c.etyMode === "root" && unnamed(c, hub, langs)) || c;
        if (!heads.has(head.item.id)) {
          heads.set(head.item.id, head);
          if (below) graftedHeads.add(head.item.id);
        }
        continue;
      }
      if (c.item.lang.id === target.id && listed(c, common, below)) own.push({ node: c, depth });
      collect(c, depth + 1, below);
    }
  };
  collect(hub, 1, false);
  const under = (h: TreeNode) => wordsUnder(h, target.id, common, folds, graftedHeads.has(h.item.id));

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
    const words = dealt(groups, !!common);
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
        common
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
  if (own.length) add(target.code, target.name, [group(hub, true, rank(own, common))]);

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

// Judge the node itself, not how many words its parent rack contains.
// A single named source with one displayed destination needs no extra click.
export function autoExpandGroup(branch: Branch, target: Lang): Group | null {
  if (branch.groups.length !== 1 || branch.words.length !== 1) return null;
  const g = branch.groups[0];
  return g.head.item.term && canOpen(g, target) ? g : null;
}
