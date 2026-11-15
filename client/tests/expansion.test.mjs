import assert from "node:assert/strict";
import test from "node:test";
import { canFinishTogether, linearGroups, newestFirst, unbranchedLineage } from "../src/constellation/expansion.ts";

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

test("a fork is distinct from a continuous lineage even when each arm has one endpoint", () => {
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

test("small tray subtrees finish across one split, regardless of path length", () => {
  const word = (term) => ({item:{term}});
  const one = word("one"), two = word("two"), earlier = word("earlier");
  const endpoint = (...terms) => ({words:terms.map(word),groups:[{own:true}]});
  const branch = (...heads) => ({words:[word("first"),word("second")],groups:heads.map(head=>({head,own:false}))});
  const next=new Map([[one,[endpoint("first")]], [two,[endpoint("second")]], [earlier,[branch(two)]]]);
  const read=head=>next.get(head)??[];
  assert.equal(canFinishTogether([branch(one,two)],read),true);
  assert.equal(canFinishTogether([branch(one,earlier)],read),true);
  assert.equal(canFinishTogether([branch(earlier)],read),true);
  next.set(two,[branch(word("missing"))]);
  assert.equal(canFinishTogether([branch(one,two)],read),false);
  next.set(two,[branch(one,one)]);
  assert.equal(canFinishTogether([branch(one,two)],read),false);
  next.set(two,[branch(earlier)]);
  assert.equal(canFinishTogether([branch(earlier)],read),false);
  assert.equal(canFinishTogether([branch(word(""))],read),false);
  assert.equal(canFinishTogether([],read),false);
  assert.equal(canFinishTogether([{words:[],groups:[]}],read),false);
  assert.equal(canFinishTogether([endpoint("a","b","c","d","e")],read),false);
});

test("one modern spelling reached through two sources is still a historical fork", () => {
  const head = { item: { term: "source" } };
  const branch = { words: ["same"], groups: [{ head, own: false }] };
  const children = () => [{ words: ["same"], groups: ["one", "two"].map((term) => ({ head: { item: { term } }, own: false })) }];
  assert.equal(unbranchedLineage(branch, children), false);
});


test("a deeper fork does not block linear siblings, even within one language", () => {
  const head = (term) => ({ item: { term } });
  const words = ["first", "second", "third", "fourth", "fifth"].map(head);
  const group = (head, words) => ({ head, words, own: false });
  const branch = (groups) => ({ groups, words: groups.flatMap((g) => g.words) });
  const endpoint = (word) => ({ groups: [{own:true,words:[word]}], words:[word] });
  const short = head("short"), long = head("long"), middle = head("middle"), fork = head("fork"), nested = head("nested");
  const nestedGroups = [group(head("choice-a"), [words[2]]), group(head("choice-b"), [words[3]])];
  const next = new Map([
    [short, [endpoint(words[0])]],
    [long, [branch([group(middle, [words[1]])])]],
    [middle, [endpoint(words[1])]],
    [fork, [branch([group(nested, words.slice(2, 4)), group(head("other"), [words[4]])])]],
    [nested, [branch(nestedGroups)]],
  ]);
  const read = (head) => next.get(head) ?? [];
  const choices = [group(short, [words[0]]), group(fork, words.slice(2)), group(long, [words[1]])];
  assert.equal(canFinishTogether([branch(choices)], read), false);
  // One column and separate language columns produce the same decisions.
  for (const front of [[branch(choices)], choices.map((g) => branch([g]))]) {
    assert.deepEqual(front.flatMap((b) => linearGroups(b, read)).map((g) => g.head), [short, long]);
  }
  assert.deepEqual(linearGroups(endpoint(words[0]), read), []);
  assert.deepEqual(linearGroups(branch([group(head("unknown"), words)]), read), []);
});
