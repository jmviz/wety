// Dates here guide spacing, not assertions about when an individual word
// existed. The source periods stay unchanged in the dating registry.
import type { BlobSpec } from "./layout";
import type { LangTree } from "./branches";

interface Range {
  earliest: number | null;
  latest: number | null;
}

// Keep neighboring lanes with the same later language sequence together.
// The target language alone is not evidence: every path ends there.
export function orderLanguagePaths<T>(items: T[], paths: (item: T) => string[][]): T[] {
  if (items.length < 2) return items;
  const remaining = items.map((item) => ({ item, paths: paths(item) }));
  const ordered = [remaining.shift()!];
  while (remaining.length) {
    const previous = ordered[ordered.length - 1];
    const score = (candidate: typeof previous) => Math.max(0, ...previous.paths.flatMap((a) => candidate.paths.map((b) => {
      let same = 0;
      while (same < Math.min(a.length, b.length) && a[a.length - 1 - same] === b[b.length - 1 - same]) same++;
      return same;
    })));
    let best = 0;
    for (let i = 1; i < remaining.length; i++) if (score(remaining[i]) > score(remaining[best])) best = i;
    ordered.push(remaining.splice(best, 1)[0]);
  }
  return ordered.map(({ item }) => item);
}

export function groupLanguageLanes(node: BlobSpec): BlobSpec {
  const paths = (b: BlobSpec): string[][] => {
    if (b.terminal) return [[]];
    const next = (b.lanes ?? []).flatMap((l) => l.blobs.flatMap(paths));
    return (next.length ? next : [[]]).map((p) => [b.stage ?? b.id, ...p]);
  };
  const lanes = node.lanes?.map((lane) => ({ ...lane, blobs: orderLanguagePaths(lane.blobs.map(groupLanguageLanes), paths) }));
  if (!lanes) return node;
  // Order inward to outward first. Language affinity can reorder sources
  // from one click, but cannot pull an older source between the tray and a
  // new breakout. Mirroring vertically keeps this invariant above PIE too.
  const batches = new Map<number, typeof lanes>();
  for (const lane of lanes.filter((l) => !l.preview)) {
    const revealed = lane.revealed ?? 0;
    const batch = batches.get(revealed) ?? [];
    batch.push(lane);
    batches.set(revealed, batch);
  }
  const ordered = [
    ...lanes.filter((l) => l.preview),
    ...[...batches].sort(([a], [b]) => b - a).flatMap(([, batch]) => orderLanguagePaths(batch, (lane) => lane.blobs.flatMap(paths))),
  ];
  return { ...node, lanes: node.outwardY === -1 ? ordered.reverse() : ordered };
}

export interface LayoutPeriod {
  code: string;
  spoken: { start: Range | null; end: Range | null };
  estimate?: { value: number };
}

const middle = (r: Range | null): number | undefined => {
  const values = [r?.earliest, r?.latest].filter((n): n is number => typeof n === "number");
  return values.length ? values.reduce((sum, n) => sum + n, 0) / values.length : undefined;
};

// Complete periods use their midpoint; a published estimate keeps its own
// central value. One-sided periods and undated stages are spacing hints:
// an end alone is placed between that end and a dated ancestor, and an
// undated stage between the nearest dated stages of its language lineage.
export function languageTimes(periods: LayoutPeriod[], langs: LangTree, target: string, now: number): Map<string, number> {
  const years = new Map<string, number>();
  const ends = new Map<string, number>();
  for (const p of periods) {
    const start = middle(p.spoken.start);
    const end = middle(p.spoken.end);
    if (start !== undefined && end !== undefined) years.set(p.code, (start + end) / 2);
    else if (p.estimate) years.set(p.code, p.estimate.value);
    else if (start !== undefined) years.set(p.code, start);
    else if (end !== undefined) ends.set(p.code, end);
  }
  const ancestor = (code: string): { code: string; hops: number } | undefined => {
    const seen = new Set([code]);
    let hops = 0;
    while (langs.parents[code] && !seen.has(langs.parents[code])) {
      code = langs.parents[code];
      seen.add(code);
      hops++;
      if (years.has(code)) return { code, hops };
    }
  };
  for (const [code, end] of ends) {
    const before = ancestor(code);
    years.set(code, before ? (years.get(before.code)! + end) / 2 : end);
  }
  if (!years.has(target)) years.set(target, now);
  // Keep inferred points from becoming evidence for other inferred points.
  const anchors = new Map(years);
  const inferred = new Map<string, number>();
  const descendants = new Map<string, { year: number; hops: number }>();
  for (const [descendant, year] of anchors) {
    const seen = new Set([descendant]);
    let at = descendant;
    let hops = 0;
    while (langs.parents[at] && !seen.has(langs.parents[at])) {
      at = langs.parents[at];
      seen.add(at);
      hops++;
      const had = descendants.get(at);
      if (!had || hops < had.hops || (hops === had.hops && year < had.year)) descendants.set(at, { year, hops });
    }
  }
  const codes = new Set([...Object.keys(langs.parents), ...Object.values(langs.parents)]);
  for (const code of codes) {
    if (anchors.has(code)) continue;
    const before = ancestor(code);
    const after = descendants.get(code);
    if (before) {
      const start = anchors.get(before.code)!;
      const end = after?.year ?? now;
      inferred.set(code, start + (end - start) * before.hops / (before.hops + (after?.hops ?? 1)));
    } else if (after) inferred.set(code, after.year - after.hops * 500);
  }
  return new Map([...years, ...inferred]);
}

