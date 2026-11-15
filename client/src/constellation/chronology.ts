// Dates here guide spacing, not assertions about when an individual word
// existed. The source periods stay unchanged in the dating registry.
import type { BlobSpec } from "./layout";
import type { LangTree } from "./branches";

interface Range {
  earliest: number | null;
  latest: number | null;
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
  const stages = new Map<string, { year: number; width: number; next: Set<string>; incoming: number }>();
  const visit = (node: BlobSpec, parent?: string) => {
    if (!node.size || !node.stage) return;
    let s = stages.get(node.stage);
    if (!s) stages.set(node.stage, (s = { year: node.year ?? 0, width: 0, next: new Set(), incoming: 0 }));
    s.width = Math.max(s.width, node.size.w);
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
    if (previous) x += Math.max(previous.width + 112, (s.year - previous.year) * 0.3);
    offsets.set(key, x);
    previous = s;
    for (const child of s.next) stages.get(child)!.incoming--;
  }
  const place = (node: BlobSpec, at: number): BlobSpec => ({
    ...node,
    lanes: node.lanes?.map((lane) => ({
      ...lane,
      blobs: lane.blobs.map((child) => {
        const run = lane.inline ? 44 : 112;
        const next = Math.max(at + (node.size?.w ?? 0) + run, offsets.get(child.stage!) ?? 0);
        return { ...place(child, next), gap: next - at - (node.size?.w ?? 0) };
      }),
    })),
  });
  return place(b, 0);
}
