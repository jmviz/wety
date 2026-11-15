import assert from "node:assert/strict";
import test from "node:test";
import { cableRouter, join, layout, rootDirection, stemArrival, wire, whole } from "../src/constellation/layout.ts";

test("root cables flow into a branch stem outside its cards without an obstacle hook", () => {
  for (const side of [-1, 1]) for (const vertical of [-1, 1]) {
    const sources = [
      { x: side > 0 ? 100 : -700, y: vertical > 0 ? 260 : -600, w: 600, h: 38 },
      { x: side > 0 ? 100 : -700, y: vertical > 0 ? 420 : -440, w: 600, h: 38 },
    ];
    const x = side > 0 ? sources[0].x - 24 : sources[0].x + sources[0].w + 24;
    const arrival = stemArrival({ x: 0, y: 0 }, x, sources);
    assert.equal(arrival.y, vertical > 0 ? 228 : -370);
    const direction = vertical * Math.PI / 2;
    const line = join({ x: 0, y: 0 }, 96, { ...arrival, w: 0, h: 0 }, [], direction, true);
    assert.ok(line.d.includes(" C "));
    assert.ok(!line.d.includes(" L "));
    const coords = line.d.match(/-?\d+\.\d+/g).map(Number);
    assert.equal(coords[4], arrival.x); // Last control approaches along the stem.
    assert.ok(vertical * coords[5] < vertical * coords[7]);
  }
});

for (const side of [-1, 1]) {
  test(`proto root cards follow their own subtrees on the ${side === 1 ? "right" : "left"}`, () => {
    const leaf = (id, stage = "gmw-pro") => ({ id, stage, size: { w: 250, h: 64 }, gap: 44 });
    const branch = {
      id: "germanic", clustered: true, size: { w: 320, h: 38 },
      lanes: [
        { hub: "fra", mid: 20, blobs: [leaf("forget"), leaf("fret"), leaf("forgive")] },
        { hub: "fram", mid: 60, blobs: [leaf("from", "ang"), leaf("fro", "non"), leaf("frame")] },
        { hub: "furdu", mid: 100, blobs: [leaf("ford")] },
      ],
    };
    const root = { id: "root", r: 96, blobs: [branch] };
    const size = whole(branch);
    const before = { hubs: new Map([["root", { x: 0, y: 0 }]]), blobs: new Map(), sides: new Map(),
      seats: new Map([["germanic", { ...size, x: side > 0 ? 200 : -200 - size.w, y: 0 }]]) };
    const placed = layout(root, undefined, before);
    const column = placed.blobs.get("germanic");
    let previousBottom = -Infinity;
    for (const lane of branch.lanes) {
      const source = placed.sources.get(lane.hub);
      const children = lane.blobs.map((b) => placed.blobs.get(b.id));
      const top = Math.min(...children.map((b) => b.y));
      const bottom = Math.max(...children.map((b) => b.y + b.h));
      assert.equal(source.h, 38);
      assert.equal(source.x, column.x);
      assert.equal(source.y + source.h / 2, (top + bottom) / 2);
      assert.ok(top >= previousBottom + 26);
      assert.equal(placed.hubs.get(lane.hub).y, source.y + source.h / 2);
      for (const child of children) {
        assert.equal(side > 0 ? child.x - source.x - source.w : source.x - child.x - child.w, 44);
      }
      previousBottom = bottom;
    }
    const from = placed.blobs.get("from"), fro = placed.blobs.get("fro");
    assert.equal(fro.y - from.y - from.h, 28); // Room for the changed language heading.
    assert.ok(placed.sources.get("furdu").y > placed.sources.get("fra").y + 300);
  });
}

test("proto cards reserve separate lanes even before their children are measured", () => {
  const branch = { id: "proto", size: { w: 320, h: 38 }, clustered: true,
    lanes: ["first", "second", "third"].map((hub) => ({ hub, blobs: [] })) };
  assert.deepEqual(whole(branch), { w: 320, h: 166 });
  const placed = layout({ id: "root", r: 96, blobs: [branch] });
  const sources = [...placed.sources.values()];
  assert.equal(sources.length, 3);
  assert.equal(sources[1].y - sources[0].y, 64);
});

