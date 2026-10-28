// Independent ordinary Native reopening of the UI-created factory and old Run.
import fs from 'node:fs/promises';import path from 'node:path';import assert from 'node:assert/strict';
import {execFile} from 'node:child_process';import {promisify} from 'node:util';import {createRequire} from 'node:module';
const [base,attempt,previous]=process.argv.slice(2),resume=process.argv[5]==='resume',directory=path.join(base,attempt),out=path.join(directory,resume?'reopen-resumed':'reopen');await fs.mkdir(out);
const read=async file=>JSON.parse((await fs.readFile(file,'utf8')).replace(/^\uFEFF/,''));
const launch=await read(path.join(directory,'launch.json')),saved=await read(path.join(base,previous,'save/receipt.json')),baseline=await read(path.join(base,'st03-template-02/flow/receipt.json')),terminal=(await read(path.join(base,previous,'factory-live/terminal.json'))).snapshot,prior={...baseline,saved:saved.saved};
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
 const original=await read(path.join(base,'st03-template-02/flow/actual-run.json'));
 const full=await api('get_run',{run_id:prior.run_id,query:{kind:'full_result'}});assert.equal(full.kind,'full');assert.deepEqual(full.data,original);
 const execution=await api('get_execution_snapshot',{execution_contract_version:'sf.execution/2.0-draft11',identity:terminal.identity,selection:{kind:'latest'}});
 assert.deepEqual(execution.snapshot,terminal);assert.notEqual(execution.access.ownership,'current_host_owner');assert.deepEqual(execution.access.allowed_session_controls,[]);
 const close=await invoke('project_close_state');assert.equal(close.unsaved,false);assert.deepEqual(close.execution_blockers,[]);assert.deepEqual(close.active_tasks,[]);
 await write('receipt.json',{status:'new_process_restores_exact_factory_execution_and_original_Run',pid:launch.pid,project_id:projectId,run_id:prior.run_id,identity:terminal.identity,R:terminal.execution_revision,read_only:true,source:prior.saved,errors});assert.deepEqual(errors,[]);
 await exec(pwsh,['-NoProfile','-File',path.join(base,'close-release.ps1'),'-NativeDirectory',directory],{windowsHide:true});
}catch(error){await write('failure.json',{error:String(error),stack:error.stack});throw error;}finally{await browser.close();}
