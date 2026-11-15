import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { autoExpandGroup, branchesOf, canOpen, directEarlyEnglish, groupsForWords, previewLines, previewRows, restrictBranches, singleEndpoint, sourceBranches, uncommonLatinEnglish, withoutUndefinedRoots } from "../src/constellation/branches.ts";

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

test("descendant preview rows retain their own source when several roots lead to port", () => {
  const porto = node(latin, "porto", [
    node(french, "porter", [word("report"), word("sport"), word("port")]),
    node({ id: 14, code: "it", name: "Italian" }, "importare", [word("import")]),
    node({ id: 15, code: "enm", name: "Middle English" }, "important", [word("important")]),
  ]);
  const porta = node(latin, "porta", [node(french, "porte", [word("port")])]);
  const portus = node(latin, "portus", [node(french, "port", [word("port")])]);
  const root = node(italic, "root", [porto, porta, portus]);
  const read = (n) => branchesOf(n, english, langs, null);
  const rows = previewRows(read(root)[0], english, read);
  assert.deepEqual(rows.map((r) => r.groups[0].head.item.term), ["porto", "porta", "portus"]);
  assert.deepEqual(new Set(rows[0].words.map((w) => w.item.term)), new Set(["report", "sport", "port", "import", "important"]));
  assert.deepEqual(rows.slice(1).map((r) => r.words.map((w) => w.item.term)), [["port"], ["port"]]);
});

test("a preview row opens only its words, including when rows share the same proto root", () => {
  const shared = node(hellenic, "shared", [
    node(greek, "praktikos", [word("practice"), word("practise")]),
    node(french, "peri", [word("periodic")]),
  ]);
  const other = node(hellenic, "other", [node(greek, "pirate-source", [word("pirate")])]);
  const root = node(pie, "root", [shared, other]);
  const read = (n) => branchesOf(n, english, langs, null);
  const [branch] = read(root);
  const rows = previewLines(branch, english, read);
  const family = rows.find((row) => row.some((w) => w.item.term === "practice"));
  const singles = rows.find((row) => row.some((w) => w.item.term === "periodic"));
  assert.deepEqual(groupsForWords(branch, family).map((g) => g.head), [shared]);
  const clicked = groupsForWords(branch, singles);
  assert.equal(clicked.length, 2);
  assert.deepEqual(new Set(clicked.flatMap((g) => g.words.map((w) => w.item.term))), new Set(["periodic", "pirate"]));
  const scope = new Set(singles.map((w) => w.item.term));
  const children = sourceBranches(restrictBranches(read(shared), scope));
  assert.equal(children.length, 1);
  assert.equal(children[0].groups[0].head.item.term, "peri");
  assert.deepEqual(children[0].words.map((w) => w.item.term), ["periodic"]);
  assert.equal(children[0].wordCount, 1);
  assert.deepEqual(restrictBranches(read(shared), new Set(["missing"])), []);
  assert.equal(branch.wordCount, 4);
  assert.equal(branch.groups.find((g) => g.head === shared).words.length, 3);
});

test("undefined-root experiment removes their words and counts without changing the underlying groups", () => {
  const recorded = node(hellenic, "known", [node(greek, "known", [word("kept")])]);
  const hub = node(pie, "root", [recorded, node(greek, "unanchored", [word("hidden")])]);
  const original = branchesOf(hub, english, langs, null);
  assert.equal(original[0].groups.length, 2);
  const visible = withoutUndefinedRoots(original);
  assert.equal(visible[0].groups.length, 1);
  assert.equal(visible[0].groups[0].head, recorded);
  assert.deepEqual(visible[0].words.map((w) => w.item.term), ["kept"]);
  assert.equal(visible[0].wordCount, 1);
  assert.equal(original[0].groups.length, 2);
  assert.equal(original[0].wordCount, 2);
  const allUnknown = branchesOf(node(pie, "root", [node(greek, "unanchored", [word("hidden")])]), english, langs, null);
  assert.deepEqual(withoutUndefinedRoots(allUnknown), []);
});

