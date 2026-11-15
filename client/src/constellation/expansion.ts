import type { Branch } from "./branches";

export interface ExpansionHub {
  id: string;
  via: string | null;
  branches: Branch[];
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
      const branches = h.branches.map((b) => b.groups.every((g) => g.own)
        ? { depth: 0, complete: true } : visit(`${h.id}/${b.key}`));
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