test("the preserved preview box keeps its measured height beside opened proto roots", () => {
  const branch = { id: "proto", size: { w: 320, h: 38 }, clustered: true, lanes: [
    { hub: "folded", height: 104, blobs: [] },
    { hub: "opened", blobs: [{ id: "child", size: { w: 250, h: 64 } }] },
  ] };
  const placed = layout({ id: "root", r: 96, blobs: [branch] });
  const folded = placed.sources.get("folded"), source = placed.sources.get("opened"), child = placed.blobs.get("child");
  assert.equal(folded.h, 104);
  assert.equal(source.h, 38);
  assert.equal(child.y - folded.y - folded.h, 26);
  assert.equal(source.y + source.h / 2, child.y + child.h / 2);
});

test("root siblings keep separate directions even with tall previews and previous bad seats", () => {
  const root = { id: "root", r: 96, blobs: [
    { id: "greek", size: { w: 320, h: 220 } },
    { id: "italic", size: { w: 320, h: 800 } },
    { id: "germanic", size: { w: 320, h: 500 } },
    { id: "slavic", size: { w: 250, h: 100 } },
  ] };
  const bad = { hubs: new Map([["root", { x: 0, y: 0 }]]), blobs: new Map(), sides: new Map(), seats: new Map([
    ["italic", { x: 150, y: -400, w: 320, h: 800 }],
    ["germanic", { x: 510, y: -250, w: 320, h: 500 }],
  ]) };
  for (const previous of [undefined, bad]) {
    const placed = layout(root, { w: 1000, h: 700 }, previous);
    const angles = root.blobs.map((b, i) => {
      const r = placed.blobs.get(b.id);
      const angle = Math.atan2(r.y + r.h / 2, r.x + r.w / 2);
      const intended = (-150 + i * 90) * Math.PI / 180;
      assert.ok(Math.abs(Math.atan2(Math.sin(angle - intended), Math.cos(angle - intended))) < 1e-6);
      return angle;
    });
    assert.ok(Math.abs(angles[1] - angles[2]) >= Math.PI / 3);
  }
});

test("root cables retain distinct departure directions when visible rack boxes move", () => {
  for (let i = 0; i < 4; i++) {
    const direction = rootDirection(i, 4);
    const line = join({ x: 0, y: 0 }, 96, { x: 300, y: 100, w: 320, h: 600 }, [], direction);
    const [, x, y] = /^M (-?[\d.]+) (-?[\d.]+)/.exec(line.d);
    assert.ok(Math.abs(+x - 96 * Math.cos(direction)) < 0.1);
    assert.ok(Math.abs(+y - 96 * Math.sin(direction)) < 0.1);
  }
});

test("root connectors route around a tall sibling instead of appearing to descend from it", () => {
  for (const side of [-1, 1]) {
    const obstacle = { x: side === 1 ? 150 : -350, y: -220, w: 200, h: 440 };
    const target = { x: side === 1 ? 450 : -650, y: -80, w: 200, h: 160 };
    const line = join({ x: 0, y: 0 }, 96, target, [obstacle]);
    assert.ok(line.d.includes(" L "));
    const coords = samplePath(line.d);
    assert.ok(Math.abs(Math.hypot(coords[0].x, coords[0].y) - 96) < 0.1);
    for (const { x, y } of coords)
      assert.ok(!(x > obstacle.x && x < obstacle.x + obstacle.w && y > obstacle.y && y < obstacle.y + obstacle.h));
    assertOrthogonalLines(line.d);
  }
  assert.ok(join({ x: 0, y: 0 }, 96, { x: 400, y: -50, w: 200, h: 100 }).d.includes(" Q "));
});

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

