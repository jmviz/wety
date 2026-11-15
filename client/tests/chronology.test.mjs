import assert from "node:assert/strict";
import test from "node:test";
import { chronological, languageTimes } from "../src/constellation/chronology.ts";
import { layout } from "../src/constellation/layout.ts";

test("long compact ancestry chains retain all stages with 24px gaps and aligned endpoints", () => {
  const stages = ["gem-pro", "gmw-pro", "goh", "gmh", "gml", "gmq-oda", "enm", "en"];
  // Some recorded borrowing steps go backwards in language period; the
  // path still follows its actual edges without inserting extra columns.
  const years = [-500, 200, 900, 1200, 1250, 1100, 1300, 2026];
  let chain;
  for (let i = stages.length - 1; i >= 0; i--) chain = {
    id: stages[i], stage: stages[i], year: years[i], compactChain: true,
    terminal: i === stages.length - 1, size: { w: 190, h: 38 },
    lanes: chain ? [{ hub: `${i}-word`, mid: 19, inline: true, blobs: [chain] }] : undefined,
  };
  const compact = chronological(chain);
  let at = compact, width = at.size.w, count = 1;
  while (at.lanes?.length) {
    at = at.lanes[0].blobs[0];
    assert.equal(at.gap, 24);
    width += at.gap + at.size.w;
    count++;
  }
  assert.equal(count, 8);
  assert.ok(width < 8 * 320);
  const placed = layout({ id: "root", r: 96, blobs: [compact] });
  const centers = stages.map((id) => { const r = placed.blobs.get(id); return r.y + r.h / 2; });
  assert.ok(centers.every((y) => y === centers[0]));
});

const range = (year) => ({ earliest: year, latest: year });
test("period midpoints order Latin, Late Latin and Old French; missing stages are interpolated", () => {
  const dates = [
    { code: "la", spoken: { start: range(-650), end: null } },
    { code: "la-lat", spoken: { start: range(200), end: range(600) } },
    { code: "fro", spoken: { start: range(800), end: range(1300) } },
    { code: "ine-pro", spoken: { start: null, end: range(-6000) }, estimate: { value: -6120 } },
  ];
  const langs = { parents: { la: "ine-pro", "la-lat": "la", fro: "la-lat", frm: "fro", en: "frm" }, names: {} };
  const times = languageTimes(dates, langs, "en", 2026);
  assert.equal(times.get("la-lat"), 400);
  assert.equal(times.get("fro"), 1050);
  assert.equal(times.get("ine-pro"), -6120);
  assert.equal(times.get("en"), 2026);
  assert.ok(times.get("frm") > times.get("fro") && times.get("frm") < 2026);
  assert.deepEqual(dates[0].spoken.end, null);
});

test("inferred dates do not depend on language table iteration order", () => {
  const dates = [{ code: "root", spoken: { start: range(0), end: range(0) } }];
  const parents = { a: "root", b: "a", en: "b" };
  const one = languageTimes(dates, { parents, names: {} }, "en", 2000);
  const two = languageTimes(dates, { parents: Object.fromEntries(Object.entries(parents).reverse()), names: {} }, "en", 2000);
  assert.equal(one.get("a"), two.get("a"));
  assert.equal(one.get("b"), two.get("b"));
  assert.ok(one.get("a") < one.get("b"));
});

test("a historical reader language keeps its period instead of becoming present-day", () => {
  const times = languageTimes([
    { code: "enm", spoken: { start: range(1150), end: range(1500) } },
  ], { parents: {}, names: {} }, "enm", 2026);
  assert.equal(times.get("enm"), 1325);
});

test("overlapping periods and borrowing cycles keep each child beyond its own parent", () => {
  const branch = chronological({
    id: "latin", stage: "la", year: 500, size: { w: 320, h: 100 },
    lanes: [{ hub: "a", mid: 50, blobs: [{
      id: "greek", stage: "grc", year: 200, size: { w: 320, h: 100 },
      lanes: [{ hub: "b", mid: 50, blobs: [{
        id: "latin-again", stage: "la", year: 500, size: { w: 320, h: 100 },
      }] }],
    }] }],
  });
  const greek = branch.lanes[0].blobs[0];
  assert.ok(greek.gap >= 44);
  assert.ok(greek.lanes[0].blobs[0].gap >= 44);
});

