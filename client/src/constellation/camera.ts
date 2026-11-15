import type { Point, Rect } from "./layout";

export interface CameraViewport extends Rect {
  scale: number;
}

// Browser pinch zoom crops the layout viewport without resizing the stage.
// Camera coordinates must use the visible intersection, in stage-local CSS px.
export function visibleStage(stage: Rect, viewport: CameraViewport): CameraViewport {
  const x = Math.max(stage.x, viewport.x);
  const y = Math.max(stage.y, viewport.y);
  return {
    x: x - stage.x, y: y - stage.y,
    w: Math.max(0, Math.min(stage.x + stage.w, viewport.x + viewport.w) - x),
    h: Math.max(0, Math.min(stage.y + stage.h, viewport.y + viewport.h) - y),
    scale: viewport.scale,
  };
}

// Prefer the root at centre, but let it move within the middle fifth of
// the screen to make room for uneven branches before shrinking the diagram.
export function rootOverview(root: Point, bounds: Rect, viewport: CameraViewport) {
  const scale = viewport.scale;
  const pad = 48 / scale;
  const left = Math.max(root.x - bounds.x, 1);
  const right = Math.max(bounds.x + bounds.w - root.x, 1);
  const above = Math.max(root.y - bounds.y, 1);
  const below = Math.max(bounds.y + bounds.h - root.y, 1);
  const k = Math.max(0.1 / scale, Math.min(
    1 / scale,
    (viewport.w - 2 * pad) / bounds.w,
    (viewport.h - 2 * pad) / bounds.h,
    (viewport.w * 0.6 - pad) / Math.max(left, right),
    (viewport.h * 0.6 - pad) / Math.max(above, below),
  ));
  // At this scale, shift only as far from centre as the edges require.
  const x = Math.max(pad + left * k, Math.min(viewport.w / 2, viewport.w - pad - right * k));
  const y = Math.max(pad + above * k, Math.min(viewport.h / 2, viewport.h - pad - below * k));
  return { x: viewport.x + x - root.x * k, y: viewport.y + y - root.y * k, k };
}

// Favour the clicked node slightly. Fit the local transition where possible,
// but never shrink the text merely to include a large auto-expanded tree.
export function expansionView(clicked: Rect, expanded: Rect, viewport: CameraViewport) {
  const from = { x: clicked.x + clicked.w / 2, y: clicked.y + clicked.h / 2 };
  const to = { x: expanded.x + expanded.w / 2, y: expanded.y + expanded.h / 2 };
  const center = { x: from.x * 0.6 + to.x * 0.4, y: from.y * 0.6 + to.y * 0.4 };
  const left = Math.min(clicked.x, expanded.x);
  const right = Math.max(clicked.x + clicked.w, expanded.x + expanded.w);
  const top = Math.min(clicked.y, expanded.y) - 24;
  const bottom = Math.max(clicked.y + clicked.h, expanded.y + expanded.h);
  const width = 2 * Math.max(center.x - left, right - center.x, 1);
  const height = 2 * Math.max(center.y - top, bottom - center.y, 1);
  const scale = viewport.scale;
  const k = Math.max(0.85 / scale, Math.min(1 / scale, (viewport.w - 80 / scale) / width, (viewport.h - 120 / scale) / height));
  return { x: viewport.x + viewport.w / 2 - center.x * k, y: viewport.y + viewport.h / 2 - center.y * k, k };
}
