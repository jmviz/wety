import assert from 'node:assert/strict';
import test from 'node:test';
import { dominantLanguage, gatherLanguage } from '../src/constellation/magnetism.ts';
import { layout } from '../src/constellation/layout.ts';

function fixture(target = 'language-a') {
  const end = (id) => ({id,stage:'final',terminal:true,size:{w:80,h:38}});
  const card = (id,stage,children) => ({id,stage,clustered:true,size:{w:130,h:38},lanes:[{hub:id+'-word',blobs:children}]});
  const direct = (id,stage) => card(id,stage,[end(id+'-end')]);
  const northern = card('north','north-language',[direct('first',target),direct('other','language-b')]);
  const borrowed = card('borrowed','language-c',[
    direct('install','language-d'),card('italian','language-e',[direct('pedestal','language-d')]),
    card('older','language-f',[direct('meaning-one',target),direct('meaning-two',target)]),
  ]);
  const southern = card('south','south-language',[borrowed,card('old','language-g',[direct('last',target)])]);
  const family = {id:'family',stage:'proto',clustered:true,size:{w:130,h:38},lanes:[
    {hub:'north-root',revealed:1,blobs:[northern]},
    {hub:'south-root',revealed:2,blobs:[southern]},
    {hub:'tray',preview:true,height:80,blobs:[]},
  ]};
  return {id:'root',r:96,blobs:[family]};
}
const viewport = {x:-10000,y:-10000,w:20000,h:20000};
const endings = (at) => [...at.blobs].filter(([id])=>id.endsWith('-end')).sort(([,a],[,b])=>a.y-b.y).map(([id])=>id);

test('the most numerous scattered visible language gathers across connected subtrees', () => {
  for (const language of ['language-a','renamed-without-any-english']) {
    const root = fixture(language), base = layout(root);
    assert.equal(dominantLanguage(root,base,viewport),language);
    const grouped = gatherLanguage(root,language), after = layout(grouped,undefined,base);
    const order=endings(after);
    const matching=['first-end','meaning-one-end','meaning-two-end','last-end'].map(id=>order.indexOf(id)).sort((a,b)=>a-b);
    assert.equal(matching.at(-1)-matching[0],3,order.join(', '));
    // Stable reveal batches and the inward tray are not reordered.
    assert.deepEqual(grouped.blobs[0].lanes.map(l=>l.hub),root.blobs[0].lanes.map(l=>l.hub));
    assert.deepEqual(new Set(after.blobs.keys()),new Set(base.blobs.keys()));
    assert.deepEqual(new Set(after.hubs.keys()),new Set(base.hubs.keys()));
  }
});

test('off-screen words do not select the language to gather', () => {
  const root=fixture(), at=layout(root);
  const box=at.blobs.get('first');
  assert.equal(dominantLanguage(root,at,{x:box.x-1,y:box.y-1,w:box.w+2,h:box.h+2}),undefined);
});

test('priority goes to the language with the most scattered words, not a fixed language', () => {
  const root={id:'root',r:96,blobs:[{id:'family',stage:'proto',size:{w:120,h:38},clustered:true,
    lanes:['b','a','b','a','b'].map((stage,i)=>({hub:'root'+i,blobs:[{
      id:'card'+i,stage,size:{w:120,h:38},clustered:true,lanes:[{hub:'word'+i,blobs:[{
        id:'end'+i,stage:'final',terminal:true,size:{w:90,h:38},
      }]}],
    }]})),
  }]};
  assert.equal(dominantLanguage(root,layout(root),viewport),'b');
  const edges=(root)=>{
    const result=[];
    const visit=(b)=>{for(const lane of b.lanes??[])for(const child of lane.blobs){result.push(lane.hub+'>'+child.id);visit(child)}};
    root.blobs.forEach(visit);return result.sort();
  };
  assert.deepEqual(edges(gatherLanguage(root,'b')),edges(root));
});
