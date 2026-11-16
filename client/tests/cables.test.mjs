import assert from "node:assert/strict";
import test from "node:test";
import { cables } from "../src/constellation/cables.ts";

const branch = { key: "gem-pro", name: "Proto-Germanic", groups: [], words: [], wordCount: 0, hasSelected: false };
const hub = (id, via = null, branches = []) =>
  ({ id, via, branches, hues: new Map(branches.map((b) => [b.key, "1,2,3"])), hue: via ? "1,2,3" : null, shared: [] });

function scene({ sources = new Map(), entries = [], recent = new Set(), heads = [] } = {}) {
  const root = hub("r", null, [branch]);
  const box = { x: 300, y: -40, w: 320, h: 80 };
  return {
    hubs: [root, ...heads],
    at: {
      hubs: new Map([["r", { x: 0, y: 0 }], ...heads.map((h, i) => [h.id, { x: 640, y: i * 100 }])]),
      blobs: new Map([["r/gem-pro", box]]),
      seats: new Map([["r/gem-pro", box]]),
      sides: new Map([["r/gem-pro", 1]]),
      sources,
    },
    rootRadius: 96,
    recent,
    drawn: () => box,
    entries: () => entries,
    clustered: () => entries.length > 0,
    remainingGroups: () => [],
  };
}

test("a folded branch gets one cable from the root, in its branch's colour", () => {
  const out = cables(scene());
  assert.equal(out.length, 1);
  assert.equal(out[0].hue, "1,2,3");
  assert.equal(out[0].fresh, false);
  assert.match(out[0].d, /^M /);
});

test("an opened column has one cable to its stem, the stem, and a wire to each card", () => {
  const a = { x: 300, y: -100, w: 200, h: 38 }, b = { x: 300, y: 60, w: 200, h: 38 };
  const heads = [hub("a", "r/gem-pro"), hub("b", "r/gem-pro")];
  const out = cables(scene({
    sources: new Map([["a", a], ["b", b]]),
    entries: heads.map((h) => ({ id: h.id, head: h })),
    recent: new Set(["a"]),
    heads,
  }));
  assert.equal(out.length, 4);
  const stem = out.find((c) => /^M 276 -81 L 276 79$/.test(c.d));
  assert.ok(stem, "a vertical stem 24 px outside the cards, between their centres");
  assert.deepEqual(out.map((c) => c.fresh), [false, false, true, false]);
});
