// Read-only closure of the actual Native72052 independent acceptance session.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFile,writeFile} from 'node:fs/promises';
import {isDeepStrictEqual} from 'node:util';
const require=createRequire(import.meta.url),{chromium}=require(process.argv[2]);
const out=new URL('../../../.work/whole-packages/window5-timeline-evaluation/native-context-72052/',import.meta.url);
const read=async name=>JSON.parse(await readFile(new URL(name,out),'utf8'));
const browser=await chromium.connectOverCDP(process.argv[3]);
const page=browser.contexts().flatMap(c=>c.pages()).find(p=>!p.url().startsWith('devtools:'));
const identity=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('agent_connection_info'));
assert.equal(identity.host_instance_id,'desktop-host-72052');
const requests=[];
const equal=(actual,expected,label)=>assert.ok(isDeepStrictEqual(actual,expected),label);
async function api(operation,params,projectId='desktop-host-72052:project:2',scope={kind:'project'}) {
 const request={api_version:'0.1',request_id:'w5-final-read-'+crypto.randomUUID(),project_id:projectId,scope,operation,params};
 const response=await page.evaluate(request=>window.__TAURI_INTERNALS__.invoke('project_api',{request}),request);
 requests.push({request,response});
 assert.equal(response.outcome.status,'ok');return response.outcome.data;
}
const context=await api('get_context',{selection:[]},null,{kind:'workspace'});
assert.equal(context.project.id,'desktop-host-72052:project:2');assert.equal(context.project.revision,18);
assert.deepEqual(context.tasks,[]);
const project=(await api('get_object',{object_id:{kind:'project',id:context.project.id},source:{kind:'draft'},node_index:null})).data;
equal(project,await read('policy/final-Project.json'),'Final Project exact equality');
const expectedRuns=[];
for(const name of ['business/run1.json','business/run2.json','business/run3.json','actual-two-order-source-Run.json','actual-multi-target-Run.json'])expectedRuns.push(await read(name));
expectedRuns.push((await read('policy/source-actual-Run.json')).run);
const source=(await read('policy/original-input-and-expectations.json')).fixture.scenario;
const scenarioRevisions=[];
for(const name of ['W5-POLICY-UNKNOWN','W5-POLICY-KNOWN']) {
 const saved=await read('policy/'+name+'.json');expectedRuns.push(saved.result.run);
 const current=(await api('get_object',{object_id:{kind:'scenario',id:saved.input.id},source:{kind:'draft'},node_index:null})).data;
 // A later Project commit advances existing current Scenario revision metadata.
 // Frozen Run input_ref remains compared in full below.
 const {revision,project_revision,...body}=current;
 const {revision:oldRevision,project_revision:oldProjectRevision,...oldBody}=saved.input;
 equal(body,oldBody,name+' current Scenario body unchanged');
 assert.equal(project_revision,18);assert.equal(revision,name==='W5-POLICY-UNKNOWN'?2:1);
 scenarioRevisions.push({scenario:current.id,captured_revision:oldRevision,captured_project_revision:oldProjectRevision,current_revision:revision,current_project_revision:project_revision});
 for(const key of ['scope','time_reference','cost_inputs','parameter_overrides','acceptance_goal'])assert.deepEqual(current[key],source[key]);
}
for(const run of expectedRuns)equal((await api('get_run',{run_id:run.id,query:{kind:'full_result'}})).data,run,'Full frozen Run '+run.id);
assert.equal(context.run_summaries.length,8);
const result={mode:'actual Native read-only; no GUI interaction or mutation',identity,
 project:{id:project.id,revision:project.revision,dirty:context.project.dirty},scenario_revision_observations:scenarioRevisions,
 checks:[{name:'same_actual_Host_and_final_Project',passed:true},{name:'all_eight_recorded_full_Runs_unchanged',passed:true,run_ids:expectedRuns.map(r=>r.id)},
 {name:'both_policy_inputs_keep_original_scope_time_prices_parameters_and_optional_goal',passed:true},{name:'no_active_tasks',passed:true}],
 system_picker_selected:false,new_delivery_bytes_verified:false,whole_task_done:false,human_acceptance:false};
await writeFile(new URL('final-readback.json',out),JSON.stringify(result,null,2));
await writeFile(new URL('final-readonly-request-responses.json',out),JSON.stringify(requests,null,2));
console.log(JSON.stringify(result));
// Drop this CDP client only; leave the coordinator's actual Native running.
process.exit(0);
