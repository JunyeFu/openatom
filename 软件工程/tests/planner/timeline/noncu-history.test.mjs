import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';
import {source} from './load-source.mjs';
await source('time');const p=await source('projection');
const record=name=>JSON.parse(readFileSync(new URL(`../../../.work/delivery/host/${name}.actual.json`,import.meta.url)));
const a=record('baseline'),b=record('delayed-pcb'),output=record('output-wait');
const expected=JSON.parse(readFileSync(new URL('../../../packages/commands/tests/fixtures/whole_ix/original-expectations.json',import.meta.url))).ix01;
for(const [value,name] of [[a,'baseline'],[b,'delayed_pcb']]){
 test(name+' all independent buffer quantities and contents/holder relations survive all historical boundaries',()=>{
  const e=expected.find(e=>e.scenario===name);
  for(const point of e.buffer_events){
   const f=value.observations.find(f=>f.at_ms===point.at_ms);assert.ok(f,'independent buffer boundary '+point.at_ms);
   const shells=f.objects.map(o=>o.data).filter(i=>i.item_type_ref.id==='shell'&&i.holder.kind==='location'&&i.holder.id==='shell-buffer'&&!['transformed','scrapped'].includes(i.lifecycle));assert.equal(shells.length,point.quantity);
  }
  for(const f of value.observations){
   const items=p.observedItems(f,value.run,f.at_ms),byId=new Map(items.map(i=>[i.id,i]));
   for(const i of items){for(const child of i.contents){assert.ok(byId.has(child));assert.deepEqual(byId.get(child).holder,{kind:'container',id:i.id});}}
  }
 });
 test(name+' actual resource/phase/wait projections retain causes, exact identities and source snapshots',()=>{
  const original=JSON.stringify(value),primitives=p.actualProjection(value.project,value.run);
  for(const w of value.run.waits){const segment=primitives.find(s=>s.kind==='wait'&&s.selection.wait===w);assert.ok(segment);assert.equal(segment.phase,w.phase_ref??undefined);assert.ok(segment.selection.fields.some(f=>f.label==='结束原因'&&f.value===w.end_cause));}
  for(const c of value.run.resource_claims)for(let i=0;i<c.intervals.length;i++){const segment=primitives.find(s=>s.kind==='claim'&&s.row===`resource:${c.resource_id}`&&s.interval===c.intervals[i]);assert.ok(segment);assert.equal(segment.known_end,c.interval_end_causes[i]==='released');}
  assert.equal(JSON.stringify(value),original);
 });
}
test('output-capacity wait includes the actual pending output holder, separate from consumed input',()=>{
 const projected=p.actualProjection(output.project,output.run),selected=projected.find(s=>s.kind==='wait'&&s.interval.start_ms===45000).selection;
 const job=output.run.jobs[0],frame=output.observations.find(f=>f.at_ms===45000),items=p.observedItems(frame,output.run,45000);
 const context=p.waitContext(selected,output.run,45000,items);
 assert.equal(selected.wait.phase_ref,null);assert.ok(context.held_resources.some(c=>c.resource_id==='CAP-CNC'));
 for(const identity of job.pending_outputs){const item=context.cargo.find(i=>i.id===identity);assert.ok(item);assert.deepEqual(item.holder,{kind:'job',id:job.id});}
 assert.equal(output.run.phase_work_intervals.reduce((n,w)=>n+w.interval.end_ms-w.interval.start_ms,0),40000);
});
test('an observation-end resource cap is still held at the closed observation boundary, never treated as natural release',()=>{
 const selected=p.actualProjection(output.project,output.run).find(s=>s.kind==='wait'&&s.interval.start_ms===45000).selection;
 const frame=output.observations.find(f=>f.at_ms===100000),context=p.waitContext(selected,output.run,100000,p.observedItems(frame,output.run,100000));
 assert.ok(context.held_resources.some(c=>c.resource_id==='CAP-CNC'&&c.interval_end_causes.includes('observation_end')));
 const trip=b.run.transport.find(t=>t.arrived_ms===370000);const wait=p.actualProjection(b.project,b.run).find(s=>s.kind==='wait'&&s.object.id===trip.id).selection;
 const released=b.observations.find(f=>f.at_ms===425000);assert.equal(p.waitContext(wait,b.run,425000,p.observedItems(released,b.run,425000)).held_resources.length,0);
});
test('device detail is a same-Run projection with separate exact phase, wait and resource-reference rows',()=>{
 const c=record('phase-calendar'),before=JSON.stringify(c),primitives=p.actualProjection(c.project,c.run),device=c.run.jobs[0].execution_ref.data.device_id;
 const detail=p.deviceDetailProjection(c.project,c.run,primitives,device,c.run.window);
 assert.equal(detail.rows.filter(r=>r.kind==='phase').length,3);assert.equal(detail.rows.filter(r=>r.kind==='wait').length,1);
 assert.ok(detail.rows.some(r=>r.kind==='resource'&&r.object.id==='W-C'));
 for(const work of primitives.filter(s=>s.kind==='work')){const moved=detail.primitives.find(s=>s.key===work.key);assert.equal(moved.interval,work.interval);assert.equal(moved.selection,work.selection);assert.equal(moved.run_id,c.run.id);assert.ok(moved.row.includes(work.selection.job.id));}
 assert.equal(JSON.stringify(c),before);assert.equal(new Set(detail.rows.map(r=>r.id)).size,detail.rows.length);
});
test('detail windows clip display only, keep real absolute intervals and keep computation caps open',()=>{
 const c=record('baseline-prefix'),primitives=p.actualProjection(c.project,c.run),device=c.run.jobs[0].execution_ref.data.device_id;
 const detail=p.deviceDetailProjection(c.project,c.run,primitives,device,{start_ms:5000,end_ms:25000});
 assert.ok(detail.rows.some(r=>r.kind==='phase'));for(const segment of detail.primitives){assert.equal(segment.run_id,c.run.id);assert.ok(!segment.interval||segment.interval.end_ms<=c.run.computed_until_ms);}
 assert.ok(detail.primitives.some(s=>!s.known_end));
});