test("revealed language roots have independent folded nodes and one-to-one actions", () => {
  const hub = node(hellenic, "root", [
    node(greek, "peri", [word("periodic")]),
    node(greek, "praktikos", [word("practice"), word("practise")]),
  ]);
  const raw = branchesOf(hub, english, langs, null);
  assert.equal(raw.length, 1);
  const split = sourceBranches(raw);
  assert.equal(split.length, 2);
  assert.equal(new Set(split.map((b) => b.key)).size, 2);
  assert.ok(split.every((b) => b.language === "grc" && b.name === "Ancient Greek" && b.groups.length === 1));
  assert.ok(autoExpandGroup(split.find((b) => b.words.length === 1), english));
  assert.equal(autoExpandGroup(split.find((b) => b.words.length === 2), english), null);
});

test("Latin-origin English uses tier 35 through later languages while other origins stop at tier 50", () => {
  const common = new Map([["common", 35], ["rarer", 40], ["limit", 50], ["overlimit", 55], ["rarest", 60], ["selected", 60], ["anti", 35]]);
  const viaFrench = node(french, "source", [word("common"), word("rarer"), word("rarest"), word("selected", true), word("anti-")]);
  const fromLatin = node(latin, "source", [viaFrench]);
  const other = node({ id: 22, code: "ang", name: "Old English" }, "source", [word("rarer"), word("limit"), word("overlimit"), word("rarest")]);
  const root = node(pie, "root", [fromLatin, other]);
  const omitted = uncommonLatinEnglish(root, common);
  assert.deepEqual([...omitted], viaFrench.children.slice(1, 3).map((n) => n.key));
  assert.equal(uncommonLatinEnglish(root, null).size, 0);
  const read = (head) => branchesOf(head, english, langs, common, omitted);
  assert.deepEqual(new Set(read(viaFrench).flatMap((b) => b.words.map((w) => w.item.term))), new Set(["common", "selected", "anti-"]));
  assert.deepEqual(new Set(read(other).flatMap((b) => b.words.map((w) => w.item.term))), new Set(["rarer", "limit"]));
  viaFrench.parent = fromLatin;
  assert.deepEqual([...uncommonLatinEnglish(viaFrench, common)], [...omitted]);
});

test("Late Latin and Old Latin qualify without treating Proto-Italic as Latin", () => {
  const common = new Map([["rarer", 55]]);
  for (const lang of [{ id: 25, code: "la-lat", name: "Late Latin" }, { id: 26, code: "itc-ola", name: "Old Latin" }]) {
    const englishWord = word("rarer");
    assert.ok(uncommonLatinEnglish(node(lang, "source", [englishWord]), common).has(englishWord.key));
  }
  assert.equal(uncommonLatinEnglish(node(italic, "source", [word("rarer")]), common).size, 0);
});

test("Latin-origin affixes must meet the cutoff without their hyphen", () => {
  const common = new Map([["anti", 35], ["apres", 60]]);
  const head = node(latin, "source", [word("après-"), word("apres-"), word("anti-"), word("après-", true)]);
  const omitted = uncommonLatinEnglish(head, common);
  assert.deepEqual([...omitted], head.children.slice(0, 2).map((n) => n.key));
  assert.equal(uncommonLatinEnglish(head, null).size, 0);
  const shown = branchesOf(head, english, langs, common, omitted).flatMap((b) => b.words.map((w) => w.item.term));
  assert.deepEqual(new Set(shown), new Set(["anti-", "après-"]));
});

