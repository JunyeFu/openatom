import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdir,rename,readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {root,out,event} from './component.mjs';
import {bridge,read,clone,plain,ok,form,tab,fill,select,apply,runSaved,artifact} from './session.mjs';
const business=value=>({project:value.project,scenarios:value.scenarios});
const scenario=session=>session.state.scenarios.find(s=>s.id==='ORDERS-A');
const outcome=run=>Object.fromEntries(run.orders.map(o=>[o.order_id,[o.completion_ms,o.tardiness_ms]]));
const unchanged=async(host,runs)=>{for(const run of runs)assert.deepEqual(await host.full(run),run);};
async function externalEdit(session,change){const state=await session.host.snapshot();const changed=clone(state.scenarios.find(s=>s.id===session.state.scenarios[0].id));change(changed);
 const commit=ok(await session.host.call('apply_commands',{base_revision:state.project.revision,commands:[{kind:'put_object',data:{kind:'scenario',data:changed}}]}));return {commit,state:await session.host.snapshot()};}

test('actual form and timeline handlers share one Scenario, keep both order Runs, and close history/conflict/save/project-switch branches',async()=>{
 let host=await bridge(['--fixture','packages/commands/tests/fixtures/production/p104-A-first.json']),reopened,other,session;
 const checkpoints=[];
 try{
  const fixture=await read('packages/commands/tests/fixtures/production/p104-A-first.json');session=await form(host,fixture);const view=session.view;
  const first=clone(await runSaved(session)),original=business(await host.snapshot());assert.deepEqual(outcome(first),{A:[145000,0],B:[205000,5000]});
  assert.equal(first.evidence_origin,'actual_software');assert.equal(first.whole_window_computed,true);assert.equal(session.runRecords.length,1);
  await tab(view,'产品与版本');await tab(view,'复制记录');
  await tab(view,'BOM用料');await tab(view,'复制记录');await fill(view,'版本','p205-r2');await fill(view,'数值','2');
  await tab(view,'工艺路线');await tab(view,'复制记录');await fill(view,'版本','p205-r2');await apply(view);
  const copiedBom=session.state.project.boms.find(b=>!original.project.boms.some(old=>old.id===b.id)),copiedRoute=session.state.project.routes.find(r=>!original.project.routes.some(old=>old.id===r.id));
  assert.equal(session.state.project.products.length,original.project.products.length+1);assert.equal(copiedBom.revision,'p205-r2');assert.equal(copiedBom.rows[0].quantity.value.coefficient,2);assert.equal(copiedRoute.revision,'p205-r2');assert.equal(copiedRoute.steps[1].predecessors[0],copiedRoute.steps[0].id);
  for(const collection of ['products','boms','routes'])for(const old of original.project[collection])assert.deepEqual(session.state.project[collection].find(value=>value.id===old.id),old);
  assert.equal(session.runRecords.length,1);await unchanged(host,[first]);checkpoints.push({step:1,canonical_copies:['product','bom','route'],new_BOM_revision:copiedBom.revision,new_BOM_first_quantity:2,route_predecessors_remapped:true,original_records_and_Run_unchanged:true});
  await tab(view,'生产任务与分配');await select(view,'业务记录','TASK-A');await fill(view,'优先序（数值越大越优先）','0');await select(view,'业务记录','TASK-B');await fill(view,'优先序（数值越大越优先）','1');await apply(view);
  assert.equal(session.runRecords.length,1);const priority=clone(await runSaved(session));assert.deepEqual(outcome(priority),{A:[205000,45000],B:[145000,0]});
  const beforeTimeline=clone(scenario(session)),timeline=session.mountTimeline();await timeline.flush();await tab(timeline,'输入');
  const marker=timeline.find('选择输入标记');await timeline.act(()=>marker.props.onChange(event(marker,{target:{value:'input:ORDERS-A:order_due:A'}})));
  await fill(timeline,'输入精确毫秒','180000');await tab(timeline,'预览精确时点');await tab(timeline,'确认应用一次');await view.flush();
  assert.equal(scenario(session).orders.find(o=>o.id==='A').due_at_ms,180000);await tab(view,'任务与订单');await select(view,'业务记录','A');assert.equal(view.find('交期（ms，可未知）').props.value,'180000');
  assert.equal(session.runRecords.length,2);assert.equal(scenario(session).arrivals[0].at_ms,beforeTimeline.arrivals[0].at_ms);
  await tab(timeline,'编辑班次');await view.flush();assert.ok(view.text().includes('班次与日历'));
  await timeline.act(()=>timeline.render({comparison:{project:original.project,scenario:original.scenarios[0],run:first}}));await tab(timeline,'对比');assert.ok(timeline.text().includes('A '+priority.id+' / B '+first.id));
  await tab(view,'实际接受与等待');const wait=priority.waits.find(w=>w.reasons.some(r=>r.blocking_object));assert.ok(wait);
  const button=view.nodes().find(n=>n.type==='button'&&view.textOf(n).startsWith(wait.object_ref.id+' · '));assert.ok(button);await view.act(()=>button.props.onClick());
  const located=session.locations.at(-1);assert.equal(located.source,'run');assert.equal(located.run_id,priority.id);assert.equal(located.at_ms,wait.interval.start_ms);
  const page=ok(await host.call('get_run',{run_id:located.run_id,query:{kind:'objects_at',data:{at_ms:located.at_ms,objects:[],page:{after_index:null,limit:1000}}}})).data;
  assert.equal(page.next_index,null);await view.act(()=>view.render({observation:page,atMs:page.at_ms}));assert.ok(view.text().includes(`${page.objects.length}条真实身份记录`));
  await unchanged(host,[first,priority]);checkpoints.push({step:3,orders_before:outcome(first),orders_after:outcome(priority),actual_wait_location:located,complete_observation_count:page.objects.length});

  await tab(view,'撤销草稿');assert.equal(scenario(session).orders[0].due_at_ms,160000);const undo=session.state.project.revision;
  await tab(view,'重做草稿');assert.equal(scenario(session).orders[0].due_at_ms,180000);assert.equal(session.state.project.revision,undo+1);
  await tab(view,'撤销草稿');await tab(view,'任务与订单');await select(view,'业务记录','A');await fill(view,'交期（ms，可未知）','181000');await apply(view);
  assert.equal(session.history.can_redo,false);assert.equal(view.button('重做草稿').props.disabled,true);
  await fill(view,'交期（ms，可未知）','182000');const staleBase=session.state.project.revision;
  const concurrent=await externalEdit(session,s=>{s.title='Explicit remote title';});await session.refresh();assert.equal(view.find('交期（ms，可未知）').props.value,'182000');
  await tab(view,'检查变更');assert.ok(view.text().includes(`工程 r${staleBase}`));await tab(view,'确认应用草稿');assert.ok(view.text().includes('RevisionConflict'));assert.equal(view.find('交期（ms，可未知）').props.value,'182000');
  assert.deepEqual(business(await host.snapshot()),business(concurrent.state));await tab(view,'读取当前工程');await tab(view,'基于当前输入重新预览');assert.ok(view.find('修订冲突字段核对'));await tab(view,'接受核对并重新检查');await apply(view);
  assert.equal(scenario(session).title,'Explicit remote title');assert.equal(scenario(session).orders[0].due_at_ms,182000);
  await fill(view,'交期（ms，可未知）','183000');await externalEdit(session,s=>{s.orders[0].due_at_ms=184000;});await session.refresh();await tab(view,'基于当前输入重新预览');
  assert.equal(view.button('接受核对并重新检查').props.disabled,true);await tab(view,'采用我的此字段');await tab(view,'接受核对并重新检查');await apply(view);assert.equal(scenario(session).orders[0].due_at_ms,183000);
  const textUndo=event(view.find('交期（ms，可未知）'),{key:'z',ctrlKey:true}),beforeText=session.proposals.length;await view.act(()=>view.find('业务输入').props.onKeyDown(textUndo));assert.equal(textUndo.defaultPrevented,false);assert.equal(session.proposals.length,beforeText);
  await fill(view,'交期（ms，可未知）','184001');await tab(view,'关闭业务面板');assert.ok(view.find('保留未应用表单'));await tab(view,'返回保留表单');assert.equal(view.find('交期（ms，可未知）').props.value,'184001');assert.ok(session.dirty.at(-1).dirty);
  await tab(view,'检查变更');const release=session.holdNextCommit(),confirm=view.button('确认应用草稿'),beforePending=session.proposals.length;
  confirm.props.onClick();view.render();confirm.props.onClick();view.render();assert.equal(session.proposals.length,beforePending+1);assert.equal(view.button('关闭业务面板').props.disabled,true);assert.equal(view.nodes().find(n=>n.type==='fieldset'&&n.props.className==='sf-business-body').props.disabled,true);
  release();await view.flush();assert.equal(scenario(session).orders[0].due_at_ms,184001);assert.equal(session.runRecords.length,2);await unchanged(host,[first,priority]);
  checkpoints.push({step:4,stale_base:staleBase,conflict_retained:true,explicit_field_choice:true,undo_creates_new_revision:undo,redo_branch_cleared:true,text_undo_not_dispatched:true,pending_duplicate_submissions:1,close_preserved_unsaved:true});

  const final=business(await host.snapshot()),saved=new URL(`runs/${Date.now()}-${process.pid}/saved/`,out),moved=new URL('../moved/',saved);await mkdir(saved,{recursive:true});
  const save=await host.send({op:'save',directory:fileURLToPath(saved)});assert.equal(save.task.state,'completed');await rename(saved,moved);await assert.rejects(readFile(new URL('project.json',saved)),{code:'ENOENT'});const oldPid=host.pid;await host.close();host=null;
  reopened=await bridge(['--open',fileURLToPath(moved)]);assert.notEqual(reopened.pid,oldPid);assert.deepEqual(business(await reopened.snapshot()),final);await unchanged(reopened,[first,priority]);
  const newForm=await form(reopened,{...fixture,scenario:final.scenarios[0]});assert.equal(newForm.view.find('交期（ms，可未知）').props.value,'184001');assert.equal(newForm.view.button('撤销草稿').props.disabled,true);newForm.unmount();
  const otherFixture=await read('.work/p205-closeout/delivery/inputs/other-project.explicit.json');other=await bridge(['--fixture','.work/p205-closeout/delivery/inputs/other-project.explicit.json']);
  await session.attach(other,otherFixture);assert.equal(session.state.project.id,'P205-OTHER-PROJECT');assert.equal(view.button('撤销草稿').props.disabled,true);assert.ok(!view.text().includes('184001'));await tab(view,'任务与订单');assert.equal(view.find('交期（ms，可未知）').props.value,'160000');
  assert.ok(session.dirty.some(v=>v.project_id==='P1'&&!v.dirty));assert.ok(session.dirty.some(v=>v.project_id==='P205-OTHER-PROJECT'&&!v.dirty));
  checkpoints.push({step:4,saved:fileURLToPath(saved),moved:fileURLToPath(moved),original_pid:oldPid,reopened_pid:reopened.pid,project_switch_real_project:session.state.project.id,old_Runs_exact_after_reopen:true});
  await artifact('workflow-p104.json',{status:'passed',original_steps:[1,2,3,4],checkpoints,commits:session.commits,proposals:session.proposals,run_requests:session.runRequests,old_and_new_runs:session.runRecords.map(r=>r.id),shared_Scenario:true,ordinary_Native_acceptance:false});
  await artifact('p104-baseline.run.json',first);await artifact('p104-priority.run.json',priority);
 }finally{session?.unmount();if(host)await host.close();if(reopened)await reopened.close();if(other)await other.close();}
});

