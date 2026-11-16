import { CARD_GAP, type Layout, type Rect, type RootSpec } from "./layout.ts";

export interface DensityNode {
  id: string;
  width: number;
  compact: boolean;
  children: DensityNode[];
}

// Decide from ordinary card widths, not the already compressed layout,
// so resizing cannot alternate between expanding and shrinking the cards.
export function compactBranches(roots: DensityNode[], availableWidth: number): Set<string> {
  const selected = new Set<string>();
  const span = (node: DensityNode): number => node.width + Math.max(0, ...node.children.map((c) => CARD_GAP + span(c)));
  const depth = (node: DensityNode): number => 1 + Math.max(0, ...node.children.map(depth));
  const select = (node: DensityNode) => {
    if (node.compact) selected.add(node.id);
    node.children.forEach(select);
  };
  for (const root of roots) {
    if (depth(root) >= 4 || span(root) > Math.max(800, availableWidth)) select(root);
  }
  return selected;
}

// Keep the stable tree layout, but remove empty horizontal strips from the
// visible rows. Off-screen sibling rows cannot reserve blank cable space.
export function compactVisibleLanes(at: Layout, root: RootSpec, viewport: Rect): Layout {
  const blobs = new Map(at.blobs), hubs = new Map(at.hubs), sources = new Map(at.sources);
  for (const branch of root.blobs) {
    const ids = new Set<string>(), hubIds = new Set<string>();
    const columns = new Set<string>(), owners = new Map<string, string>();
    const collect = (b: RootSpec["blobs"][number]) => {
      ids.add(b.id);
      if (b.clustered) columns.add(b.id);
      for (const lane of b.lanes ?? []) { hubIds.add(lane.hub); owners.set(lane.hub, b.id); collectChildren(lane.blobs); }
    };
    const collectChildren = (children: RootSpec["blobs"]) => children.forEach(collect);
    collect(branch);
    const base = at.blobs.get(branch.id);
    if (!base) continue;
    const side = at.sides.get(branch.id) ?? (base.x + base.w / 2 >= 0 ? 1 : -1);
    const origin = side > 0 ? base.x : base.x + base.w;
    const intervals: { start: number; end: number }[] = [];
    const add = (r: Rect) => {
      if (r.y >= viewport.y + viewport.h || r.y + r.h <= viewport.y) return;
      const start = side > 0 ? r.x - origin : origin - r.x - r.w;
      intervals.push({ start, end: start + r.w });
    };
    for (const id of ids) {
      const r = at.blobs.get(id);
      // Virtual source columns span many rows; use their actual cards.
      if (r && !columns.has(id)) add(r);
    }
    for (const id of hubIds) { const r = at.sources.get(id); if (r) add(r); }
    // Always keep the branch's starting column fixed.
    intervals.push({ start: 0, end: base.w });
    intervals.sort((a, b) => a.start - b.start);
    const gaps: { end: number; remove: number }[] = [];
    let end = intervals[0].end;
    for (const next of intervals.slice(1)) {
      if (next.start - end > 64) gaps.push({ end: next.start, remove: next.start - end - CARD_GAP });
      end = Math.max(end, next.end);
    }
    if (!gaps.length) continue;
    const shift = (start: number) => gaps.filter((g) => g.end <= start + 0.01).reduce((sum, g) => sum + g.remove, 0);
    const moved = (r: Rect): Rect => {
      const start = side > 0 ? r.x - origin : origin - r.x - r.w;
      return { ...r, x: r.x - side * shift(start) };
    };
    for (const id of ids) { const r = blobs.get(id); if (r) blobs.set(id, moved(r)); }
    for (const id of hubIds) {
      const r = at.sources.get(id);
      const p = at.hubs.get(id);
      if (r) sources.set(id, moved(r));
      if (p) {
        const b = at.blobs.get(owners.get(id)!);
        const start = b ? (side > 0 ? b.x - origin : origin - b.x - b.w) : side * (p.x - origin);
        hubs.set(id, { ...p, x: p.x - side * shift(start) });
      }
    }
  }
  return { ...at, blobs, hubs, sources };
}
