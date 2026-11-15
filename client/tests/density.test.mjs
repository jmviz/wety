import assert from "node:assert/strict";
import test from "node:test";
import { compactBranches, compactVisibleLanes } from "../src/constellation/density.ts";

const card = (id, children = [], compact = true) => ({ id, width: 320, children, compact });

test("deep branching families compress short side-path cards as well as the main lineage", () => {
  const tree = card("root", [
    card("main", [card("middle", [card("endpoint", [], false)])]),
    card("short", [card("cynrede", [card("kindred")])]),
  ], false);
  assert.deepEqual(compactBranches([tree], 2000), new Set(["main", "middle", "short", "cynrede", "kindred"]));
});

test("available width compresses a wide shallow family without changing its previews", () => {
  const tree = card("root", [card("child", [card("preview", [], false)])]);
  assert.deepEqual(compactBranches([tree], 1300), new Set());
  assert.deepEqual(compactBranches([tree], 900), new Set(["root", "child"]));
  assert.deepEqual(compactBranches([card("short")], 900), new Set());
});


for (const side of [-1, 1]) test(`off-screen sibling rows release empty cable space on side ${side}`, () => {
  const rect = (x, y, w = 200) => ({ x: side > 0 ? x : -x-w, y, w, h: 38 });
  const root = { id: "pie", r: 96, blobs: [{ id: "proto", size: { w: 200, h: 38 }, clustered: true, lanes: [
    { hub: "source", blobs: [{ id: "later", size: { w: 200, h: 38 } }] },
    { hub: "sibling", blobs: [{ id: "middle", size: { w: 1000, h: 38 } }] },
  ] }] };
  const at = { hubs: new Map([["source", { x: side*200, y: 119 }], ["sibling", { x: side*200, y: 519 }]]),
    blobs: new Map([["proto", rect(0,100)], ["later", rect(1600,100)], ["middle", rect(244,500,1000)]]),
    sources: new Map([["source", rect(0,100)], ["sibling", rect(0,500)]]), sides: new Map([["proto",side]]), seats: new Map() };
  const hidden = compactVisibleLanes(at, root, { x:-2000,y:50,w:4000,h:200 });
  const near = hidden.blobs.get("later");
  assert.equal(side > 0 ? near.x : -near.x-near.w,244);
  const visible = compactVisibleLanes(at, root, { x:-2000,y:50,w:4000,h:600 });
  const far = visible.blobs.get("later");
  assert.equal(side > 0 ? far.x : -far.x-far.w,1288);
  assert.equal(at.blobs.get("later").x,rect(1600,100).x);
});
