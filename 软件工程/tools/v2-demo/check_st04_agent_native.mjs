import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {spawn,execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {chromium} from 'file:///C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs';
const [base,attempt,output,resume]=process.argv.slice(2),dir=path.join(base,attempt),out=path.join(dir,output||'agent');await fs.mkdir(out);
const launch=JSON.parse((await fs.readFile(path.join(dir,'launch.json'),'utf8')).replace(/^\uFEFF/,''));
const browser=await chromium.connectOverCDP(launch.cdp),p=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://tauri.localhost/');assert(p);p.setDefaultTimeout(60000);
const invoke=(command,args)=>p.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const write=(name,value)=>fs.writeFile(path.join(out,name),JSON.stringify(value,null,2));
const info=await invoke('agent_connection_info'),before=await invoke('project_close_state');assert(before.project_id);assert(!before.unsaved);
if(resume){
 const previous=path.join(dir,resume),failure=JSON.parse(await fs.readFile(path.join(previous,'failure.json'),'utf8'));
 assert.match(failure.error,/locator.inputValue/); // Earlier ordered stdio, authority and save assertions completed.
 const task=JSON.parse(await fs.readFile(path.join(previous,'save-task.json'),'utf8'));
 assert(!before.active_tasks.includes(task.id)); // Original owner already observed completed before the selector-only failure.
 const goal=p.getByRole('textbox',{name:'用一句话说明产品、目标和场地',exact:true});assert.equal(await goal.inputValue(),'ST04 保留未提交目标');
 await p.getByRole('button',{name:'关闭工作台',exact:true}).click();await p.getByRole('alertdialog',{name:'未提交Agent输入'}).waitFor();await p.screenshot({path:path.join(out,'unsaved-input-leave-guard.png')});await p.getByRole('button',{name:'保留输入，返回',exact:true}).click();assert.equal(await goal.inputValue(),'ST04 保留未提交目标');
 await goal.fill('');await p.getByRole('button',{name:'关闭工作台',exact:true}).click();await p.locator('.agent-workspace').waitFor({state:'hidden'});
 const saved=path.join(previous,'saved-project');assert((await fs.readdir(saved)).includes('project.json'));
 const after=await invoke('project_close_state');assert(!after.unsaved);assert.deepEqual(after.active_tasks,[]);
 await write('receipt.json',{status:'real_stdio_UI_authority_and_OS_directory_save_verified',pid:launch.pid,prior_ordered_test:previous,resumed_selector_failure:failure.error,connection_requires_UI_authorization:true,authorized_read:true,unauthorized_write_rejected:true,UI_directory_and_real_save:true,save_task:task.id,save_completed:true,unsaved_input_preserved:true,revocation_retested:false,client_exited:true,project_id:before.project_id,revision:before.project_revision,saved});
 await browser.close();console.log('Original stdio save confirmed; actual unsaved-input leave guard passed.');process.exit(0);
}
const child=spawn('F:/SuperFactory-v2-temp/01a1731e/retained-root-target/debug/superfactory-mcp-bridge.exe',['--host-pipe',info.endpoint,'--host-instance-id',info.host_instance_id],{windowsHide:true,stdio:['pipe','pipe','pipe']});
let buffer='',sequence=0;const pending=new Map();child.stderr.on('data',()=>{});
child.stdout.on('data',chunk=>{buffer+=chunk;let end;while((end=buffer.indexOf('\n'))>=0){const line=buffer.slice(0,end);buffer=buffer.slice(end+1);if(!line.trim())continue;const reply=JSON.parse(line);const item=pending.get(reply.id);if(item){pending.delete(reply.id);clearTimeout(item.timer);item.resolve(reply);}}});
function rpc(method,params){const id=++sequence;return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{pending.delete(id);reject(new Error('MCP request timed out: '+method));},60000);pending.set(id,{resolve,timer});child.stdin.write(JSON.stringify({jsonrpc:'2.0',id,method,params})+'\n');});}
async function call(operation,params){const response=await rpc('tools/call',{name:'superfactory_'+operation,arguments:{api_version:'0.1',request_id:'st04-agent-'+crypto.randomUUID(),project_id:before.project_id,scope:{kind:'project'},operation,params}});assert(!response.error,JSON.stringify(response.error));const result=response.result;const raw=result.content?.find(c=>c.type==='resource'&&c.resource.mimeType==='application/json');return raw?JSON.parse(raw.resource.text):result.structuredContent;}
async function picker(target){await fs.mkdir(target);const exec=promisify(execFile),pwsh='C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe';let dialog;const end=Date.now()+15000;while(Date.now()<end){const r=await exec(pwsh,['-NoProfile','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});if(r.stdout.trim()){dialog=JSON.parse(r.stdout);if(dialog.controls.some(c=>c.class==='Edit'&&c.id===1152))break;}await new Promise(r=>setTimeout(r,100));}assert(dialog);const edit=dialog.controls.find(c=>c.class==='Edit'&&c.id===1152),button=dialog.controls.find(c=>c.class==='Button'&&c.id===1);assert(edit&&button);await exec(pwsh,['-NoProfile','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe,'-SelectDirectory',target,'-EditHandle',String(edit.handle),'-ButtonHandle',String(button.handle)],{windowsHide:true});}
try{
 if(await p.locator('.cloud-production').count())await p.getByRole('button',{name:'返回画布',exact:true}).click();
 if(!await p.locator('.agent-workspace').count()){await p.getByText('Agent',{exact:true}).click();await p.getByRole('button',{name:'连接、权限与任务…',exact:true}).click();}
 const select=p.getByRole('combobox',{name:'本机已连接的客户端',exact:true});await select.locator('option').filter({hasText:'进程 '+child.pid}).waitFor({state:'attached'});const session=await select.locator('option').filter({hasText:'进程 '+child.pid}).getAttribute('value');await select.selectOption(session);
 await p.getByRole('button',{name:'确认并授权此连接',exact:true}).click();await p.getByRole('button',{name:'提交新的授权范围',exact:true}).waitFor();
 const initialized=await rpc('initialize',{protocolVersion:'2024-11-05',capabilities:{},clientInfo:{name:'st04-native-reception',version:'1'}});assert(initialized.result);child.stdin.write(JSON.stringify({jsonrpc:'2.0',method:'notifications/initialized'})+'\n');
 
 const context=await call('get_context',{selection:[]});assert.equal(context.outcome.status,'ok');assert.equal(context.outcome.data.project.id,before.project_id);
 const deniedWrite=await call('apply_commands',{base_revision:before.project_revision,commands:[{kind:'undo'}]});assert.equal(deniedWrite.outcome.status,'error');assert.equal(deniedWrite.outcome.data.code,'PermissionDenied');
 await p.getByLabel('保存工程',{exact:true}).check();await p.getByRole('button',{name:'提交新的授权范围',exact:true}).click();await p.waitForFunction(()=>!document.querySelector('.agent-workspace fieldset')?.disabled);
 await p.getByRole('button',{name:'选择此连接可使用的目录',exact:true}).click();const saved=path.join(out,'saved-project');await picker(saved);
 const details=p.locator('details').filter({has:p.locator('summary',{hasText:'目录授权详情'})});await details.locator('pre').waitFor({state:'attached'});const directory=JSON.parse(await details.locator('pre').textContent());
 const save=await call('save_project',{revision:before.project_revision,selected_directory_ref:directory});assert.equal(save.outcome.status,'ok');const task=save.outcome.data;await write('save-task.json',task);
 const taskId=task.id;assert(taskId);let finished;const end=Date.now()+120000;while(Date.now()<end){const r=await call('get_task',{task_id:taskId});assert.equal(r.outcome.status,'ok');finished=r.outcome.data.task;if(['completed','failed','cancelled'].includes(finished.state))break;await new Promise(r=>setTimeout(r,250));}assert.equal(finished.state,'completed',JSON.stringify(finished));assert((await fs.readdir(saved)).length>0);
 await p.getByRole('textbox',{name:'用一句话说明产品、目标和场地',exact:true}).fill('ST04 保留未提交目标');await p.getByRole('button',{name:'关闭工作台',exact:true}).click();await p.getByRole('alertdialog',{name:'未提交Agent输入'}).waitFor();await p.getByRole('button',{name:'保留输入，返回',exact:true}).click();assert.equal(await p.getByRole('textbox',{name:'用一句话说明产品、目标和场地',exact:true}).inputValue(),'ST04 保留未提交目标');
 await p.screenshot({path:path.join(out,'authorized-and-dirty-preserved.png')});await p.getByRole('textbox',{name:'用一句话说明产品、目标和场地',exact:true}).fill('');await p.getByRole('button',{name:'撤销此连接',exact:true}).click();
 await p.waitForFunction(async session=>{const info=await window.__TAURI_INTERNALS__.invoke('agent_connection_info');return !info.connections.some(c=>c.session_id===session&&!c.revoked);},session);await p.getByRole('button',{name:'关闭工作台',exact:true}).click();await p.locator('.agent-workspace').waitFor({state:'hidden'});
 const after=await invoke('project_close_state');assert.equal(after.project_revision,before.project_revision);assert(!after.unsaved);
 await write('receipt.json',{status:'real_stdio_UI_authority_and_OS_directory_save_verified',pid:launch.pid,client_pid:child.pid,connection_requires_UI_authorization:true,authorized_read:true,unauthorized_write_rejected:true,UI_directory_and_real_save:true,unsaved_input_preserved:true,revocation_verified:true,project_id:before.project_id,revision:before.project_revision,saved});console.log('Actual Agent stdio, Native UI authority, OS directory and save passed.');
}catch(error){await write('failure.json',{error:String(error),stack:error.stack});await p.screenshot({path:path.join(out,'failure.png')}).catch(()=>{});throw error;}finally{child.stdin.end();await browser.close();}
