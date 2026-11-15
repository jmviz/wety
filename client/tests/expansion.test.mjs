import assert from "node:assert/strict";
import test from "node:test";
import { newestFirst, unbranchedLineage } from "../src/constellation/expansion.ts";

test("the newest click's sources sit together ahead of older branches", () => {
  const sources = [
    { id: "old-first", revealed: 1 }, { id: "old-second", revealed: 1 },
    { id: "new-first", revealed: 3 }, { id: "new-second", revealed: 3 },
    { id: "middle", revealed: 2 },
  ];
  assert.deepEqual(newestFirst(sources).map((s) => s.id), ["new-first", "new-second", "middle", "old-first", "old-second"]);
  assert.equal(sources[0].id, "old-first");
});

test("one historical lineage auto-expands to a list of modern words", () => {
  const words = ["kindergarten", "kinder", "kindergärtner", "Kindergärtner"];
  const source = (term) => ({ item: { term } });
  const heads = ["kintha", "kind", "kind", "kint", "Kind"].map(source);
  const step = (head) => ({ words, groups: [{ head, own: false }] });
  const endpoint = { words, groups: [{ own: true }] };
  const next = new Map(heads.map((head, i) => [head, [i + 1 < heads.length ? step(heads[i + 1]) : endpoint]]));
  assert.equal(unbranchedLineage(step(heads[0]), (head) => next.get(head)), true);
  // Two separate historical sources are a real fork, even within one language.
  next.set(heads[2], [{ words, groups: [{ head: heads[3], own: false }, { head: source("other"), own: false }] }]);
  assert.equal(unbranchedLineage(step(heads[0]), (head) => next.get(head)), false);
  // A second language lane is also a fork.
  next.set(heads[2], [step(heads[3]), endpoint]);
  assert.equal(unbranchedLineage(step(heads[0]), (head) => next.get(head)), false);
  next.set(heads[2], [step(heads[0])]);
  assert.equal(unbranchedLineage(step(heads[0]), (head) => next.get(head)), false);
  assert.equal(unbranchedLineage({ words: [], groups: [] }, () => []), false);
});

test("a fork stays folded even when every historical branch has only one endpoint", () => {
  const words = ["first", "second"];
  const root = { item: { term: "root" } };
  const heads = words.map((term) => ({ item: { term } }));
  const children = (head) => head === root
    ? [{ words, groups: heads.map((head, i) => ({ head, own: false, words: [words[i]] })) }]
    : [{ words: [head.item.term], groups: [{ own: true }] }];
  const branch = { words, groups: [{ head: root, own: false }] };
  assert.equal(unbranchedLineage(branch, children), false);
  // Selecting one of the resulting rows still opens its uninterrupted path.
  for (const head of heads) {
    assert.equal(unbranchedLineage({ words: [head.item.term], groups: [{ head, own: false }] }, children), true);
  }
});

test("one modern spelling reached through two sources is still a historical fork", () => {
  const head = { item: { term: "source" } };
  const branch = { words: ["same"], groups: [{ head, own: false }] };
  const children = () => [{ words: ["same"], groups: ["one", "two"].map((term) => ({ head: { item: { term } }, own: false })) }];
  assert.equal(unbranchedLineage(branch, children), false);
});
