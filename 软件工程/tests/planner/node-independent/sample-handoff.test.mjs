import {test,after} from 'node:test';import assert from 'node:assert/strict';import {readFile,writeFile} from 'node:fs/promises';
import {openCore} from '../node-inspector/core.mjs';import {root,out,authoring,sampleReceiver} from './load-source.mjs';
const evidence=[],cores=[];
after(async()=>{await writeFile(new URL('actual-sample-handoff.json',out),JSON.stringify({evidence,records:cores.map(c=>c.records),Native:false,browser:false},null,2));for(const c of cores)await c.close();});
async function setup(transport=false){
  const path=transport?'packages/commands/tests/fixtures/production/p202-B14.json':'scenarios/图片时间轴/runtime/phase.input.json',input=JSON.parse(await readFile(new URL(path,root),'utf8')),core=await openCore(path);cores.push(core);const pid=core.ready.snapshot.project.id;
  const ports={async dispatch(q){const r=await core.rpc({kind:'dispatch',request:q});if(r.outcome.status!=='ok')throw Object.assign(new Error(r.outcome.data.message),{detail:r.outcome.data});return r.outcome.data;}},request=(operation,params,scope={kind:'project'})=>ports.dispatch({api_version:'0.1',request_id:'sample-review-'+crypto.randomUUID(),project_id:pid,scope,operation,params});
  const ref=transport?input.definitions.find(d=>d.payload.category==='transport'):input.definitions.find(d=>d.id==='p303.owned.manual-assembly'),pkg=await request('get_object',{object_id:{kind:'module',id:ref.id},source:{kind:'module_version',data:{module_ref:{id:ref.id,version:ref.version}}},node_index:null}),commit=await request('apply_commands',{base_revision:input.project.revision,commands:[{kind:'create_definition_draft',data:pkg.data.definition}]}),scope={kind:'definition_draft',data:commit.scope.data.id},draft=await authoring.readDraft(ports,pid,scope.data),task=await request('run_scenario',{scenario_id:input.scenario.id,input_revision:1,observation_window:input.scenario.window,intent:{kind:'definition_cycle',data:{definition_revision:1,test_input:input.scenario}}},scope),done=(await core.rpc({kind:'await_task',scope,task_id:task.id})).outcome.data;
  assert.equal(done.task.state,'completed',JSON.stringify(done));const run=(await request('get_run',{run_id:done.result.data.run_id,query:{kind:'full_result'}},scope)).data;
  return {core,ports,request,scope,draft,run,pid};
}
async function receiver(value,afterReply){const active={current:{id:'actual-sample-origin',draft:value.draft,scope:value.scope,selection:{kind:'definition',module_ref:{id:value.draft.definition.id,version:value.draft.definition.version}}}},workspace={current:{project:value.core.ready.snapshot.project,context:value.core.ready.context}},changes=[];
  const authoringIO={async dispatch(q){const result=await value.ports.dispatch(q);if(afterReply)await afterReply({q,result,active,workspace});return result;}},receive=await sampleReceiver({deviceProcessRef:active,workspaceRef:workspace,authoringIO,authoringRequest:authoring.authoringRequest,setDeviceProcess:next=>changes.push(next)});return {receive,active,workspace,changes};}
