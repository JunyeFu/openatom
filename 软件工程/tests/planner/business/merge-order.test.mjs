import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {root,source} from './closeout-source.mjs';
const {mergeDraft}=await source('draft');
const fixture=JSON.parse(await readFile(new URL('packages/commands/tests/fixtures/production/p104-A-first.json',root),'utf8'));
const observations=[];
test('explicit task row reorder survives a concurrent edit to a different field',()=>{
 const base=fixture.scenario,mine=structuredClone(base),current=structuredClone(base);
 mine.production_tasks.reverse();current.title='concurrent scenario title';current.production_tasks[0].release_at_ms=1;
 const merged=mergeDraft(base,mine,current);observations.push({kind:'reorder_and_title',merged});
 assert.deepEqual(merged.conflicts,[]);
 assert.deepEqual(merged.value.production_tasks.map(x=>x.id),mine.production_tasks.map(x=>x.id));
 assert.equal(merged.value.title,current.title);
 assert.equal(merged.value.production_tasks.find(t=>t.id==='TASK-A').release_at_ms,1);
});
test('different concurrent row orders require an explicit choice',()=>{
 const base={rows:[{id:'A',value:1},{id:'B',value:2},{id:'C',value:3}]};
 const mine={rows:[base.rows[1],base.rows[0],base.rows[2]]},current={rows:[base.rows[0],base.rows[2],base.rows[1]]};
 const merged=mergeDraft(base,mine,current);observations.push({kind:'different_reorders',merged});
 assert.equal(merged.conflicts.length,1);assert.equal(merged.conflicts[0].path,'/rows');
 assert.deepEqual(mergeDraft(base,mine,current,{'/rows':'mine'}).value,mine);
 assert.deepEqual(mergeDraft(base,mine,current,{'/rows':'current'}).value,current);
});
test('record edits, removals and concurrent new IDs still merge by identity',()=>{
 const base={rows:[{id:'A',value:1},{id:'B',value:2}]},mine={rows:[{id:'B',value:4}]},current={rows:[...base.rows,{id:'C',value:3}]};
 const merged=mergeDraft(base,mine,current);assert.deepEqual(merged.conflicts,[]);
 assert.deepEqual(merged.value,{rows:[{id:'B',value:4},{id:'C',value:3}]});
});
test.after(async()=>writeFile(new URL('.work/business-input-closeout/delivery/merge-order-observations.json',root),JSON.stringify(observations,null,2)));
