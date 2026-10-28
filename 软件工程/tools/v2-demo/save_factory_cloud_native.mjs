import fs from 'node:fs/promises';import path from 'node:path';import assert from 'node:assert/strict';
import {execFile} from 'node:child_process';import {promisify} from 'node:util';
import {chromium} from 'file:///C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs';
const [base,attempt,production='factory-live',output='save',resume='']=process.argv.slice(2),native=path.join(base,attempt),out=path.join(native,output);if(!resume)await fs.mkdir(out);
const read=async name=>JSON.parse((await fs.readFile(name,'utf8')).replace(/^\uFEFF/,'')),write=(name,value)=>fs.writeFile(path.join(out,name),JSON.stringify(value,null,2));
const launch=await read(path.join(native,'launch.json'));
const browser=await chromium.connectOverCDP(launch.cdp),p=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://tauri.localhost/');assert(p);p.setDefaultTimeout(60000);
const exec=promisify(execFile),pwsh='C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe';
async function picker(target){await fs.mkdir(target);let dialog;const end=Date.now()+15000;while(Date.now()<end){const r=await exec(pwsh,['-NoProfile','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});if(r.stdout.trim()){dialog=JSON.parse(r.stdout);if(dialog.controls.some(c=>c.class==='Edit'&&c.id===1152)&&dialog.controls.some(c=>c.id===1001&&c.class==='ToolbarWindow32'))break;}await new Promise(r=>setTimeout(r,100));}assert(dialog);const edit=dialog.controls.find(c=>c.class==='Edit'&&c.id===1152),button=dialog.controls.find(c=>c.class==='Button'&&c.id===1);assert(edit&&button);await exec(pwsh,['-NoProfile','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe,'-SelectDirectory',target,'-EditHandle',String(edit.handle),'-ButtonHandle',String(button.handle)],{windowsHide:true});}
async function waitForSavedState(requireSaved=false){
 // The installed CDP bridge resolves async waitForFunction predicates immediately.
 // Await the actual IPC result here before deciding that saving/closing is allowed.
 const deadline=Date.now()+600000;
 while(Date.now()<deadline){
  const state=await p.evaluate(()=>window.__TAURI_INTERNALS__.invoke('project_close_state'));
  if(!state.active_tasks.length&&!state.execution_blockers.length&&(!requireSaved||!state.unsaved))return state;
  await new Promise(resolve=>setTimeout(resolve,1000));
 }
 throw new Error('Original execution/save still has obligations; retain Native');
}
try {
 await waitForSavedState();
 const saved=path.join(out,'saved-project');
 if(resume!=='saving'){
  await p.getByRole('button',{name:'返回画布',exact:true}).click();await p.getByText('文件',{exact:true}).click();await p.getByRole('button',{name:'保存工程…',exact:true}).click();await picker(saved);
 }
 await waitForSavedState(true);
 await p.getByRole('status').filter({hasText:'已保存'}).waitFor({timeout:600000});
 await waitForSavedState(true);
 assert((await fs.readdir(saved)).includes('project.json'));
 await p.waitForFunction(()=>document.querySelector('main.workbench')?.getAttribute('aria-busy')==='false',{},{timeout:600000});
 const terminal=(await read(path.join(native,production,'terminal.json'))).snapshot;
 await write('receipt.json',{status:'terminal_execution_saved',saved,execution_state:terminal.lifecycle.state,identity:terminal.identity,R:terminal.execution_revision});
 await exec(pwsh,['-NoProfile','-File',path.join(base,'close-release.ps1'),'-NativeDirectory',native],{windowsHide:true});console.log('Original terminal execution saved and Native normally closed.');
}finally{await browser.close();}
