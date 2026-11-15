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

// Always fit the newly exposed branch from source to frontier. Keep the
// clicked cluster as context only when it fits at the same reading scale.
export function expansionView(clicked: Rect, expanded: Rect, viewport: CameraViewport) {
  const scale = viewport.scale;
  const pad = 48 / scale;
  const width = Math.max(1, viewport.w - 2 * pad);
  const height = Math.max(1, viewport.h - 2 * pad);
  const k = Math.min(1 / scale, width / Math.max(1, expanded.w), height / Math.max(1, expanded.h));
  const x = Math.min(clicked.x, expanded.x);
  const y = Math.min(clicked.y, expanded.y);
  const right = Math.max(clicked.x + clicked.w, expanded.x + expanded.w);
  const bottom = Math.max(clicked.y + clicked.h, expanded.y + expanded.h);
  const contextFits = (right - x) * k <= width && (bottom - y) * k <= height;
  const frame = contextFits ? { x, y, w: right - x, h: bottom - y } : expanded;
  return {
    x: viewport.x + viewport.w / 2 - (frame.x + frame.w / 2) * k,
    y: viewport.y + viewport.h / 2 - (frame.y + frame.h / 2) * k,
    k,
  };
}