test('actual quality form edits preserve the original pending/scrap Run and consume only explicit new quality inputs',async()=>{
 const host=await bridge(['--fixture','packages/commands/tests/fixtures/quality/quality-original.json']);let session;
 try{
  session=await form(host,await read('packages/commands/tests/fixtures/quality/quality-original.json'));const view=session.view;
  const original=clone(await runSaved(session));assert.deepEqual(original.acceptance_events.map(e=>e.at_ms),[40000,100000]);assert.equal(original.goal_met,false);assert.equal(original.final_items.filter(i=>i.lifecycle==='scrapped').length,1);
  await tab(view,'声明质量决定');await select(view,'业务记录','manual-release-3');await fill(view,'决定生效（ms）','110000');await apply(view);assert.equal(session.runRecords.length,1);
  const delayed=clone(await runSaved(session));assert.deepEqual(delayed.acceptance_events.map(e=>e.at_ms),[40000,120000]);
  await tab(view,'质量结果预设');await select(view,'业务记录','QC-3');
  const label=view.nodes().find(n=>n.type==='label'&&view.textOf(n)==='填写按尝试顺序的预设结果第1行');assert.ok(label);
  const checkbox=label.props.children.flat(Infinity).find(n=>n?.type==='input');await view.act(()=>checkbox.props.onChange(event(checkbox,{target:{checked:true}})));
  await select(view,'按尝试顺序的预设结果第1行类型','文字');await fill(view,'数值','PASS');await apply(view);
  const explicit=clone(await runSaved(session));assert.deepEqual(explicit.acceptance_events.map(e=>e.at_ms),[40000,80000]);assert.equal(explicit.goal_met,null);assert.ok(explicit.limitations.some(i=>i.code==='simulation.quality_decision_unapplied'));
  await tab(view,'声明质量决定');await select(view,'业务记录','manual-release-3');await tab(view,'移除记录');await apply(view);
  const final=clone(await runSaved(session));assert.deepEqual(final.acceptance_events.map(e=>e.at_ms),[40000,80000]);assert.equal(final.goal_met,false);assert.equal(final.final_items.filter(i=>i.lifecycle==='scrapped').length,1);
  await unchanged(host,[original,delayed,explicit]);assert.ok(session.state.project.acknowledgments.length===0&&session.state.project.bindings.length===0);
  await artifact('workflow-quality.json',{status:'passed',ordinary_Native_acceptance:false,acceptance_ms:session.runRecords.map(r=>r.acceptance_events.map(e=>e.at_ms)),commits:session.commits,proposals:session.proposals,old_Runs_unchanged:true,no_professional_acknowledgment_or_binding:true,original_scrap_retained:true});
  for(const[name,value]of Object.entries({original,delayed,explicit,final}))await artifact(`quality-${name}.run.json`,value);
 }finally{session?.unmount();await host.close();}
});

