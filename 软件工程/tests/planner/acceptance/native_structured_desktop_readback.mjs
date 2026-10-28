// Independent read-only recheck of the W5 actual desktop interaction batch.
// Attach only to the coordinator's existing Native. Never launch/close Native,
// change inputs, trigger a picker, fabricate a selection token or rerun Engine.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFile, writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const require=createRequire(import.meta.url);
const {chromium}=require(process.argv[2]);
const endpoint=process.argv[3], expectedHost=process.argv[4];
assert.ok(endpoint&&expectedHost,'Pass provided CDP endpoint and exact desktop-host-PID');
const root=new URL('../../../',import.meta.url);
const evidence=new URL('.work/whole-packages/window5-timeline-evaluation/native-structured-79676/',root);
const json=async name=>JSON.parse(await readFile(new URL(name,evidence),'utf8'));
const proof=await json('result.json'), baseline=await json('baseline-input.json');
assert.equal(expectedHost,proof.expectedHost);
const browser=await chromium.connectOverCDP(endpoint);
const page=browser.contexts().flatMap(c=>c.pages()).find(p=>!p.url().startsWith('devtools:'));
assert.ok(page,'Existing product WebView required');
page.setDefaultTimeout(5000);
const identity=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('agent_connection_info'));
assert.equal(identity.host_instance_id,expectedHost);
const pid=baseline.project.id, checks=[];
async function api(operation,params,projectId=pid) {
 return page.evaluate(async({operation,params,projectId})=>{
  const response=await window.__TAURI_INTERNALS__.invoke('project_api',{request:{
   api_version:'0.1',request_id:'w5-final-readback-'+crypto.randomUUID(),
   project_id:projectId,scope:{kind:projectId?'project':'workspace'},operation,params
  }});
  if(response.outcome.status!=='ok')throw new Error(JSON.stringify(response));
  return response.outcome.data;
 },{operation,params,projectId});
}
const object=async(kind,id,source={kind:'draft'})=>(await api('get_object',{
 object_id:{kind,id},source,node_index:null
})).data;
const full=async id=>(await api('get_run',{run_id:id,query:{kind:'full_result'}})).data;
const context=await api('get_context',{selection:[]},null);
assert.equal(context.project.id,pid);
assert.equal(context.host.session_id,'desktop-main-ui');
checks.push('same_actual_Native_Project_and_UI_session');
for(const [name,expectedTimes] of [['run1.json',[70000,110000]],['run2.json',[72000]],['run3.json',[73000]]]) {
 const saved=await json(name), actual=await full(saved.id);
 assert.deepEqual(actual,saved);
 assert.equal(actual.evidence_origin,'actual_software');
 assert.deepEqual(actual.acceptance_events.map(e=>e.at_ms),expectedTimes);
 checks.push(name+':complete_frozen_Run_and_original_times');
}
const run1=await json('run1.json'), run2=await json('run2.json'), run3=await json('run3.json');
assert.equal(run1.goal_met,true);
for(const run of [run2,run3]) {
 assert.equal(run.goal_met,false);
 assert.equal(run.orders[0].completion_ms,null);
 assert.equal(run.orders[0].tardiness_ms,null);
}
checks.push('unknown_order_completion_and_tardiness_never_become_zero');
for(const at of [4999,5000]) {
 const actual=await api('get_run',{run_id:run2.id,query:{kind:'objects_at',data:{
  at_ms:at,objects:[],page:{after_index:null,limit:500}
 }}});
 assert.deepEqual(actual,await json('run2-objects-'+at+'.json'));
 assert.equal(actual.data.next_index,null);
 assert.equal(actual.data.objects.some(o=>o.kind==='item'&&o.data.id==='W5-EXPLICIT-FUTURE-SHELL'),at===5000);
}
checks.push('real_future_arrival_4999_5000_boundary');
const cloneProof=await json('clone-readback.json'), clone=cloneProof.clone;
assert.deepEqual({...clone,id:baseline.scenario.id,title:baseline.scenario.title,
 revision:baseline.scenario.revision,project_revision:baseline.scenario.project_revision},baseline.scenario);
assert.deepEqual({...cloneProof.after_project,revision:cloneProof.before_project.revision,
 scenario_refs:cloneProof.before_project.scenario_refs},cloneProof.before_project);
assert.equal(cloneProof.after_project.revision-cloneProof.before_project.revision,1);
assert.equal(clone.initial_conditions.items.length,4);
assert.equal(run1.final_items.length,16);
checks.push('clone_complete_initial_inputs_one_commit_and_current_Project_retained');
const currentProject=await object('project',pid);
assert.deepEqual(currentProject,proof.final.project);
for(const scenario of proof.final.scenarios)assert.deepEqual(await object('scenario',scenario.id),scenario);
assert.equal(currentProject.devices[0].configuration.cycle_seconds.value.value,41000);
assert.equal(currentProject.setup_policies[0].changeovers[0].duration.value,23000);
checks.push('exact_final_Project_and_both_actual_draft_Scenarios');
const delivery=page.getByRole('region',{name:'方案评估与交接工作区',exact:true});
const panel=delivery.getByRole('region',{name:'评估报告准备',exact:true});
assert.equal(await delivery.locator('.delivery-run-choice').filter({hasText:run1.id}).getByRole('checkbox').isChecked(),true);
for(const id of [run2.id,run3.id])assert.equal(await delivery.locator('.delivery-run-choice').filter({hasText:id}).getByRole('checkbox').isChecked(),false);
assert.equal(await panel.getByRole('radio',{name:'实际运行评估',exact:true}).isChecked(),true);
for(const format of ['PDF','HTML（离线）','CSV（15分表）','JSON'])assert.equal(await panel.getByRole('checkbox',{name:format,exact:true}).isChecked(),true);
assert.equal(await panel.getByRole('button',{name:'选择目录并生成报告',exact:true}).isEnabled(),true);
checks.push('single_original_achieved_Run_four_formats_ready_before_Root_picker');
await writeFile(new URL('final-independent-readback.json',evidence),JSON.stringify({
 identity,project_id:pid,project_revision:currentProject.revision,checks,
 interaction_groups:proof.counts,known_open_finding:'W5-P205-01',
 product_mutations:0,picker_triggered:false,native_left_running:true,
 scope:'Read-only recheck of existing actual interaction evidence; not additional acceptance groups.',
 whole_task_done:false,human_acceptance:false
},null,2));
console.log(JSON.stringify({passed:checks.length,interaction_groups:proof.counts,
 report_ready_for_Root_picker:true,known_open_finding:'W5-P205-01',
 output:fileURLToPath(new URL('final-independent-readback.json',evidence))}));
// End only this client; browser.close() must not close coordinator's Native.
process.exit(0);
