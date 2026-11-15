import type { Branch } from "./branches";

// New click groups sit nearest the remaining preview. Keep source words
// revealed by the same click together, in their existing lane order.
export function newestFirst<T extends { revealed: number }>(items: T[]): T[] {
  return [...items].sort((a, b) => b.revealed - a.revealed);
}

export interface ExpansionHub {
  id: string;
  via: string | null;
  branches: Branch[];
  shared?: { host: { id: string }; branch: Branch }[];
}

export interface ExpansionDepth {
  depth: number;
  complete: boolean;
}

// Once a root is opened, its revealed child nodes can open together only
// when every one of them leads to exactly one displayed modern word.
export function singleDescendantChildren(children: Pick<ExpansionHub, "branches">[]): boolean {
  return children.length > 0 && children.every((h) => h.branches.length > 0
    && h.branches.every((b) => b.words.length === 1));
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

// Count the opened source stages, rather than rendered boxes or chronology
// columns. "Complete" means every visible path has reached the target
// language; another sibling can need more stages to reach that same horizon.
export function expansionDepth(id: string, hubs: ExpansionHub[]): ExpansionDepth {
  const children = new Map<string, ExpansionHub[]>();
  for (const h of hubs) if (h.via) {
    const list = children.get(h.via) ?? [];
    list.push(h);
    children.set(h.via, list);
  }
  const visit = (blob: string): ExpansionDepth => {
    const open = children.get(blob);
    if (!open?.length) return { depth: 0, complete: false };
    const profiles = open.map((h) => {
      const branches = [
        ...h.branches.map((b) => b.groups.every((g) => g.own)
          ? { depth: 0, complete: true } : visit(`${h.id}/${b.key}`)),
        ...(h.shared ?? []).map((s) => visit(`${s.host.id}/${s.branch.key}`)),
      ];
      return {
        depth: 1 + Math.max(0, ...branches.map((p) => p.depth)),
        complete: branches.every((p) => p.complete),
      };
    });
    return { depth: Math.max(...profiles.map((p) => p.depth)), complete: profiles.every((p) => p.complete) };
  };
  return visit(id);
}

export function sharedDepth(profiles: ExpansionDepth[]): ExpansionDepth | null {
  const open = profiles.filter((p) => p.depth > 0);
  return open.length ? { depth: Math.max(...open.map((p) => p.depth)), complete: open.some((p) => p.complete) } : null;
}
