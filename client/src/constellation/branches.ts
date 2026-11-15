// What the constellation draws around one word (a hub): the words of the
// reader's language that descend from it, grouped by the branch of the
// language tree they came down and, within a branch, by the first word of
// another language on the way (a head).
//
// Words of the hub's own language are passed through, so the Proto-Indo-
// European formations between a root and its Proto-Germanic reflexes never
// stand between the root and its branches.

import { TreeNode } from "../ety/model";
import { Lang } from "../search/types";

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

function dealt(groups: Group[]): TreeNode[] {
  const all = groups.flatMap((g) =>
    g.words.map((node, i) => ({ node, at: (i + 0.5) / g.words.length, of: g.words.length }))
  );
  all.sort(
    (a, b) => Number(b.node.isSelected) - Number(a.node.isSelected) || a.at - b.at || b.of - a.of
  );
  const seen = new Set<string>();
  return all.flatMap(({ node }) => (seen.has(node.item.term) ? [] : (seen.add(node.item.term), [node])));
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

// The words a blob lists: single words that are not proper nouns. Phrases
// ("video chat") and names ("Winterbottom") crowd out the cognates a reader
// came for, and so do the words no desk dictionary has (Wiktionary has
// "logodaedaly" and "theologaster" beside "logic"): given `common`, a word
// not in it is left out. The searched word is listed whatever it is.
function listed(n: TreeNode, common: Common | null): boolean {
  if (n.isSelected) return true;
  const { term, pos } = n.item;
  if (/\s/.test(term) || pos?.includes("name")) return false;
  if (common && level(common, term) === undefined) return false;
  // A capitalised word with no part of speech recorded is nearly always a name.
  return !(!pos?.length && /^\p{Lu}/u.test(term));
}

// The words an English word may be a plain derivative of: itself without
// un- or non-, or without an inflection or one of the suffixes that keep a
// word's sense (forgiven, forgiving, forgiveness, forgivable and unforgiven
// all lead back to forgive). Spelling is allowed for: a dropped e, y become
// i, a doubled consonant.
const PREFIXES = ["un", "non"];
const SUFFIXES = ["ness", "less", "able", "ible", "ful", "ing", "ly", "ed", "en", "es", "s"];
function bases(term: string): string[] {
  const out: string[] = [];
  for (const p of PREFIXES) if (term.startsWith(p) && term.length >= p.length + 3) out.push(term.slice(p.length));
  for (const s of SUFFIXES) {
    if (!term.endsWith(s) || term.length < s.length + 3) continue;
    const stem = term.slice(0, -s.length);
    out.push(stem, stem + "e");
    if (stem.endsWith("i")) out.push(stem.slice(0, -1) + "y");
    if (stem.at(-1) === stem.at(-2)) out.push(stem.slice(0, -1));
  }
  return out;
}

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
    .filter(
      (f) => !common || f.node.isSelected || !bases(f.node.item.term.toLowerCase()).some((b) => byTerm.has(b))
    )
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
function wordsUnder(head: TreeNode, target: number, common: Common | null): TreeNode[] {
  const found: Found[] = [];
  const visit = (n: TreeNode, depth: number) => {
    if (n.item.lang.id === target && listed(n, common)) found.push({ node: n, depth });
    for (const c of n.children) visit(c, depth + 1);
  };
  visit(head, 0);
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
  const own: Found[] = [];
  const collect = (n: TreeNode, depth: number) => {
    for (const c of n.children) {
      if (c.item.lang.id !== hub.item.lang.id) {
        if (top && c.item.lang.id === target.id) continue;
        if (!heads.has(c.item.id)) heads.set(c.item.id, c);
        continue;
      }
      if (c.item.lang.id === target.id && listed(c, common)) own.push({ node: c, depth });
      collect(c, depth + 1);
    }
  };
  collect(hub, 1);

  // The language tree below the hub's language, as far as the heads reach.
  // Heads of the reader's language, and of languages that do not descend from
  // the hub's, each stand under their own language.
  const root: Trie = { code: hub.item.lang.code, heads: [], kids: new Map() };
  const names = new Map<string, string>();
  for (const head of heads.values()) {
    const lang = head.item.lang;
    names.set(lang.code, lang.name);
    const line = chain(lang.code, langs);
    const at = lang.id === target.id ? -1 : line.indexOf(root.code);
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

  for (let t of root.kids.values()) {
    // A branch is named by the oldest language on it that has a head, or
    // where its heads' languages part ways; at the top of a family, by the
    // language that comes straight from the hub's.
    while (!top && t.heads.length === 0 && t.kids.size === 1) t = t.kids.values().next().value!;
    const members = below(t);
    const name = langs.names[t.code] ?? names.get(t.code) ?? t.code;
    if (t.code === target.code) {
      // The reader's own language: one list, no heads.
      const words = rank(
        members.flatMap((h) => wordsUnder(h, target.id, common).map((node, depth) => ({ node, depth }))),
        common
      );
      add(t.code, name, words.length ? [group(hub, true, words)] : []);
    } else {
      add(
        t.code,
        name,
        members.map((h) => group(h, false, wordsUnder(h, target.id, common))).filter((g) => g.words.length > 0)
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
