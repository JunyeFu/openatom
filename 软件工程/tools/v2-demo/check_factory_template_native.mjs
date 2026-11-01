// Ordinary Native UI, its real Host and owned Windows folder picker.
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {createRequire} from 'node:module';
const [base,attempt,expectedFile]=process.argv.slice(2),directory=path.join(base,attempt),out=path.join(directory,'flow');
await fs.mkdir(out);
const launch=JSON.parse((await fs.readFile(path.join(directory,'launch.json'),'utf8')).replace(/^\uFEFF/,''));
assert(launch.monitor.connected===1||!launch.monitor.primary);assert(launch.owned_browser);
const expected=expectedFile?JSON.parse(await fs.readFile(expectedFile,'utf8')):{template_ref:{id:'community.template-paper-packaging-factory',version:'1.2.0'},manufacturing_jobs:145,transport_trips:78,accepted_orders:{CASE:3,SLEEVE:3,TUBE:6,LABEL:18,BAG:3},standalone_package:false};
const settledTime=expected.observation_ms??7200000;
const acceptedTotal=Object.values(expected.accepted_orders).reduce((a,b)=>a+b,0),followTitle=expected.follow_title??'180×80×200mm双提手纸袋';
const require=createRequire(import.meta.url),{chromium}=require('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const browser=await chromium.connectOverCDP(launch.cdp);
const page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://tauri.localhost/');assert(page);page.setDefaultTimeout(30000);
const run=promisify(execFile),pwsh='C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe';
const errors=[],events=[];page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
const write=(name,data)=>fs.writeFile(path.join(out,name),JSON.stringify(data,null,2));
const event=async(name,data={})=>{events.push({at:new Date().toISOString(),name,...data});await write('events.json',events);console.log(name,JSON.stringify(data));};
const invoke=(command,args)=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
let projectId=null;
const api=async(operation,params={},project=projectId)=>{
 const reply=await invoke('project_api',{request:{api_version:'0.1',request_id:'factory-template-'+crypto.randomUUID(),project_id:project,scope:{kind:project?'project':'workspace'},operation,params}});
 assert.equal(reply.outcome.status,'ok',JSON.stringify(reply.outcome));return reply.outcome.data;
};
const project=async()=>{const value=await api('get_object',{object_id:{kind:'project',id:projectId},source:{kind:'draft'},node_index:null});assert.equal(value.kind,'project');return value.data;};
async function picker(folder){
 await fs.mkdir(folder,{recursive:true});let dialog;
 const until=Date.now()+15000;
 do{
  const result=await run(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});
  if(result.stdout.trim()){const current=JSON.parse(result.stdout);if(current.controls.some(c=>c.class==='Edit'&&c.id===1152)&&current.controls.some(c=>c.class==='Button'&&c.id===1)){dialog=current;break;}}
  await new Promise(resolve=>setTimeout(resolve,100));
 }while(Date.now()<until);
 assert(dialog,'Actual owned folder picker must be present');await write('save-OS-dialog.json',dialog);
 const edit=dialog.controls.find(c=>c.class==='Edit'&&c.id===1152),button=dialog.controls.find(c=>c.class==='Button'&&c.id===1);assert(edit&&button);
 await run(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe,'-SelectDirectory',folder,'-EditHandle',String(edit.handle),'-ButtonHandle',String(button.handle)],{windowsHide:true});
}
try{
 const initial=await api('get_context',{selection:[]},null);assert.equal(initial.project,null,'Use a new owned Native with no existing Project');
 await page.locator('#start-template').click();await page.getByLabel('工程名称',{exact:true}).fill('纸包装模板验证');await page.getByRole('button',{name:'创建',exact:true}).click();
 await page.getByRole('button',{name:/纸包装多产品工厂 · 92台设备/}).click();
 await page.getByRole('heading',{name:'配置模板工厂',exact:true}).waitFor();
 await page.getByRole('button',{name:'使用模板演示配置',exact:true}).click();
 await page.getByLabel('工厂前缀',{exact:true}).fill('演示工厂');await page.getByLabel('方案名称',{exact:true}).fill('纸包装演示生产');
 await event('demo_configuration_filled');
 await page.screenshot({path:path.join(out,'01-demo-configuration.png')});
 await page.getByRole('button',{name:'预览创建对象',exact:true}).click();
 await page.getByRole('heading',{name:'创建预览',exact:true}).waitFor();
 await page.getByRole('button',{name:'创建模板工厂',exact:true}).click();
 await page.locator('.module-library').waitFor({state:'hidden',timeout:120000});
 const context=await api('get_context',{selection:[]},null);projectId=context.project.id;
 const adopted=await project();assert.equal(adopted.devices.length,92);assert.equal(adopted.setup_policies.length,92);
 assert.equal(adopted.template_adoptions.length,1);assert.deepEqual(adopted.template_adoptions[0].template_ref,expected.template_ref);assert(adopted.devices.every(d=>d.id.startsWith('演示工厂:')));
 await write('actual-adopted-project.json',adopted);await event('ordinary_UI_template_created',{project_id:projectId,devices:adopted.devices.length});
 await page.getByLabel('当前方案',{exact:true}).selectOption('纸包装演示生产');
 await page.getByRole('button',{name:'运行方案',exact:true}).click();
 await page.waitForFunction(()=>document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true',{},{timeout:120000});
 let completed=await api('get_context',{selection:[]});
 assert.equal(completed.run_summaries.length,1);
 const runId=completed.run_summaries[0].run_id;
 const full=await api('get_run',{run_id:runId,query:{kind:'full_result'}});assert.equal(full.kind,'full');
 const result=full.data;assert.equal(result.orders.length,Object.keys(expected.accepted_orders).length);assert.equal(result.transport.length,expected.transport_trips);
 assert.deepEqual(result.orders.map(o=>o.accepted.value.coefficient/10**o.accepted.value.decimal_places).sort((a,b)=>a-b),Object.values(expected.accepted_orders).sort((a,b)=>a-b));
  for(const [identity,count] of Object.entries(expected.accepted_orders)){
    const order=result.orders.find(o=>o.order_id.endsWith(identity));assert(order,identity);
    assert.equal(order.accepted.value.coefficient/10**order.accepted.value.decimal_places,count,identity);
  }
 assert(result.orders.every(o=>o.outstanding.value.coefficient===0));
 assert.equal(result.jobs.filter(j=>j.execution_ref.kind==='project_device').length,expected.manufacturing_jobs);
 await write('actual-run.json',result);await event('ordinary_UI_run_completed',{run_id:runId,accepted_products:acceptedTotal});
 await page.waitForFunction(id=>document.querySelector('canvas')?.readFactoryDiagnostics?.().run_id===id,runId,{timeout:120000});
 await page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{},{timeout:120000});
 const scene=await page.evaluate(()=>({diagnostics:document.querySelector('canvas').readFactoryDiagnostics(),labels:[...document.querySelectorAll('[data-scene-label-key^="device:"]')].map(e=>e.title)}));
 assert.equal(scene.diagnostics.asset_loading.devices.length,92);assert(scene.diagnostics.asset_loading.devices.every(d=>d.loaded));
 assert(!scene.diagnostics.asset_loading.devices.some(d=>/读取素材中|表现素材未连接|阶段未知|unavailable/.test(d.title)));
 assert.deepEqual(scene.diagnostics.asset_loading.items.failures,[]);await write('actual-native-scene.json',scene);
 await page.screenshot({path:path.join(out,'02-native-factory.png')});
 await page.getByLabel('观察毫秒',{exact:true}).fill(String(settledTime));await page.getByRole('button',{name:'读取时点',exact:true}).click();
 await page.waitForFunction(at=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.observation_ms===at&&d.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},settledTime,{timeout:120000});
  if(expected.verify_stock_separation){
    const diagnostics=await page.evaluate(()=>document.querySelector('canvas').readFactoryDiagnostics());
    const stocked=result.final_items.filter(i=>i.lifecycle==='available'&&i.holder.kind==='location'&&(expected.finished_location_suffixes??[':PF-SHIPPING']).some(suffix=>i.holder.id.endsWith(suffix)));
    assert.equal(stocked.length,acceptedTotal);const overlaps=[];
    for(let a=0;a<stocked.length;a++)for(let b=a+1;b<stocked.length;b++){
      const left=diagnostics.item_bounds[stocked[a].id],right=diagnostics.item_bounds[stocked[b].id];assert(left&&right);
      if(left.min.every((v,i)=>Math.min(left.max[i],right.max[i])-Math.max(v,right.min[i])>1e-6))overlaps.push([stocked[a].id,stocked[b].id]);
    }
    await write('actual-shipping-bounds.json',{items:stocked.map(i=>({id:i.id,type:i.item_type_ref.id,bounds:diagnostics.item_bounds[i.id]})),overlaps});assert.deepEqual(overlaps,[]);
    assert(!((await page.locator('.runtime-items').innerText()).includes('未绑定的物料')));
  }
 await page.locator('.runtime-items button').filter({hasText:followTitle}).first().click();
 await page.getByRole('button',{name:'跟随选中物品',exact:true}).click();
 await page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.(),b=d?.item_bounds[d.following_item];return b&&d.camera.target.every((v,i)=>Math.abs(v-(b.min[i]+b.max[i])/2)<1e-5)&&d.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{},{timeout:120000});
 await write('actual-native-bag-follow.json',await page.evaluate(()=>document.querySelector('canvas').readFactoryDiagnostics()));
 await page.screenshot({path:path.join(out,'03-native-bag-follow.png')});
 const followed=await page.evaluate(()=>document.querySelector('canvas').readFactoryDiagnostics().following_item);
 await page.getByLabel('观察毫秒',{exact:true}).fill(String(settledTime-1));await page.getByRole('button',{name:'读取时点',exact:true}).click();
 await page.waitForFunction(({id,at})=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.observation_ms===at&&d.following_item===id&&d.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{id:followed,at:settledTime-1},{timeout:120000});
 await write('actual-native-bag-follow-after-seek.json',await page.evaluate(()=>document.querySelector('canvas').readFactoryDiagnostics()));
 await page.getByRole('button',{name:'全厂适配',exact:true}).click();
 await page.getByText('文件',{exact:true}).click();await page.getByRole('button',{name:'保存工程…',exact:true}).click();
 const saved=path.join(directory,'saved-project');await picker(saved);
 await page.waitForFunction(()=>document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true',{},{timeout:120000});
 await page.getByRole('status').filter({hasText:'已保存'}).waitFor({timeout:120000});
 const closeState=await invoke('project_close_state');await write('actual-close-state.json',closeState);
 assert.equal(closeState.active_tasks.length,0);assert.equal(closeState.execution_blockers.length,0);
 assert.equal(errors.length,0,JSON.stringify(errors));
 await run(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'close-release.ps1'),'-NativeDirectory',directory],{windowsHide:true});
 await event('saved_and_normally_closed',{saved});
 await write('receipt.json',{status:'ordinary_Native_template_UI_adoption_run_render_follow_save_closed',candidate:launch.exe,source_linked_resources:!expected.standalone_package,standalone_package_verified:!!expected.standalone_package,template_ref:expected.template_ref,project_id:projectId,run_id:runId,devices:92,manufacturing_jobs:expected.manufacturing_jobs,transport_trips:expected.transport_trips,accepted_products:acceptedTotal,follow_title:followTitle,product_follow_at_ms:settledTime,initial_frame_ms:scene.diagnostics.observation_ms,saved,errors,whole_factory_accepted:false,remaining:'Fresh ordinary Native reopen and subsequent window stages'});
 process.exit(0);
}catch(error){
 await page.screenshot({path:path.join(out,'failure.png'),timeout:10000}).catch(()=>{});
 await write('failure.json',{error:String(error),errors,body:await page.locator('body').innerText().catch(()=>''),events});
 console.error(error);process.exit(1); // Disconnect this helper; retain the owned Native for diagnosis.
}
