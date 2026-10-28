import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { source, rootUrl } from './load-source.mjs';
const time = await source('time'), input = await source('input'), projection = await source('projection');
const baseline = JSON.parse(await readFile(new URL('.work/delivery/host/baseline.actual.json', rootUrl)));
const delayed = JSON.parse(await readFile(new URL('.work/delivery/host/delayed-pcb.actual.json', rootUrl)));
const a = projection.actualProjection(baseline.project, baseline.run), b = projection.actualProjection(delayed.project, delayed.run);
const prefs = {version:1,group_overrides:[],independent_device_ids:[],row_order:[],collapsed_group_ids:[],pinned_resource_ids:[],axis_windows:{},row_scroll:0,snap_enabled:true};

test('input calendar boundaries retain the declared business interval without editable time handles',()=>{
  const project=structuredClone(delayed.project), scenario=structuredClone(delayed.scenario);
  project.calendars.push({id:'CHECK-SHIFT',title:'核验班次',intervals:[{start_ms:123456,end_ms:234567}],source:'user_provided'});
  const original=structuredClone({project,scenario}), rows=projection.calendarInputProjection(project,scenario);
  const boundary=rows.find(p=>p.object.id==='CHECK-SHIFT');
  assert.equal(boundary.row,'input:calendar:CHECK-SHIFT');assert.equal(boundary.source,'input');
  assert.equal(boundary.input_field,undefined);assert.deepEqual(boundary.interval,{start_ms:123456,end_ms:234567});
  assert.deepEqual(boundary.selection.true_interval,boundary.interval);assert.deepEqual({project,scenario},original);
});

test('waiting context uses exact current cargo holders, half-open claims and associated frozen events',()=>{
  const wait=b.find(p=>p.kind==='wait'&&p.interval.start_ms===370000).selection;
  const frame=delayed.observations.find(f=>f.at_ms===370000);
  const items=projection.observedItems(frame,delayed.run,370000);
  const context=projection.waitContext(wait,delayed.run,370000,items);
  assert.ok(context.held_resources.some(c=>c.resource_id==='T1'&&c.job_id===wait.object.id));
  assert.equal(context.cargo[0].holder.kind,'transport');assert.equal(context.cargo[0].holder.id,wait.object.id);
  assert.ok(context.previous.at_ms<=370000);assert.ok(context.next.at_ms>370000);
  const later=delayed.observations.find(f=>f.at_ms===425000);
  const returned=projection.waitContext(wait,delayed.run,425000,projection.observedItems(later,delayed.run,425000));
  assert.equal(returned.held_resources.length,0);assert.notEqual(returned.cargo[0].holder.kind,'transport');
  assert.equal(projection.waitContext(wait,delayed.run,370000,null).cargo,null);
  assert.equal(projection.waitContext(wait,{...delayed.run,id:'other'},370000,items),null);
});

