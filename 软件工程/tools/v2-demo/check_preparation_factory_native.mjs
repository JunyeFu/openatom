// Open the actual Host-produced project in the ordinary owned Native application.
import fs from 'node:fs/promises';import path from 'node:path';import assert from 'node:assert/strict';
import {execFile} from 'node:child_process';import {promisify} from 'node:util';import {pathToFileURL} from 'node:url';
const [base,attempt,proof,resume='']=process.argv.slice(2),directory=path.join(base,attempt),out=path.join(directory,'preparation'+(resume?'-'+resume:''));await fs.mkdir(out);
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
 for(const name of ['paper-unwinder','paper-rewinder','rotary-sheet-cutter','board-edge-trimmer','sheet-slitter','varnish-coater','uv-curer']){
  const device=expected.devices.find(d=>d.id==='PF-'+name),job=original.jobs.find(j=>j.execution_ref.kind==='project_device'&&j.execution_ref.data.device_id===device.id);
  const work=original.phase_work_intervals.find(w=>w.job_id===job.id&&w.phase_id==='auto');assert(work);
  const at=Math.floor((work.interval.start_ms+work.interval.end_ms)/2);let d=await seek(at);
  const machine=d.asset_loading.devices.find(d=>d.id===device.id);assert.equal(machine.phase,'auto');assert(machine.progress>0&&machine.progress<1);
  await page.locator('.object-tree').getByRole('button',{name:'▣ '+device.title,exact:true}).click();await page.getByRole('button',{name:'聚焦选中对象',exact:true}).click();await page.waitForFunction(()=>{const d=document.querySelector('canvas')?.readFactoryDiagnostics?.();return d?.asset_loading.pending_devices===0&&d.asset_loading.items?.pending===0;},{},{timeout:120000});
  d=await page.evaluate(()=>document.querySelector('canvas').readFactoryDiagnostics());
  const primaryType=device.role_bindings.item_types.input;
  const inputs=job.input_refs.filter(id=>original.final_items.find(i=>i.id===id)?.item_type_ref.id===primaryType);assert(inputs.length);
  for(const id of inputs){const b=d.item_bounds[id];assert(b,`${name}: missing ${id}`);assert(b.max.some((v,i)=>v-b.min[i]>.01),`${name}: collapsed ${id}`);}
  await page.screenshot({path:path.join(out,name+'.png')});observations.push({device:device.id,at_ms:at,inputs,diagnostics:d});await write('observations.json',observations);
  console.log('Actual Native phase and material visible',name,at);await page.getByRole('button',{name:'全厂适配',exact:true}).click();
 }
 const final=await seek(7200000);await page.getByRole('button',{name:'工厂全景模式',exact:true}).click();await page.screenshot({path:path.join(out,'whole-factory-final.png')});await write('final-scene.json',final);
 const close=await invoke('project_close_state');assert.equal(close.active_tasks.length,0);assert.equal(close.execution_blockers.length,0);assert.deepEqual(errors,[]);
 await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'close-release.ps1'),'-NativeDirectory',directory],{windowsHide:true});
 await write('receipt.json',{status:'ordinary_Native_exact_saved_project_Run_and_seven_new_device_material_phases_verified',candidate:launch.exe,pid:launch.pid,project_id:projectId,run_id:original.id,actual_reopen_exact:true,new_devices:7,connected_devices:26,independent_cells:66,observations:observations.map(x=>({device:x.device,at_ms:x.at_ms})),normal_close:true,errors,template_adoption_verified:false,whole_factory_accepted:false});process.exit(0);
}catch(error){await page.screenshot({path:path.join(out,'failure.png'),timeout:10000}).catch(()=>{});await write('failure.json',{error:String(error),errors,observations:observations.map(x=>({device:x.device,at_ms:x.at_ms})),body:await page.locator('body').innerText().catch(()=>''),project_id:projectId});console.error(error);process.exit(1);}
