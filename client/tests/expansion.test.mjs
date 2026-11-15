import assert from "node:assert/strict";
import test from "node:test";
import { expansionDepth, sharedDepth } from "../src/constellation/expansion.ts";

const branch = (key, own = false) => ({ key, groups: [{ own }] });
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
