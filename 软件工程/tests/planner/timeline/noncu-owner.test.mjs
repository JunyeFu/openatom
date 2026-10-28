// Read-only execution of the existing owning callbacks, not a Native UI test.
import test from 'node:test';import assert from 'node:assert/strict';import vm from 'node:vm';import {readFileSync} from 'node:fs';import {stripTypeScriptTypes} from 'node:module';
const source=readFileSync(new URL('../../../apps/planner/src/features/factory/Workbench.tsx',import.meta.url),'utf8');
const start=source.indexOf('  function ownsObservation('),end=source.indexOf('  async function seekRun(',start);
const commitStart=source.indexOf('  async function commitTimeline('),commitEnd=source.indexOf('  async function ',commitStart+5);
assert.ok(start>=0&&end>start&&commitStart>=0&&commitEnd>commitStart,'owning callback source boundaries');
const code=stripTypeScriptTypes(source.slice(start,end)+source.slice(commitStart,commitEnd))+'\nmodule.exports={readObservation,commitTimeline};';
const actual=JSON.parse(readFileSync(new URL('../../../.work/delivery/host/baseline.actual.json',import.meta.url)));
function owner(){
 const pending=[],calls=[],module={exports:{}},context={module,project:{...actual.project,revision:2},projectIdentity:{current:'P1'},observation:{current:1},pendingRun:{current:actual.run.id},
  host:{getRun:(...params)=>new Promise(resolve=>pending.push({params,resolve})),applyCommands:async(...params)=>{calls.push(params);return {new_revision:2};}},
  rememberCommit(){},refresh:async()=>({scenarios:[{...actual.scenario,revision:2,project_revision:2}]}),setTimelineSelection(){},setMessage(){}};
 vm.runInNewContext(code,context);return {context,api:module.exports,pending,calls};
}
function frame(at){return actual.observations.find(f=>f.at_ms===at);}
test('actual owner rejects a late same-Run old-time reply after the newer epoch has completed',async()=>{
 const o=owner(),old=o.api.readObservation(actual.run.id,0,'P1',1);o.context.observation.current=2;
 const latest=o.api.readObservation(actual.run.id,70000,'P1',2);
 o.pending[1].resolve({kind:'objects',data:frame(70000)});assert.equal((await latest).at_ms,70000);
 o.pending[0].resolve({kind:'objects',data:frame(0)});assert.equal(await old,null);
});
test('actual owner discards late old-project or old-Run replies rather than replacing current state',async()=>{
 for(const field of ['projectIdentity','pendingRun']){const o=owner(),old=o.api.readObservation(actual.run.id,0,'P1',1);o.context[field].current='changed';o.pending[0].resolve({kind:'objects',data:frame(0)});assert.equal(await old,null);}
});
test('actual owner preserves full paging identity and rejects a mismatched response millisecond',async()=>{
 const o=owner(),request=o.api.readObservation(actual.run.id,0,'P1',1);
 const objects=frame(0).objects;o.pending[0].resolve({kind:'objects',data:{...frame(0),objects:objects.slice(0,1),next_index:1}});await new Promise(r=>setImmediate(r));
 assert.equal(o.pending[1].params[2].data.page.after_index,1);o.pending[1].resolve({kind:'objects',data:{...frame(0),objects:objects.slice(1)}});
 assert.equal((await request).objects.length,objects.length);
 const wrong=o.api.readObservation(actual.run.id,0,'P1',1);o.pending[2].resolve({kind:'objects',data:{...frame(0),at_ms:1}});await assert.rejects(wrong,/观察时点或 Run/);
});
test('actual owner forwards both captured revisions through the existing single Host transaction',async()=>{
 const o=owner(),scenario=actual.scenario,proposal={project_id:'P1',scenario_id:scenario.id,base_project_revision:1,base_scenario_revision:scenario.revision,command:{kind:'put_object',data:{kind:'scenario',data:scenario}},changes:[]};
 const result=await o.api.commitTimeline(proposal);assert.equal(result.ok,true);assert.equal(o.calls.length,1);assert.equal(o.calls[0][0].revision,1);assert.equal(o.calls[0][1][0].data.data.revision,scenario.revision);
 assert.equal(JSON.stringify(o.calls[0][1]),JSON.stringify([proposal.command]));
});
