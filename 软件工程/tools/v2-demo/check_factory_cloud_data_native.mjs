import fs from 'node:fs/promises';
import path from 'node:path';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {pathToFileURL} from 'node:url';
import assert from 'node:assert/strict';

const [base,attempt,production='factory-live',expectedAccepted='167']=process.argv.slice(2);
const scope=production==='factory-live'?'whole_factory':'core_demo';
const native=path.join(base,attempt),out=path.join(native,scope==='whole_factory'?'factory-cloud-data':'core-cloud-data');await fs.mkdir(out);
const read=async p=>JSON.parse((await fs.readFile(p,'utf8')).replace(/^\uFEFF/,''));
const write=(name,data)=>fs.writeFile(path.join(out,name),JSON.stringify(data,null,2));
const launch=await read(path.join(native,'launch.json')),terminal=(await read(path.join(native,production,'terminal.json'))).snapshot;assert.equal(terminal.lifecycle.state,'completed');
const source={identity:terminal.identity,R:terminal.execution_revision,T:terminal.coverage.observed_until_host_ms},identityKey=i=>JSON.stringify([i.project_id,i.host_context_session_id,i.session_id,i.execution_epoch_id]);
const utcMicros=s=>s.replace(/(?:\.(\d{1,6}))?Z$/,(_,fraction='')=>'.'+fraction.padEnd(6,'0')+'Z');
const pwsh='C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe';
const exec=promisify(execFile),sleep=ms=>new Promise(r=>setTimeout(r,ms));
const {chromium}=await import(pathToFileURL('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs').href);
const browser=await chromium.connectOverCDP(launch.cdp),page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://tauri.localhost/');assert(page);
page.setDefaultTimeout(60000);const errors=[],warnings=[],checks=[];
page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());if(m.type()==='warning')warnings.push(m.text());});
const invoke=(command,args)=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
let projectId=source.identity.project_id;
const api=async(operation,params={},id=projectId)=>{const r=await invoke('project_api',{request:{api_version:'0.1',request_id:'cloud-forms-'+crypto.randomUUID(),project_id:id,scope:{kind:id?'project':'workspace'},operation,params}});assert.equal(r.outcome.status,'ok',JSON.stringify(r.outcome));return r.outcome.data;};
const check=async(name,facts={})=>{checks.push({name,...facts});await write('progress.json',checks);console.log(name);};
const capture=name=>page.screenshot({path:path.join(out,name+'.png')});
const picker=async(directory,title)=>{
  await fs.mkdir(directory);let dialog;const until=Date.now()+15000;
  do{const r=await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});if(r.stdout.trim()){const d=JSON.parse(r.stdout);if(d.controls.some(c=>c.class==='Edit'&&c.id===1152)&&d.controls.some(c=>c.id===1001&&c.class==='ToolbarWindow32')){dialog=d;break;}}await sleep(100);}while(Date.now()<until);
  assert(dialog);assert.equal(dialog.title,title);
  const edit=dialog.controls.find(c=>c.class==='Edit'&&c.id===1152),button=dialog.controls.find(c=>c.class==='Button'&&c.id===1);assert(edit&&button);
  await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe,'-SelectDirectory',directory,'-EditHandle',String(edit.handle),'-ButtonHandle',String(button.handle)],{windowsHide:true});
};
const panel=()=>page.locator('.cloud-production');
const query=async()=>{
  await page.getByRole('button',{name:'查询并固定结果',exact:true}).click();
  await page.locator('.cloud-page-status').waitFor({timeout:60000});
  assert.equal(await page.locator('.cloud-alert').count(),0);
  return JSON.parse(await page.locator('.cloud-json').filter({has:page.locator('summary',{hasText:'完整筛选与固定来源'})}).locator('pre').textContent());
};
const exportData=async(name)=>{
  const directory=path.join(out,name);
  const previousTask=await page.locator('.cloud-task .cloud-json pre').allTextContents();
  await page.getByRole('button',{name:'导出全部筛选结果',exact:true}).click();await picker(directory,'选择筛选数据导出目录');
  await page.waitForFunction(previous=>{const p=document.querySelector('.cloud-task .cloud-json pre');return p&&p.textContent!==previous;},previousTask[0]??'',{timeout:60000});
  await page.waitForFunction(()=>{const p=document.querySelector('.cloud-task .cloud-json pre');return p&&JSON.parse(p.textContent).operation==='export_cloud_data'&&JSON.parse(p.textContent).status==='completed';},{},{timeout:120000});
  assert.equal(await page.locator('.cloud-alert').count(),0);
  const children=await fs.readdir(directory,{withFileTypes:true});const folder=children.find(x=>x.isDirectory());assert(folder);
  const packagePath=path.join(directory,folder.name),lines=(await fs.readFile(path.join(packagePath,'data.jsonl'),'utf8')).trim().split(/\r?\n/).map(JSON.parse);
  const header=lines[0],footer=lines.at(-1),rows=lines.slice(1,-1).flatMap(p=>p.result.rows);
  assert.equal(footer.complete,true);assert.equal(footer.row_count,rows.length);
  assert((await fs.readFile(path.join(packagePath,'data.csv'),'utf8')).length>0);
  return{package:packagePath,header,footer,rows};
};
try{
 await page.getByRole('tab',{name:'执行',exact:true}).click();
 const versions=page.locator('.cloud-version-options');if(!await versions.evaluate(e=>e.open))await versions.locator('summary').click();
 await page.getByLabel('记录版本',{exact:true}).fill(String(source.R));await page.getByRole('button',{name:'读取这个版本',exact:true}).click();
 await page.waitForFunction(r=>document.querySelector('.cloud-source')?.textContent.includes('第 '+r+' 版'),source.R);
 await page.getByRole('button',{name:'云端存档',exact:true}).click();await page.waitForFunction(()=>document.querySelector('.cloud-source')?.textContent.includes('云端存档（只读）'));
 await page.getByRole('tab',{name:'数据',exact:true}).click();await page.getByRole('combobox',{name:'数据集',exact:true}).selectOption('@execution');
 await page.getByRole('combobox',{name:'查看内容',exact:true}).selectOption('accepted_quantity');await page.getByRole('button',{name:'整段记录',exact:true}).click();
 const pinned=await query();assert.equal(pinned.selection.execution_revision,source.R);assert.equal(pinned.query.host_until_ms,source.T+1);
 const quantities=await exportData('accepted-quantity');assert.deepEqual(quantities.header.selection,pinned.selection);
 const quantity=q=>q.value.coefficient/10**q.value.decimal_places;
 const hostAccepted=terminal.business_facts.acceptance_events.reduce((n,e)=>n+quantity(e.quantity),0);assert.equal(hostAccepted,Number(expectedAccepted));
 assert.equal(quantities.rows.reduce((n,r)=>{assert.equal(r.quantity.unit,'piece');return n+quantity(r.quantity);},0),hostAccepted);
 await capture('cloud-accepted-quantity');
 await page.getByRole('combobox',{name:'查看内容',exact:true}).selectOption('rows');const detailed=await query();const exported=await exportData('all-execution-records');assert.deepEqual(exported.header.selection,detailed.selection);
 for(const row of exported.rows){assert.deepEqual(row.identity,source.identity);assert.equal(row.execution_revision,source.R);}
 const events=exported.rows.filter(r=>r.kind==='acceptance').map(r=>r.event);assert.deepEqual(events,terminal.business_facts.acceptance_events);
 const attempts=exported.rows.filter(r=>r.kind==='execution_attempt').map(r=>r.attempt);assert.deepEqual(attempts,terminal.attempts);
 assert.deepEqual(errors,[]);await write('receipt.json',{status:scope+'_cloud_query_and_all_row_export_match_Host',scope,...source,accepted:hostAccepted,attempts:attempts.length,acceptance_events:events.length,quantity_export:quantities.package,full_export:exported.package,errors});
 await page.getByRole('tab',{name:'执行',exact:true}).click();await page.getByRole('button',{name:'跟随本机会话',exact:true}).click();
}catch(error){await write('failure.json',{error:String(error),stack:error.stack,source});throw error;}finally{await browser.close();}
