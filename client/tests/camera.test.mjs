import assert from "node:assert/strict";
import test from "node:test";
import { expansionView, rootOverview, visibleStage } from "../src/constellation/camera.ts";

test("asymmetric overview grows the diagram while keeping the root near centre and every edge visible", () => {
  const root = { x: 40, y: -20 };
  const bounds = { x: -500, y: -1000, w: 1200, h: 1300 };
  for (const viewport of [{ w: 1460, h: 819 }, { w: 900, h: 700 }]) {
    const v = rootOverview(root, bounds, { x: 0, y: 0, scale: 1, ...viewport });
    assert.equal(v.x + root.x * v.k, viewport.w / 2);
    assert.ok(v.y + root.y * v.k > viewport.h / 2);
    assert.ok(v.y + root.y * v.k <= viewport.h * 0.6 + 0.001);
    const strictlyCentered = Math.min(1, (viewport.w - 96) / 1320, (viewport.h - 96) / 1960);
    assert.ok(v.k > strictlyCentered * 1.15);
    assert.ok(v.x + bounds.x * v.k >= 47);
    assert.ok(v.y + bounds.y * v.k >= 47);
    assert.ok(v.x + (bounds.x + bounds.w) * v.k <= viewport.w - 47);
    assert.ok(v.y + (bounds.y + bounds.h) * v.k <= viewport.h - 47);
  }
});

test("expansion centres the new branch vertically even when the old cluster fits above it", () => {
  const clicked = { x: 0, y: -300, w: 320, h: 300 };
  const expanded = { x: 0, y: 60, w: 700, h: 200 };
  const viewport = { x: 0, y: 0, w: 1200, h: 800, scale: 1 };
  const view = expansionView(clicked, expanded, viewport);
  assert.equal(view.k, 1);
  assert.equal(view.x + 350, 600);
  assert.equal(view.y + 160, 400);
});

test("long automatic lineages fit both their historical root and youngest descendant", () => {
  const expanded = { x: 100, y: 400, w: 2200, h: 100 };
  const view = expansionView({ x: 100, y: -1200, w: 320, h: 1500 }, expanded,
    { x: 0, y: 0, w: 1400, h: 800, scale: 1 });
  assert.ok(view.k < 0.85);
  assert.ok(view.x + expanded.x * view.k >= 47.99);
  assert.ok(view.x + (expanded.x + expanded.w) * view.k <= 1352.01);
  assert.equal(view.y + (expanded.y + expanded.h / 2) * view.k, 400);
});

test("small expansions restore normal reading scale", () => {
  const view = expansionView({ x: 0, y: 0, w: 200, h: 38 },
    { x: 244, y: 0, w: 180, h: 38 }, { x: 0, y: 0, w: 1200, h: 800, scale: 1 });
  assert.equal(view.k, 1);
});


test("retained browser pinch zoom fits the actual visible area with the same screen margins", () => {
  const scale = 1.3704592;
  const viewport = { x: 130, y: 70, w: 1465 / scale, h: 828 / scale, scale };
  const area = visibleStage({ x: 0, y: 0, w: 1465, h: 828 }, viewport);
  const root = { x: 0, y: 0 };
  const bounds = { x: -600, y: -550, w: 1200, h: 800 };
  const view = rootOverview(root, bounds, area);
  assert.ok(Math.abs((view.x - viewport.x) * scale - 1465 / 2) < 0.001);
  const normal = rootOverview(root, bounds, { x: 0, y: 0, w: 1465, h: 828, scale: 1 });
  assert.ok(Math.abs((view.y - viewport.y) * scale - normal.y) < 0.001);
  assert.ok(Math.abs(view.k * scale - normal.k) < 0.001);
  assert.ok((view.x + bounds.x * view.k - viewport.x) * scale >= 47.9);
  assert.ok((view.y + bounds.y * view.k - viewport.y) * scale >= 47.9);
  assert.ok((view.x + (bounds.x + bounds.w) * view.k - viewport.x) * scale <= 1465 - 47.9);
  assert.ok((view.y + (bounds.y + bounds.h) * view.k - viewport.y) * scale <= 828 - 47.9);
});

test("visible area is clipped to the stage and expressed relative to its origin", () => {
  assert.deepEqual(visibleStage({ x: 20, y: 80, w: 1000, h: 900 },
    { x: 50, y: 0, w: 800, h: 700, scale: 1.25 }),
    { x: 30, y: 0, w: 800, h: 620, scale: 1.25 });
});

test("expansion reading size accounts for browser magnification and viewport offset", () => {
  const clicked = { x: 0, y: 0, w: 200, h: 38 };
  const expanded = { x: 244, y: 0, w: 180, h: 38 };
  const normal = expansionView(clicked, expanded, { x: 0, y: 0, w: 1200, h: 800, scale: 1 });
  const zoomed = expansionView(clicked, expanded, { x: 90, y: 40, w: 800, h: 800 / 1.5, scale: 1.5 });
  assert.ok(Math.abs((zoomed.x - 90) * 1.5 - normal.x) < 0.001);
  assert.ok(Math.abs((zoomed.y - 40) * 1.5 - normal.y) < 0.001);
  assert.equal(zoomed.k * 1.5, normal.k);
});


test("balanced and small overviews keep the root exactly centred", () => {
  const viewport = { x: 20, y: 10, w: 1200, h: 800, scale: 1 };
  for (const bounds of [{ x: -500, y: -500, w: 1000, h: 1000 },
    { x: -200, y: -100, w: 300, h: 300 }]) {
    const view = rootOverview({ x: 0, y: 0 }, bounds, viewport);
    assert.equal(view.x, 620);
    assert.equal(view.y, 410);
  }
});

test("composition makes equal room for overflow in any direction", () => {
  for (const side of [-1, 1]) {
    const viewport = { x: 0, y: 0, w: 1000, h: 800, scale: 1 };
    const bounds = { x: side === -1 ? -1000 : -200, y: -100, w: 1200, h: 200 };
    const view = rootOverview({ x: 0, y: 0 }, bounds, viewport);
    assert.ok(Math.abs(view.x - (side === -1 ? 600 : 400)) < 0.001);
    assert.equal(view.y, 400);
    assert.ok(view.x + bounds.x * view.k >= 47.9);
    assert.ok(view.x + (bounds.x + bounds.w) * view.k <= 952.1);
  }
});


test("expansion shifts the new branch down enough to retain its source-language heading", () => {
  const expanded = { x: 0, y: 60, w: 700, h: 200 };
  const label = { x: 28, y: -324, w: 160, h: 16 };
  const view = expansionView({ x: 0, y: -300, w: 320, h: 300 }, expanded,
    { x: 0, y: 0, w: 1200, h: 800, scale: 1 }, label);
  assert.ok(view.k < 1);
  assert.ok(Math.abs(view.y + label.y * view.k - 48) < 0.001);
  assert.ok(Math.abs(view.y + 160 * view.k - 400) <= 40.001);
  assert.ok(view.y + (expanded.y + expanded.h) * view.k <= 752);
});