test("collapsed subcognate lines follow expanded lanes and keep nested families together", () => {
  const hub = node(italic, "root");
  const latinHead = node(latin, "premo", [
    node(french, "impression", [word("impressionable"), word("impressionism"), word("impressionist")]),
    node(french, "depression", [word("depressive")]),
    node(french, "expression", [word("expressionism")]),
    node({ id: 13, code: "it", name: "Italian" }, "espresso", [word("espresso")]),
  ]);
  hub.children = [latinHead];
  const read = (head) => branchesOf(head, english, langs, null);
  const [branch] = read(hub);
  const lines = previewLines(branch, english, read).map((line) => line.map((w) => w.item.term));
  assert.equal(lines.length, 2);
  assert.deepEqual(lines[0], ["impressionism", "impressionist", "impressionable", "depressive", "expressionism"]);
  assert.deepEqual(lines[1], ["espresso"]);
  assert.equal(new Set(lines.flat()).size, branch.wordCount);
});

test("lone descendants share one frequency-ordered section without splitting larger families", () => {
  const old = { id: 22, code: "ang", name: "Old English" };
  const heads = [
    node(old, "family", [word("forget"), word("forgive")]),
    ...["wayfaring", "ford", "from", "fear", "frame"].map((term) => node(old, term, [word(term)])),
    node(old, "pair", [word("firth"), word("fjord")]),
  ];
  const hub = node(italic, "root", heads);
  const read = (head) => branchesOf(head, english, langs, null);
  const [branch] = read(hub);
  const common = new Map([["from", 10], ["frame", 20], ["fear", 20], ["ford", 35], ["wayfaring", 50]]);
  const groups = previewLines(branch, english, read, common).map((line) => line.map((w) => w.item.term));
  assert.equal(groups.length, 3);
  const singles = groups.find((g) => g.includes("from"));
  assert.equal(singles.length, 5);
  assert.equal(singles[0], "from");
  assert.equal(singles.at(-1), "wayfaring");
  assert.ok(singles.indexOf("ford") > singles.indexOf("fear"));
  assert.ok(groups.some((g) => g.length === 2 && g.includes("firth") && g.includes("fjord")));
  assert.equal(new Set(groups.flat()).size, branch.wordCount);
});

test("default commonness excludes tier-70 fremd but rare mode and the searched word retain it", () => {
  const common = new Map();
  for (const size of [10, 20, 35, 40, 50, 55, 60, 70]) {
    const terms = JSON.parse(readFileSync(new URL(`../node_modules/wordlist-english/english-words-${size}.json`, import.meta.url), "utf8"));
    for (const term of terms) if (!common.has(term)) common.set(term, size);
  }
  assert.equal(common.get("fremd"), 70);
  const head = node(latin, "root", [word("fremd"), word("ford"), word("fjord"), word("ombudsman")]);
  const words = (frequency) => branchesOf(head, english, langs, frequency).flatMap((b) => b.words.map((w) => w.item.term));
  assert.deepEqual(new Set(words(common)), new Set(["ford", "fjord", "ombudsman"]));
  assert.ok(words(null).includes("fremd"));
  head.children[0].isSelected = true;
  assert.ok(words(common).includes("fremd"));
});

test("a lone English result never hides its Latin and French intermediate stages", () => {
  const periodic = word("periodic");
  const frenchHead = node(french, "périodique", [periodic]);
  const latinHead = node(latin, "periodicus", [frenchHead]);
  const greekHead = node(greek, "περιοδικός", [latinHead]);
  const omitted = directEarlyEnglish(greekHead, (code) => code === "grc" || code === "la");
  assert.equal(omitted.size, 0);
  const greekBranches = branchesOf(greekHead, english, langs, null, omitted);
  assert.equal(greekBranches[0].name, "Latin");
  assert.equal(singleEndpoint(greekBranches), false);
  const latinBranches = branchesOf(latinHead, english, langs, null, omitted);
  assert.equal(latinBranches[0].name, "Old French");
  assert.equal(singleEndpoint(latinBranches), false);
  assert.equal(singleEndpoint(branchesOf(frenchHead, english, langs, null, omitted)), true);
});

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
