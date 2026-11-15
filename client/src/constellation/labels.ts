import type { Rect } from "./layout";

export interface LanguageAnchor {
  box: Rect;
  cluster: string;
  branching: boolean;
}

// Share a heading locally, not across an entire language column. Repeat
// it at independent clusters, major forks, or after a long vertical run.
export function languageAnchors(entries: LanguageAnchor[]): LanguageAnchor[] {
  const columns: LanguageAnchor[][] = [];
  for (const entry of [...entries].sort((a, b) => a.box.x - b.box.x || a.box.y - b.box.y)) {
    let column = columns.find((c) => Math.abs(c[0].box.x - entry.box.x) < 72);
    if (!column) columns.push(column = []);
    column.push(entry);
  }
  return columns.flatMap((column) => {
    const labels: LanguageAnchor[] = [];
    for (const entry of column.sort((a, b) => a.box.y - b.box.y)) {
      const last = labels.at(-1);
      if (!last || entry.box.y - last.box.y >= 260 ||
        ((last.cluster !== entry.cluster || entry.branching) && entry.box.y - last.box.y >= 64)) labels.push(entry);
    }
    return labels;
  });
}
