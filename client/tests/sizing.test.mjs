import test from 'node:test';
import assert from 'node:assert/strict';
import { languageWidths } from '../src/constellation/sizing.ts';

test('same-language trays and roots share widths based on content, with compact source columns', () => {
  const content = [
    { language: 'one', source: 140, words: 210, root: 0 },
    { language: 'one', source: 80, words: 280, root: 0 },
    { language: 'one', root: 250 },
    { language: 'two', source: 80, words: 100, root: 0 },
  ];
  const sizes = languageWidths(content, 900);
  assert.deepEqual(sizes.get('one'), {width: 500, source: 140});
  assert.deepEqual(sizes.get('two'), {width: 260, source: 80});
  assert.ok(sizes.get('one').source < sizes.get('one').width / 3);
  assert.ok(languageWidths(content.slice(1), 900).get('one').width < sizes.get('one').width);
});

test('long text is capped and limited viewport space is reserved for descendants', () => {
  const content = [{language:'one',source:900,words:900,root:900}];
  assert.deepEqual(languageWidths(content, 900).get('one'), {width:520,source:220});
  assert.deepEqual(languageWidths(content, 300).get('one'), {width:300,source:120});
});
