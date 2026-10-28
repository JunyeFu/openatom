// Full original W5 actual desktop chain after W5-P205-01 repair.
// Requires the coordinator-owned Native and a GUI-created, empty W5 Project.
// Never creates another Host, invokes a picker, alters product source, or closes Native.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const require=createRequire(import.meta.url),{chromium}=require(process.argv[2]);
const endpoint=process.argv[3],expectedHost=process.argv[4];
assert.ok(endpoint&&expectedHost);
const root=new URL('../../../',import.meta.url);
const out=new URL('.work/whole-packages/p001-20261017/native/business/',root);
await mkdir(out,{recursive:true});
const browser=await chromium.connectOverCDP(endpoint);
const page=browser.contexts().flatMap(c=>c.pages()).find(p=>!p.url().startsWith('devtools:'));
page.setDefaultTimeout(7000);
const errors=[],groups=[];page.on('pageerror',e=>errors.push(String(e)));
const identity=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('agent_connection_info'));
assert.equal(identity.host_instance_id,expectedHost);
async function api(operation,params,projectId=null) {
 return page.evaluate(async({operation,params,projectId})=>{
  const response=await window.__TAURI_INTERNALS__.invoke('project_api',{request:{
   api_version:'0.1',request_id:'w5-repaired-desktop-'+crypto.randomUUID(),
   project_id:projectId,scope:{kind:projectId?'project':'workspace'},operation,params
  }});
  if(response.outcome.status!=='ok')throw new Error(JSON.stringify(response));
  return response.outcome.data;
 },{operation,params,projectId});
}
const context=()=>api('get_context',{selection:[]});
const initial=await context(),pid=initial.project.id;
const object=async(kind,id,source={kind:'draft'})=>(await api('get_object',{object_id:{kind,id},source,node_index:null},pid)).data;
const project=()=>object('project',pid),scenarioId='W5-STRUCTURED-BASE',cloneId='W5-CLONED-FROZEN';
const input=(id=scenarioId)=>object('scenario',id);
const full=async id=>(await api('get_run',{run_id:id,query:{kind:'full_result'}},pid)).data;
const save=(name,value)=>writeFile(new URL(name,out),JSON.stringify(value,null,2));
const snapshot=async name=>page.screenshot({path:fileURLToPath(new URL(name,out))});
const revision=n=>page.waitForFunction(n=>document.querySelector('.title-status')?.textContent==='r'+n,n);
const flush=()=>save('result.json',{identity,initial,groups,errors,whole_task_done:false,human_acceptance:false});
async function check(name,work) {
 try {const details=await work();groups.push({name,passed:true,details});}
 catch(error){groups.push({name,passed:false,failure:String(error)});throw error;}
 finally {await flush();console.log(JSON.stringify(groups.at(-1)));}
}
async function menu(name,button) {
 const d=page.locator('nav details').filter({has:page.locator('summary').getByText(name,{exact:true})});
 if(await d.getAttribute('open')===null)await d.locator('summary').click();
 await d.getByRole('button',{name:button,exact:true}).click();
}
const biz=page.getByRole('region',{name:'业务输入',exact:true});
const sections=biz.getByLabel('业务面板分类');
const section=name=>sections.getByRole('button',{name,exact:true}).click();
const tl=page.getByRole('region',{name:'全厂时间轴',exact:true});
async function commit(n) {
 await biz.getByRole('button',{name:'检查变更',exact:true}).click();
 await biz.getByRole('button',{name:'确认应用草稿',exact:true}).click();await revision(n);
}
const count=async()=>(await context()).run_summaries?.length??0;
const runUI=async previous=>{
 await biz.getByRole('button',{name:'从初始输入重跑',exact:true}).click();
 await page.waitForFunction(previous=>{const e=document.querySelector('[aria-label="历史结果"]');return e?.value&&e.value!==previous;},previous);
 return full(await page.getByLabel('历史结果',{exact:true}).inputValue());
};
let baseline,run1,run2,run3;
try {
 await check('GUI_created_owned_empty_Project_on_previously_nonempty_actual_Host',async()=>{
  assert.equal(initial.project.title,'W5 repaired business independent');assert.equal(initial.project.revision,1);
  const p=await project();assert.equal(p.devices.length,0);assert.equal(p.scenario_refs.length,0);assert.equal(await count(),0);
  await save('original-expectations.json',JSON.parse(await readFile(new URL('.work/whole-packages/window5-timeline-evaluation/native-structured-79676/combined-input-expectations.json',root),'utf8')));
  return {project_id:pid,revision:1,prior_Host_not_empty:true};
 });
 let before;
 await check('actual_nine_categories_four_candidates_and_readonly_template_preview',async()=>{
  before=await project();await page.getByRole('button',{name:'模块与模板库',exact:true}).click();
  const lib=page.getByRole('region',{name:'模块与模板库',exact:true});
  const cats=await lib.locator('select').first().locator('option').evaluateAll(es=>es.map(e=>e.value).filter(Boolean).sort());
  assert.deepEqual(cats,['devices','items','media','operations','professional','resources','tasks','templates','transport']);
  await lib.locator('select').first().selectOption('templates');await page.waitForFunction(()=>document.querySelectorAll('.library-list>button').length===4);
  await lib.locator('.library-list>button').filter({hasText:'community.template-purchased-assembly'}).click();
  assert.deepEqual(await project(),before);return {categories:cats,candidates:4};
 });
 const form=page.locator('.template-preparation');
 await check('explicit_seed_preview_configuration_invalidation_and_one_adoption',async()=>{
  await form.getByLabel('新建对象前缀',{exact:true}).fill('w5-structured');
  for(const s of await form.getByRole('combobox').all())if(await s.locator('option').filter({hasText:'新建该声明示例资源'}).count())await s.selectOption({label:'新建该声明示例资源'});
  for(const name of ['导入模板产品、物料清单与工艺示例','明确创建模板示例方案输入','采用声明的示例供料','采用声明的示例订单','采用模板声明的检验预设（示例输入）'])await form.getByLabel(name,{exact:true}).check();
  await form.getByLabel('新方案标识',{exact:true}).fill(scenarioId);
  for(const s of await form.getByRole('combobox').all())if(await s.locator('option[value="released"]').count())await s.selectOption('released');
  await form.getByRole('button',{name:'预览创建对象',exact:true}).click();await form.locator('.template-preview').waitFor();
  assert.deepEqual(await project(),before);await form.getByLabel('新建对象前缀',{exact:true}).fill('w5-structured-final');
  assert.equal(await form.getByRole('button',{name:'确认采纳模板到工程',exact:true}).isDisabled(),true);
  await form.getByRole('button',{name:'预览创建对象',exact:true}).click();await form.getByRole('button',{name:'确认采纳模板到工程',exact:true}).click();await revision(2);
  baseline={project:await project(),scenario:await input()};assert.equal(baseline.project.devices.length,3);assert.equal(baseline.scenario.revision,1);assert.equal(await count(),0);await save('baseline-input.json',baseline);
  return {project_revision:2,scenario_revision:1,explicit_initial_items:4};
 });
 await check('original_70_110_real_Run_and_goal_preserved',async()=>{
  await page.getByLabel('当前方案',{exact:true}).selectOption(scenarioId);await page.getByRole('button',{name:'运行方案',exact:true}).click();
  await page.waitForFunction(()=>!!document.querySelector('[aria-label="历史结果"]')?.value);
  run1=await full(await page.getByLabel('历史结果',{exact:true}).inputValue());assert.deepEqual(run1.acceptance_events.map(e=>e.at_ms),[70000,110000]);assert.equal(run1.goal_met,true);assert.equal(run1.evidence_origin,'actual_software');await save('run1.json',run1);
  return {run:run1.id,accepted_ms:[70000,110000]};
 });
 await page.getByRole('button',{name:'业务输入',exact:true}).click();await biz.getByLabel('业务记录',{exact:true}).waitFor();
 await check('timeline_navigation_preserves_unsaved_priority_and_exposes_real_declared_parameter',async()=>{
  await biz.getByLabel('优先序（数值越大越优先）',{exact:true}).fill('8');await tl.getByRole('button',{name:'输入',exact:true}).click();
  await page.getByRole('button',{name:'编辑班次',exact:true}).click();await biz.getByRole('heading',{name:'班次与日历',exact:true}).waitFor();
  await section('任务与订单');assert.equal(await biz.getByLabel('优先序（数值越大越优先）',{exact:true}).inputValue(),'8');
  await page.getByRole('button',{name:'编辑阶段工时',exact:true}).click();await biz.getByLabel('参数设备',{exact:true}).waitFor();
  assert.equal(await biz.getByRole('group',{name:'单周期工时 · millisecond',exact:true}).getByLabel('数值',{exact:true}).inputValue(),'40000');
  assert.deepEqual(await project(),baseline.project);return {unsaved_priority:8,declared_cycle_ms:40000};
 });
 await check('original_structured_fields_and_first_23000_matrix_commit_exact_values',async()=>{
  const parameter=biz.getByRole('group',{name:'单周期工时 · millisecond',exact:true});
  await parameter.getByLabel('数值',{exact:true}).fill('41000');await biz.getByLabel('cycle_seconds来源',{exact:true}).selectOption('user_provided');
  await section('生产任务与分配');await biz.getByLabel('优先序（数值越大越优先）',{exact:true}).fill('6');await biz.getByLabel('释放时点（ms）',{exact:true}).fill('1000');
  await section('班次与日历');await biz.getByLabel('结束（ms，不含）',{exact:true}).fill('300001');
  await section('质量结果预设');await biz.getByLabel('选择按尝试顺序的预设结果行',{exact:true}).selectOption('1');await biz.getByRole('checkbox',{name:'填写按尝试顺序的预设结果第2行',exact:true}).uncheck();
  await section('后续到料');await biz.getByRole('button',{name:'新增记录',exact:true}).click();await biz.getByLabel('预计到达（ms）',{exact:true}).fill('5000');await biz.getByLabel('接受库位',{exact:true}).fill('w5-structured-final:store-shell');await biz.getByLabel('来料质量规则',{exact:true}).selectOption('unchecked_allowed');await biz.getByRole('button',{name:'新增物品输入行',exact:true}).click();
  const item=biz.getByRole('group',{name:'物品输入第1行',exact:true});await item.getByLabel('输入持有位置类型',{exact:true}).selectOption({label:'库位'});await item.getByRole('group',{name:'输入持有位置',exact:true}).getByLabel('身份ID',{exact:true}).fill('w5-structured-final:store-shell');await item.getByLabel('身份ID',{exact:true}).nth(1).fill('W5-EXPLICIT-FUTURE-SHELL');
  const type=item.getByRole('group',{name:'物品类型及版本',exact:true});await type.getByLabel('身份ID',{exact:true}).fill('w5-structured-final:shell');await type.getByLabel('版本',{exact:true}).fill('r1');await item.getByLabel('输入质量状态',{exact:true}).selectOption('released');await item.getByLabel('单位',{exact:true}).fill('piece');await item.getByLabel('数值',{exact:true}).fill('1');await item.getByLabel('数据来源',{exact:true}).selectOption('user_provided');
  await section('家族与换单');await biz.getByRole('button',{name:'新增记录',exact:true}).click();await biz.getByLabel('执行设备',{exact:true}).fill('w5-structured-final:Assembly01');await biz.getByRole('checkbox',{name:'填写初始家族',exact:true}).check();await biz.getByLabel('初始家族',{exact:true}).fill('A');await biz.getByRole('button',{name:'新增换单矩阵行',exact:true}).click();
  await biz.getByLabel('来源性质',{exact:true}).selectOption('user_provided');await biz.getByLabel('单位',{exact:true}).selectOption('millisecond');await biz.getByLabel('数值',{exact:true}).fill('23000');await biz.getByRole('checkbox',{name:'填写原家族（可冷启动）',exact:true}).check();await biz.getByLabel('原家族（可冷启动）',{exact:true}).fill('A');await biz.getByLabel('准备方式',{exact:true}).selectOption('internal');await biz.getByLabel('目标家族',{exact:true}).fill('B');
  await biz.getByRole('button',{name:'检查变更',exact:true}).click();assert.match(await biz.innerText(),/4条共同命令/);await biz.getByRole('button',{name:'确认应用草稿',exact:true}).click();await revision(3);
  const p=await project(),s=await input();assert.equal(s.revision,2);assert.equal(s.orders[0].priority,8);assert.equal(s.production_tasks[0].priority,6);assert.equal(s.production_tasks[0].release_at_ms,1000);assert.equal(s.arrivals[0].at_ms,5000);assert.equal(s.arrivals[0].items[0].id,'W5-EXPLICIT-FUTURE-SHELL');assert.deepEqual(s.quality_presets[0].results,[{kind:'string',value:'PASS'},null]);assert.equal(p.calendars[0].intervals[0].end_ms,300001);assert.equal(p.devices[0].configuration.cycle_seconds.value.value,41000);
  const matrix=p.setup_policies[0];assert.equal(matrix.device_id,'w5-structured-final:Assembly01');assert.equal(matrix.changeovers[0].duration.value,23000);assert.equal(matrix.changeovers[0].duration.basis,'user_provided');assert.equal(matrix.changeovers[0].duration.unit,'millisecond');assert.equal(matrix.first_article,null);assert.equal(matrix.changeovers[0].duration.source_ref,null);
  assert.equal(await count(),1);assert.deepEqual(await full(run1.id),run1);await save('structured-committed.json',{project:p,scenario:s});return {project_revision:3,scenario_revision:2,four_commands:true,matrix_ms:23000};
 });
 await check('W5_P205_01_actual_form_advances_r3_clears_dirty_and_enables_history_and_Run_without_recovery',async()=>{
  await page.waitForFunction(()=>document.querySelector('.sf-business')?.textContent.includes('工程 r3 · 表单基于 r3 · 已同步'));
  assert.doesNotMatch(await biz.innerText(),/Host回读未包含请求的换单矩阵/);assert.equal(await biz.getByRole('button',{name:'撤销草稿',exact:true}).isEnabled(),true);assert.equal(await biz.getByRole('button',{name:'从初始输入重跑',exact:true}).isEnabled(),true);await snapshot('fixed-first-matrix-commit-r3.png');
  return {finding:'W5-P205-01',original_reproduction:'passed',form_base_revision:3,dirty:false,recovery_or_discard:false};
 });
 await check('business_undo_and_workbench_redo_share_history_new_revisions_no_old_Run_change',async()=>{
  await biz.getByRole('button',{name:'撤销草稿',exact:true}).click();await revision(4);assert.equal((await input()).orders[0].priority,0);await biz.getByRole('button',{name:'关闭业务面板',exact:true}).click();await menu('编辑','重做');await revision(5);assert.equal((await input()).orders[0].priority,8);assert.equal((await input()).revision,4);assert.equal((await project()).setup_policies[0].changeovers[0].duration.value,23000);assert.deepEqual(await full(run1.id),run1);
  await page.getByRole('button',{name:'业务输入',exact:true}).click();await biz.getByLabel('业务记录',{exact:true}).waitFor();return {revisions:[3,4,5],recovery_required:false};
 });
 await check('new_business_branch_invalidates_shared_redo',async()=>{
  await section('任务与订单');await biz.getByLabel('优先序（数值越大越优先）',{exact:true}).fill('9');await commit(6);await biz.getByRole('button',{name:'关闭业务面板',exact:true}).click();await menu('编辑','撤销');await revision(7);assert.equal((await input()).orders[0].priority,8);await page.getByRole('button',{name:'业务输入',exact:true}).click();await biz.getByLabel('优先序（数值越大越优先）',{exact:true}).fill('10');await commit(8);await biz.getByRole('button',{name:'关闭业务面板',exact:true}).click();
  const d=page.locator('nav details').filter({has:page.locator('summary').getByText('编辑',{exact:true})});if(await d.getAttribute('open')===null)await d.locator('summary').click();assert.equal(await d.getByRole('button',{name:'重做',exact:true}).isDisabled(),true);await d.locator('summary').click();assert.equal((await input()).orders[0].priority,10);await page.getByRole('button',{name:'业务输入',exact:true}).click();return {project_revision:8,scenario_revision:7};
 });
 await check('real_second_Run_original_72000_unknown_quality_and_arrival_boundary',async()=>{
  run2=await runUI(run1.id);assert.deepEqual(run2.acceptance_events.map(e=>e.at_ms),[72000]);assert.equal(run2.goal_met,false);assert.equal(run2.orders[0].completion_ms,null);assert.equal(run2.orders[0].tardiness_ms,null);assert.equal(run2.orders[0].outstanding.value.coefficient,1);assert.deepEqual(await full(run1.id),run1);assert.equal(await count(),2);await save('run2.json',run2);
  for(const at of [4999,5000]) {const observed=await api('get_run',{run_id:run2.id,query:{kind:'objects_at',data:{at_ms:at,objects:[],page:{after_index:null,limit:500}}}},pid);assert.equal(observed.data.next_index,null);assert.equal(observed.data.objects.some(o=>o.kind==='item'&&o.data.id==='W5-EXPLICIT-FUTURE-SHELL'),at===5000);await save('run2-objects-'+at+'.json',observed);}
  return {run:run2.id,acceptance_ms:[72000],completion_ms:null,arrival_boundary:[4999,5000]};
 });
 await check('resource_wait_item_navigation_and_two_canvases_keep_same_actual_Run_time',async()=>{
  await section('实际接受与等待');await biz.getByRole('heading',{name:'实际接受与等待 · 只读',exact:true}).waitFor();await biz.getByRole('button',{name:'w5-structured-final:W-A',exact:true}).first().click();await page.waitForFunction(()=>document.querySelector('[aria-label="观察毫秒"]')?.value==='1000');
  const wait=run2.waits.find(w=>w.reasons.some(r=>r.kind==='quality'));await biz.locator('article').filter({hasText:wait.object_ref.id}).getByRole('button').first().click();await page.waitForFunction(()=>document.querySelector('[aria-label="观察毫秒"]')?.value==='93000');const item=run2.final_items.find(i=>i.quality==='pending');await biz.getByRole('button',{name:item.id,exact:true}).click();await biz.getByRole('button',{name:'关闭业务面板',exact:true}).click();
  for(const canvas of ['多厂房画布','流水线画布']){await page.getByRole('button',{name:canvas,exact:true}).click();assert.equal(await page.getByLabel('历史结果',{exact:true}).inputValue(),run2.id);assert.equal(await page.getByLabel('观察毫秒',{exact:true}).inputValue(),'93000');assert.equal(await tl.getAttribute('data-run-id'),run2.id);await snapshot(canvas==='多厂房画布'?'factory-same-run93.png':'pipeline-same-run93.png');}
  assert.deepEqual(await full(run1.id),run1);assert.deepEqual(await full(run2.id),run2);return {resource_ms:1000,quality_wait_ms:93000,item:item.id,canvases:2};
 });
 await menu('评审与交付','方案比较');
 const delivery=page.getByRole('region',{name:'方案评估与交接工作区',exact:true}),choices=delivery.locator('.delivery-run-choice');
 const rp=delivery.getByRole('region',{name:'评估报告准备',exact:true}),cp=delivery.getByRole('region',{name:'方案比较',exact:true});
 await check('single_achieved_Run_four_format_report_preparation',async()=>{
  await choices.filter({hasText:run1.id}).getByRole('checkbox').check();await delivery.getByRole('button',{name:'评估报告',exact:true}).click();
  await page.waitForFunction(()=>{const b=[...document.querySelectorAll('button')].find(e=>e.textContent==='选择目录并生成报告');return b&&!b.disabled;});assert.match(await rp.innerText(),/W5-STRUCTURED-BASE r1/);for(const f of ['PDF','HTML（离线）','CSV（15分表）','JSON'])assert.equal(await rp.getByRole('checkbox',{name:f,exact:true}).isChecked(),true);return {run:run1.id,picker_triggered:false};
 });
 await check('design_description_sends_no_Run_references',async()=>{
  await rp.getByRole('radio',{name:'尚未运行的设计说明',exact:true}).check();assert.match(await rp.innerText(),/实际Run引用\n不适用/);assert.match(await rp.innerText(),/不发送Run引用/);assert.equal(await rp.getByRole('button',{name:'选择目录并生成报告',exact:true}).isEnabled(),true);return {run_refs:[],picker_triggered:false};
 });
 await check('two_actual_Runs_list_frozen_changes_and_forbid_invented_improvement',async()=>{
  await delivery.getByRole('button',{name:'方案比较',exact:true}).click();await choices.filter({hasText:run2.id}).getByRole('checkbox').check();await cp.getByRole('button',{name:'比较所选两次运行',exact:true}).click();await cp.getByRole('heading',{name:/可比性/}).waitFor();
  const text=await cp.innerText();assert.match(text,/可比性：仅并列，不作改善结论/);assert.match(text,/\/project\/setup_policies/);assert.match(text,/\/scenario\/quality_presets/);assert.match(text,/Demand is unfinished/);assert.match(text,/不可比，不显示改善量/);await writeFile(new URL('run1-run2-comparison.txt',out),text);return {runs:[run1.id,run2.id],status:'side_by_side_only'};
 });
 let clone;
 await check('actual_clone_single_commit_copies_complete_frozen_initial_Scenario_keeps_current_Project',async()=>{
  const before=await project();await cp.getByLabel('新方案ID',{exact:true}).fill(cloneId);await cp.getByLabel('新方案标题',{exact:true}).fill('W5 frozen initial input clone');await cp.getByRole('button',{name:'创建方案草稿',exact:true}).click();await revision(9);await cp.getByRole('status').filter({hasText:'已创建方案'}).waitFor();
  clone=await input(cloneId);const after=await project();assert.deepEqual({...clone,id:baseline.scenario.id,title:baseline.scenario.title,revision:1,project_revision:baseline.scenario.project_revision},baseline.scenario);assert.deepEqual({...after,revision:before.revision,scenario_refs:before.scenario_refs},before);assert.equal(clone.project_revision,9);assert.equal(await count(),2);assert.equal(clone.initial_conditions.items.length,4);assert.equal(run1.final_items.length,16);assert.deepEqual(await full(run1.id),run1);await save('clone-readback.json',{before_project:before,after_project:after,clone});return {revision_delta:1,initial_items:4,source_final_items:16,implicit_Run:false};
 });
 await check('duplicate_clone_id_rejected_atomically',async()=>{
  await cp.getByRole('button',{name:'创建方案草稿',exact:true}).click();await cp.getByRole('alert').filter({hasText:'新方案ID已存在'}).waitFor();assert.equal((await project()).revision,9);assert.deepEqual(await input(cloneId),clone);assert.equal(await count(),2);return {project_revision:9,unchanged:true};
 });
 await check('clone_remains_editable_with_current_Project_and_requires_explicit_Run',async()=>{
  await delivery.getByRole('button',{name:'返回流水线工作台',exact:true}).click();await page.getByLabel('当前方案',{exact:true}).selectOption(cloneId);await page.getByRole('button',{name:'业务输入',exact:true}).click();await biz.getByLabel('业务记录',{exact:true}).waitFor();await section('生产任务与分配');await biz.getByLabel('释放时点（ms）',{exact:true}).fill('2000');await section('方案与观察窗');await biz.getByLabel('结束（ms，不含）',{exact:true}).fill('90000');await commit(10);
  assert.equal((await input(cloneId)).production_tasks[0].release_at_ms,2000);assert.equal((await input()).production_tasks[0].release_at_ms,1000);assert.equal((await project()).devices[0].configuration.cycle_seconds.value.value,41000);assert.equal(await count(),2);return {project_revision:10,clone_revision:2,Run_count_before_explicit_run:2};
 });
 await check('clone_actual_Run_original_73000_and_two_old_Runs_preserved',async()=>{
  run3=await runUI(run2.id);assert.deepEqual(run3.acceptance_events.map(e=>e.at_ms),[73000]);assert.equal(run3.input_ref.scenario_id,cloneId);assert.equal(run3.window.end_ms,90000);assert.equal(run3.orders[0].completion_ms,null);assert.equal(run3.goal_met,false);assert.equal(await count(),3);assert.deepEqual(await full(run1.id),run1);assert.deepEqual(await full(run2.id),run2);await save('run3.json',run3);return {run:run3.id,acceptance_ms:[73000]};
 });
 await biz.getByRole('button',{name:'关闭业务面板',exact:true}).click();await menu('评审与交付','方案比较');
 await check('clone_different_actual_observation_window_comparison_no_improvement_claim',async()=>{
  await choices.filter({hasText:run1.id}).getByRole('checkbox').check();await choices.filter({hasText:run3.id}).getByRole('checkbox').check();await cp.getByRole('button',{name:'比较所选两次运行',exact:true}).click();await cp.getByRole('heading',{name:/可比性/}).waitFor();const text=await cp.innerText();assert.match(text,/Different actual observation windows/);assert.match(text,/可比性：仅并列，不作改善结论/);assert.match(text,/不可比，不显示改善量/);await writeFile(new URL('run1-run3-comparison.txt',out),text);await snapshot('clone-comparison.png');return {windows:[[0,300000],[0,90000]],status:'side_by_side_only'};
 });
 await check('editable_handoff_and_final_single_Run_report_ready_no_picker',async()=>{
  await delivery.getByRole('button',{name:'可编辑交接与贡献',exact:true}).click();const hp=delivery.getByRole('region',{name:'可编辑工程与贡献交接',exact:true});await hp.getByRole('checkbox',{name:new RegExp('^'+run1.id)}).check();assert.match(await hp.innerText(),/报告JSON不具备工程重开能力/);assert.match(await hp.innerText(),/不承诺冷启动续跑/);assert.match(await hp.innerText(),/许可未知|许可：未知/);assert.equal(await hp.getByRole('button',{name:'选择位置并导出ZIP包',exact:true}).isEnabled(),true);
  await choices.filter({hasText:run3.id}).getByRole('checkbox').uncheck();await delivery.getByRole('button',{name:'评估报告',exact:true}).click();await rp.getByRole('radio',{name:'实际运行评估',exact:true}).check();await page.waitForFunction(()=>{const b=[...document.querySelectorAll('button')].find(e=>e.textContent==='选择目录并生成报告');return b&&!b.disabled;});await snapshot('final-report-ready.png');assert.deepEqual(errors,[]);await save('final-context.json',await context());await save('final-inputs.json',{project:await project(),scenarios:[await input(),await input(cloneId)]});return {run:run1.id,picker_triggered:false,Native_left_running:true};
 });
} catch(error) {await snapshot('failure.png');console.error(String(error));}
await flush();
console.log(JSON.stringify({groups:groups.length,passed:groups.filter(g=>g.passed).length,failed:groups.filter(g=>!g.passed).length,project_id:pid}));
// Ending this CDP client leaves coordinator Native running.
process.exit(groups.length===20&&groups.every(g=>g.passed)?0:1);