test("all connectors use the same early gutter before passing intermediate columns", () => {
  for (const side of [-1, 1]) {
    const link = wire({ x: 0, y: 100 }, { x: side * 900, y: 20 });
    assertOrthogonalLines(link.d);
    assert.ok(link.d.includes(`${side * 44}.0`));
    assert.deepEqual(link.end, { x: side * 900, y: 20 });
    const sibling = wire({ x: 0, y: 100 }, { x: side * 500, y: 200 });
    assertOrthogonalLines(sibling.d);
    assert.ok(sibling.d.includes(`${side * 44}.0`));
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


function assertOrthogonalLines(d) {
  let from;
  for (const segment of d.matchAll(/([MLQC]) ([^MLQC]+)/g)) {
    const n = segment[2].trim().split(/\s+/).map(Number);
    const to = { x: n.at(-2), y: n.at(-1) };
    if (segment[1] === "L") assert.ok(from.x === to.x || from.y === to.y, `Diagonal straight segment in ${d}`);
    from = to;
  }
}

function samplePath(d) {
  const points = [];
  let from;
  for (const segment of d.matchAll(/([MLQ]) ([^MLQC]+)/g)) {
    const n = segment[2].trim().split(/\s+/).map(Number);
    const to = { x: n.at(-2), y: n.at(-1) };
    if (!from) points.push(to);
    else for (let step = 1; step <= 100; step++) {
      const t = step / 100, u = 1 - t;
      points.push(segment[1] === "Q"
        ? { x: u*u*from.x + 2*u*t*n[0] + t*t*to.x, y: u*u*from.y + 2*u*t*n[1] + t*t*to.y }
        : { x: u*from.x + t*to.x, y: u*from.y + t*to.y });
    }
    from = to;
  }
  return points;
}

test("connectors contain only curves and horizontal or vertical straight runs", () => {
  for (const side of [-1, 1]) {
    for (const y of [-250, -4, 0, 4, 250]) assertOrthogonalLines(wire({ x: 0, y: 0 }, { x: side * 400, y }).d);
    const obstacle = { x: side > 0 ? 130 : -330, y: -180, w: 200, h: 360 };
    const target = { x: side > 0 ? 450 : -650, y: -70, w: 200, h: 140 };
    const d = join({ x: 0, y: 0 }, 96, target, [obstacle]).d;
    assertOrthogonalLines(d);
    assert.ok(d.includes(" Q "));
  }
});

test("unrelated connectors occupy separate turning lanes on both sides", () => {
  for (const side of [-1, 1]) {
    const route = cableRouter([]);
    const first = route({ x: 0, y: 0 }, { x: side * 300, y: 120 });
    const second = route({ x: 0, y: 160 }, { x: side * 300, y: 40 });
    const verticals = (d) => {
      let prior;
      const lanes = [];
      for (const match of d.matchAll(/([MLQ]) ([^MLQ]+)/g)) {
        const n = match[2].trim().split(/\s+/).map(Number);
        const end = { x: n.at(-2), y: n.at(-1) };
        if (match[1] === 'L' && prior?.x === end.x && prior.y !== end.y) lanes.push(end.x);
        prior = end;
      }
      return lanes;
    };
    assert.ok(verticals(first.d).every((x) => !verticals(second.d).includes(x)));
    assertOrthogonalLines(first.d);
    assertOrthogonalLines(second.d);
  }
});

test("connectors bypass unrelated cards even when the card spans the direct corridor", () => {
  for (const side of [-1, 1]) {
    const obstacle = { x: side > 0 ? 80 : -240, y: -30, w: 160, h: 260 };
    const route = cableRouter([obstacle]);
    const line = route({ x: 0, y: 0 }, { x: side * 340, y: 160 });
    assertOrthogonalLines(line.d);
    for (const p of samplePath(line.d)) assert.ok(!(p.x > obstacle.x && p.x < obstacle.x + obstacle.w
      && p.y > obstacle.y && p.y < obstacle.y + obstacle.h), JSON.stringify(p));
  }
});

test("independent turns leave space beside an existing family stem", () => {
  const route = cableRouter([], [{ x: 44, top: -100, bottom: 200 }]);
  const line = route({ x: 0, y: 0 }, { x: 300, y: 120 });
  assertOrthogonalLines(line.d);
  assert.ok(!line.d.includes("L 44.0 108.0"));
});
