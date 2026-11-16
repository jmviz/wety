import { isUndefinedRoot, type Branch } from "./branches.ts";

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
    if (isUndefinedRoot(g.head) || seen.has(g.head)) return false;
    seen.add(g.head);
    const next = children(g.head);
    return next.length === 1 && visit(next[0]);
  };
  return visit(branch);
}

// Evaluate each newly exposed source independently. A neighbouring fork
// must not turn an uninterrupted path into an extra expansion button,
// including when both sources share the same language/preview column.
export function linearGroups(branch: Branch, children: (head: Branch["groups"][number]["head"]) => Branch[]): Branch["groups"] {
  return branch.groups.filter((g) => !g.own && unbranchedLineage({ ...branch, groups: [g], words: g.words }, children));
}

// A selection can finish a small subtree with at most one split into parallel,
// uninterrupted paths. Path length and language names do not matter. An
// unknown continuation or another split requires another deliberate choice.
export function canFinishTogether(branches: Branch[], children: (head: Branch["groups"][number]["head"]) => Branch[]): boolean {
  const words = new Set(branches.flatMap((b) => b.words.map((w) => w.item.term)));
  if (!words.size || words.size > 4) return false;
  let forks = 0;
  const visit = (frontier: Branch[], ancestors: Set<Branch["groups"][number]["head"]>): boolean => {
    if (!frontier.length || frontier.some((b) => !b.words.length || !b.groups.length)) return false;
    const groups = frontier.flatMap((b) => b.groups);
    if (groups.length > 1 && ++forks > 1) return false;
    return groups.every((g) => {
      if (g.own) return true;
      if (isUndefinedRoot(g.head) || ancestors.has(g.head)) return false;
      return visit(children(g.head), new Set([...ancestors, g.head]));
    });
  };
  return visit(branches, new Set());
}
