// Open the actual Host-produced project in the ordinary owned Native application.
import fs from 'node:fs/promises';import path from 'node:path';import assert from 'node:assert/strict';
import {execFile} from 'node:child_process';import {promisify} from 'node:util';import {pathToFileURL} from 'node:url';
const [base,attempt,proof,resume='']=process.argv.slice(2),directory=path.join(base,attempt),out=path.join(directory,'pallet'+(resume?'-'+resume:''));await fs.mkdir(out);
const read=async p=>JSON.parse((await fs.readFile(p,'utf8')).replace(/^\uFEFF/,'')),write=(n,v)=>fs.writeFile(path.join(out,n),JSON.stringify(v,null,2));
const launch=await read(path.join(directory,'launch.json')),original=await read(path.join(proof,'actual-connected-run.json')),expected=await read(path.join(proof,'actual-project.json'));
assert(launch.monitor.connected===1||!launch.monitor.primary);
const {chromium}=await import(pathToFileURL('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs').href);
const browser=await chromium.connectOverCDP(launch.cdp),page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://tauri.localhost/');assert(page);page.setDefaultTimeout(30000);
const exec=promisify(execFile),pwsh='C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe',errors=[],observations=[];
page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
const invoke=(command,args)=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});let projectId=null;
const api=async(operation,params={},id=projectId)=>{const r=await invoke('project_api',{request:{api_version:'0.1',request_id:'preparation-native-'+crypto.randomUUID(),project_id:id,scope:{kind:id?'project':'workspace'},operation,params}});assert.equal(r.outcome.status,'ok',JSON.stringify(r.outcome));return r.outcome.data;};
async function picker(saved){const until=Date.now()+15000;let dialog;do{const r=await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});if(r.stdout.trim()){const d=JSON.parse(r.stdout);if(d.controls.some(c=>c.class==='Edit'&&c.id===1152)&&d.controls.some(c=>c.class==='Button'&&c.id===1)){dialog=d;break;}}await page.waitForTimeout(100);}while(Date.now()<until);assert(dialog);await write('open-dialog.json',dialog);await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe,'-SelectDirectory',saved,'-EditHandle',String(dialog.controls.find(c=>c.class==='Edit'&&c.id===1152).handle),'-ButtonHandle',String(dialog.controls.find(c=>c.class==='Button'&&c.id===1).handle)],{windowsHide:true});}
async function seek(at){await page.getByLabel('观察毫秒',{exact:true}).fill(String(at));await page.getByRole('button',{name:'读取时点',exact:true}).click();await page.waitForFunction(at=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.observation_ms===at&&d.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},at,{timeout:120000});const d=await page.evaluate(()=>document.querySelector('canvas').readFactoryDiagnostics());assert.equal(d.run_id,original.id);assert.equal(d.asset_loading.devices.length,92);assert(d.asset_loading.devices.every(d=>d.loaded));assert.deepEqual(d.asset_loading.items.failures,[]);return d;}
try{
 if(!resume.startsWith('loaded')){assert.equal((await api('get_context',{selection:[]},null)).project,null);
 if(resume!=='picker'){await page.getByText('文件',{exact:true}).click();await page.getByRole('button',{name:'打开工程目录…',exact:true}).click();}await picker(path.join(proof,'project'));await page.getByRole('button',{name:'确认打开',exact:true}).click();
 await page.waitForFunction(()=>!document.querySelector('[role="dialog"]')&&document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true',{},{timeout:120000});}
 const context=await api('get_context',{selection:[]},null);projectId=context.project.id;assert.equal(projectId,expected.id);
 const project=await api('get_object',{object_id:{kind:'project',id:projectId},source:{kind:'draft'},node_index:null});assert.deepEqual(project.data,expected);
 const full=await api('get_run',{run_id:original.id,query:{kind:'full_result'}});assert.deepEqual(full.data,original);await write('context.json',context);
 await page.getByLabel('历史结果',{exact:true}).selectOption(original.id);await page.waitForFunction(id=>document.querySelector('canvas')?.readFactoryDiagnostics?.().run_id===id,original.id,{timeout:120000});
 const pallet=original.final_items.find(i=>i.item_type_ref.id==='PF-PALLET-loaded');assert(pallet);
 for(const name of ['manual-packing','pallet-stretch-wrapper']){
  const device=expected.devices.find(d=>d.id==='PF-'+name),job=original.jobs.find(j=>j.task_ref==='PF-TASK-PALLET'&&j.execution_ref.data.device_id===device.id);
  const work=original.phase_work_intervals.find(w=>w.job_id===job.id&&w.phase_id==='auto');assert(work);
  const snapshots=[];
  for(const fraction of [.13,.38,.9]){
   const at=Math.floor(work.interval.start_ms+(work.interval.end_ms-work.interval.start_ms)*fraction);let d=await seek(at);
   if(fraction===.13){await page.locator('.object-tree').getByRole('button',{name:'▣ '+device.title,exact:true}).click();await page.getByRole('button',{name:'聚焦选中对象',exact:true}).click();d=await seek(at);}
   assert.equal(d.asset_loading.devices.find(x=>x.id===device.id).phase,'auto');
   if(name==='pallet-stretch-wrapper'){const film=d.item_bounds['auxiliary-pallet-stretch-wrapper--normal'];assert(film&&film.max[1]<=.72&&film.min[1]>=.2,'Film must stay on the low carriage and actual one-layer stack');}
   for(const id of pallet.contents){const bounds=d.item_bounds[id];assert(bounds,`${name}: nested case missing ${id}`);assert(bounds.max.some((v,i)=>v-bounds.min[i]>.2));}
   snapshots.push({at_ms:at,diagnostics:d});await page.screenshot({path:path.join(out,name+'-'+fraction+'.png')});
  }
  if(name==='pallet-stretch-wrapper'){
   const id=pallet.contents[0],a=snapshots[0].diagnostics.item_bounds[id],b=snapshots[1].diagnostics.item_bounds[id];
   assert(Math.hypot(...a.min.map((v,i)=>(b.min[i]+b.max[i]-v-a.max[i])/2))>.02,'Nested case must rotate with pallet');
  }
  observations.push({device:device.id,snapshots});await write('observations.json',observations);
  console.log('Actual Native pallet phase visible',name);await page.getByRole('button',{name:'全厂适配',exact:true}).click();
 }
 const final=await seek(7200000),bounds=final.item_bounds[pallet.id];assert(bounds);
 const dims=bounds.max.map((v,i)=>v-bounds.min[i]).sort((a,b)=>a-b);assert(dims.every((v,i)=>Math.abs(v-[.36,.8,1.2][i])<.025),`actual pallet dimensions ${dims}`);
 const stocked=original.final_items.filter(i=>i.lifecycle==='available'&&i.holder.kind==='location'&&i.holder.id==='PF-SHIPPING');assert.equal(stocked.length,100);
 const overlaps=[];for(let a=0;a<stocked.length;a++)for(let b=a+1;b<stocked.length;b++){const x=final.item_bounds[stocked[a].id],y=final.item_bounds[stocked[b].id];assert(x&&y);if(x.min.every((v,i)=>Math.min(x.max[i],y.max[i])-Math.max(v,y.min[i])>.00001))overlaps.push([stocked[a].id,stocked[b].id]);}
 await write('shipping-bounds.json',{dims,overlaps});assert.deepEqual(overlaps,[]);
 await page.getByRole('button',{name:'工厂全景模式',exact:true}).click();await page.screenshot({path:path.join(out,'whole-factory-final.png')});await write('final-scene.json',final);
 const close=await invoke('project_close_state');assert.equal(close.active_tasks.length,0);assert.equal(close.execution_blockers.length,0);assert.deepEqual(errors,[]);
 await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'close-release.ps1'),'-NativeDirectory',directory],{windowsHide:true});
 await write('receipt.json',{status:'ordinary_Native_exact_saved_project_Run_pallet_loading_wrapping_and_shipping_verified',candidate:launch.exe,pid:launch.pid,project_id:projectId,run_id:original.id,actual_reopen_exact:true,new_devices:1,connected_devices:31,independent_cells:61,observations:observations.map(x=>({device:x.device,at_ms:x.snapshots.map(s=>s.at_ms)})),normal_close:true,errors,template_adoption_verified:false,whole_factory_accepted:false});process.exit(0);
}catch(error){await page.screenshot({path:path.join(out,'failure.png'),timeout:10000}).catch(()=>{});await write('failure.json',{error:String(error),errors,observations:observations.map(x=>({device:x.device,at_ms:x.snapshots.map(s=>s.at_ms)})),body:await page.locator('body').innerText().catch(()=>''),project_id:projectId});console.error(error);process.exit(1);}
