import type { BlobSpec, LaneSpec, Layout, Rect, RootSpec } from "./layout";

const intersects = (a: Rect, b: Rect) => a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y;

// Count actual visible words, not trays or the final-language column. A
// language already present along one uninterrupted run of paths needs no pull.
export function dominantLanguage(root: RootSpec, at: Layout, viewport: Rect): string | undefined {
  const counts = new Map<string, number>();
  const paths: { y: number; stages: Set<string> }[] = [];
  const visit = (b: BlobSpec, inherited: Set<string>) => {
    const box = at.blobs.get(b.id);
    if (!box) return;
    const lanes = b.lanes ?? [];
    if (!lanes.length) {
      if (box.y < viewport.y + viewport.h && box.y + box.h > viewport.y)
        paths.push({ y: box.y + box.h / 2, stages: inherited });
      return;
    }
    for (const lane of lanes) {
      const stages = new Set(inherited);
      const card = at.sources.get(lane.hub) ?? box;
      if (!lane.preview && !b.terminal && b.stage && intersects(card, viewport)) {
        counts.set(b.stage, (counts.get(b.stage) ?? 0) + 1);
        stages.add(b.stage);
      }
      if (lane.blobs.length) lane.blobs.forEach((child) => visit(child, stages));
      else if (card.y < viewport.y + viewport.h && card.y + card.h > viewport.y)
        paths.push({ y: card.y + card.h / 2, stages });
    }
  };
  root.blobs.forEach((b) => visit(b, new Set()));
  paths.sort((a, b) => a.y - b.y);
  return [...counts].map(([stage, count]) => {
    let runs = 0, before = false;
    for (const path of paths) {
      const has = path.stages.has(stage);
      if (has && !before) runs++;
      before = has;
    }
    return { stage, count, runs };
  }).filter((s) => s.count > 1 && s.runs > 1)
    .sort((a, b) => b.count - a.count || b.runs - a.runs || a.stage.localeCompare(b.stage))[0]?.stage;
}

type Edge = "top" | "bottom" | undefined;
interface Footprint { count: number; pure: boolean }

// Reorder whole connected subtrees only. Target-bearing neighbours turn
// their matching paths toward their shared boundary; pure matching subtrees
// go first when pulling upward, last when pulling downward. No edge is
// reparented and no individual word is detached from its ancestry.
export function gatherLanguage(root: RootSpec, stage: string): RootSpec {
  const footprints = new WeakMap<BlobSpec, Footprint>();
  const combine = (items: Footprint[]): Footprint => ({
    count: items.reduce((n, p) => n + p.count, 0),
    pure: items.length > 0 && items.every((p) => p.pure),
  });
  const footprint = (b: BlobSpec): Footprint => {
    const cached = footprints.get(b);
    if (cached) return cached;
    const words = b.stage === stage ? (b.lanes ?? []).filter((l) => !l.preview).length : 0;
    const result = words ? { count: words, pure: true }
      : combine((b.lanes ?? []).map((l) => combine(l.blobs.map(footprint))));
    footprints.set(b, result);
    return result;
  };
  const laneFootprint = (l: LaneSpec) => combine(l.blobs.map(footprint));
  const order = <T>(items: T[], info: (item: T) => Footprint, edge: Edge): T[] => {
    if (!edge) return items;
    return [...items].sort((a, b) => {
      const rank = (item: T) => info(item).pure ? 2 : info(item).count ? 1 : 0;
      return (edge === "top" ? -1 : 1) * (rank(a) - rank(b));
    });
  };
  const edges = <T>(items: T[], info: (item: T) => Footprint, edge: Edge): Edge[] => {
    const targets = items.flatMap((item, i) => info(item).count ? [i] : []);
    return items.map((_, i) => targets.length < 2 ? edge
      : i === targets[0] ? "bottom" : i === targets.at(-1) ? "top" : edge);
  };
  const visit = (b: BlobSpec, edge: Edge): BlobSpec => {
    if (!b.lanes?.length || b.stage === stage) return b;
    // Keep the tray edge and reveal batches fixed. Within a batch, language
    // affinity may reorder siblings, just as it does in the base layout.
    const lanes: LaneSpec[] = [];
    for (let i = 0; i < b.lanes.length;) {
      let end = i + 1;
      while (end < b.lanes.length && !b.lanes[i].preview && !b.lanes[end].preview
        && b.lanes[end].revealed === b.lanes[i].revealed) end++;
      lanes.push(...order(b.lanes.slice(i, end), laneFootprint, edge));
      i = end;
    }
    const laneEdges = edges(lanes, laneFootprint, edge);
    return { ...b, lanes: lanes.map((lane, i) => {
      const children = order(lane.blobs, footprint, laneEdges[i]);
      const childEdges = edges(children, footprint, laneEdges[i]);
      return { ...lane, blobs: children.map((child, j) => visit(child, childEdges[j])) };
    }) };
  };
  return { ...root, blobs: root.blobs.map((b) => visit(b, undefined)) };
}
