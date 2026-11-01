// Attach ONLY to a coordinator-provided isolated actual product WebView2 CDP
// endpoint after its native lease is released. Does not launch an app/service,
// mock Tauri, construct a Host/Engine, substitute a picker or change config.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {writeFile,mkdir,readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const require=createRequire(import.meta.url);
const {chromium}=require(process.argv[2]);
const endpoint=process.argv[3],expectedHost=process.argv[4];
const includeLibrary=process.argv.includes('--library');
const continueLibrary=process.argv.includes('--continue-library');
assert.ok(endpoint&&expectedHost,'Provide actual isolated native CDP endpoint and exact desktop-host-PID');
const originalOut=new URL('.work/whole-packages/window5-planner-acceptance/native-ui/',new URL('../../../',import.meta.url));
const out=continueLibrary?new URL('library-continuation/',originalOut):originalOut;await mkdir(out,{recursive:true});
const groups=[],errors=[];
async function check(name,work){try{await work();groups.push({name,passed:true});}catch(error){groups.push({name,passed:false,failure:String(error)});throw error;}}
const browser=await chromium.connectOverCDP(endpoint);
const page=browser.contexts().flatMap(c=>c.pages()).find(p=>!p.url().startsWith('devtools:'));
assert.ok(page,'Actual product WebView2 page required');
page.on('pageerror',e=>errors.push(String(e)));
async function call(operation,params,projectId=null){
 return page.evaluate(async({operation,params,projectId})=>{
  const request={api_version:'0.1',request_id:'w5-native-ui-'+crypto.randomUUID(),project_id:projectId,scope:{kind:projectId?'project':'workspace'},operation,params};
  const response=await window.__TAURI_INTERNALS__.invoke('project_api',{request});
  if(response.outcome.status!=='ok')throw new Error(JSON.stringify(response));return response.outcome.data;
 },{operation,params,projectId});
}
const menu=async(name,button)=>{await page.getByText(name,{exact:true}).click();await page.locator('nav details[open]').getByRole('button',{name:button,exact:true}).click();};
const ctx=()=>call('get_context',{selection:[]});
const project=async()=>{const c=await ctx();const value=await call('get_object',{object_id:{kind:'project',id:c.project.id},source:{kind:'draft'},node_index:null},c.project.id);return value.data;};
const revision=async n=>page.waitForFunction(n=>document.querySelector('.title-status')?.textContent===`r${n}`,n);
let initialContext,actualIdentity;
try{
 if(!continueLibrary){
 await check('actual_native_product_identity_empty_isolated_host_and_sdk_connection',async()=>{
  assert.equal(await page.evaluate(()=>('__TAURI_INTERNALS__' in window)),true);
  actualIdentity=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('agent_connection_info'));
  assert.equal(actualIdentity.host_instance_id,expectedHost);
  initialContext=await ctx();assert.equal(initialContext.host.session_id,'desktop-main-ui');
  assert.equal(initialContext.project,null,'Never replace an existing user project to run this isolated test');
  assert.ok((await page.locator('.app-status').innerText()).includes('桌面 Host'));
 });
 await check('actual_product_new_project_and_hall_UI_commit_to_same_native_host',async()=>{
  await menu('文件','新建工程…');await page.getByLabel('工程名称',{exact:true}).fill('W5 Native UI independent isolated');await page.getByRole('button',{name:'创建',exact:true}).click();await revision(1);
  await page.getByRole('button',{name:'新建厂房',exact:true}).click();await page.getByLabel('名称',{exact:true}).fill('W5 Hall');
  await page.getByLabel('宽度 · m',{exact:true}).fill('20');await page.getByLabel('进深 · m',{exact:true}).fill('12');await page.getByRole('button',{name:'应用到工程',exact:true}).click();await revision(2);
  const p=await project();assert.equal(p.title,'W5 Native UI independent isolated');assert.equal(p.halls.length,1);assert.equal(p.halls[0].height_m,null);assert.equal(p.halls[0].width_m,20);
 });
 await check('actual_product_selection_edit_and_common_undo_redo_create_new_revisions',async()=>{
  await page.getByRole('button',{name:'▱ W5 Hall',exact:true}).click();await page.getByRole('button',{name:'打开详情编辑',exact:true}).click();await page.getByLabel('厂房净高 · m',{exact:true}).fill('3.5');await page.getByRole('button',{name:'应用到工程',exact:true}).click();await revision(3);
  assert.equal((await project()).halls[0].height_m,3.5);
  await menu('编辑','撤销');await revision(4);assert.equal((await project()).halls[0].height_m,null);
  await menu('编辑','重做');await revision(5);assert.equal((await project()).halls[0].height_m,3.5);
 });
 await check('actual_product_stale_form_preserves_values_and_does_not_overwrite_shared_revision',async()=>{
  await page.getByRole('button',{name:'打开详情编辑',exact:true}).click();await page.getByLabel('宽度 · m',{exact:true}).fill('21');
  const p=await project();await call('apply_commands',{base_revision:p.revision,commands:[{kind:'set_project_settings',data:{title:'W5 Native UI concurrent change',time_reference:p.time_reference,generation_context:p.generation_context,sources:p.sources}}]},p.id);
  await page.getByRole('button',{name:'应用到工程',exact:true}).click();await page.getByRole('alert').waitFor();
  assert.equal(await page.getByLabel('宽度 · m',{exact:true}).inputValue(),'21');
  const current=await project();assert.equal(current.revision,6);assert.equal(current.halls[0].width_m,20);assert.equal(current.title,'W5 Native UI concurrent change');
 });
 await page.screenshot({path:fileURLToPath(new URL('actual-native-product-conflict.png',out))});
 }else{
  const proof=JSON.parse(await readFile(new URL('result.json',originalOut),'utf8'));
  assert.equal(proof.expectedHost,expectedHost);assert.equal(proof.initialContext.project,null);
  assert.equal(proof.groups.slice(0,4).filter(group=>group.passed).length,4);
  actualIdentity=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('agent_connection_info'));assert.equal(actualIdentity.host_instance_id,expectedHost);initialContext=await ctx();
  const owned=await project();assert.equal(owned.title,'W5 Native UI concurrent change');assert.equal(owned.devices.length,0);assert.equal(owned.halls.length,1);assert.equal(owned.halls[0].title,'W5 Hall');assert.equal(owned.halls[0].width_m,20);
 }
 if(includeLibrary){
  let beforeLibrary;
  await check('actual_native_library_four_exact_template_candidates_and_preview_are_readonly',async()=>{
   if(await page.getByRole('button',{name:'放弃更改',exact:true}).count())await page.getByRole('button',{name:'放弃更改',exact:true}).click();
   const previousLibrary=page.getByRole('region',{name:'模块与模板库',exact:true});
   if(await previousLibrary.count())await previousLibrary.getByRole('button',{name:'返回画布',exact:true}).click();
   await menu('文件','刷新工程');beforeLibrary=await project();await revision(beforeLibrary.revision);
   await page.getByRole('button',{name:'模块与模板库',exact:true}).click();const library=page.getByRole('region',{name:'模块与模板库',exact:true});await library.locator('.library-filters select').selectOption('templates');
   await page.waitForFunction(()=>document.querySelectorAll('.library-list>button').length===4);
   const candidates=await call('query_module_library',{category_filters:['templates'],industry:null,product_family:null,site_envelope:null,source_scope:['bundled']},beforeLibrary.id);assert.equal(candidates.candidates.length,4);
   const chosen=candidates.candidates.find(c=>c.module_ref.id.includes('purchased-assembly'));assert.ok(chosen);await library.locator('.library-list>button').filter({hasText:chosen.module_ref.id}).click();await library.getByLabel('新建对象前缀',{exact:true}).waitFor();assert.deepEqual(await project(),beforeLibrary);
  });
  await check('actual_native_template_preparation_then_one_explicit_seedless_commit_preserves_existing_inputs',async()=>{
   const form=page.locator('.template-preparation');await form.getByLabel('新建对象前缀',{exact:true}).fill('w5-native-library');
   for(const select of await form.getByRole('combobox').all()){if(await select.locator('option').filter({hasText:'新建该声明示例资源'}).count())await select.selectOption({label:'新建该声明示例资源'});}
   assert.equal(await form.getByLabel('明确创建模板示例方案输入',{exact:true}).isChecked(),false);await form.getByRole('button',{name:'预览创建对象',exact:true}).click();const preview=form.locator('.template-preview');await preview.waitFor();assert.match(await preview.innerText(),/不创建方案、初始库存或订单输入/);assert.deepEqual(await project(),beforeLibrary);
   await form.getByLabel('新建对象前缀',{exact:true}).fill('w5-native-library-edited');assert.equal(await preview.getByRole('button',{name:'确认采纳模板到工程',exact:true}).isDisabled(),true);await form.getByRole('button',{name:'预览创建对象',exact:true}).click();await preview.getByRole('button',{name:'确认采纳模板到工程',exact:true}).click();await revision(beforeLibrary.revision+1);
   const after=await project();assert.equal(after.devices.length,beforeLibrary.devices.length+3);assert.deepEqual(after.scenario_refs,beforeLibrary.scenario_refs);assert.deepEqual(after.artifact_refs,beforeLibrary.artifact_refs);assert.deepEqual(after.acknowledgments,beforeLibrary.acknowledgments);assert.ok(after.devices.filter(d=>!beforeLibrary.devices.some(old=>old.id===d.id)).every(d=>d.id.startsWith('w5-native-library-edited')));
   await page.screenshot({path:fileURLToPath(new URL('actual-native-template-adoption.png',out))});
  });
 }
}catch(error){errors.push(String(error));}
await writeFile(new URL('result.json',out),JSON.stringify({endpoint,expectedHost,page_url:page.url(),actualIdentity,initialContext,library_included:includeLibrary,continuation_of_original4:continueLibrary,groups,errors,scope:'actual product UI on coordinator-owned isolated native process; no alternative Host/Engine or Tauri mock',not_exercised:['actual native Open/Save picker','device models/connections/group members/static check','old Run and professional acknowledgment preservation in product UI','full IX/NFR-B60/user acceptance'],whole_task_done:false,human_acceptance:false},null,2));
for(const group of groups)console.log(JSON.stringify(group));
// Drop this client's connection by ending the driver; keep the actual native
// product process alive for its coordinator to inspect and close.
process.exit(groups.length===(continueLibrary?2:includeLibrary?6:4)&&groups.every(x=>x.passed)?0:1);
