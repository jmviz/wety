import assert from "node:assert/strict";
import test from "node:test";
import { autoExpandGroup, branchesOf, canOpen, directEarlyEnglish } from "../src/constellation/branches.ts";

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

test("direct early-to-English jumps and English derivatives disappear from lists and expanded nodes", () => {
  const direct = node(english, "direct", [word("directness")]);
  const selected = word("selected", true);
  const viaFrench = word("viafrench");
  const late = { id: 10, code: "la-lat", name: "Late Latin" };
  const viaLate = word("vialate");
  const head = node(latin, "root", [direct, selected, node(french, "source", [viaFrench]), node(late, "late", [viaLate])]);
  const root = node(pie, "root", [head]);
  const omitted = directEarlyEnglish(root, (code) => ["ine-pro", "la", "grc"].includes(code));
  assert.ok(omitted.has(direct.key));
  assert.ok(omitted.has(direct.children[0].key));
  assert.ok(!omitted.has(viaFrench.key));
  assert.ok(!omitted.has(viaLate.key));
  assert.ok(!omitted.has(selected.key));
  const [folded] = branchesOf(root, english, langs, null, omitted);
  assert.deepEqual(new Set(folded.words.map((w) => w.item.term)), new Set(["selected", "viafrench", "vialate"]));
  const expanded = branchesOf(head, english, langs, null, omitted);
  assert.deepEqual(new Set(expanded.flatMap((b) => b.words.map((w) => w.item.term))), new Set(["selected", "viafrench", "vialate"]));
});

test("an English spelling remains visible through a separate later-language path", () => {
  const direct = word("shared");
  const later = word("shared");
  const head = node(latin, "root", [direct, node(french, "source", [later])]);
  const omitted = directEarlyEnglish(head, (code) => code === "la");
  const branches = branchesOf(head, english, langs, null, omitted);
  assert.equal(branches.length, 1);
  assert.equal(branches[0].name, french.name);
  assert.deepEqual(branches[0].words, [later]);
});

test("one-to-one child nodes auto-expand independently of a multi-word parent", () => {
  const oldEnglish = { id: 11, code: "ang", name: "Old English" };
  const middleEnglish = { id: 12, code: "enm", name: "Middle English" };
  for (const term of ["stride", "unclose", "quiver", "brim"]) {
    const child = node(middleEnglish, term, [word(term)]);
    const [branch] = branchesOf(node(oldEnglish, term, [child]), english, langs, null);
    assert.equal(autoExpandGroup(branch, english)?.head, child);
  }
  const oneSource = node(middleEnglish, "source", [word("one"), word("two")]);
  const [manyWords] = branchesOf(node(oldEnglish, "root", [oneSource]), english, langs, null);
  assert.equal(autoExpandGroup(manyWords, english), null);
  const [manySources] = branchesOf(node(oldEnglish, "root", [
    node(middleEnglish, "source-a", [word("same")]),
    node(middleEnglish, "source-b", [word("same")]),
  ]), english, langs, null);
  assert.equal(autoExpandGroup(manySources, english), null);
  const [unnamed] = branchesOf(node(oldEnglish, "root", [node(middleEnglish, "", [word("only")])]), english, langs, null);
  assert.equal(autoExpandGroup(unnamed, english), null);
});

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

test("a common Old Latin ancestor never groups Latin, Late Latin and Medieval Latin heads", () => {
  const old = { id: 8, code: "itc-ola", name: "Old Latin" };
  const late = { id: 9, code: "la-lat", name: "Late Latin" };
  const medieval = { id: 10, code: "la-med", name: "Medieval Latin" };
  const tree = {
    parents: { ...langs.parents, "itc-ola": "itc-pro", la: "itc-ola", "la-lat": "la", "la-med": "la-lat" },
    names: { ...langs.names, "itc-ola": old.name, "la-lat": late.name, "la-med": medieval.name },
  };
  const heads = [
    node(medieval, "crisma", [word("chrism")]),
    node(latin, "exhortātīvus", [word("exhortative")]),
    node(late, "exhortātōrius", [word("exhortatory")]),
    node(latin, "hortativus", [word("hortative")]),
    node(old, "old form", [word("old")]),
  ];
  const branches = branchesOf(node(italic, "root", heads), english, tree, null);
  assert.deepEqual(new Set(branches.map((b) => b.name)), new Set([old.name, latin.name, late.name, medieval.name]));
  for (const b of branches) for (const g of b.groups) {
    assert.equal(g.head.item.lang.code, b.key);
    assert.equal(g.head.item.lang.name, b.name);
  }
  assert.equal(branches.find((b) => b.key === "la").groups.length, 2);
  assert.deepEqual(branches.find((b) => b.key === "itc-ola").groups.map((g) => g.head), [heads[4]]);
});

test("an Undefined Root also splits its children by exact language", () => {
  const old = { id: 8, code: "itc-ola", name: "Old Latin" };
  const medieval = { id: 10, code: "la-med", name: "Medieval Latin" };
  const tree = {
    parents: { ...langs.parents, "itc-ola": "itc-pro", la: "itc-ola", "la-med": "la" },
    names: { ...langs.names, "itc-ola": old.name, "la-med": medieval.name },
  };
  const hub = node(pie, "root", [
    node(latin, "hortativus", [word("hortative")]),
    node(medieval, "crisma", [word("chrism")]),
  ]);
  const [family] = branchesOf(hub, english, tree, null);
  assert.equal(family.name, italic.name);
  assert.equal(family.groups.length, 1);
  const branches = branchesOf(family.groups[0].head, english, tree, null);
  assert.deepEqual(new Set(branches.map((b) => b.name)), new Set([latin.name, medieval.name]));
  assert.ok(branches.every((b) => b.groups.every((g) => g.head.item.lang.code === b.key)));
});
