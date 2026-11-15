import type { Rect, Size } from "./layout";

// Favour the clicked node slightly. Fit the local transition where possible,
// but never shrink the text merely to include a large auto-expanded tree.
export function expansionView(clicked: Rect, expanded: Rect, viewport: Size) {
  const from = { x: clicked.x + clicked.w / 2, y: clicked.y + clicked.h / 2 };
  const to = { x: expanded.x + expanded.w / 2, y: expanded.y + expanded.h / 2 };
  const center = { x: from.x * 0.6 + to.x * 0.4, y: from.y * 0.6 + to.y * 0.4 };
  const left = Math.min(clicked.x, expanded.x);
  const right = Math.max(clicked.x + clicked.w, expanded.x + expanded.w);
  const top = Math.min(clicked.y, expanded.y) - 24;
  const bottom = Math.max(clicked.y + clicked.h, expanded.y + expanded.h);
  const width = 2 * Math.max(center.x - left, right - center.x, 1);
  const height = 2 * Math.max(center.y - top, bottom - center.y, 1);
  const k = Math.max(0.85, Math.min(1, (viewport.w - 80) / width, (viewport.h - 120) / height));
  return { x: viewport.w / 2 - center.x * k, y: viewport.h / 2 - center.y * k, k };
}