test('actual legal project_device sample maps its unique frozen Device version and does not rewrite original Run refs',async()=>{
  const value=await setup(),before=structuredClone(value.run),handle=await receiver(value);assert.ok(before.jobs.some(j=>j.execution_ref.kind==='project_device'));await handle.receive(value.run);
  assert.equal(handle.changes.length,1);const selected=handle.changes[0].selection;assert.equal(selected.kind,'actual_run');assert.equal(selected.run_id,value.run.id);assert.equal(selected.at_ms,value.run.computed_until_ms);
  assert.deepEqual((await value.request('get_run',{run_id:value.run.id,query:{kind:'full_result'}},value.scope)).data,before);evidence.push({name:'actual project_device handoff',selection:selected,run:value.run,original_refs_preserved:true});
});
test('actual existing transport DefinitionCycle produces pure definition_cycle ref and is mapped by exact module version',async()=>{
  const value=await setup(true),before=structuredClone(value.run),handle=await receiver(value);const exact=value.run.jobs.filter(j=>j.execution_ref.kind==='definition_cycle'&&j.execution_ref.data.module_ref.id===value.draft.definition.id);assert.equal(exact.length,1);await handle.receive(value.run);assert.equal(handle.changes.length,1);assert.equal(handle.changes[0].selection.job_id,exact[0].id);assert.deepEqual((await value.request('get_run',{run_id:value.run.id,query:{kind:'full_result'}},value.scope)).data,before);evidence.push({name:'actual transport pure definition_cycle handoff',selection:handle.changes[0].selection,run:value.run,does_not_claim_Device_UI_support_for_transport:true});
});
test('wrong exact Draft version and multiple matching callback jobs reject without selecting first or rewriting actual Run',async()=>{
  const value=await setup(),original=structuredClone(value.run);
  await value.request('apply_commands',{base_revision:1,commands:[{kind:'commit_definition_version',data:{version:'0.9.0'}}]},value.scope);value.draft=await authoring.readDraft(value.ports,value.pid,value.scope.data);
  const noMatch=await receiver(value);await assert.rejects(noMatch.receive(value.run),/0项/);assert.equal(noMatch.changes.length,0);
  // The duplicate is explicitly a callback-boundary fixture; Host Run stays original.
  const second=await setup(),callback=structuredClone(second.run),selected=callback.jobs.find(j=>j.execution_ref.kind==='project_device'&&j.execution_ref.data.device_id==='CNC');assert.ok(selected);callback.jobs.push({...selected,id:selected.id+'-controlled-duplicate'});const multiple=await receiver(second);await assert.rejects(multiple.receive(callback),/2项/);assert.equal(multiple.changes.length,0);assert.deepEqual((await value.request('get_run',{run_id:value.run.id,query:{kind:'full_result'}},value.scope)).data,original);evidence.push({name:'wrong-version actual Draft and duplicate callback fixture',actual_wrong_version:'0.9.0',duplicate_is_callback_fixture_not_Host_run:true,original_run_unchanged:true});
});
test('late source or ScopeR changes while actual frozen Project reply returns cannot navigate an old sample',async()=>{
  const value=await setup();for(const [name,change]of [['entry',a=>{a.current={...a.current,id:'new-entry'};}],['DraftR',a=>{a.current={...a.current,draft:{...a.current.draft,summary:{...a.current.draft.summary,revision:2}}};}]]){
    let changed=false;const handle=await receiver(value,({q,active})=>{if(!changed&&q.operation==='get_object'){changed=true;change(active);}});await assert.rejects(handle.receive(value.run),/已产生.*来源已切换/);assert.equal(handle.changes.length,0);evidence.push({name:'late actual frozen Project handoff rejected',change:name});
  }
});
test('actual Host DraftR change during frozen Project read is detected even before Root local props refresh',async()=>{
  const value=await setup();let changed=false;
  const handle=await receiver(value,async({q})=>{if(!changed&&q.operation==='get_object'){
    changed=true;const next={...value.draft.definition,title:value.draft.definition.title+' real external DraftR2'};
    await value.request('apply_commands',{base_revision:1,commands:[{kind:'put_definition',data:next}]},value.scope);
  }});let failure;try{await handle.receive(value.run);}catch(e){failure=e;}
  const actual=await authoring.readDraft(value.ports,value.pid,value.scope.data);assert.equal(actual.summary.revision,2);
  evidence.push({name:'actual Host DraftR2 race with still-R1 local props',actual_draft:actual.summary,local_draft:value.draft.summary,navigations:handle.changes,error:failure?.message});
  assert.equal(handle.changes.length,0,'actual Host ScopeR changed but sample receiver still navigated using old local R1');assert.ok(failure);
});