test('explicit new declared-phase candidate uses real parameter and calendar form handlers, without mutating immutable definitions',async()=>{
 const fixture=await read('.work/p205-closeout/delivery/inputs/declared-phase.explicit.json'),frozenDefinitions=clone(fixture.definitions);
 const host=await bridge(['--fixture','.work/p205-closeout/delivery/inputs/declared-phase.explicit.json']);let session;
 try{
  session=await form(host,fixture);const view=session.view,original=clone(await runSaved(session));assert.equal(original.jobs[0].finished_at_ms,45000);
  await tab(view,'设备阶段与参数');await fill(view,'数值','10000');await select(view,'单位','毫秒');await select(view,'来源性质','人工/企业声明');await apply(view);assert.equal(session.proposals.at(-1).commands.length,1);assert.equal(session.proposals.at(-1).commands[0].kind,'set_declared_parameter');assert.equal(session.runRecords.length,1);
  const parameter=clone(await runSaved(session));assert.equal(parameter.jobs[0].finished_at_ms,50000);
  await tab(view,'班次与日历');await select(view,'业务记录','declared non-continuous worker calendar');await select(view,'选择可用区间行',text=>text.startsWith('2.'));await fill(view,'开始（ms，含）','45000');await apply(view);
  const calendar=clone(await runSaved(session));assert.equal(calendar.jobs[0].finished_at_ms,55000);await unchanged(host,[original,parameter]);
  assert.deepEqual(fixture.definitions,frozenDefinitions);const wait=calendar.waits.find(w=>w.phase_ref==='unload');assert.ok(wait);assert.deepEqual(wait.interval,{start_ms:35000,end_ms:45000});assert.ok(wait.reasons.some(r=>r.kind==='calendar'));
  await artifact('workflow-phase.json',{status:'passed',explicit_new_candidate:true,original_fixture_unchanged:true,immutable_definitions_unchanged:true,finish_ms:[45000,50000,55000],old_Runs_unchanged:true,wait,commits:session.commits,proposals:session.proposals,ordinary_Native_acceptance:false});
  await artifact('phase-original.run.json',original);await artifact('phase-parameter.run.json',parameter);await artifact('phase-calendar.run.json',calendar);
 }finally{session?.unmount();await host.close();}
});

