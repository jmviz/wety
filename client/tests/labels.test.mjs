import assert from "node:assert/strict";
import test from "node:test";
import { languageAnchors } from "../src/constellation/labels.ts";

const anchor = (y, x = 0, h = 38) => ({ box: { x, y, w: 320, h } });
const viewport = { x: -50, y: -50, w: 1000, h: 800 };

test("only the highest visible card carries a language heading", () => {
  const entries = [anchor(320), anchor(0), anchor(128), anchor(600, 420)];
  assert.deepEqual(languageAnchors(entries, viewport), [anchor(0)]);
  assert.equal(entries[0].box.y, 320);
});

test("panning transfers a language heading to the next visible card", () => {
  const entries = [anchor(0), anchor(160), anchor(320)];
  assert.deepEqual(languageAnchors(entries, { ...viewport, y: 100 })[0].box, anchor(160).box);
  assert.deepEqual(languageAnchors(entries, { ...viewport, x: 400 }), []);
});

test("a partially visible cluster keeps its language heading inside the viewport", () => {
  const label = languageAnchors([anchor(-100, -80, 400)], viewport)[0];
  assert.equal(label.box.y - 24, viewport.y + 8);
  assert.equal(label.box.x, viewport.x + 8);
});

test("a visible descendant cluster above a root carries their shared label", () => {
  assert.deepEqual(languageAnchors([anchor(400), anchor(40, 0, 250)], viewport), [anchor(40, 0, 250)]);
});
