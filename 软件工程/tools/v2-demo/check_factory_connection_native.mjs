import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {chromium} from 'file:///C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs';
const [base,attempt,privateDir]=process.argv.slice(2),resume='',output='connection',connectionFile=path.join(privateDir,'connection.json'),dir=path.join(base,attempt),out=path.join(dir,output||(resume?'connection-'+resume:'connection'));await fs.mkdir(out);
const read=async f=>JSON.parse((await fs.readFile(f,'utf8')).replace(/^\uFEFF/,''));
const launch=await read(path.join(dir,'launch.json')),prior=await read('F:/SuperFactory-v2-temp/01a1731e/full-factory/factory-native-1_6-standalone/st03-template-02/flow/receipt.json');
const browser=await chromium.connectOverCDP(launch.cdp),p=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://tauri.localhost/');assert(p);p.setDefaultTimeout(120000);
const exec=promisify(execFile),pwsh='C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe';
const write=(n,v)=>fs.writeFile(path.join(out,n),JSON.stringify(v,null,2));
async function picker(target){
 let dialog;const deadline=Date.now()+15000;
 do{const r=await exec(pwsh,['-NoProfile','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});if(r.stdout.trim()){const d=JSON.parse(r.stdout);if(d.controls.some(c=>c.class==='Edit'&&[1148,1152].includes(c.id))){dialog=d;break;}}await new Promise(r=>setTimeout(r,100));}while(Date.now()<deadline);
 assert(dialog);const edit=dialog.controls.find(c=>c.class==='Edit'&&[1148,1152].includes(c.id)),button=dialog.controls.find(c=>c.class==='Button'&&c.id===1);assert(button);
 await exec(pwsh,['-NoProfile','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe,'-SelectDirectory',target,'-EditHandle',String(edit.handle),'-ButtonHandle',String(button.handle)],{windowsHide:true});
}
try{
 if(!resume){await p.getByText('文件',{exact:true}).click();await p.getByRole('button',{name:'打开工程目录…',exact:true}).click();await picker(prior.saved);}if(!['cloud','valid','valid2'].includes(resume))await p.getByRole('button',{name:'确认打开',exact:true}).click({timeout:120000});
 await p.waitForFunction(()=>document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true',{},{timeout:120000});
 if(!['cloud','valid','valid2'].includes(resume))await p.getByRole('button',{name:'云端生产',exact:true}).click();await p.getByRole('tab',{name:'设备',exact:true}).click();
 for(const caseName of ['model','port','unit']) {
 await p.waitForFunction(async()=>{const s=await window.__TAURI_INTERNALS__.invoke('project_close_state');return !s.active_tasks.length&&!s.execution_blockers.length;});
 await p.getByRole('button',{name:'导入连接文件…',exact:true}).click();await picker(path.join(privateDir,'wrong-'+caseName+'.json'));
 await p.locator('.cloud-alert').filter({hasText:'设备绑定与当前工厂方案不一致'}).waitFor();
 await p.screenshot({path:path.join(out,'wrong-'+caseName+'-rejected.png')});
 }
 await p.waitForFunction(async()=>{const s=await window.__TAURI_INTERNALS__.invoke('project_close_state');return !s.active_tasks.length&&!s.execution_blockers.length;});await p.getByRole('button',{name:'导入连接文件…',exact:true}).click();await picker(connectionFile);
 await p.getByText(/已校验并保存 93 台设备的连接/).waitFor();
 assert(!(await p.locator('.cloud-production').innerText()).includes('reader_token'));
 await p.screenshot({path:path.join(out,'connection-imported.png')});
 const state=await p.evaluate(()=>window.__TAURI_INTERNALS__.invoke('project_close_state'));assert(!state.unsaved);assert.equal(state.active_tasks.length,0);assert.equal(state.execution_blockers.length,0);
 await write('receipt.json',{status:'ordinary_Native_OS_file_import_verified',wrong_model_port_unit_rejected:true,valid_device_count:93,valid_stage_bindings:944,credentials_remain_Native:true,new_configuration_requires_restart:true,project_id:prior.project_id,source_archive:prior.saved});
 await exec(pwsh,['-NoProfile','-File',path.join(base,'close-release.ps1'),'-NativeDirectory',dir],{windowsHide:true});console.log('Connection import and rejection passed; Native normally closed.');
}catch(e){await write('failure.json',{error:String(e)});await p.screenshot({path:path.join(out,'failure.png')}).catch(()=>{});throw e;}finally{await browser.close();}