test('actual arrival, changeover and supply handlers preserve future material boundaries and explicit accepted targets',async()=>{
 const fixture=await read('packages/commands/tests/fixtures/production/p104-A-first.json'),host=await bridge(['--fixture','packages/commands/tests/fixtures/production/p104-A-first.json']);let session;
 try{
  session=await form(host,fixture);const view=session.view,original=clone(await runSaved(session));assert.deepEqual(outcome(original),{A:[145000,0],B:[205000,5000]});
  await tab(view,'后续到料');await fill(view,'预计到达（ms）','120000');await apply(view);assert.equal(session.runRecords.length,1);
  const timeline=session.mountTimeline();await timeline.flush();await tab(timeline,'输入');const marker=timeline.find('选择输入标记').props.children.flat(Infinity).find(n=>n?.props?.value==='input:ORDERS-A:arrival:PCB-100');assert.ok(marker);assert.ok(timeline.textOf(marker).includes('120秒'));
  const delayed=clone(await runSaved(session));assert.equal(delayed.orders.find(o=>o.order_id==='A').completion_ms,145000);assert.equal(delayed.orders.find(o=>o.order_id==='B').completion_ms,null);
  const readAt=async at=>ok(await host.call('get_run',{run_id:delayed.id,query:{kind:'objects_at',data:{at_ms:at,objects:[],page:{after_index:null,limit:1000}}}})).data;
  const before=await readAt(119999),after=await readAt(120000);assert.equal(before.next_index,null);assert.equal(after.next_index,null);
  for(const id of ['pcb3','pcb4']){assert.ok(!before.objects.some(o=>o.kind==='item'&&o.data.id===id));assert.ok(after.objects.some(o=>o.kind==='item'&&o.data.id===id));}
  const pcbIds=new Set(fixture.scenario.arrivals[0].items.map(i=>i.id)),assembly=delayed.jobs.filter(j=>j.task_ref==='TASK-B'&&j.execution_ref.kind==='project_device'&&j.execution_ref.data.route_step_id==='step-assembly'&&j.input_refs.some(id=>pcbIds.has(id)));assert.equal(assembly.length,2);assert.ok(assembly.every(j=>j.started_at_ms>=120000));
  await tab(view,'后续到料');await fill(view,'预计到达（ms）','100000');await tab(view,'家族与换单');await fill(view,'数值','20000');await tab(view,'供应与运输输入');await fill(view,'数值','2');await apply(view);
  assert.equal(session.state.project.setup_policies[0].changeovers[0].duration.value,20000);assert.equal(scenario(session).supplies[0].target_quantity.value.coefficient,2);
  const supplied=clone(await runSaved(session));assert.equal(supplied.orders.find(o=>o.order_id==='A').completion_ms,145000);assert.equal(supplied.orders.find(o=>o.order_id==='B').completion_ms,null);assert.equal(supplied.orders.find(o=>o.order_id==='B').outstanding.value.coefficient,2);
  assert.equal(supplied.acceptance_events.reduce((sum,e)=>sum+e.quantity.value.coefficient,0),2);
  assert.equal(supplied.final_items.filter(i=>i.item_type_ref.id==='package'&&i.lifecycle==='available'&&i.holder.kind==='location'&&i.holder.id==='dispatch-store').length,2);
  assert.deepEqual(scenario(session).cost_inputs,fixture.scenario.cost_inputs);await unchanged(host,[original,delayed]);
  await artifact('workflow-arrival-setup-supply.json',{status:'passed',explicit_variants:{late_arrival_ms:120000,restored_arrival_ms:100000,changeover_A_B_ms:20000,supply_target_pieces:2},original_inputs_unchanged:true,old_Runs_unchanged:true,future_material_boundaries_ms:[119999,120000],B_assembly_start_ms:assembly.map(j=>j.started_at_ms),accepted_pieces:2,physical_dispatch_remainder:2,cost_missing_did_not_block_run:true,commits:session.commits,proposals:session.proposals,ordinary_Native_acceptance:false});
  await artifact('arrival-original.run.json',original);await artifact('arrival-delayed.run.json',delayed);await artifact('supply-target.run.json',supplied);await artifact('arrival-before.objects.json',before);await artifact('arrival-at.objects.json',after);
 }finally{session?.unmount();await host.close();}
});
