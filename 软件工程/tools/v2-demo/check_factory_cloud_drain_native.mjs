import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {chromium} from 'file:///C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs';
const [base,attempt,resume='']=process.argv.slice(2),output=resume?'factory-live-'+resume:'factory-live',dir=path.join(base,attempt),out=path.join(dir,output||(resume?'connection-'+resume:'connection'));await fs.mkdir(out);
const read=async f=>JSON.parse((await fs.readFile(f,'utf8')).replace(/^\uFEFF/,''));
const launch=await read(path.join(dir,'launch.json')),prior=await read('F:/SuperFactory-v2-temp/01a1731e/full-factory/factory-native-1_6-standalone/st03-template-02/flow/receipt.json');
const browser=await chromium.connectOverCDP(launch.cdp),p=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://tauri.localhost/');assert(p);p.setDefaultTimeout(120000);
const exec=promisify(execFile),pwsh='C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe';
const write=(n,v)=>fs.writeFile(path.join(out,n),JSON.stringify(v,null,2));
async function picker(target){
 let dialog;const deadline=Date.now()+15000;
 do{const r=await exec(pwsh,['-NoProfile','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});if(r.stdout.trim()){const d=JSON.parse(r.stdout);if(d.controls.some(c=>c.class==='Edit'&&[1148,1152].includes(c.id))&&d.controls.some(c=>c.id===1001&&c.class==='ToolbarWindow32')){dialog=d;break;}}await new Promise(r=>setTimeout(r,100));}while(Date.now()<deadline);
 assert(dialog);const edit=dialog.controls.find(c=>c.class==='Edit'&&[1148,1152].includes(c.id)),button=dialog.controls.find(c=>c.class==='Button'&&c.id===1);assert(button);
 await exec(pwsh,['-NoProfile','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe,'-SelectDirectory',target,'-EditHandle',String(edit.handle),'-ButtonHandle',String(button.handle)],{windowsHide:true});
}
let identity;const events=[];
const invoke=(command,args)=>p.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const api=async(operation,params={},projectId=prior.project_id)=>{const r=await invoke('project_api',{request:{api_version:'0.1',request_id:'factory-cloud-'+crypto.randomUUID(),project_id:projectId,scope:{kind:projectId?'project':'workspace'},operation,params}});assert.equal(r.outcome.status,'ok',JSON.stringify(r.outcome));return r.outcome.data;};
const snapshot=()=>api('get_execution_snapshot',{execution_contract_version:'sf.execution/2.0-draft11',identity,selection:{kind:'latest'}});
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
try{
 if(!resume){
 await p.getByText('文件',{exact:true}).click();await p.getByRole('button',{name:'打开工程目录…',exact:true}).click();await picker(prior.saved);
 await p.getByRole('button',{name:'确认打开',exact:true}).click();
 await p.waitForFunction(()=>!document.querySelector('[role="dialog"]')&&document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true',{},{timeout:180000});
 await p.getByRole('button',{name:'云端生产',exact:true}).click();await p.getByRole('tab',{name:'执行',exact:true}).click();
 }
 if(!resume || resume==='form'){
 await p.getByLabel('最终目标数量',{exact:true}).fill('167');await p.getByRole('combobox',{name:'目标单位',exact:true}).selectOption('piece');
 await p.getByLabel('停止新派工时间（秒）',{exact:true}).fill('7200');await p.getByLabel('设备状态有效期（秒）',{exact:true}).fill('30');
 await p.waitForFunction(()=>document.querySelectorAll('.cloud-bindings input[type=checkbox]').length===93);
 await p.getByRole('button',{name:'选择全部设备',exact:true}).click();await p.waitForFunction(()=>document.querySelectorAll('.cloud-bindings input[type=checkbox]:checked').length===93);
 await write('prepare-requested.json',{target:167,devices:93,stages:944,cutoff_ms:7200000,software_demo_speed:30,minimum_phase_ms:1000,real_Host_clock:true,pid:launch.pid});
 await p.getByRole('button',{name:'准备生产',exact:true}).click();
 }
 await p.waitForFunction(()=>{const e=[...document.querySelectorAll('.cloud-controls button')].find(e=>e.textContent==='启动');return e&&!e.disabled;},{},{timeout:600000});
 identity=await p.locator('.cloud-source dl').evaluate(e=>Object.fromEntries([...e.querySelectorAll(':scope > div')].map(x=>[x.querySelector('dt').textContent,x.querySelector('dd').textContent])));
 await write('identity.json',identity);await write('prepared.json',await snapshot());
 await p.getByRole('button',{name:'启动',exact:true}).click();console.log('Actual whole-factory cloud production started',JSON.stringify(identity));
 let last;const startedDeadline=Date.now()+300000;
 while(Date.now()<startedDeadline){
  last=await snapshot();const s=last.snapshot;
  if(s.attempts.some(a=>a.attempt.phase_id==='load'&&a.started_at_host_ms!==null))break;
  await sleep(500);
 }
 assert(last.snapshot.attempts.some(a=>a.attempt.phase_id==='load'&&a.started_at_host_ms!==null),'An actual multistage job must start before stopping');
 const startedJobs=last.snapshot.bound_jobs.filter(j=>j.started_at_host_ms!==null).map(j=>j.job_id);assert(startedJobs.length>0);
 await write('before-stop.json',last);await p.getByRole('button',{name:'停止新派工',exact:true}).click();
 const deadline=Date.now()+600000;
 while(Date.now()<deadline){
  last=await snapshot();const s=last.snapshot;
  const summary={at:new Date().toISOString(),state:s.lifecycle.state,T:s.coverage.observed_until_host_ms,R:s.execution_revision,attempts:s.attempts.length,cloud_revision:last.delivery.cloud_published_revision,unresolved:s.lifecycle.unresolved_command_ids.length};
  events.push(summary);await write('progress.json',events);console.log(JSON.stringify(summary));
  if(['stopped','completed','failed','needs_resolution'].includes(s.lifecycle.state)){await write('terminal.json',last);break;}
  await sleep(2000);
 }
 assert.equal(last.snapshot.lifecycle.state,'stopped','Started jobs must drain without being stranded by the device command gate');
 assert(startedJobs.every(id=>last.snapshot.bound_jobs.some(j=>j.job_id===id&&j.state==='completed')),'Every job already started before stop must finish');
 assert(last.snapshot.attempts.some(a=>a.attempt.phase_id==='unload'&&a.completed_at_host_ms!==null),'Original multistage job must reach its actual unload receipt');
 assert.equal(last.snapshot.lifecycle.unresolved_command_ids.length,0);assert.equal(last.snapshot.pending_reservations.length,0);
 let close;const cleanupDeadline=Date.now()+120000;
 do {close=await invoke('project_close_state');if(!close.active_tasks.length&&!close.execution_blockers.length)break;await sleep(1000);}while(Date.now()<cleanupDeadline);
 assert.equal(close.active_tasks.length,0);assert.equal(close.execution_blockers.length,0);
 await write('receipt.json',{status:'actual_93_endpoint_multistage_stop_drain_and_cleanup_verified',identity,terminal_revision:last.snapshot.execution_revision,terminal_ms:last.snapshot.coverage.observed_until_host_ms,commands:last.snapshot.attempts.length,started_jobs_before_stop:startedJobs,close,software_in_loop:true,whole_factory_order_completion:false});
 console.log('93 endpoint started-work drain and original cleanup completed; the full 167-unit order remains separate.');
}catch(e){await write('failure.json',{error:String(e),stack:e.stack,identity});if(identity)await write('last-snapshot.json',await snapshot()).catch(()=>{});await p.screenshot({path:path.join(out,'failure.png')}).catch(()=>{});throw e;}finally{await browser.close();}
