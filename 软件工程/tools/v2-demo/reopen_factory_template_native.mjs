// Independent ordinary Native reopening of the UI-created factory and old Run.
import fs from 'node:fs/promises';import path from 'node:path';import assert from 'node:assert/strict';
import {execFile} from 'node:child_process';import {promisify} from 'node:util';import {createRequire} from 'node:module';
const [base,attempt,previous]=process.argv.slice(2),resume=process.argv[5]==='resume',directory=path.join(base,attempt),out=path.join(directory,resume?'reopen-resumed':'reopen');await fs.mkdir(out);
const read=async file=>JSON.parse((await fs.readFile(file,'utf8')).replace(/^\uFEFF/,''));
const launch=await read(path.join(directory,'launch.json')),prior=await read(path.join(base,previous,'flow/receipt.json'));
assert(launch.pid!==JSON.parse((await fs.readFile(path.join(base,previous,'launch.json'),'utf8')).replace(/^\uFEFF/,'')).pid);
assert(launch.monitor.connected===1||!launch.monitor.primary);assert(launch.owned_browser);
const require=createRequire(import.meta.url),{chromium}=require('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const browser=await chromium.connectOverCDP(launch.cdp),page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://tauri.localhost/');assert(page);page.setDefaultTimeout(30000);
const exec=promisify(execFile),pwsh='C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe',errors=[];let projectId=null;
page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
const write=(name,value)=>fs.writeFile(path.join(out,name),JSON.stringify(value,null,2));
const invoke=(command,args)=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const api=async(operation,params={},id=projectId)=>{const r=await invoke('project_api',{request:{api_version:'0.1',request_id:'factory-reopen-'+crypto.randomUUID(),project_id:id,scope:{kind:id?'project':'workspace'},operation,params}});assert.equal(r.outcome.status,'ok',JSON.stringify(r.outcome));return r.outcome.data;};
try{
 const initial=await api('get_context',{selection:[]},null);
 if(resume){assert.equal(initial.project?.id,prior.project_id);await fs.access(path.join(directory,'reopen/actual-reopened-context.json'));}
 else {
 assert.equal(initial.project,null);
 await page.getByText('文件',{exact:true}).click();await page.getByRole('button',{name:'打开工程目录…',exact:true}).click();
 let dialog;const deadline=Date.now()+15000;
 do{const value=await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});if(value.stdout.trim()){const current=JSON.parse(value.stdout);if(current.controls.some(c=>c.class==='Edit'&&c.id===1152)&&current.controls.some(c=>c.class==='Button'&&c.id===1)){dialog=current;break;}}await new Promise(r=>setTimeout(r,100));}while(Date.now()<deadline);
 assert(dialog);await write('actual-open-OS-dialog.json',dialog);
 const edit=dialog.controls.find(c=>c.class==='Edit'&&c.id===1152),button=dialog.controls.find(c=>c.class==='Button'&&c.id===1);assert(edit&&button);
 await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe,'-SelectDirectory',prior.saved,'-EditHandle',String(edit.handle),'-ButtonHandle',String(button.handle)],{windowsHide:true});
 await page.getByRole('button',{name:'确认打开',exact:true}).click();
 await page.waitForFunction(()=>!document.querySelector('[role="dialog"]')&&document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true',{},{timeout:120000});
 }
 const context=await api('get_context',{selection:[]},null);projectId=context.project.id;assert.equal(projectId,prior.project_id);assert.equal(context.run_summaries.length,1);assert.equal(context.run_summaries[0].run_id,prior.run_id);
 const object=await api('get_object',{object_id:{kind:'project',id:projectId},source:{kind:'draft'},node_index:null});assert.equal(object.data.devices.length,92);assert.equal(object.data.setup_policies.length,92);
 const original=await read(path.join(base,previous,'flow/actual-run.json'));
 if(!resume){const full=await api('get_run',{run_id:prior.run_id,query:{kind:'full_result'}});assert.equal(full.kind,'full');assert.deepEqual(full.data,original);}
 else {const verified=await read(path.join(directory,'reopen/actual-reopened-context.json'));assert.equal(verified.project.id,projectId);assert.equal(verified.run_summaries[0].run_id,prior.run_id);}
 await write('actual-reopened-context.json',context);await write('actual-reopened-project.json',object.data);
 await page.getByLabel('历史结果',{exact:true}).selectOption(prior.run_id);
 await page.waitForFunction(id=>document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true'&&document.querySelector('canvas')?.readFactoryDiagnostics?.().run_id===id,prior.run_id,{timeout:120000});
 console.log('Actual restored Run is selected and ready',prior.run_id);
 const newBranch=original.transport.find(t=>JSON.stringify(t.supply_ref).includes('PF-BRANCH-FEED-')&&t.departed_ms!=null&&t.arrived_ms!=null);
 const trip=newBranch??original.transport.find(t=>t.departed_ms!=null&&t.arrived_ms!=null),mid=Math.floor((trip.departed_ms+trip.arrived_ms)/2),observations=[];
 const device=object.data.devices.find(d=>d.id.endsWith(newBranch?':PF-carton-counter-stacker':':PF-paper-bag-tuber'));
 const job=original.jobs.find(j=>j.execution_ref.kind==='project_device'&&j.execution_ref.data.device_id===device.id);
 const work=original.phase_work_intervals.filter(w=>w.job_id===job.id).sort((a,b)=>(b.interval.end_ms-b.interval.start_ms)-(a.interval.end_ms-a.interval.start_ms))[0];
 const activeMid=Math.floor((work.interval.start_ms+work.interval.end_ms)/2);
 const selectionOnly=process.argv.includes('--selection-label-only');
 for(const at of selectionOnly?[]:[mid,activeMid,100000,prior.product_follow_at_ms,mid]){
  await page.getByLabel('观察毫秒',{exact:true}).fill(String(at));await page.getByRole('button',{name:'读取时点',exact:true}).click();
  await page.waitForFunction(at=>document.querySelector('canvas')?.readFactoryDiagnostics?.().observation_ms===at,at,{timeout:120000});
  await page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{},{timeout:120000});
  const diagnostics=await page.evaluate(()=>document.querySelector('canvas').readFactoryDiagnostics());assert.equal(diagnostics.run_id,prior.run_id);assert.deepEqual(diagnostics.asset_loading.items.failures,[]);
  assert.equal(diagnostics.asset_loading.devices.length,92);assert(diagnostics.asset_loading.devices.every(d=>d.loaded));
  if(at===mid){
   assert(diagnostics.visible_objects.some(o=>o.kind==='transport'&&o.id===trip.id));
   for(const id of trip.cargo)assert(diagnostics.visible_objects.some(o=>o.kind==='item'&&o.id===id));
  }
  if(at===activeMid){
   const pose=diagnostics.asset_loading.devices.find(d=>d.id===device.id);assert.equal(pose.phase,work.phase_id);assert.equal(pose.effective_ms,work.effective_progress_start_ms+at-work.interval.start_ms);assert.ok(pose.progress>0&&pose.progress<1);
   await page.locator('.object-tree').getByRole('button',{name:`▣ ${device.title}`,exact:true}).click();await page.getByRole('button',{name:'聚焦选中对象',exact:true}).click();
   await page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{},{timeout:120000});
   await page.screenshot({path:path.join(out,`active-${newBranch?'carton-counter-stacker':'bag-tuber'}.png`)});await page.getByRole('button',{name:'全厂适配',exact:true}).click();
  }
  observations.push(diagnostics);await write('actual-observations.json',observations);
  if(observations.length<=3)await page.screenshot({path:path.join(out,`frame-${at}.png`)});
  console.log('reopened actual frame',at,JSON.stringify(diagnostics.render));
 }
 if(selectionOnly){
  await page.getByLabel('观察毫秒',{exact:true}).fill('7200000');await page.getByRole('button',{name:'读取时点',exact:true}).click();
  await page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.observation_ms===7200000&&d.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{},{timeout:120000});
  const selectedButton=page.locator('.runtime-items button').filter({hasText:'500×500mm裁张原纸'}).first();
  const selectedId=await selectedButton.getAttribute('title');assert(selectedId);await selectedButton.click();
  const description=page.locator('.description');assert.equal(await description.locator('div > strong').innerText(),'500×500mm裁张原纸');
  assert.equal(await description.locator('div > p').first().innerText(),'物品');
  const details=description.locator('details');assert.equal(await details.getAttribute('open'),null);
  assert(!(await description.innerText()).includes(selectedId));
  await details.locator('summary').click();assert.equal(await details.locator('p').innerText(),selectedId);
  await details.locator('summary').click();await page.screenshot({path:path.join(out,'selected-material-name.png')});
  await write('selection-label-receipt.json',{business_title:'500×500mm裁张原纸',identifier_hidden_by_default:true,identifier_available_on_expansion:true,actual_saved_project_and_Run_reopened:true});
 }
 await write('actual-sampled-identities.json',{transport_trip:trip.id,cargo:trip.cargo,device_id:device.id,phase:work.phase_id,transport_at_ms:mid,working_at_ms:activeMid});
 const state=await invoke('project_close_state');assert.equal(state.active_tasks.length,0);assert.equal(state.execution_blockers.length,0);await write('actual-close-state.json',state);assert.deepEqual(errors,[]);
 await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'close-release.ps1'),'-NativeDirectory',directory],{windowsHide:true});
 await write('receipt.json',{status:selectionOnly?'fresh_Native_selection_label_verified_and_normal_close':'fresh_ordinary_Native_project_and_Run_restored_actual_seeks_and_normal_close',pid:launch.pid,previous_pid:JSON.parse((await fs.readFile(path.join(base,previous,'launch.json'),'utf8')).replace(/^\uFEFF/,'')).pid,project_id:projectId,run_id:prior.run_id,run_exact:true,devices:92,remapped_setup_policies:92,observations:observations.map(d=>d.observation_ms),actual_transport_and_cargo_visible:!selectionOnly,selection_label_verified:selectionOnly,automatic_LOD_default:true,source_linked_resources:prior.source_linked_resources,standalone_package_verified:prior.standalone_package_verified,template_ref:prior.template_ref,whole_factory_accepted:false,errors});console.log('Independent Native reopen passed');process.exit(0);
}catch(error){await page.screenshot({path:path.join(out,'failure.png'),timeout:10000}).catch(()=>{});await write('failure.json',{error:String(error),errors,body:await page.locator('body').innerText().catch(()=>''),project_id:projectId});console.error(error);process.exit(1);}