test("sibling Late Latin sits beyond Latin and before Old French, with English endpoints aligned", () => {
  const english = (id) => ({ id, stage: "en", year: 2026, terminal: true, size: { w: 320, h: 40 } });
  const branch = chronological({
    id: "branch", stage: "root", year: -6000, size: { w: 320, h: 180 },
    lanes: [
      { hub: "latin-head", mid: 40, blobs: [{
        id: "latin", stage: "la", year: -650, size: { w: 320, h: 100 },
        lanes: [{ hub: "french-head", mid: 50, blobs: [{
          id: "french", stage: "fro", year: 1050, size: { w: 320, h: 100 },
          lanes: [{ hub: "french-word", mid: 50, inline: true, blobs: [english("fish")] }],
        }] }],
      }] },
      { hub: "late-head", mid: 130, blobs: [{
        id: "late", stage: "la-lat", year: 400, size: { w: 320, h: 40 },
        lanes: [{ hub: "late-word", mid: 20, inline: true, blobs: [english("piscine")] }],
      }] },
    ],
  });
  const root = { id: "root", r: 96, blobs: [branch] };
  const initial = layout(root);
  const seat = initial.seats.get("branch");
  for (const side of [-1, 1]) {
    const before = { ...initial, seats: new Map([["branch", { ...seat, x: side === 1 ? 200 : -200 - seat.w }]]) };
    const placed = layout(root, undefined, before);
    const pos = (id) => placed.blobs.get(id);
    const start = (id) => side === 1 ? pos(id).x : -pos(id).x - pos(id).w;
    assert.ok(start("latin") < start("late"));
    assert.ok(start("late") < start("french"));
    assert.equal(start("fish"), start("piscine"));
    assert.ok(start("fish") > start("french"));
  }
});

test("parallel language branches share horizontal space instead of taking full columns", () => {
  const child = (stage, year) => ({
    id: stage, stage, year, size: { w: 320, h: 100 },
    lanes: [{ hub: `${stage}-word`, mid: 50, inline: true, blobs: [{
      id: `${stage}-english`, stage: "en", year: 2026, terminal: true, size: { w: 320, h: 40 },
    }] }],
  });
  const branch = chronological({
    id: "latin", stage: "la", year: -650, size: { w: 320, h: 100 },
    lanes: [{ hub: "grandis", mid: 50, blobs: [child("fro", 1050), child("osp", 1300), child("it", 1800)] }],
  });
  const children = branch.lanes[0].blobs;
  const positions = children.map((c) => 320 + c.gap);
  assert.ok(positions[0] < positions[1] && positions[1] < positions[2]);
  assert.ok(positions[2] - positions[0] < 320);
  const width = Math.max(...children.map((c, i) => positions[i] + 320 + c.lanes[0].blobs[0].gap + 320));
  assert.ok(width <= 1300, `parallel branches took ${width}px`);
});

test("folded Latin boxes end before the aligned English column", () => {
  const branch = chronological({
    id: "proto", stage: "proto", year: -1000, size: { w: 320, h: 200 },
    lanes: [
      { hub: "latin-a", mid: 40, blobs: [{ id: "latin-a", stage: "la", year: -650, size: { w: 320, h: 150 } }] },
      { hub: "english", mid: 100, inline: true, blobs: [{ id: "english", stage: "en", year: 2026, terminal: true, size: { w: 320, h: 40 } }] },
      { hub: "latin-b", mid: 160, blobs: [{ id: "latin-b", stage: "la", year: -650, size: { w: 400, h: 150 } }] },
    ],
  });
  const initial = layout({ id: "root", r: 96, blobs: [branch] });
  const seat = initial.seats.get("proto");
  for (const side of [-1, 1]) {
    const before = { ...initial, seats: new Map([["proto", { ...seat, x: side === 1 ? 200 : -200 - seat.w }]]) };
    const placed = layout({ id: "root", r: 96, blobs: [branch] }, undefined, before);
    const english = placed.blobs.get("english");
    for (const id of ["latin-a", "latin-b"]) {
      const latin = placed.blobs.get(id);
      const gap = side === 1 ? english.x - latin.x - latin.w : latin.x - english.x - english.w;
      assert.ok(gap >= 44, `${id} intrudes into English by ${44 - gap}px`);
    }
  }
});
