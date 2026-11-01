import test from 'node:test';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {createInterface} from 'node:readline';
import {mkdir,rename,stat,writeFile,appendFile,readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {root,source} from './closeout-source.mjs';
const draft=await source('draft'),schema=await source('schema');
const directory=fileURLToPath(new URL('.work/business-input-closeout/delivery/',root));
const runDirectory=path.join(directory,'runs',`${Date.now()}-${process.pid}`);
await mkdir(path.join(runDirectory,'temp'),{recursive:true});
const catalog=JSON.parse(await readFile(new URL('packages/commands/generated/schemas/get_object.response.schema.json',root),'utf8'));
const records=[];
async function bridge(args){
 const child=spawn(path.join(directory,'host-bridge.exe'),args,{cwd:fileURLToPath(root),windowsHide:true,env:{...process.env,TEMP:path.join(runDirectory,'temp'),TMP:path.join(runDirectory,'temp')},stdio:['pipe','pipe','pipe']});
 const pending=[],exit=once(child,'exit');let transcript=Promise.resolve();
 child.stderr.on('data',data=>{transcript=transcript.then(()=>appendFile(path.join(runDirectory,`host-${child.pid}.stderr.log`),data));});
 const lines=createInterface({input:child.stdout});
 lines.on('line',line=>{const p=pending.shift();assert.ok(p,'one actual response per request');const value=JSON.parse(line);transcript=transcript.then(()=>appendFile(path.join(runDirectory,`host-${child.pid}.ndjson`),JSON.stringify({request:p.request,response:value})+'\n'));p.resolve(value);});
 exit.then(([code])=>{for(const p of pending)p.reject(new Error(`real Host exited ${code}`));});
 return {pid:child.pid,send(request){return new Promise((resolve,reject)=>{pending.push({request,resolve,reject});child.stdin.write(JSON.stringify(request)+'\n');});},async close(){child.stdin.end();const[code]=await exit;await transcript;assert.equal(code,0,'own Host exits normally');}};
}
const ok=reply=>{assert.equal(reply.outcome.status,'ok',JSON.stringify(reply));return reply.outcome.data;};
const error=reply=>{assert.equal(reply.outcome.status,'error',JSON.stringify(reply));return reply.outcome.data;};
const business=s=>({project:s.project,scenarios:s.scenarios});
const clone=v=>structuredClone(v);
const scenario=s=>s.scenarios.find(s=>s.id==='ORDERS-A');
async function commit(host,base,next){
 const proposal=draft.commitProposal(business(base),business(next),true);
 assert.ok(proposal.commands.length);
 return ok(await host.send({op:'call',operation:'apply_commands',params:{base_revision:proposal.base_project_revision,commands:proposal.commands}}));
}
async function objects(host,run,at){
 const value=ok(await host.send({op:'call',operation:'get_run',params:{run_id:run.id,query:{kind:'objects_at',data:{at_ms:at,objects:[],page:{after_index:null,limit:100}}}}}));
 assert.equal(value.kind,'objects');assert.equal(value.data.next_index,null);return value.data;
}
const at=run=>run.orders.map(o=>({id:o.order_id,accepted:o.accepted.value.coefficient,completion_ms:o.completion_ms,tardiness_ms:o.tardiness_ms}));

test('all three P0-02 steps use production form proposals and the actual Host/Engine, then reopen in a fresh OS process',async()=>{
 let host=await bridge(['--fixture','packages/commands/tests/fixtures/production/p104-A-first.json']);
 let reopened;
 try{
  const initial=await host.send({op:'snapshot'});
  for(const [collection,type]of Object.entries({products:'ProductRevision',boms:'BomDefinition',routes:'RouteDefinition',devices:'DeviceInstance'}))for(const value of initial.project[collection])assert.deepEqual(schema.validateForm(schema.fieldSchema(type,catalog),value,catalog),[]);
  const next=clone(initial),device=next.project.devices.find(d=>d.route_operation_refs.length);
  const type=schema.definitionName(catalog.$defs.DeviceInstance.properties.route_operation_refs.items);
  const copied=draft.copyRecord(device.route_operation_refs[0],type);device.route_operation_refs.push(copied);
  assert.deepEqual(copied,device.route_operation_refs[0]);
  await commit(host,initial,next);
  const withReference=await host.send({op:'snapshot'});
  assert.deepEqual(withReference.project.devices.find(d=>d.id===device.id).route_operation_refs,device.route_operation_refs);
  const first=await host.send({op:'run',scenario:'ORDERS-A'});
  assert.equal(first.evidence_origin,'actual_software');assert.equal(first.whole_window_computed,true);
  assert.equal(first.orders.find(o=>o.order_id==='B').completion_ms,205000);
  const wip=await objects(host,first,30000);
  assert.ok(wip.objects.some(o=>o.kind==='item'&&o.data.lifecycle==='in_process'&&o.data.holder.kind==='job'));
  const beforeArrival=await objects(host,first,99999),afterArrival=await objects(host,first,100000);
  for(const id of ['pcb3','pcb4']){assert.ok(!beforeArrival.objects.some(o=>o.kind==='item'&&o.data.id===id));assert.ok(afterArrival.objects.some(o=>o.kind==='item'&&o.data.id===id));}
  records.push({step:1,reference_copy:copied,first_run:first.id,first_orders:at(first),wip_at_ms:30000,future_arrival_boundary:[99999,100000]});

  const base=await host.send({op:'snapshot'}),mine=clone(base),current=clone(base);
  scenario(mine).production_tasks.reverse();
  for(const task of scenario(mine).production_tasks)task.priority=task.id==='TASK-B'?1:0;
  scenario(current).production_tasks.find(t=>t.id==='TASK-A').release_at_ms=1;
  scenario(current).title='Explicit concurrent title';await commit(host,base,current);
  const latest=await host.send({op:'snapshot'}),stale=draft.commitProposal(business(base),business(mine),true);
  const staleError=error(await host.send({op:'call',operation:'apply_commands',params:{base_revision:stale.base_project_revision,commands:stale.commands}}));
  assert.equal(staleError.code,'RevisionConflict');assert.deepEqual(await host.send({op:'snapshot'}),latest);
  const merged=draft.mergeDraft(business(base),business(mine),business(latest));assert.deepEqual(merged.conflicts,[]);
  assert.deepEqual(scenario(merged.value).production_tasks.map(t=>t.id),['TASK-B','TASK-A']);
  assert.equal(scenario(merged.value).production_tasks.find(t=>t.id==='TASK-A').release_at_ms,1);
  await commit(host,latest,merged.value);
  const second=await host.send({op:'run',scenario:'ORDERS-A'});
  assert.equal(second.orders.find(o=>o.order_id==='A').completion_ms,205000);
  assert.equal(second.orders.find(o=>o.order_id==='A').tardiness_ms,45000);
  assert.notDeepEqual(at(second),at(first));
  const fieldBase=await host.send({op:'snapshot'}),local=clone(fieldBase),remote=clone(fieldBase);
  scenario(local).orders[0].due_at_ms=210000;scenario(remote).orders[0].due_at_ms=220000;
  const conflict=draft.mergeDraft(business(fieldBase),business(local),business(remote));assert.equal(conflict.conflicts.length,1);
  const chosen=draft.mergeDraft(business(fieldBase),business(local),business(remote),{[conflict.conflicts[0].path]:'mine'});
  assert.deepEqual(chosen.conflicts,[]);assert.equal(scenario(chosen.value).orders[0].due_at_ms,210000);
  const atomic=clone(fieldBase);atomic.project.title='must not escape failed batch';scenario(atomic).title='stale scenario metadata';
  const proposal=draft.commitProposal(business(fieldBase),business(atomic),true);
  proposal.commands.find(c=>c.kind==='put_object'&&c.data.kind==='scenario').data.data.revision=999;
  const atomicError=error(await host.send({op:'call',operation:'apply_commands',params:{base_revision:proposal.base_project_revision,commands:proposal.commands}}));
  assert.equal(atomicError.code,'InvalidInput');assert.deepEqual(await host.send({op:'snapshot'}),fieldBase);
  records.push({step:2,stale_error:staleError,atomic_error:atomicError,field_conflict:conflict.conflicts,second_run:second.id,second_orders:at(second)});

  const doubleDraft=clone(fieldBase),double=clone(scenario(fieldBase));double.id='DOUBLE-HARD';double.revision=1;
  double.initial_conditions.material_allocations=['TASK-A','TASK-B'].map((demand_ref,i)=>({id:`DOUBLE-${i}`,demand_ref,item_refs:['shell1'],quantity:{value:{coefficient:1,decimal_places:0},unit:'piece'},mode:'hard_reserved'}));
  doubleDraft.scenarios.push(double);await commit(host,fieldBase,doubleDraft);
  const blocked=await host.send({op:'snapshot'});
  assert.ok(blocked.issues.some(i=>i.code==='business_inputs.material_hard_overallocated'&&i.blocks.includes('execution')));
  const runError=error(await host.send({op:'call',operation:'run_scenario',params:{scenario_id:double.id,input_revision:blocked.project.revision,observation_window:double.window,intent:{kind:'project_scenario'}}}));
  assert.ok(['InvalidInput','MissingReference'].includes(runError.code));assert.deepEqual(await host.send({op:'snapshot'}),blocked);
  const remove=clone(blocked);remove.scenarios=remove.scenarios.filter(s=>s.id!==double.id);await commit(host,blocked,remove);

  const allocationBase=await host.send({op:'snapshot'}),allocated=clone(allocationBase);
  scenario(allocated).initial_conditions.material_allocations=['A','B'].map((letter,i)=>({id:`RESERVE-${letter}`,demand_ref:`TASK-${letter}`,item_refs:[`shell${i+1}`],quantity:{value:{coefficient:1,decimal_places:0},unit:'piece'},mode:'hard_reserved'}));
  await commit(host,allocationBase,allocated);
  const beforeFork=await host.send({op:'snapshot'}),fork=await host.send({op:'fork',scenario:'ORDERS-A',new_id:'CANCEL-A-NEW-PLAN',orders:['A'],tasks:[]});
  assert.ok(fork.candidate,JSON.stringify(fork));assert.deepEqual(await host.send({op:'snapshot'}),beforeFork);
  assert.deepEqual(fork.candidate.orders.map(o=>o.id),['B']);assert.deepEqual(fork.candidate.production_tasks.map(t=>t.id),['TASK-B']);
  assert.deepEqual(fork.candidate.initial_conditions.items,scenario(beforeFork).initial_conditions.items);
  assert.deepEqual(fork.candidate.initial_conditions.material_allocations.map(a=>a.id),['RESERVE-B']);
  const withFork=clone(beforeFork);withFork.scenarios.push(fork.candidate);await commit(host,beforeFork,withFork);
  const cancelled=await host.send({op:'run',scenario:fork.candidate.id});
  assert.deepEqual(cancelled.orders.map(o=>o.order_id),['B']);assert.ok(cancelled.acceptance_events.every(e=>e.order_id==='B'));
  assert.ok(cancelled.jobs.length);assert.ok(cancelled.jobs.every(j=>j.task_ref!=='TASK-A'));
  assert.ok(cancelled.jobs.filter(j=>j.execution_ref.kind==='project_device').every(j=>j.task_ref==='TASK-B'));
  for(const previous of [first,second])assert.deepEqual(await host.send({op:'full',run:previous.id}),previous);
  assert.deepEqual(await objects(host,first,30000),wip);
  const final=business(await host.send({op:'snapshot'}));
  const saved=path.join(runDirectory,'saved'),moved=path.join(runDirectory,'moved');await mkdir(saved);
  const save=await host.send({op:'save',directory:saved});assert.equal(save.task.state,'completed');
  const originalPid=host.pid;await host.close();host=null;
  for(const target of [saved,moved])assert.ok(path.resolve(target).startsWith(path.resolve(runDirectory)+path.sep));
  await rename(saved,moved);await assert.rejects(stat(saved),{code:'ENOENT'});
  reopened=await bridge(['--open',moved]);assert.notEqual(reopened.pid,originalPid);
  assert.deepEqual(business(await reopened.send({op:'snapshot'})),final);
  for(const previous of [first,second,cancelled])assert.deepEqual(await reopened.send({op:'full',run:previous.id}),previous);
  assert.deepEqual(await objects(reopened,first,30000),wip);
  const rerun=await reopened.send({op:'run',scenario:fork.candidate.id});assert.deepEqual(at(rerun),at(cancelled));
  assert.ok(rerun.jobs.every(j=>j.task_ref!=='TASK-A'));
  assert.ok(rerun.jobs.filter(j=>j.execution_ref.kind==='project_device').every(j=>j.task_ref==='TASK-B'));
  const finalDirectory=path.join(runDirectory,'final-four-runs');await mkdir(finalDirectory);
  const finalSave=await reopened.send({op:'save',directory:finalDirectory});assert.equal(finalSave.task.state,'completed');
  const receiverPid=reopened.pid;await reopened.close();reopened=null;
  reopened=await bridge(['--open',finalDirectory]);assert.notEqual(reopened.pid,receiverPid);
  assert.deepEqual(business(await reopened.send({op:'snapshot'})),final);
  for(const previous of [first,second,cancelled,rerun])assert.deepEqual(await reopened.send({op:'full',run:previous.id}),previous);
  assert.deepEqual(await objects(reopened,first,30000),wip);
  await writeFile(path.join(directory,'native-project-record.json'),JSON.stringify({project_directory:finalDirectory,project:final.project,scenarios:final.scenarios,runs:[first,second,cancelled,rerun],old_wip_at_30000:wip,original_pid:originalPid,receiver_pid:receiverPid,final_readback_pid:reopened.pid},null,2));
  records.push({step:3,cancelled_run:cancelled.id,cancelled_orders:at(cancelled),released_reservation:'RESERVE-A',retained_reservation:'RESERVE-B',old_wip_retained:true,old_runs_retained:[first.id,second.id],saved,moved,actual_directory_move:true,final_saved_project:finalDirectory,original_pid:originalPid,reopened_pid:receiverPid,final_readback_pid:reopened.pid,new_process_rerun:rerun.id,double_hard_run_error:runError});
  await writeFile(path.join(directory,'combined-host-result.json'),JSON.stringify({task_id:'P0-02',status:'passed',evidence_level:'production TS functions -> real public Host -> unique Engine -> archive -> fresh OS Host',records,run_directory:runDirectory},null,2));
  await writeFile(path.join(directory,'fresh-business-record.json'),JSON.stringify({state:{snapshot:business(initial)},run:first},null,2));
 }finally{if(host)await host.close();if(reopened)await reopened.close();}
});
