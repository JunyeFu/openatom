import fs from 'node:fs/promises';import path from 'node:path';import {execFile} from 'node:child_process';import {promisify} from 'node:util';import {pathToFileURL} from 'node:url';import assert from 'node:assert/strict';
const [base,attempt='native-01',resume='']=process.argv.slice(2);assert(base);const native=path.join(base,attempt),out=path.join(native,resume?'live-phases-'+resume:'live-phases');await fs.mkdir(out);
const read=async p=>JSON.parse((await fs.readFile(p,'utf8')).replace(/^\uFEFF/,''));const write=(name,value)=>fs.writeFile(path.join(out,name),JSON.stringify(value,null,2));
const launch=await read(path.join(native,'launch.json'));const {chromium}=await import(pathToFileURL('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs').href);
const b=await chromium.connectOverCDP(launch.cdp),p=b.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://tauri.localhost/');assert(p);p.setDefaultTimeout(20000);
const errors=[],events=[];p.on('pageerror',e=>errors.push(String(e)));p.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
const event=async(name,facts={})=>{events.push({at:new Date().toISOString(),name,...facts});await write('progress.json',events);console.log(name);};
const invoke=(command,args)=>p.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const api=async(operation,params={},projectId='Task8-two-device-six-units')=>{const r=await invoke('project_api',{request:{api_version:'0.1',request_id:'live-phase-'+crypto.randomUUID(),project_id:projectId,scope:{kind:projectId?'project':'workspace'},operation,params}});assert.equal(r.outcome.status,'ok',JSON.stringify(r.outcome));return r.outcome.data;};
let identity=null;const snapshot=revision=>api('get_execution_snapshot',{execution_contract_version:'sf.execution/2.0-draft11',identity,selection:revision===undefined?{kind:'latest'}:{kind:'revision',execution_revision:revision}});
const sleep=ms=>new Promise(r=>setTimeout(r,ms));const capture=name=>p.screenshot({path:path.join(out,name+'.png')});
const openView=async()=>{await p.getByRole('tab',{name:'执行',exact:true}).click();await p.getByRole('button',{name:'回看执行画布与时间轴',exact:true}).click();await p.locator('.execution-view-scene[aria-busy="false"]').waitFor({timeout:60000});assert.equal(await p.locator('.execution-view-error,.scene-error').count(),0);await p.waitForFunction(()=>document.querySelector('.execution-view-scene canvas')?.readFactoryDiagnostics?.()?.asset_loading.pending_devices===0);return p.evaluate(()=>document.querySelector('.execution-view-scene canvas').readFactoryDiagnostics());};
const back=()=>p.getByRole('button',{name:'返回云端生产',exact:true}).click();
const versions=async()=>{const d=p.locator('.cloud-version-options');if(!await d.evaluate(e=>e.open))await d.locator('summary').click();};
const follow=async()=>{await versions();await p.getByRole('button',{name:'跟随本机会话',exact:true}).click();await p.waitForFunction(()=>document.querySelector('.cloud-source')?.textContent.includes('当前本机会话'));};
try{
 await p.setViewportSize({width:1920,height:1080});const initial=await api('get_context',{selection:[]},null);await write('initial-context.json',initial);
 await p.getByRole('tab',{name:'执行',exact:true}).click();
 if(!resume){
 await p.getByLabel('最终目标数量',{exact:true}).fill('12');await p.getByRole('combobox',{name:'目标单位',exact:true}).selectOption('piece');
 await p.getByLabel('停止新派工时间（秒）',{exact:true}).fill('240');await p.getByLabel('设备状态有效期（秒）',{exact:true}).fill('5');
 const bindings=p.locator('.cloud-bindings input[type=checkbox]');assert.equal(await bindings.count(),4);for(const input of await bindings.all())await input.check();
 await write('prepare-requested.json',{pid:launch.pid,target:12,cutoff_ms:240000,max_age_ms:5000,actual_UI_click_once:true});
 await p.getByRole('button',{name:'准备生产',exact:true}).click();
 }
 await p.waitForFunction(()=>{const e=[...document.querySelectorAll('.cloud-controls button')].find(e=>e.textContent==='启动');return e&&!e.disabled;},{},{timeout:60000});
 identity=await p.locator('.cloud-source dl').evaluate(e=>Object.fromEntries([...e.querySelectorAll(':scope > div')].map(x=>[x.querySelector('dt').textContent,x.querySelector('dd').textContent])));
 const prepared=await snapshot();assert.equal(prepared.snapshot.policy.report_cutoff_ms,240000);assert.equal(prepared.snapshot.policy.observation_max_age_ms,5000);assert.equal(prepared.access.ownership,'current_host_owner');await write('prepared.json',prepared);
 await p.getByRole('button',{name:'启动',exact:true}).click();await event('Actual UI started the twelve-piece two-stage production once',{identity});
 let sample,measured;const reportDeadline=Date.now()+60000;
 while(Date.now()<reportDeadline){const latest=await snapshot();const published=latest.delivery.cloud_published_revision;if(published!==null){const candidate=await snapshot(published);const d=candidate.snapshot.devices.find(d=>d.binding.device_id==='L1-D01'&&d.latest_observation?.observation.phase_progress?.effective_work_ms>0&&d.latest_observation.observation.phase_progress.effective_work_ms<d.latest_observation.observation.phase_progress.duration_ms);if(d&&candidate.snapshot.bound_jobs.some(j=>j.state==='working'&&j.execution_ref.kind==='project_device'&&j.execution_ref.data.device_id===d.binding.device_id)){sample=candidate;measured=d;break;}}await sleep(300);}
 assert(sample&&measured,'No actual admitted, published active phase report arrived; retain this same live Native.');
 const report=measured.latest_observation.observation.phase_progress;await write('actual-published-active-snapshot.json',sample);
 await p.getByRole('tab',{name:'设备',exact:true}).click();const row=p.locator('.cloud-tab-content tbody tr').filter({hasText:'粗加工 A'});assert.equal(await row.count(),1);
 await row.getByRole('button',{name:'模拟设备维护',exact:true}).click();const maintenanceAt=(await snapshot()).snapshot.coverage.observed_until_host_ms;await event('Actual device-side maintenance requested; operator acceptance remains enabled',{device:'L1-D01',at_ms:maintenanceAt});
 await p.getByRole('tab',{name:'执行',exact:true}).click();await versions();await p.getByLabel('记录版本',{exact:true}).fill(String(sample.snapshot.execution_revision));await p.getByRole('button',{name:'读取这个版本',exact:true}).click();await p.waitForFunction(r=>document.querySelector('.cloud-source')?.textContent.includes('第 '+r+' 版'),sample.snapshot.execution_revision);
 let host=await openView();const hostDevice=host.asset_loading.devices.find(d=>d.id===measured.binding.device_id);assert(hostDevice);assert.equal(host.execution_source.read_origin,'host');assert.equal(host.execution_source.execution_revision,sample.snapshot.execution_revision);assert.equal(hostDevice.phase,report.phase_id);assert.equal(hostDevice.effective_ms,report.effective_work_ms);assert.equal(hostDevice.duration_ms,report.duration_ms);assert(Math.abs(hostDevice.progress-report.effective_work_ms/report.duration_ms)<1e-12);await write('host-render-diagnostics.json',host);await capture('01-host-actual-phase');
 await sleep(1200);const held=await p.evaluate(()=>document.querySelector('.execution-view-scene canvas').readFactoryDiagnostics());assert.equal(held.asset_loading.devices.find(d=>d.id===measured.binding.device_id).effective_ms,report.effective_work_ms);assert((await snapshot()).snapshot.coverage.observed_until_host_ms>sample.snapshot.coverage.observed_until_host_ms);
 await event('Current Native rig uses the exact device report and holds it while actual production continues',{R:sample.snapshot.execution_revision,phase:report.phase_id,effective_ms:report.effective_work_ms,duration_ms:report.duration_ms});
 await back();await p.getByRole('button',{name:'云端存档',exact:true}).click();await p.waitForFunction(()=>document.querySelector('.cloud-source')?.textContent.includes('云端存档（只读）'));
 const cloud=await openView(),cloudDevice=cloud.asset_loading.devices.find(d=>d.id===measured.binding.device_id);assert.equal(cloud.execution_source.read_origin,'cloud');assert.equal(cloud.execution_source.execution_revision,sample.snapshot.execution_revision);assert.equal(cloudDevice.effective_ms,report.effective_work_ms);assert.equal(cloudDevice.duration_ms,report.duration_ms);await write('cloud-render-diagnostics.json',cloud);await capture('02-cloud-actual-phase');await event('Actual HTTPS cloud archive and Host produce the identical reported phase in the ordinary Native scene');
 await back();await follow();let maintenanceObserved=false,alternate=false,recovered=false,reused=false,last;
 const end=Date.now()+300000;
 while(Date.now()<end){last=await snapshot();const s=last.snapshot,a=s.devices.find(d=>d.binding.device_id==='L1-D01');
  if(a?.latest_observation?.observation.accept_new_work===false&&!maintenanceObserved){maintenanceObserved=true;await write('maintenance-observed.json',last);await event('Host admitted the device maintenance observation without changing the operator switch');}
  if(maintenanceObserved&&!recovered&&s.attempts.some(x=>x.device_id==='L1-D02'&&x.started_at_host_ms!==null&&x.started_at_host_ms>maintenanceAt)){alternate=true;}
  if(!recovered&&s.coverage.observed_until_host_ms>=maintenanceAt+20000){await p.getByRole('tab',{name:'设备',exact:true}).click();await row.getByRole('button',{name:'恢复模拟设备',exact:true}).click();recovered=true;await event('Actual UI requested device recovery after the maintenance interval');}
  if(recovered&&a?.latest_observation?.observation.accept_new_work&&s.attempts.some(x=>x.device_id==='L1-D01'&&x.started_at_host_ms!==null&&x.started_at_host_ms>maintenanceAt+20000))reused=true;
  if(['completed','stopped','failed'].includes(s.lifecycle.state)){await write('terminal.json',last);break;}
  await sleep(500);
 }
 assert(last&&last.snapshot.lifecycle.state==='completed','The original running session has not completed; retain its process and journal.');assert(maintenanceObserved&&alternate&&recovered&&reused);
 const s=last.snapshot;assert.equal(s.attempts.length,24);assert(s.attempts.every(a=>a.state==='accepted'));const accepted=s.business_facts.acceptance_events.reduce((n,e)=>n+e.quantity.value.coefficient/10**e.quantity.value.decimal_places,0);assert.equal(accepted,12);
 await p.getByRole('tab',{name:'执行',exact:true}).click();await follow();await p.waitForFunction(()=>document.querySelector('.cloud-facts')?.textContent.includes('已完成收尾'));
 await capture('03-actual-completed');await event('Actual two-stage production completed twelve pieces and24 accepted commands; alternate dispatch and recovered-device reuse observed',{accepted,commands:s.attempts.length,T:s.coverage.observed_until_host_ms});
 const closeDeadline=Date.now()+60000;
 let closeState;
 do {
  closeState=await invoke('project_close_state');
  if(!closeState.active_tasks.length&&!closeState.execution_blockers.length)break;
  await new Promise(resolve=>setTimeout(resolve,1000));
 } while(Date.now()<closeDeadline);
 assert.equal(closeState.active_tasks.length,0);
 assert.equal(closeState.execution_blockers.length,0);
 assert.deepEqual(errors,[]);await write('receipt.json',{status:'actual_SDK_HTTPS_cloud_Host_Native_phase_and_maintenance_production_verified',pid:launch.pid,identity,events,errors,accepted:12,commands:24,terminal_revision:s.execution_revision,terminal_ms:s.coverage.observed_until_host_ms,maintenance_observed:true,alternate_dispatch_observed:true,recovered_device_reused:true,reported_phase:report,Host_and_cloud_same_report:true,no_Host_clock_interpolation:true,source_linked_resources:true,standalone_package_verified:false,whole_factory_accepted:false,close:await invoke('project_close_state')});
 console.log('Live production/phase verification completed; save the new archive before normal Native close.');
}catch(error){await write('failure.json',{error:String(error),stack:error.stack,pid:launch.pid,identity,events,errors});if(identity)await write('last-live-snapshot.json',await snapshot()).catch(()=>{});await capture('failure').catch(()=>{});throw error;}finally{await b.close();}
