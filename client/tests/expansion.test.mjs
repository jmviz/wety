import assert from "node:assert/strict";
import test from "node:test";
import { expansionDepth, sharedDepth, singleDescendantChildren, unbranchedLineage } from "../src/constellation/expansion.ts";

const branch = (key, own = false) => ({ key, groups: [{ own }] });

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

test("a multi-descendant root opens all children when each has one modern destination", () => {
  const child = (key, ...words) => ({ ...branch(key, key === "en"), words });
  const porter = [{ branches: [child("enm:disporten", "sport"), child("enm:reporten", "report"), child("en", "port")] }];
  assert.equal(singleDescendantChildren(porter), true);
  // One child with two destinations vetoes the automatic expansion for all.
  porter[0].branches[0].words.push("disport");
  assert.equal(singleDescendantChildren(porter), false);
  assert.equal(singleDescendantChildren([]), false);
  assert.equal(singleDescendantChildren([{ branches: [] }]), false);
});

test("a complete sibling records the English horizon as well as its stage depth", () => {
  const hubs = [
    { id: "greek", via: "parent/grc", branches: [branch("la")] },
    { id: "latin", via: "greek/la", branches: [branch("fr")] },
    { id: "french", via: "latin/fr", branches: [branch("en", true)] },
  ];
  assert.deepEqual(expansionDepth("parent/grc", hubs), { depth: 3, complete: true });
  assert.deepEqual(expansionDepth("other/grc", hubs), { depth: 0, complete: false });
});

test("a partial sibling supplies a finite depth rather than opening every later stage", () => {
  const hubs = [{ id: "greek", via: "parent/grc", branches: [branch("la")] }];
  assert.deepEqual(expansionDepth("parent/grc", hubs), { depth: 1, complete: false });
  assert.deepEqual(sharedDepth([expansionDepth("parent/grc", hubs), { depth: 0, complete: false }]), { depth: 1, complete: false });
  assert.equal(sharedDepth([{ depth: 0, complete: false }]), null);
});

test("a source reaches the English horizon only when all of its opened paths finish", () => {
  const hubs = [
    { id: "greek", via: "parent/grc", branches: [branch("la:first"), branch("la:second")] },
    { id: "latin", via: "greek/la:first", branches: [branch("en", true)] },
  ];
  assert.deepEqual(expansionDepth("parent/grc", hubs), { depth: 2, complete: false });
  assert.deepEqual(sharedDepth([{ depth: 2, complete: false }, { depth: 3, complete: true }]), { depth: 3, complete: true });
});

test("a shared descendant preview stays incomplete until its shared paths reach English", () => {
  const latin = branch("la");
  const hubs = [
    { id: "porta", via: "root/itc-pro", branches: [latin] },
    { id: "portus", via: "root/itc-pro", branches: [], shared: [{ host: { id: "porta" }, branch: latin }] },
  ];
  assert.deepEqual(expansionDepth("root/itc-pro", hubs), { depth: 1, complete: false });
  hubs.push({ id: "porto", via: "porta/la", branches: [branch("en", true)] });
  assert.deepEqual(expansionDepth("root/itc-pro", hubs), { depth: 2, complete: true });
});
