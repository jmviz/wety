import assert from "node:assert/strict";
import test from "node:test";
import { selectWords } from "../src/constellation/selection.ts";

const latin = { id: 1, code: "la", name: "Latin" };
const french = { id: 2, code: "fr", name: "French" };
const english = { id: 3, code: "en", name: "English" };
const langs = { parents: { fr: "la" }, names: { la: "Latin", fr: "French", en: "English" } };

let nextId = 0;
function node(lang, term, children = [], etyMode = null) {
  const id = nextId++;
  const n = {
    key: id,
    item: { id, lang, term, reconstructed: false, imputed: false, pos: null },
    children,
    etyMode,
    isSelected: false,
    parent: null,
  };
  for (const c of children) c.parent = n;
  return n;
}

// Latin natura → French naturel → English natural, and naturally formed
// from natural with a suffix.
function family() {
  const naturally = node(english, "naturally", [], "suffix");
  const root = node(latin, "natura", [node(french, "naturel", [node(english, "natural", [naturally])])]);
  return root;
}
const listed = (selection, root) =>
  selection.branchesFor(root).flatMap((b) => b.words.map((w) => w.item.term)).sort();
const common = new Map([["natural", 10], ["naturally", 10]]);
const scores = new Map([["natural", 5.0], ["naturally", 4.0]]);

test("the representatives pass leaves out a recorded derivative of a much commoner word", () => {
  const root = family();
  assert.deepEqual(listed(selectWords(root, english, langs, common, scores, new Map()), root), ["natural"]);
});

test("rare words switched on (no word list) lists the derivative again", () => {
  const root = family();
  assert.deepEqual(listed(selectWords(root, english, langs, null, scores, new Map()), root), ["natural", "naturally"]);
});

test("a language without a word list lists every word and has no frequencies", () => {
  const root = node(latin, "natura", [node(english, "natural"), node(french, "naturel", [node(french, "naturellement")])]);
  const selection = selectWords(root, french, langs, common, scores, new Map());
  assert.equal(selection.common, null);
  assert.equal(selection.scores, undefined);
  assert.deepEqual(listed(selection, root), ["naturel", "naturellement"]);
});

test("an English word straight from Latin is left out; one through French stays", () => {
  const root = node(latin, "natura", [node(english, "nature"), node(french, "naturel", [node(english, "natural")])]);
  const selection = selectWords(root, english, langs, null, scores, new Map());
  assert.deepEqual(listed(selection, root), ["natural"]);
});
