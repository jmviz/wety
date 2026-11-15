import assert from "node:assert/strict";
import test from "node:test";
import { languageAnchors } from "../src/constellation/labels.ts";

const anchor = (y, cluster = "one", branching = false, x = 0) => ({
  box: { x, y, w: 320, h: 38 }, cluster, branching,
});

test("nearby same-language rows share a label but tall columns repeat it", () => {
  const anchors = [0, 64, 128, 192, 256, 320, 384].map((y) => anchor(y));
  assert.deepEqual(languageAnchors(anchors).map((a) => a.box.y), [0, 320]);
});

test("independent subtrees and major forks get local language context", () => {
  const anchors = [anchor(0), anchor(64), anchor(128, "two"), anchor(192, "two"), anchor(256, "two", true)];
  assert.deepEqual(languageAnchors(anchors).map((a) => a.box.y), [0, 128, 256]);
});

test("different horizontal stages each have a label regardless of input order", () => {
  const anchors = [anchor(50, "one", false, 420), anchor(100), anchor(0)];
  assert.deepEqual(languageAnchors(anchors).map((a) => [a.box.x, a.box.y]), [[0, 0], [420, 50]]);
  assert.equal(anchors[0].box.x, 420);
});
