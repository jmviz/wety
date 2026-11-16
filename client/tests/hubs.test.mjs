import assert from "node:assert/strict";
import test from "node:test";
import { branchesOf } from "../src/constellation/branches.ts";
import { PALETTE, blobId, expand, huesOf, makeHub, shareColumns } from "../src/constellation/hubs.ts";

const pie = { id: 1, code: "ine-pro", name: "Proto-Indo-European" };
const italic = { id: 2, code: "itc-pro", name: "Proto-Italic" };
const latin = { id: 3, code: "la", name: "Latin" };
const french = { id: 4, code: "fro", name: "Old French" };
const english = { id: 5, code: "en", name: "English" };
const langs = {
  parents: { "itc-pro": "ine-pro", la: "itc-pro", fro: "la" },
  names: Object.fromEntries([pie, italic, latin, french, english].map((l) => [l.code, l.name])),
};
let nextId = 0;
function node(lang, term, children = []) {
  const id = nextId++;
  return {
    key: id,
    item: { id, lang, term, reconstructed: lang.code.endsWith("-pro"), imputed: false, pos: null },
    children,
    etyMode: null,
    isSelected: false,
  };
}
const word = (term) => node(english, term);
const branchesFor = (n) => branchesOf(n, english, langs, null);
const terms = (hubs) => hubs.map((h) => h.node.item.term);

test("the commonest families keep their colour, and the rest get distinct ones", () => {
  const hues = huesOf(["gem-pro", "itc-pro", "xyz", "abc"]);
  assert.equal(hues.get("gem-pro"), PALETTE[0]);
  assert.equal(hues.get("itc-pro"), PALETTE[1]);
  assert.equal(new Set(hues.values()).size, 4);
  assert.equal(huesOf(["abc"]).get("abc"), huesOf(["abc", "qrs"]).get("abc"));
});

test("a lineage with no forks opens through to the reader's language in one click", () => {
  const root = node(italic, "root", [node(latin, "porta", [node(french, "porte", [word("port")])])]);
  const rootHub = makeHub(branchesFor, root, null, null, null);
  const [branch] = rootHub.branches;
  const { all, touched } = expand([rootHub], rootHub, branch, branch.groups, undefined,
    { target: english, branchesFor, fromTray: false, revealed: 1 });
  assert.deepEqual(terms(all), ["root", "porta", "porte"]);
  assert.equal(touched.size, 2);
  assert.equal(all[1].via, blobId(rootHub, branch));
  assert.equal(all[2].parent, all[1]);
});

test("sibling sources feeding one language share a single preview column", () => {
  const a = makeHub(branchesFor, node(latin, "porto", [node(french, "porter", [word("report")])]), null, null, null);
  const b = makeHub(branchesFor, node(latin, "portus", [node(french, "port", [word("port")])]), null, null, null);
  shareColumns([a, b]);
  const [column] = a.branches;
  assert.deepEqual(column.groups.map((g) => g.head.item.term), ["porter", "port"]);
  assert.deepEqual(column.words.map((w) => w.item.term), ["report", "port"]);
  assert.deepEqual(b.branches, []);
  assert.equal(b.shared[0].host, a);
  assert.deepEqual([...column.parents.values()].map((p) => [...p]), [[a.id], [b.id]]);
});

test("a second row reaching a drawn entry widens it instead of drawing it twice", () => {
  const root = node(italic, "root", [node(latin, "porto", [node(french, "porter", [word("report"), word("sport")])])]);
  const rootHub = makeHub(branchesFor, root, null, null, null);
  const [branch] = rootHub.branches;
  const options = { target: english, branchesFor, fromTray: false, revealed: 1 };
  const first = expand([rootHub], rootHub, branch, branch.groups, { id: "row1", words: new Set(["report"]) }, options);
  assert.deepEqual(terms(first.all), ["root", "porto", "porter"]);
  const second = expand(first.all, rootHub, branch, branch.groups, { id: "row2", words: new Set(["sport"]) },
    { ...options, revealed: 2 });
  assert.deepEqual(terms(second.all), ["root", "porto", "porter"]);
  for (const hub of second.all.slice(1)) {
    assert.equal(hub.scope.id, "row1");
    assert.deepEqual([...hub.scope.words].sort(), ["report", "sport"]);
  }
});
