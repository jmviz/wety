import assert from "node:assert/strict";
import test from "node:test";
import { branchesOf, canOpen } from "../src/constellation/branches.ts";

const pie = { id: 1, code: "ine-pro", name: "Proto-Indo-European" };
const hellenic = { id: 2, code: "grk-pro", name: "Proto-Hellenic" };
const greek = { id: 3, code: "grc", name: "Ancient Greek" };
const english = { id: 4, code: "en", name: "English" };
const italic = { id: 5, code: "itc-pro", name: "Proto-Italic" };
const latin = { id: 6, code: "la", name: "Latin" };
const french = { id: 7, code: "fro", name: "Old French" };
const langs = {
  parents: { "grk-pro": "ine-pro", grc: "grk-pro", "itc-pro": "ine-pro", la: "itc-pro", fro: "la" },
  names: Object.fromEntries([pie, hellenic, greek, english, italic, latin, french].map((l) => [l.code, l.name])),
};
let nextId = 0;
function node(lang, term, children = [], extra = {}) {
  const id = nextId++;
  return {
    key: id,
    item: { id, lang, term, reconstructed: lang.code.endsWith("-pro"), imputed: false, pos: null },
    children,
    etyMode: null,
    isSelected: false,
    ...extra,
  };
}
const word = (term, selected = false) => node(english, term, [], { isSelected: selected });

test("Proto-Hellenic has its two forms and one undefined root for all unanchored Greek words", () => {
  const recorded = [
    node(hellenic, "ampʰí", [node(greek, "ἀμφί", [word("amphibian", true)])]),
    node(hellenic, "aná", [node(greek, "ἀνά", [word("ana")])]),
  ];
  const unanchored = [node(greek, "ἄμφω", [word("both")]), node(greek, "πύξος", [word("box")])];
  const hub = node(pie, "h₂ent-", [...recorded, ...unanchored]);
  const [branch] = branchesOf(hub, english, langs, null);
  assert.equal(branch.name, "Proto-Hellenic");
  assert.equal(branch.groups.length, 3);
  assert.deepEqual(branch.groups.filter((g) => g.head.item.term).map((g) => g.head), recorded);
  const missing = branch.groups.find((g) => !g.head.item.term);
  assert.equal(missing.head.item.lang.code, hellenic.code);
  assert.deepEqual(missing.head.children, unanchored);
  assert.equal(canOpen(missing, english), true);
  const [next] = branchesOf(missing.head, english, langs, null);
  assert.equal(next.name, "Ancient Greek");
  assert.equal(next.groups.length, 2);
  assert.deepEqual(new Set(next.words.map((w) => w.item.term)), new Set(["both", "box"]));
  assert.equal(branch.wordCount, 4);
  assert.equal(branch.hasSelected, true);
  assert.deepEqual(hub.children, [...recorded, ...unanchored]);
});

test("a branch with no recorded proto forms still opens into one Ancient Greek node", () => {
  const hub = node(pie, "root", [node(greek, "ἄμφω", [word("both", true)])]);
  const [branch] = branchesOf(hub, english, langs, null);
  assert.equal(branch.groups.length, 1);
  const missing = branch.groups[0];
  assert.equal(missing.head.item.term, "");
  assert.equal(missing.head.item.lang.code, hellenic.code);
  assert.equal(missing.hasSelected, true);
  const [next] = branchesOf(missing.head, english, langs, null);
  assert.equal(next.name, "Ancient Greek");
  assert.equal(next.words[0].item.term, "both");
});

test("missing branch roots and missing later forms have distinct IDs and keep their languages", () => {
  const tagged = node(french, "intellectuel", [word("intellectual")], { etyMode: "root" });
  const hub = node(pie, "leǵ-", [tagged]);
  const [branch] = branchesOf(hub, english, langs, null);
  const root = branch.groups[0].head;
  assert.equal(root.item.lang.code, italic.code);
  const [next] = branchesOf(root, english, langs, null);
  assert.equal(next.name, "Latin");
  const missingLatin = next.groups[0].head;
  assert.equal(missingLatin.item.term, "");
  assert.notEqual(root.item.id, missingLatin.item.id);
  assert.notEqual(root.key, missingLatin.key);
  const [last] = branchesOf(missingLatin, english, langs, null);
  assert.equal(last.name, "Old French");
  assert.equal(last.groups[0].head, tagged);
  assert.equal(last.words[0].item.term, "intellectual");
});

test("non-family hubs still group their recorded heads directly", () => {
  const head = node(greek, "ἀμφί", [word("amphibian")]);
  const [branch] = branchesOf(node(hellenic, "ampʰí", [head]), english, langs, null);
  assert.equal(branch.name, "Ancient Greek");
  assert.equal(branch.groups[0].head, head);
});
