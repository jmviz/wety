// Deterministic display selection. Corpus usage decides ordering; only
// recorded derivation paths can make one word stand in for another.
import type { TreeNode } from "../ety/model";

export type Frequencies = ReadonlyMap<string, number>;
export const frequency = (scores: Frequencies, term: string): number =>
  scores.get(term.toLowerCase().replace(/^-+|-+$/g, "")) ?? 0;

const FORMING = new Set(["prefix", "suffix", "affix", "morphological derivation", "form"]);

// A derivative must share a recorded source with its proposed base, and
// add a prefix/suffix on the way from that source. Generic "derived" or
// an unrelated spelling resemblance is not sufficient evidence.
function derivationExtra(derived: TreeNode, base: TreeNode): number {
  const basePath = new Map<number, number>();
  let changes = 0;
  for (let n: TreeNode | null = base; n; n = n.parent) {
    basePath.set(n.item.id, changes);
    if (FORMING.has(n.etyMode ?? "")) changes++;
  }
  changes = 0;
  for (let n: TreeNode | null = derived; n; n = n.parent) {
    const baseChanges = basePath.get(n.item.id);
    if (baseChanges !== undefined) return n.item.reconstructed ? 0 : changes - baseChanges;
    if (FORMING.has(n.etyMode ?? "")) changes++;
  }
  return 0;
}

export function representatives(groups: TreeNode[][], scores: Frequencies): Set<string> {
  const keep = new Set<string>();
  for (const words of groups) {
    const sorted = [...words].sort((a, b) =>
      Number(b.isSelected) - Number(a.isSelected) ||
      frequency(scores, b.item.term) - frequency(scores, a.item.term) ||
      a.item.term.length - b.item.term.length || a.item.term.localeCompare(b.item.term)
    );
    const pruned = sorted.filter((word) => word.isSelected || !words.some((base) =>
      base !== word && base.item.term !== word.item.term &&
      frequency(scores, word.item.term) > 0 &&
      base.item.term.length < word.item.term.length &&
      frequency(scores, base.item.term) >= frequency(scores, word.item.term) + 0.8 &&
      derivationExtra(word, base) > 0
    ));
    // Four representatives normally suffice. Larger families get a little
    // more room; words frequent enough in ordinary usage can exceed the
    // budget. Unknown scores are not grounds for discarding a word.
    const budget = Math.max(4, Math.ceil(Math.log2(pruned.length + 1) * 1.5));
    pruned.forEach((word, i) => {
      const score = frequency(scores, word.item.term);
      if (word.isSelected || i < budget || score === 0 || score >= 4.8) keep.add(word.item.term);
    });
  }
  return keep;
}
