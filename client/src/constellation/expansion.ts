import type { Branch } from "./branches";

// New click groups sit nearest the remaining preview. Keep source words
// revealed by the same click together, in their existing lane order.
export function newestFirst<T extends { revealed: number }>(items: T[]): T[] {
  return [...items].sort((a, b) => b.revealed - a.revealed);
}

// Multiple modern words are one endpoint list. A lineage branches only
// when it passes through different historical sources before that endpoint.
export function unbranchedLineage(branch: Branch, children: (head: Branch["groups"][number]["head"]) => Branch[]): boolean {
  const seen = new Set<Branch["groups"][number]["head"]>();
  const visit = (b: Branch): boolean => {
    if (!b.words.length || !b.groups.length) return false;
    if (b.groups.every((g) => g.own)) return true;
    if (b.groups.length !== 1) return false;
    const g = b.groups[0];
    if (!g.head.item.term || seen.has(g.head)) return false;
    seen.add(g.head);
    const next = children(g.head);
    return next.length === 1 && visit(next[0]);
  };
  return visit(branch);
}
