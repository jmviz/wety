import assert from "node:assert/strict";
import test from "node:test";
import { chronological, languageTimes } from "../src/constellation/chronology.ts";
import { layout } from "../src/constellation/layout.ts";

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
  assert.ok(greek.gap >= 112);
  assert.ok(greek.lanes[0].blobs[0].gap >= 112);
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
