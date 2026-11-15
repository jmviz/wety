import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { frequency, representatives } from "../src/constellation/representatives.ts";

let next = 0;
const node = (term, parent = null, mode = "inherited", lang = "en", selected = false) => {
  const n = { key: next, item: { id: next++, term, lang: { code: lang }, reconstructed: lang.endsWith("-pro") }, parent, etyMode: mode, isSelected: selected };
  return n;
};

test("bundled corpus scores distinguish words within a SCOWL tier", () => {
  const asset = JSON.parse(readFileSync(new URL("../data/english-frequencies.json", import.meta.url), "utf8"));
  const scores = new Map(Object.entries(asset.bins).flatMap(([n, terms]) => terms.split("\n").map((term) => [term, +n / 100])));
  assert.ok(frequency(scores, "from") > frequency(scores, "fear"));
  assert.ok(frequency(scores, "fear") > frequency(scores, "frame"));
  assert.ok(frequency(scores, "frame") > frequency(scores, "fro"));
  assert.equal(asset.version, "3.1.1");
});

test("pruning follows historical derivation edges and retains common derived words", () => {
  const source = node("prover", null, null, "fro");
  const prove = node("prove", source);
  const disprove = node("disprove", node("desprover", source, "prefix", "fro"), "borrowed");
  const approve = node("approve", source, "prefix");
  const selected = node("selected", source, "suffix", "en", true);
  const unknown = node("unknownword", source, "suffix");
  const scores = new Map([["prove", 4.77], ["disprove", 2.93], ["approve", 4.08], ["selected", 1]]);
  assert.deepEqual(representatives([[prove, disprove, approve, selected, unknown]], scores), new Set(["selected", "prove", "approve", "unknownword"]));
});

test("spelling resemblance and a shared proto root do not establish a plain derivative", () => {
  const source = node("root", null, null, "ine-pro");
  const base = node("press", source);
  const unrelated = node("impressionable", source, "suffix");
  const disconnected = node("impress", null, "prefix");
  assert.equal(representatives([[base, unrelated, disconnected]], new Map([["press", 5], ["impressionable", 2], ["impress", 2]])).size, 3);
});

test("soft budgets retain selected words and especially frequent independent words", () => {
  const words = Array.from({ length: 12 }, (_, i) => node(`word${i}`, null, "inherited", "en", i === 11));
  const scores = new Map(words.map((n, i) => [n.item.term, 5.5 - i * 0.1]));
  const kept = representatives([words], scores);
  assert.ok(kept.has("word11"));
  assert.ok(kept.has("word6"));
  assert.ok(kept.size < words.length);
});
