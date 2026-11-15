import assert from "node:assert/strict";
import test from "node:test";
import { compactBranches } from "../src/constellation/density.ts";

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
