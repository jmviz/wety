import assert from "node:assert/strict";
import test from "node:test";
import { expansionView } from "../src/constellation/camera.ts";

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
