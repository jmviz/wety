import assert from "node:assert/strict";
import test from "node:test";
import { expansionView, rootOverview } from "../src/constellation/camera.ts";

test("asymmetric root overview centres the root and fits all four diagram edges", () => {
  const root = { x: 40, y: -20 };
  const bounds = { x: -500, y: -1000, w: 1200, h: 1300 };
  for (const viewport of [{ w: 1460, h: 819 }, { w: 900, h: 700 }]) {
    const v = rootOverview(root, bounds, viewport);
    assert.equal(v.x + root.x * v.k, viewport.w / 2);
    assert.equal(v.y + root.y * v.k, viewport.h / 2);
    assert.ok(v.x + bounds.x * v.k >= 47);
    assert.ok(v.y + bounds.y * v.k >= 47);
    assert.ok(v.x + (bounds.x + bounds.w) * v.k <= viewport.w - 47);
    assert.ok(v.y + (bounds.y + bounds.h) * v.k <= viewport.h - 47);
  }
});

test("expansion camera centres 60 percent toward the clicked node on either side", () => {
  for (const side of [-1, 1]) {
    const clicked = { x: -100, y: 0, w: 200, h: 100 };
    const expanded = { x: side * 500 - 100, y: 150, w: 200, h: 100 };
    const size = { w: 1200, h: 800 };
    const view = expansionView(clicked, expanded, size);
    assert.equal((size.w / 2 - view.x) / view.k, side * 200);
    assert.equal((size.h / 2 - view.y) / view.k, 110);
    assert.ok(view.k >= 0.85 && view.k <= 1);
  }
});

test("large expansions keep readable text rather than fitting the entire tree", () => {
  const view = expansionView({ x: 0, y: 0, w: 320, h: 200 },
    { x: 500, y: -2000, w: 2000, h: 4000 }, { w: 1100, h: 800 });
  assert.equal(view.k, 0.85);
});

test("small expansions restore normal reading scale", () => {
  const view = expansionView({ x: 0, y: 0, w: 200, h: 38 },
    { x: 244, y: 0, w: 180, h: 38 }, { w: 1200, h: 800 });
  assert.equal(view.k, 1);
});
