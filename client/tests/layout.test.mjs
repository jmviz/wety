import assert from "node:assert/strict";
import test from "node:test";
import { layout, wire } from "../src/constellation/layout.ts";

function fixture() {
  return {
    id: "root", r: 96,
    blobs: [{
      id: "branch", size: { w: 320, h: 200 },
      lanes: [
        {
          hub: "long", mid: 40,
          blobs: [{
            id: "latin", size: { w: 320, h: 100 },
            lanes: [{
              hub: "latin-word", mid: 50, inline: true,
              blobs: [{ id: "pyx", terminal: true, size: { w: 300, h: 40 } }],
            }],
          }],
        },
        {
          hub: "short", mid: 100, inline: true,
          blobs: [{ id: "amphoteric", terminal: true, size: { w: 250, h: 40 } }],
        },
        {
          hub: "list", mid: 160,
          blobs: [{ id: "english-list", terminal: true, size: { w: 320, h: 100 } }],
        },
      ],
    }],
  };
}

test("long terminal connectors turn before passing intermediate columns", () => {
  for (const side of [-1, 1]) {
    const link = wire({ x: 0, y: 100 }, { x: side * 900, y: 20 }, true);
    assert.ok(link.d.includes(`${side * 112}.0 20.0 L ${side * 900}.0 20.0`));
    assert.deepEqual(link.end, { x: side * 900, y: 20 });
  }
});

for (const side of [-1, 1]) {
  test(`English endpoints share the outer edge on the ${side === 1 ? "right" : "left"}`, () => {
    const spec = fixture();
    // A generous existing seat fixes which side this branch occupies.
    const before = {
      hubs: new Map([["root", { x: 0, y: 0 }]]),
      blobs: new Map(), sides: new Map(),
      seats: new Map([["branch", { x: side === 1 ? 200 : -1296, y: -100, w: 1096, h: 266 }]]),
    };
    // First find the actual dimensions, then pin that seat on the desired side.
    const initial = layout(spec);
    const size = initial.seats.get("branch");
    before.seats.set("branch", { ...size, x: side === 1 ? 200 : -200 - size.w });
    const placed = layout(spec, undefined, before);
    assert.equal(placed.sides.get("branch"), side);
    const end = (id) => {
      const rect = placed.blobs.get(id);
      return side === 1 ? rect.x + rect.w : rect.x;
    };
    assert.equal(end("amphoteric"), end("pyx"));
    assert.equal(end("english-list"), end("pyx"));
    const short = placed.blobs.get("amphoteric");
    assert.equal(short.y + short.h / 2, placed.hubs.get("short").y);
    const latin = placed.blobs.get("latin");
    const start = placed.hubs.get("long").x;
    assert.equal(side === 1 ? latin.x - start : start - latin.x - latin.w, 112);
    const boxes = ["pyx", "amphoteric", "english-list"].map((id) => placed.blobs.get(id));
    for (let i = 0; i < boxes.length; i++) for (let j = i + 1; j < boxes.length; j++) {
      const a = boxes[i], b = boxes[j];
      assert.ok(a.y + a.h <= b.y || b.y + b.h <= a.y);
    }
  });
}
