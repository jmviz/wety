import type { Rect } from "./layout";

export interface LanguageAnchor {
  box: Rect;
  id?: string;
  preview?: boolean;
}

// One heading per language in the current view, carried by its highest
// visible card. A partially visible card keeps its heading inside the view.
export function languageAnchors(entries: LanguageAnchor[], viewport: Rect, gap = 24): LanguageAnchor[] {
  const visible = entries.filter(({ box }) => box.x < viewport.x + viewport.w && box.x + box.w > viewport.x &&
    box.y < viewport.y + viewport.h && box.y + box.h > viewport.y);
  const first = visible.sort((a, b) => a.box.y - b.box.y || a.box.x - b.box.x)[0];
  if (!first) return [];
  return [{ ...first, box: { ...first.box, x: Math.max(viewport.x + 8, first.box.x),
    y: Math.max(viewport.y + gap + 8, first.box.y) } }];
}
