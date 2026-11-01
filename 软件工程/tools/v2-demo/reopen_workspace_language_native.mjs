import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {pathToFileURL} from 'node:url';

const [base,attempt='native-02']=process.argv.slice(2),native=path.join(base,attempt),out=path.join(native,'reopen');
await fs.mkdir(out);
const read=async p=>JSON.parse((await fs.readFile(p,'utf8')).replace(/^\uFEFF/,''));
const write=(name,value)=>fs.writeFile(path.join(out,name),JSON.stringify(value,null,2));
const previous=path.join(base,'native-01/language-finish02'),receipt=await read(path.join(previous,'receipt.json'));
const expected=await read(path.join(previous,'project-after-calculation.json')),launch=await read(path.join(native,'launch.json'));
const firstUse='F:/SuperFactory-v2-temp/01a1731e/full-factory/factory-native-first-use/native-01/first-use';
const oldRuns=await Promise.all(['baseline-run.json','revised-run.json'].map(name=>read(path.join(firstUse,name))));
const {chromium}=await import(pathToFileURL('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs').href);
const b=await chromium.connectOverCDP(launch.cdp),p=b.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://tauri.localhost/');
assert(p);p.setDefaultTimeout(30000);
const errors=[],checks=[];
p.on('pageerror',e=>errors.push(String(e)));
p.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
const invoke=(command,args)=>p.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const api=async(operation,params={},id=receipt.project_id)=>{
  const r=await invoke('project_api',{request:{api_version:'0.1',request_id:'g05-reopen-'+crypto.randomUUID(),project_id:id,scope:{kind:id?'project':'workspace'},operation,params}});
  assert.equal(r.outcome.status,'ok',JSON.stringify(r.outcome));return r.outcome.data;
};
const project=async()=>{const r=await api('get_object',{object_id:{kind:'project',id:receipt.project_id},source:{kind:'draft'},node_index:null});assert.equal(r.kind,'project');return r.data;};
const event=async(name,values={})=>{checks.push({name,...values});await write('progress.json',checks);console.log(name);};
const capture=name=>p.screenshot({path:path.join(out,name+'.png')});
try{
  assert.equal((await api('get_context',{selection:[]},null)).project,null);
  assert(launch.monitor.connected===1||!launch.monitor.primary);
  await p.setViewportSize({width:1920,height:1080});
  await p.getByText('文件',{exact:true}).click();await p.getByRole('button',{name:'打开工程目录…',exact:true}).click();
  const exec=promisify(execFile),pwsh='C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe';
  let dialog;const until=Date.now()+15000;
  do{
    const r=await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});
    if(r.stdout.trim()){const d=JSON.parse(r.stdout);if(d.controls.some(c=>c.class==='Edit'&&c.id===1152)&&d.controls.some(c=>c.class==='Button'&&c.id===1)){dialog=d;break;}}
    await new Promise(r=>setTimeout(r,100));
  }while(Date.now()<until);
  assert(dialog);assert.equal(dialog.title,'选择要打开的工程目录');
  const edit=dialog.controls.find(c=>c.class==='Edit'&&c.id===1152),button=dialog.controls.find(c=>c.class==='Button'&&c.id===1);
  await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe,'-SelectDirectory',receipt.saved,'-EditHandle',String(edit.handle),'-ButtonHandle',String(button.handle)],{windowsHide:true});
  await p.getByRole('button',{name:'确认打开',exact:true}).click();
  await p.waitForFunction(()=>document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true');
  assert.deepEqual(await project(),expected);assert.equal(expected.devices.length,92);
  for(const run of oldRuns){const r=await api('get_run',{run_id:run.id,query:{kind:'full_result'}});assert.deepEqual(r.data,run);}
  await event('The saved 92-device factory, professional artifact, acknowledgments, bindings and both old Runs reopen unchanged');

  await p.getByText('专业工程',{exact:true}).click();await p.getByRole('button',{name:'纸包装 · 结构与过程…',exact:true}).click();
  const professional=p.locator('.packaging-workspace');await professional.getByRole('heading',{name:'纸包装 · 结构与过程',exact:true}).waitFor();
  const artifacts=p.getByLabel('当前范围已保存的计算成果');
  await p.waitForFunction(()=>[...document.querySelectorAll('.packaging-results select')].some(e=>e.options.length>1&&!e.disabled));
  assert.equal(await artifacts.locator('option').count(),2);await artifacts.selectOption({index:1});
  await professional.locator('h4.packaging-pass').waitFor({timeout:120000});
  assert(await professional.getByRole('button',{name:'确认并记录用途认可',exact:true}).isDisabled());
  assert.doesNotMatch(await professional.innerText(),/当前 Scope|保存的 Artifact|毫米字段：polygon_mm/);
  await capture('01-reopened-professional');await p.setViewportSize({width:1366,height:768});await capture('02-professional-css1366');await p.setViewportSize({width:1920,height:1080});
  await professional.getByRole('button',{name:'关闭',exact:true}).click();await professional.waitFor({state:'hidden'});
  assert.equal(await p.getByRole('dialog',{name:'未保存的专业输入',exact:true}).count(),0);
  await event('Stored professional result opens without recomputation or human acknowledgment; viewing and closing makes no edit');

  await p.getByText('Agent',{exact:true}).click();await p.getByRole('button',{name:'连接、权限与任务…',exact:true}).click();
  const agent=p.locator('.agent-workspace'),operations=p.locator('.agent-operations'),caps=await api('describe_capabilities',{},null);
  assert.equal(await operations.getByRole('checkbox').count(),caps.implemented_operations.length);
  assert.doesNotMatch(await operations.innerText(),/read_cloud_production|export_cloud_data|apply_device_/);
  assert(await agent.getByRole('button',{name:'确认并授权此连接',exact:true}).isDisabled());
  await capture('03-agent');await agent.getByRole('button',{name:'关闭工作台',exact:true}).click();await agent.waitFor({state:'hidden'});
  await event('Actual exposed Agent operations retain Chinese labels and unauthorized grant stays disabled',{exposed_operations:caps.implemented_operations.length});

  const run=oldRuns[1],device=expected.devices.find(d=>d.title==='气浮送纸工作台');assert(device);
  await p.getByLabel('历史结果',{exact:true}).selectOption(run.id);
  await p.waitForFunction(id=>document.querySelector('canvas')?.readFactoryDiagnostics?.()?.run_id===id,run.id,{timeout:120000});
  const job=run.jobs.find(j=>j.execution_ref.kind==='project_device'&&j.execution_ref.data.device_id===device.id);assert(job&&job.started_at_ms!=null&&job.finished_at_ms>job.started_at_ms);
  const at=Math.floor((job.started_at_ms+job.finished_at_ms)/2);await p.getByLabel('观察毫秒',{exact:true}).fill(String(at));await p.getByRole('button',{name:'读取时点',exact:true}).click();
  await p.waitForFunction(at=>document.querySelector('canvas')?.readFactoryDiagnostics?.()?.observation_ms===at,at,{timeout:120000});
  await p.locator('.object-tree').getByRole('button',{name:'▣ 气浮送纸工作台',exact:true}).click();await p.locator('.inspector').getByRole('button',{name:'查看设备过程 / 图片编辑',exact:true}).click();
  await p.waitForFunction(()=>[...document.querySelectorAll('.sf-media-camera button')].some(e=>e.textContent==='适配'&&!e.disabled));
  assert.equal(await p.locator('.sf-device-process-error').count(),0);
  const media=p.locator('.sf-media');assert.doesNotMatch(await media.innerText(),/unavailable|实际 Run|\bJob\b|\bphase\b|holder|desktop-host/);
  assert.match(await media.innerText(),/加工中/);await capture('04-actual-device');
  await p.getByRole('button',{name:'检查冻结静态输入',exact:true}).click();await p.getByRole('heading',{name:'静态外观检查',exact:true}).waitFor();
  await p.waitForFunction(()=>[...document.querySelectorAll('.sf-media-camera button')].some(e=>e.textContent==='适配'&&!e.disabled));
  assert.doesNotMatch(await media.innerText(),/unavailable|实际 Run|\bJob\b|\bphase\b|holder|desktop-host/);
  assert.match(await media.innerText(),/静态外观不提供实际作业、阶段进度或产品产量/);
  await capture('05-static-device');await p.getByRole('button',{name:'返回工作台',exact:true}).click();
  await event('Actual device job at its recorded midpoint and static inspection use distinct readable states; IDs and licenses remain in details',{device:device.title,at_ms:at});
  assert.deepEqual(await project(),expected);const close=await invoke('project_close_state');assert.equal(close.unsaved,false);assert.deepEqual(close.active_tasks,[]);assert.deepEqual(close.execution_blockers,[]);assert.deepEqual(errors,[]);
  await write('receipt.json',{state:'reopened_professional_result_and_final_three_workspaces_verified',pid:launch.pid,project_id:expected.id,project_revision:expected.revision,devices:92,old_runs_unchanged:oldRuns.map(r=>r.id),checks,errors,close,source_linked_resources:true,standalone_package_verified:false,whole_factory_accepted:false,viewport_note:'1366 is CSS viewport width; physical small-monitor fit remains G06'});
  console.log('Reopen verification complete; normal close is next.');
}catch(error){await write('failure.json',{error:String(error),stack:error.stack,checks,errors,pid:launch.pid});await capture('failure').catch(()=>{});throw error;}finally{await b.close();}
