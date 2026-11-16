import { LANGUAGE_HEADING_SPACE, type Rect, type Size } from "./layout.ts";

export interface LanguageAnchor {
  box: Rect;
  id?: string;
  preview?: boolean;
}

// One heading per language in the current view, carried by its highest
// card whose full heading fits. Advance to the next card before clipping,
// without moving any cards or pinning labels to the viewport.
export function languageAnchors(entries: LanguageAnchor[], viewport: Rect, labelSize?: Size, padding = 6): LanguageAnchor[] {
  const visible = entries.filter(({ box }) => box.x < viewport.x + viewport.w && box.x + box.w > viewport.x &&
    box.y < viewport.y + viewport.h && box.y + box.h > viewport.y);
  const eligible = labelSize ? visible.filter(({ box }) => {
    const x = box.x + (box.w - labelSize.w) / 2, y = box.y - LANGUAGE_HEADING_SPACE;
    return x >= viewport.x + padding && y >= viewport.y + padding &&
      x + labelSize.w <= viewport.x + viewport.w - padding &&
      y + labelSize.h <= viewport.y + viewport.h - padding;
  }) : visible;
  const first = eligible.sort((a, b) => a.box.y - b.box.y || a.box.x - b.box.x)[0];
  if (!first) return [];
  return [first];
}
