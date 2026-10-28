// Same saved factory, ordinary Native UI; inspect per-device LOD and actual drawing cost.
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
 const full=await api('get_run',{run_id:prior.run_id,query:{kind:'full_result'}});assert.equal(full.kind,'full');
 const original=await read(path.join(base,previous,'flow/actual-run.json'));assert.deepEqual(full.data,original);await write('actual-reopened-context.json',context);await write('actual-reopened-project.json',object.data);
 await page.getByLabel('历史结果',{exact:true}).selectOption(prior.run_id);
 await page.waitForFunction(id=>document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true'&&document.querySelector('canvas')?.readFactoryDiagnostics?.().run_id===id,prior.run_id,{timeout:120000});
 console.log('Actual restored Run is selected and ready',prior.run_id);

 const baseline=process.argv.includes('--baseline'),samples={};
 const device=object.data.devices.find(d=>d.id.endsWith(':PF-paper-bag-tuber'));
 const job=original.jobs.find(j=>j.execution_ref.kind==='project_device'&&j.execution_ref.data.device_id===device.id);
 const work=original.phase_work_intervals.filter(w=>w.job_id===job.id).sort((a,b)=>(b.interval.end_ms-b.interval.start_ms)-(a.interval.end_ms-a.interval.start_ms))[0];
 const activeMid=Math.floor((work.interval.start_ms+work.interval.end_ms)/2);
 const ready=()=>page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{},{timeout:120000});
 const seek=async at=>{await page.getByLabel('观察毫秒',{exact:true}).fill(String(at));await page.getByRole('button',{name:'读取时点',exact:true}).click();await page.waitForFunction(at=>document.querySelector('canvas')?.readFactoryDiagnostics?.().observation_ms===at,at,{timeout:120000});await ready();};
 const sample=async name=>{await ready();const d=await page.evaluate(()=>document.querySelector('canvas').readFactoryDiagnostics());assert.equal(d.run_id,prior.run_id);assert.deepEqual(d.asset_loading.items.failures,[]);assert.equal(d.asset_loading.devices.length,92);assert(d.asset_loading.devices.every(v=>v.loaded));samples[name]=d;await write('lod-samples.json',samples);console.log(name,JSON.stringify({lod:d.lod,render:d.render}));return d;};
 const finishAuto=process.argv.includes('--finish-auto');
 const menuClick=async name=>{const summary=page.getByText('视图',{exact:true}),menu=summary.locator('..');if(await menu.getAttribute('open')===null)await summary.click();await menu.getByRole('button',{name,exact:true}).click();if(await menu.getAttribute('open')!==null)await summary.click();};
 if(finishAuto){Object.assign(samples,await read(path.join(directory,'reopen/lod-samples.json')));assert.equal(samples.focused.asset_loading.devices.filter(d=>d.lod==='detailed').length,1);assert(samples['manual-detailed'].asset_loading.devices.every(d=>d.lod==='detailed'));}
 else {
 await seek(activeMid);await page.getByRole('button',{name:'全厂适配',exact:true}).click();
 const overview=await sample('overview');
 if(!baseline)assert(overview.asset_loading.devices.every(d=>d.lod==='simplified'));
 await page.locator('.object-tree').getByRole('button',{name:device.title}).click();await page.getByRole('button',{name:'聚焦选中对象',exact:true}).click();
 const focus=await sample('focused');await page.screenshot({path:path.join(out,'focused-device.png')});
 const pose=focus.asset_loading.devices.find(d=>d.id===device.id);assert.equal(pose.phase,work.phase_id);assert.equal(pose.effective_ms,work.effective_progress_start_ms+activeMid-work.interval.start_ms);
 if(!baseline){
  assert.equal(focus.lod,'simplified');assert.equal(focus.asset_loading.devices.filter(d=>d.lod==='detailed').length,1);assert.equal(pose.lod,'detailed');
  await seek(activeMid+100);const after=await sample('focused-after-seek');assert.equal(after.asset_loading.devices.filter(d=>d.lod==='detailed').length,1);assert.equal(after.asset_loading.devices.find(d=>d.id===device.id).effective_ms,pose.effective_ms+100);
  const other=object.data.devices.find(d=>d.id.endsWith(':PF-paper-bag-bottomer'));
  assert(other);await page.locator('.object-tree').getByRole('button',{name:other.title}).click();await page.getByRole('button',{name:'聚焦选中对象',exact:true}).click();
  const switched=await sample('other-selected');assert.equal(switched.asset_loading.devices.filter(d=>d.lod==='detailed').length,1);assert.equal(switched.asset_loading.devices.find(d=>d.id===other.id).lod,'detailed');assert.equal(switched.asset_loading.devices.find(d=>d.id===device.id).lod,'simplified');
  await page.getByRole('button',{name:'全厂适配',exact:true}).click();const far=await sample('return-to-overview');assert(far.asset_loading.devices.every(d=>d.lod==='simplified'));
  await menuClick('详细');const manual=await sample('manual-detailed');assert(manual.asset_loading.devices.every(d=>d.lod==='detailed'));
 }
 }
 if(!baseline){await menuClick('自动呈现档位');const auto=await sample('automatic-restored');assert(auto.asset_loading.devices.every(d=>d.lod==='simplified'));
 }
 const state=await invoke('project_close_state');assert.equal(state.active_tasks.length,0);assert.equal(state.execution_blockers.length,0);assert.deepEqual(errors,[]);
 await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'close-release.ps1'),'-NativeDirectory',directory],{windowsHide:true});
 await write('receipt.json',{status:baseline?'actual_Native_Lod_baseline':'actual_Native_per_device_Lod_verified',pid:launch.pid,candidate:launch.exe,project_id:projectId,run_id:prior.run_id,run_exact:true,devices:92,baseline,focus_phase_and_clock_verified:true,per_device_precision_verified:!baseline,normal_close:true,errors,whole_factory_accepted:false});process.exit(0);
}catch(error){await page.screenshot({path:path.join(out,'failure.png'),timeout:10000}).catch(()=>{});await write('failure.json',{error:String(error),errors,body:await page.locator('body').innerText().catch(()=>''),project_id:projectId});console.error(error);process.exit(1);}