// Languages share chronological columns within one root branch. Real
// graph edges take precedence over overlapping or incomplete period dates:
// a borrowed word must still stand beyond the word it came from.
export function chronological(b: BlobSpec): BlobSpec {
  if (!b.stage) return b;
  const stages = new Map<string, { year: number; width: number; gap: number; next: Set<string>; incoming: number; floor: number }>();
  const visit = (node: BlobSpec, parent?: string) => {
    if (!node.size || !node.stage) return;
    let s = stages.get(node.stage);
    if (!s) stages.set(node.stage, (s = { year: node.year ?? 0, width: 0, gap: 0, next: new Set(), incoming: 0, floor: 0 }));
    s.width = Math.max(s.width, node.size.w);
    s.gap = Math.max(s.gap, node.compactChain ? 24 : 44);
    if (parent && parent !== node.stage && !stages.get(parent)!.next.has(node.stage)) {
      stages.get(parent)!.next.add(node.stage);
      s.incoming++;
    }
    for (const lane of node.lanes ?? []) for (const child of lane.blobs) visit(child, node.stage);
  };
  visit(b);
  const offsets = new Map<string, number>();
  let previous: { year: number; width: number } | undefined;
  let x = 0;
  while (offsets.size < stages.size) {
    const left = [...stages].filter(([key]) => !offsets.has(key));
    const ready = left.filter(([, s]) => s.incoming === 0);
    // A borrowing cycle can revisit a language. Break the column tie here;
    // the per-node minimum below still keeps every actual edge outward.
    const [key, s] = (ready.length ? ready : left).sort((a, b) => a[1].year - b[1].year || a[0].localeCompare(b[0]))[0];
    // Sibling languages occupy different vertical lanes and can share
    // horizontal space. Reserve a full box only along an actual edge;
    // chronology needs a modest stagger, not a box-wide column per language.
    if (previous) x += Math.min(240, Math.max(72, (s.year - previous.year) * 0.1));
    x = Math.max(x, s.floor);
    offsets.set(key, x);
    previous = s;
    for (const child of s.next) {
      const next = stages.get(child)!;
      next.incoming--;
      next.floor = Math.max(next.floor, x + s.width + s.gap);
    }
  }
  const place = (node: BlobSpec, at: number, terminalStart = 0): BlobSpec => ({
    ...node,
    lanes: node.lanes?.map((lane) => ({
      ...lane,
      blobs: lane.blobs.map((child) => {
        const run = node.compactChain && child.compactChain ? 24 : 44;
        const next = Math.max(
          at + (node.size?.w ?? 0) + run,
          offsets.get(child.stage!) ?? 0,
          child.terminal ? terminalStart : 0
        );
        return { ...place(child, next, terminalStart), gap: next - at - (node.size?.w ?? 0) };
      }),
    })),
  });
  // Even a folded language node can be wider than its chronological
  // stagger. Keep the entire final-word column beyond every such box,
  // including ones in other vertical lanes and repeated language stages.
  let terminalStart = 0;
  const measure = (node: BlobSpec, at: number) => {
    if (!node.size) return;
    if (!node.terminal) terminalStart = Math.max(terminalStart, at + node.size.w + (node.compactChain ? 24 : 44));
    for (const lane of node.lanes ?? []) for (const child of lane.blobs) {
      measure(child, at + node.size.w + (child.gap ?? 44));
    }
  };
  measure(place(b, 0), 0);
  return place(b, 0, terminalStart);
}
