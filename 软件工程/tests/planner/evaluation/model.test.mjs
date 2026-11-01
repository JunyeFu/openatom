import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {acceptanceScopes, evidenceMatches, quantityText, metricText, scenarioReferences,prepareScenarioFork} from '../../../apps/planner/src/features/comparison/model.ts';
import {completedOutput, observeTask} from '../../../apps/planner/src/features/reporting/task.ts';
const raw=JSON.parse(await readFile(new URL('../../../packages/commands/tests/fixtures/comparison/actual-frozen-inputs.json',import.meta.url),'utf8'));
const evidence=raw.map(([project,scenario,,run])=>({project,scenario,run}));
test('exact decimal display preserves zero, decimal places and safe large coefficients',()=>{
 assert.equal(quantityText({value:{coefficient:1,decimal_places:9},unit:'kg'}),'0.000000001 kg');
 assert.equal(quantityText({value:{coefficient:9007199254740991,decimal_places:2},unit:'piece'}),'90071992547409.91 piece');
 assert.equal(metricText({kind:'duration',data:0}),'0 ms');
 assert.equal(metricText({kind:'unknown',data:{message:'未完整接受'}}),'未知：未完整接受');
});
test('frozen evidence identity rejects borrowing a current or wrong Run project revision',()=>{
 const ids=evidence.map(x=>x.run.id);const id=evidence[0].project.id;
 assert.equal(evidenceMatches(id,ids,evidence),true);
 const other=structuredClone(evidence);other[0].project.revision++;
 assert.equal(evidenceMatches(id,ids,other),false);
 assert.equal(evidenceMatches('other-project',ids,evidence),false);
 const synthetic=structuredClone(evidence);synthetic[0].run.evidence_origin='manual_expected';
 assert.equal(evidenceMatches(id,ids,synthetic),false);
});
test('declared acceptance scopes and frozen scenario refs retain product/version/unit distinctions',()=>{
 const before=structuredClone(evidence);const scopes=acceptanceScopes(evidence);
 assert.ok(scopes.length);assert.ok(scopes.every(x=>x.product_ref&&x.unit&&x.destination&&x.stage));
 const copy=structuredClone(evidence);copy[1].run.input_ref.scenario_id=copy[0].run.input_ref.scenario_id;
 copy[1].run.input_ref.scenario_revision=copy[0].run.input_ref.scenario_revision+1;
 assert.equal(scenarioReferences(copy).length,2);
 assert.deepEqual(evidence,before);
});
const handle={id:'finite-task',scope:{kind:'edit',data:{kind:'project',data:'P'}},kind:'exporting',state:'queued',input_revision:1,progress:null,result_ref:null,failure:null};
const state=(name,result=null)=>({task:{...handle,state:name},originating_operation:'export_evaluation',allowed_controls:['cancel'],result,failure:null});
test('queued admission and cancel-requested never mean successful export',async()=>{
 const sequence=[state('queued'),state('running'),state('cancel_requested'),state('cancelled')];let reads=0;const seen=[];
 const final=await observeTask(handle,async()=>sequence[reads++],s=>seen.push(s.task.state),new AbortController().signal,async()=>{});
 assert.deepEqual(seen,['queued','running','cancel_requested','cancelled']);
 assert.equal(completedOutput(final,'evaluation'),null);
});
test('completion winning cancellation retains actual typed file result, wrong kinds do not pass',async()=>{
 const result={kind:'evaluation',data:{files:[{path:'report.json',kind:'evaluation'}]}};
 const sequence=[state('cancel_requested'),state('completed',result)];let reads=0;
 const final=await observeTask(handle,async()=>sequence[reads++],()=>{},new AbortController().signal,async()=>{});
 assert.deepEqual(completedOutput(final,'evaluation'),result);
 assert.equal(completedOutput(final,'package'),null);
 assert.equal(completedOutput(state('completed'),'evaluation'),null);
 assert.equal(completedOutput(state('completed',{kind:'evaluation',data:{files:[]}}),'evaluation'),null);
});

test('new Scenario uses exact frozen initial inputs and the existing put_object API without touching old Run',()=>{
 const source=structuredClone(evidence[0]), before=structuredClone(source);
 const project=structuredClone(source.project);project.revision+=2;
 const params=prepareScenarioFork(project,source,{source_run_id:source.run.id,new_scenario_id:' UI-NEW ',title:' 供料候选 '});
 assert.equal(params.base_revision,project.revision);
 assert.equal(params.commands[0].kind,'put_object');
 assert.equal(params.commands[0].data.kind,'scenario');
 const scenario=params.commands[0].data.data;
 assert.equal(scenario.id,'UI-NEW');assert.equal(scenario.title,'供料候选');assert.equal(scenario.revision,1);assert.equal(scenario.project_revision,project.revision);
 for(const key of Object.keys(source.scenario).filter(k=>!['id','title','revision','project_revision'].includes(k))) assert.deepEqual(scenario[key],source.scenario[key],key);
 assert.deepEqual(source,before);
});

test('new Scenario refuses a mismatched Run or existing identity instead of replacing it',()=>{
 const source=evidence[0],request={source_run_id:source.run.id,new_scenario_id:source.scenario.id,title:'重复'};
 assert.throws(()=>prepareScenarioFork(source.project,source,request),/已存在/);
 assert.throws(()=>prepareScenarioFork(source.project,source,{...request,source_run_id:'other',new_scenario_id:'UNIQUE'}),/真实冻结输入/);
});
test('stopping observation does not cancel business and never publishes a late response',async()=>{
 const control=new AbortController();let published=0;let calls=0;
 await assert.rejects(observeTask(handle,async()=>{calls++;control.abort();return state('completed');},()=>published++,control.signal,async()=>{}),{name:'AbortError'});
 assert.equal(calls,1);assert.equal(published,0);
});
test('another task or scope is rejected before publishing external state',async()=>{
 await assert.rejects(observeTask(handle,async()=>({...state('completed'),task:{...handle,id:'other'}}),()=>assert.fail('wrong task published'),new AbortController().signal,async()=>{}),/另一任务/);
});