test('DTC01 absolute axes agree and inverse returns the exact millisecond', () => {
  assert.equal(time.timeX(120000,{start_ms:0,end_ms:600000},1200,240),480);
  assert.equal(time.timeX(120000,{start_ms:100000,end_ms:200000},600,240),360);
  assert.equal(time.xTime(360,{start_ms:100000,end_ms:200000},600,240),120000);
});
test('DTC02 clipping preserves true interval and both continuation ends', () => {
  const trueInterval={start_ms:90000,end_ms:180000};
  assert.deepEqual(time.clipInterval(trueInterval,{start_ms:100000,end_ms:160000}),{visible:{start_ms:100000,end_ms:160000},true_interval:trueInterval,continues_left:true,continues_right:true});
});
test('E01 B/W/O device sets deduplicate claims and preserve denominator, overlap is allowed', () => {
  const p=(row,kind)=>({row,kind,interval:{start_ms:0,end_ms:10000}});
  const bands=projection.summarizeGroup([p('device:A','work'),p('device:A','work'),p('device:B','wait'),p('device:A','claim'),p('device:A','claim')],['A','B']);
  assert.deepEqual(bands,[{interval:{start_ms:0,end_ms:10000},busy:1,waiting:1,occupied:1,total:2}]);
});
test('both real distinct IX01 RunIds retain full actual jobs/phases/events and 440/570 completion',()=>{
  assert.notEqual(baseline.run.id,delayed.run.id);
  for(const [record,primitives,last] of [[baseline,a,440000],[delayed,b,570000]]){
    assert.equal(Math.max(...record.run.acceptance_events.map(v=>v.at_ms)),last);
    assert.equal(primitives.filter(p=>p.kind==='work').length,record.run.phase_work_intervals.length);
    assert.equal(primitives.filter(p=>p.selection.event).length,new Set(record.run.events.map(e=>e.sequence)).size);
    assert.equal(record.run.transport.length,6);
    assert.equal(primitives.filter(p=>p.kind==='job').length,record.run.jobs.filter(j=>j.started_at_ms!=null).length);
  }
});
test('actual delayed trip keeps real waiting, receive and return plus physical cargo identity at boundary',()=>{
  const trip=delayed.run.transport.find(t=>t.arrived_ms===370000); assert.equal(trip.received_ms,420000); assert.equal(trip.returned_ms,425000);
  assert.ok(b.some(p=>p.kind==='wait'&&p.interval.start_ms===370000&&p.interval.end_ms===420000));
  for(const at of [370000,419999]){const frame=delayed.observations.find(f=>f.at_ms===at);for(const id of trip.cargo) assert.equal(frame.objects.find(o=>o.data.id===id).data.holder.kind,'transport');}
  const returned=delayed.observations.find(f=>f.at_ms===425000);for(const id of trip.cargo)assert.notEqual(returned.objects.find(o=>o.data.id===id).data.holder.kind,'transport');
});
test('capacity O is only actual frozen execution resource; person holds do not invent occupancy',()=>{
  const run=structuredClone(baseline.run), job=run.jobs.find(j=>j.execution_ref.kind==='project_device');
  run.jobs=[job];run.phase_work_intervals=[];run.waits=[];run.transport=[];run.events=[];
  const base={id:'claim',job_id:job.id,units:1,kind:'job',intervals:[{start_ms:0,end_ms:1000}],interval_end_causes:['released']};
  run.resource_claims=[{...base,resource_id:'unrelated-person'}];
  assert.equal(projection.actualProjection(baseline.project,run).filter(p=>p.row===`device:${job.execution_ref.data.device_id}`&&p.kind==='claim').length,0);
  run.resource_claims.push({...base,id:'capacity',resource_id:job.execution_ref.data.execution_capacity_ref.id});
  assert.equal(projection.actualProjection(baseline.project,run).filter(p=>p.row===`device:${job.execution_ref.data.device_id}`&&p.kind==='claim').length,1);
});
test('DTC11 prefix phase identity does not depend on growing computed end; snapshots replace, no duplicates',()=>{
  const r=structuredClone(baseline.run), work=r.phase_work_intervals[0];r.phase_work_intervals=[{...work,interval:{start_ms:5000,end_ms:20000},end_cause:'computed_boundary'}];
  const first=projection.actualProjection(baseline.project,r).find(p=>p.kind==='work');r.phase_work_intervals[0].interval.end_ms=25000;
  const second=projection.actualProjection(baseline.project,r).find(p=>p.kind==='work');assert.equal(first.key,second.key);assert.equal(first.interval.end_ms,20000);assert.equal(second.interval.end_ms,25000);assert.equal(second.known_end,false);
});
test('missing exact frozen project refuses to interpret old IDs using current draft',()=>{
  assert.deepEqual(projection.actualProjection({...baseline.project,revision:baseline.project.revision+1},baseline.run),[]);
});
test('DTC09 item observation must be complete exact RunId and millisecond; final_items never substitute',()=>{
  const f=baseline.observations[0];assert.ok(projection.observedItems(f,baseline.run,f.at_ms));
  for(const bad of [{...f,run_id:'other'},{...f,at_ms:1},{...f,next_index:1},null])assert.equal(projection.observedItems(bad,baseline.run,0),null);
});
test('a followed Item inspection follows the current actual observation, never a previously selected holder',()=>{
  const frames=baseline.observations.filter(frame=>frame.next_index===null);
  const first=frames[0];
  const original=first.objects.find(object=>object.kind==='item'&&frames.some(frame=>frame.objects.some(next=>next.kind==='item'&&next.data.id===object.data.id&&JSON.stringify(next.data.holder)!==JSON.stringify(object.data.holder)))).data;
  const next=frames.find(frame=>frame.objects.some(object=>object.kind==='item'&&object.data.id===original.id&&JSON.stringify(object.data.holder)!==JSON.stringify(original.holder)));
  const actual=next.objects.find(object=>object.kind==='item'&&object.data.id===original.id).data;
  const selected={key:`${baseline.run.id}:item:${original.id}`,source:'actual',run_id:baseline.run.id,input_ref:baseline.run.input_ref,
    object:{kind:'item',id:original.id},label:original.id,at_ms:first.at_ms,fields:[{label:'持有者',value:`${original.holder.kind} ${original.holder.id}`}],related_objects:[projection.holderObject(original.holder)]};
  const saved=structuredClone({selected,run:baseline.run,first,next});
  const refreshed=projection.resolveFollowedItemSelection(selected,next,baseline.run,next.at_ms);
  assert.equal(refreshed.at_ms,next.at_ms);
  assert.equal(refreshed.fields.find(field=>field.label==='持有者').value,`${actual.holder.kind} ${actual.holder.id}`);
  assert.equal(refreshed.fields.find(field=>field.label==='生命周期').value,actual.lifecycle);
  assert.deepEqual(refreshed.related_objects,[projection.holderObject(actual.holder)]);
  for(const missing of [null,{...next,next_index:1},{...next,at_ms:next.at_ms+1},{...next,run_id:'other'}]){
    const pending=projection.resolveFollowedItemSelection(selected,missing,baseline.run,next.at_ms);
    assert.equal(pending.fields.some(field=>field.label==='持有者'),false);
    assert.deepEqual(pending.related_objects,[]);
    assert.match(pending.fields[0].value,/读取/);
  }
  const absent=projection.resolveFollowedItemSelection(selected,{...next,objects:[]},baseline.run,next.at_ms);
  assert.equal(absent.fields.some(field=>field.label==='持有者'),false);assert.match(absent.fields[0].value,/无该身份/);
  assert.equal(projection.resolveFollowedItemSelection(selected,next,{...baseline.run,id:'another'},next.at_ms),null);
  const event={...selected,key:'actual-specific-event',event:baseline.run.events[0]};
  assert.equal(projection.resolveFollowedItemSelection(event,next,baseline.run,next.at_ms),event);
  assert.deepEqual({selected,run:baseline.run,first,next},saved);
});
test('E04 linked inputs use same delta, one current-revision command, no due/calendar/inventory mutation',()=>{
  const project=structuredClone(baseline.project),scenario=structuredClone(baseline.scenario),before=structuredClone(scenario);
  scenario.arrivals=[{...scenario.arrivals[0],id:'a',at_ms:10000},{...scenario.arrivals[0],id:'b',at_ms:20000}];
  const proposal=input.inputProposal(project,scenario,[{kind:'arrival',id:'a'},{kind:'arrival',id:'b'}],5000);
  assert.deepEqual(proposal.command.data.data.arrivals.map(a=>a.at_ms),[15000,25000]);
  assert.equal(proposal.command.data.data.revision,scenario.revision);assert.equal(proposal.base_project_revision,project.revision);
  assert.deepEqual(proposal.command.data.data.orders,before.orders);assert.deepEqual(proposal.command.data.data.initial_conditions,before.initial_conditions);assert.equal(proposal.command.data.data.initial_conditions,scenario.initial_conditions);
  assert.deepEqual(scenario.arrivals.map(a=>a.at_ms),[10000,20000]);
});
test('snap uses 6px nearest, deterministic reference/business/grid tie ordering, Alt bypass exact value',()=>{
  const w={start_ms:0,end_ms:1000},targets=[{id:'g',at_ms:100,label:'grid',kind:'grid'},{id:'b',at_ms:100,label:'business',kind:'business'},{id:'r',at_ms:100,label:'ref',kind:'reference'}];
  assert.equal(time.snapTime(105,w,1000,targets,true).target.id,'r');assert.equal(time.snapTime(107,w,1000,targets,true).target,null);assert.equal(time.snapTime(105,w,1000,targets,false).at_ms,105);
});
test('manual overrides and independent devices never duplicate membership; ambiguous business refs explicit',()=>{
  const p=structuredClone(baseline.project), id=p.devices[0].id; const hall=p.devices[0].hall_id;
  const groups=projection.displayGroups(p,{...prefs,group_overrides:[{id:'manual',hall_id:hall,title:'custom',members:[id,id]}]});
  const all=groups.flatMap(g=>g.devices);assert.equal(all.filter(v=>v===id).length,1);
  const clone=structuredClone(p.lines[0]??{id:'L',hall_id:hall,members:[]});clone.id='conflict';clone.hall_id=hall;clone.members=[id];p.lines.push(clone);p.devices[0].line_id='missing-line';
  assert.ok(projection.displayGroups(p,prefs).some(g=>g.warning&&g.devices.includes(id)));
});
test('end events stay zero-duration point, right edge visible, duplicate sequence deduplicated',()=>{
  const r=structuredClone(baseline.run), e={...r.events[0],sequence:999999,at_ms:r.window.end_ms};r.events=[e,e];
  const points=projection.actualProjection(baseline.project,r).filter(p=>p.selection.event);assert.equal(points.length,1);assert.equal(points[0].interval,undefined);assert.equal(time.pointVisible(e.at_ms,r.window),true);
});
test('real run comparisons require initial/time/window/acceptance scope; location or initial edits downgrade',()=>{
  assert.equal(projection.comparisonConditions(baseline.scenario,delayed.scenario).comparable,true);
  const changed=structuredClone(baseline.scenario);changed.orders[0].destination='different';assert.equal(projection.comparisonConditions(baseline.scenario,changed).comparable,false);
  assert.equal(projection.comparisonConditions(baseline.scenario,null).comparable,false);
});
test('actual computed prefix leaves the uncomputed tail without invented completions',async()=>{
  const prefix=JSON.parse(await readFile(new URL('.work/delivery/host/baseline-prefix.actual.json',rootUrl)));
  assert.equal(prefix.run.id,baseline.run.id);assert.ok(prefix.run.computed_until_ms<600000);
  const partial=projection.actualProjection(prefix.project,prefix.run);
  assert.ok(partial.some(p=>!p.known_end));assert.ok(partial.every(p=>p.point_ms==null||p.point_ms<=prefix.run.computed_until_ms));
  assert.ok(partial.every(p=>!p.interval||p.interval.end_ms<=prefix.run.computed_until_ms));
});
test('actual worker calendar has 5s load + 5s unload, autonomous phase and machine hold remain distinct',async()=>{
  const c=JSON.parse(await readFile(new URL('.work/delivery/host/phase-calendar.actual.json',rootUrl)));
  const projected=projection.actualProjection(c.project,c.run), work=projected.filter(p=>p.kind==='work');
  assert.deepEqual(work.map(p=>[p.phase,p.interval.start_ms,p.interval.end_ms]),[['load',0,5000],['auto',5000,35000],['unload',40000,45000]]);
  const worker=projected.filter(p=>p.row==='resource:W-C'&&p.kind==='claim');assert.equal(worker.reduce((s,p)=>s+p.interval.end_ms-p.interval.start_ms,0),10000);
  const machine=projected.filter(p=>p.row==='resource:CAP-CNC'&&p.kind==='claim');assert.equal(machine.reduce((s,p)=>s+p.interval.end_ms-p.interval.start_ms,0),45000);
  assert.ok(projected.some(p=>p.kind==='wait'&&p.interval.start_ms===35000&&p.interval.end_ms===40000));
  const utilization=projection.resourceUsage(c.project,c.run,['W-C'],{start_ms:0,end_ms:100000});assert.equal(utilization[0].ratio,.25);
});
test('weighted utilization integrates capacity time by unit, never average of resource percentages',()=>{
  const p={resources:[{id:'R1',calendar_id:'C',capacity:{value:{coefficient:1,decimal_places:0},unit:'slot'}},{id:'R2',calendar_id:'C',capacity:{value:{coefficient:3,decimal_places:0},unit:'slot'}}],calendars:[{id:'C',intervals:[{start_ms:0,end_ms:100}]}]};
  const r={computed_until_ms:100,resource_claims:[{resource_id:'R1',units:1,intervals:[{start_ms:0,end_ms:100}]}]};
  assert.equal(projection.resourceUsage(p,r,['R1','R2'],{start_ms:0,end_ms:100})[0].ratio,.25);
});
test('same schema devices with changed group membership downgrade direct comparison',()=>{
  const other=structuredClone(baseline.project);other.devices[0].hall_id='different';
  assert.equal(projection.comparisonConditions(baseline.scenario,baseline.scenario,baseline.project,other).comparable,false);
});
test('private preference V1 rejects unusable stored windows instead of dividing by zero or reinterpreting identities',async()=>{
  const preferences=await source('preferences');
  assert.equal(preferences.preferenceKey('P1','R1'),'superfactory.timeline.view.v1:P1:run:R1');
  assert.equal(preferences.preferenceKey('P1',null),'superfactory.timeline.view.v1:P1:draft');
  assert.deepEqual(preferences.readPreferences('key',{getItem:()=>JSON.stringify(prefs)}),prefs);
  for(const bad of [{...prefs,axis_windows:null},{...prefs,axis_windows:{main:{start_ms:10,end_ms:10}}},{...prefs,version:2}])assert.throws(()=>preferences.readPreferences('key',{getItem:()=>JSON.stringify(bad)}));
});
test('whole IX01 projection preserves every wait, resource interval, event payload and genuine six-item acceptance',()=>{
  for(const [record,primitives] of [[baseline,a],[delayed,b]]){
    assert.equal(primitives.filter(p=>p.kind==='wait').length,record.run.waits.length);
    for(const wait of record.run.waits)assert.ok(primitives.some(p=>p.kind==='wait'&&p.selection.wait===wait&&p.interval===wait.interval));
    assert.equal(primitives.filter(p=>p.kind==='claim'&&p.row.startsWith('resource:')).length,record.run.resource_claims.reduce((n,c)=>n+c.intervals.length,0));
    for(const event of record.run.events)assert.ok(primitives.some(p=>p.selection.event===event));
    assert.equal(record.run.acceptance_events.reduce((n,e)=>n+e.quantity.value.coefficient/10**e.quantity.value.decimal_places,0),6);
    const frozen=JSON.stringify(record.run);projection.summarizeGroup(primitives,record.project.devices.map(d=>d.id));projection.actualProjection(record.project,record.run);assert.equal(JSON.stringify(record.run),frozen);
  }
});
test('6px dense interval buckets keep every true source and distinguish work from waiting',()=>{
  const p=(key,kind,start_ms,end_ms)=>({key,kind,interval:{start_ms,end_ms}}), first=p('a','work',0,1),second=p('b','work',1,2),wait=p('w','wait',0,2),long=p('long','work',100,400);
  const buckets=time.intervalDensity([first,second,wait,long],{start_ms:0,end_ms:1000},600);
  assert.equal(buckets.length,1);assert.deepEqual(buckets[0].sources,[first,second]);assert.deepEqual(first.interval,{start_ms:0,end_ms:1});
});
