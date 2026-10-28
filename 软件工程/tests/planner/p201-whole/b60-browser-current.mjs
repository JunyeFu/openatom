// Normal Workbench + actual saved Host archive. Browser verification only, not Native/NFR.
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { readFile,writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import { openCore,root,out } from './current-core.mjs';
const [variant,attempt,executable='post-scopes-core.exe']=process.argv.slice(2);assert.ok(['balanced','dense'].includes(variant)&&attempt);
const producer=JSON.parse(await readFile(new URL(`b60-${variant}.current.${attempt}.result.json`,out),'utf8'));
assert.equal(producer.actual_result,'passed_current_Host_Engine_counts_and_identity_not_Native_performance');assert.ok(producer.saved_archive);
const frozen=JSON.parse(await readFile(new URL(`b60-${variant}.current.${attempt}.run.json`,out),'utf8'));
const require=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json'),{chromium}=require('playwright');
const result={kind:'full_B60_normal_Workbench_actual_Host_archive_consumer',variant,producer_attempt:attempt,actual_Host:true,Native_app:false,formal_NFR:false,errors:[],lods:[],stage:'bootstrap',rpc_timings:[]};let core,browser,page;
try{
  core=await openCore(fileURLToPath(new URL('b60-scaling-10.inline-allowlist-fixed.input.json',out)),executable);await core.rpc({kind:'new_project'});
  browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});page=await browser.newPage({viewport:{width:1920,height:1080}});page.setDefaultTimeout(120000);
  page.on('pageerror',error=>result.errors.push(String(error)));page.on('console',message=>{if(message.type()==='error')result.errors.push(message.text());});
  await page.route('**/__p201_core*',async route=>{
    const request=route.request().method()==='GET'?null:route.request().postDataJSON(),api=request?.request,query=api?.params?.query;
    const row={kind:request?.kind??'ready',operation:api?.operation,query:query?.kind,at_ms:query?.data?.at_ms,selectors:query?.data?.objects?.length,after_index:query?.data?.page?.after_index,started_ms:performance.now()};result.rpc_timings.push(row);
    const reply=request?await core.rpc(request):core.ready;row.elapsed_ms=performance.now()-row.started_ms;row.status=reply.outcome?.status;
    if(reply.outcome?.data?.kind==='objects'){row.returned=reply.outcome.data.data.objects.length;row.next_index=reply.outcome.data.data.next_index;}
    await route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(reply)});
  });
  await page.goto('http://127.0.0.1:8922/@fs/'+fileURLToPath(root).replaceAll('\\','/').replace(/\/$/,'')+'/tests/planner/p201-whole/harness-import.html');
  await page.getByLabel('工程开始方式').waitFor();await page.evaluate(directory=>window.__p201.setOpenDirectory(directory),producer.saved_archive);await page.getByLabel('工程开始方式').getByRole('button',{name:/^展示任务/}).click();const modal=page.locator('.modal');await modal.getByRole('heading',{name:'打开工程 · 采纳前核对',exact:true}).waitFor();await modal.getByRole('combobox',{name:'离开当前工程',exact:true}).selectOption('discard_after_tasks_terminal');await modal.getByRole('button',{name:'确认打开',exact:true}).click();await modal.waitFor({state:'hidden'});
  const project=(await core.rpc({kind:'snapshot'})).project;assert.equal(project.id,'NFR-B60-'+variant);assert.equal(project.devices.length,60);assert.equal(project.halls.length,3);
  result.stage='selected_Run_end';await page.getByRole('combobox',{name:'历史结果',exact:true}).selectOption(frozen.id);await page.waitForFunction(()=>document.querySelector('.workbench')?.getAttribute('aria-busy')==='false'&&document.querySelector('.sf-timeline')?.getAttribute('data-view-time')==='1200000');
  result.stage='observation_600s';await page.getByRole('spinbutton',{name:'观察毫秒',exact:true}).fill('600000');await page.getByRole('button',{name:'读取时点',exact:true}).click();await page.waitForFunction(()=>document.querySelector('.sf-timeline')?.getAttribute('data-view-time')==='600000');
  const expectedDevices=variant==='balanced'?20:60;
  const waitModels=(minimum,maximum)=>page.waitForFunction(({expected,minimum,maximum})=>{const value=window.__p201.scene();if(!value)return false;const counts=new Map();value.scene.traverse(object=>{if(object.isMesh&&object.userData.ref?.kind==='device'){const id=object.userData.ref.id;counts.set(id,(counts.get(id)??0)+1);}});return counts.size===expected&&[...counts.values()].every(count=>count>=minimum&&(maximum===null||count<maximum));},{expected:expectedDevices,minimum,maximum});
  result.stage='detailed_models';await waitModels(105,null);
  const scene=()=>page.evaluate(()=>{const{scene,renderer}=window.__p201.scene();let meshes=0;const devices=new Set();scene.traverse(object=>{if(object.isMesh)meshes++;if(object.userData.ref?.kind==='device')devices.add(object.userData.ref.id);});return{meshes,devices:[...devices],submitted_triangles_diagnostic:renderer.info.render.triangles,draw_calls_diagnostic:renderer.info.render.calls,canvas_css:renderer.domElement.getBoundingClientRect().toJSON()};});
  result.lods.push({lod:'detailed',scene:await scene()});await page.screenshot({path:fileURLToPath(new URL(`b60-${variant}.${attempt}.normal-detailed.png`,out))});
  for(const[lod,label]of[['simplified','简化'],['region','区域']]){result.stage=lod;await page.getByText('视图',{exact:true}).click();await page.locator('nav details[open]').getByRole('button',{name:label,exact:true}).click();if(lod==='region')await page.getByLabel('区域实际汇总').waitFor();else await waitModels(23,105);result.lods.push({lod,scene:await scene()});await page.screenshot({path:fileURLToPath(new URL(`b60-${variant}.${attempt}.normal-${lod}.png`,out))});}
  result.stage='multi_hall';await page.getByRole('button',{name:'多厂房画布',exact:true}).click();await page.waitForFunction(()=>{const value=window.__p201.scene();if(!value)return false;const halls=new Set(),devices=new Set();value.scene.traverse(object=>{const ref=object.userData.ref;if(ref?.kind==='hall')halls.add(ref.id);if(ref?.kind==='device')devices.add(ref.id);});return halls.size===3&&devices.size===60;});result.multi_hall=await scene();await page.screenshot({path:fileURLToPath(new URL(`b60-${variant}.${attempt}.normal-multi-hall.png`,out))});
  const after=(await core.rpc({kind:'snapshot'})).project;assert.deepEqual(after,project);
  const reply=await core.rpc({kind:'dispatch',request:{api_version:'0.1',request_id:'B60-normal-view-frozen-read',project_id:project.id,scope:{kind:'project'},operation:'get_run',params:{run_id:frozen.id,query:{kind:'full_result'}}}});assert.equal(reply.outcome.status,'ok');assert.deepEqual(reply.outcome.data.data,frozen);
  assert.equal(result.errors.length,0,result.errors.join('\n'));result.current_project_and_complete_Run_equal=true;result.status='passed';
}catch(error){result.status='failed';result.error=String(error);result.stack=error.stack;process.exitCode=1;
  result.scene_at_failure=await page?.evaluate(()=>{const value=window.__p201.scene();if(!value)return null;const counts={};value.scene.traverse(object=>{if(object.isMesh&&object.userData.ref?.kind==='device'){const id=object.userData.ref.id;counts[id]=(counts[id]??0)+1;}});return{device_mesh_counts:counts};});
  await page?.screenshot({path:fileURLToPath(new URL(`b60-${variant}.${attempt}.normal-failure.png`,out))});}
finally{await writeFile(new URL(`b60-${variant}.${attempt}.normal-Workbench.json`,out),JSON.stringify(result,null,2));await browser?.close();await core?.close();}
console.log(JSON.stringify({status:result.status,variant,lods:result.lods,error:result.error,Native_app:false,formal_NFR:false}));
