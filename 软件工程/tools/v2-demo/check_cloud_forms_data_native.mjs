import fs from 'node:fs/promises';
import path from 'node:path';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {pathToFileURL} from 'node:url';
import assert from 'node:assert/strict';

const [base, previousRoot, attempt='native-01', resume='', outputName='']=process.argv.slice(2);
assert(base && previousRoot);
assert(['','open-dialog','current-project'].includes(resume));
const native=path.join(base,attempt),out=path.join(native,outputName||(resume?'forms-data-resume01':'forms-data'));await fs.mkdir(out);
const read=async p=>JSON.parse((await fs.readFile(p,'utf8')).replace(/^\uFEFF/,''));
const write=(name,data)=>fs.writeFile(path.join(out,name),JSON.stringify(data,null,2));
const launch=await read(path.join(native,'launch.json')),previous=await read(path.join(previousRoot,'completion.json'));
const source=previous.executions[0],identityKey=i=>JSON.stringify([i.project_id,i.host_context_session_id,i.session_id,i.execution_epoch_id]);
const utcMicros=s=>s.replace(/(?:\.(\d{1,6}))?Z$/,(_,fraction='')=>'.'+fraction.padEnd(6,'0')+'Z');
const pwsh='C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe';
const exec=promisify(execFile),sleep=ms=>new Promise(r=>setTimeout(r,ms));
const {chromium}=await import(pathToFileURL('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs').href);
const browser=await chromium.connectOverCDP(launch.cdp),page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://tauri.localhost/');assert(page);
page.setDefaultTimeout(60000);const errors=[],warnings=[],checks=[];
page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());if(m.type()==='warning')warnings.push(m.text());});
const invoke=(command,args)=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
let projectId=null;
const api=async(operation,params={},id=projectId)=>{const r=await invoke('project_api',{request:{api_version:'0.1',request_id:'cloud-forms-'+crypto.randomUUID(),project_id:id,scope:{kind:id?'project':'workspace'},operation,params}});assert.equal(r.outcome.status,'ok',JSON.stringify(r.outcome));return r.outcome.data;};
const check=async(name,facts={})=>{checks.push({name,...facts});await write('progress.json',checks);console.log(name);};
const capture=name=>page.screenshot({path:path.join(out,name+'.png')});
const picker=async(directory,title)=>{
  await fs.mkdir(directory);let dialog;const until=Date.now()+15000;
  do{const r=await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});if(r.stdout.trim()){dialog=JSON.parse(r.stdout);break;}await sleep(100);}while(Date.now()<until);
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
  const rectangle=JSON.parse((await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-rectangle.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true})).stdout);
  assert(launch.monitor.connected===1||!rectangle.primary);
  await page.setViewportSize({width:1920,height:1080});
  if (!resume) {
  await page.getByText('文件',{exact:true}).click();await page.getByRole('button',{name:'打开工程目录…',exact:true}).click();
  // Existing source is not created, merged or overwritten by the native chooser.
  let dialog;const until=Date.now()+15000;
  do{const r=await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});if(r.stdout.trim()){dialog=JSON.parse(r.stdout);break;}await sleep(100);}while(Date.now()<until);
  assert(dialog);assert.equal(dialog.title,'选择要打开的工程目录');
  const edit=dialog.controls.find(c=>c.class==='Edit'&&c.id===1152),button=dialog.controls.find(c=>c.class==='Button'&&c.id===1);
  await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(base,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe,'-SelectDirectory',path.join(previousRoot,'project-final'),'-EditHandle',String(edit.handle),'-ButtonHandle',String(button.handle)],{windowsHide:true});
  }
  if (resume!=='current-project') {
  const departure=page.getByRole('combobox',{name:'离开当前工程',exact:true});
  if (await departure.count()) {
    const close=await invoke('project_close_state');
    assert.equal(close.unsaved,false);assert.deepEqual(close.active_tasks,[]);assert.deepEqual(close.execution_blockers,[]);
    await departure.selectOption('already_saved');
  }
  await page.getByRole('button',{name:'确认打开',exact:true}).click();await page.waitForFunction(()=>document.querySelector('main.workbench')?.getAttribute('aria-busy')!=='true');
  }
  const initial=await api('get_context',{selection:[]},null);projectId=initial.project.id;assert.equal(projectId,source.identity.project_id);await write('initial-context.json',initial);
  if (!await page.locator('.cloud-production').count()) await page.getByRole('button',{name:'云端生产',exact:true}).click();
  await page.getByRole('heading',{name:'云端生产',exact:true}).waitFor();
  await page.getByRole('tab',{name:'设备',exact:true}).click();
  await page.waitForFunction(()=>document.querySelectorAll('.cloud-table-scroll tbody tr').length===4);
  assert.doesNotMatch(await panel().innerText(),/read_origin|ownership|binding_id|clock_started|source_purpose/);
  await capture('01-device-names');
  await check('Actual device and binding choices use declared names and stages, with protocol IDs in closed details');
  const records=page.getByRole('combobox',{name:'生产记录',exact:true});
  await records.locator('option').filter({hasText:'已保存记录'}).first().waitFor({state:'attached'});
  const options=await records.locator('option').evaluateAll(es=>es.map(e=>({value:e.value,text:e.textContent})));
  const index=options.findIndex(o=>o.value===identityKey(source.identity));assert(index>0);assert.doesNotMatch(options.map(o=>o.text).join('\n'),/desktop-host|execution-epoch/);
  await records.selectOption(identityKey(source.identity));await page.waitForFunction(r=>document.querySelector('.cloud-source')?.textContent.includes('第 '+r+' 版'),source.R);
  await page.getByRole('tab',{name:'数据',exact:true}).click();
  const datasets=page.getByRole('combobox',{name:'数据集',exact:true});
  await datasets.locator('option').filter({hasText:'生产设备上报'}).first().waitFor({state:'attached'});
  const datasetOptions=await datasets.locator('option').evaluateAll(es=>es.map(e=>({value:e.value,text:e.textContent})));
  await write('actual-dataset-choices.json',datasetOptions);
  const selected=datasetOptions.find(o=>o.text.includes('生产设备上报')&&o.text.includes('生产记录 '+index+' ·'));
  assert(selected,'The actual source session must have an associated execution telemetry dataset.');
  await datasets.selectOption(selected.value);await page.getByRole('button',{name:'最近24小时',exact:true}).click();
  let pinned=await query();await write('actual-page1-query-and-pin.json',pinned);
  assert.equal(pinned.query.dataset,'telemetry');assert.equal(pinned.query.device_ids.length,0);
  assert.equal(await page.locator('.cloud-query-table tbody tr').count(),50);
  assert.doesNotMatch(await page.locator('.cloud-query-table').innerText(),/\btelemetry\b|\bidle\b|\bbusy\b|source_purpose|desktop-host/);
  await page.locator('.cloud-query-table').scrollIntoViewIfNeeded();
  await capture('02-telemetry-table');
  const fields=page.locator('.cloud-data-technical');await fields.locator('summary').click();
  const explorer=page.getByRole('region',{name:'数据结果浏览器',exact:true});
  const column=explorer.getByRole('checkbox').first(),columnName=await column.getAttribute('aria-label');
  const columnCount=await explorer.locator('thead th').count();await column.uncheck();
  assert.equal(await explorer.locator('thead th').count(),columnCount-1);
  const afterHide=JSON.parse(await page.locator('.cloud-json').filter({has:page.locator('summary',{hasText:'完整筛选与固定来源'})}).locator('pre').textContent());assert.deepEqual(afterHide,pinned);
  await capture('02b-selected-columns');
  await page.getByRole('button',{name:'下一页记录',exact:true}).click();await page.waitForFunction(()=>document.querySelector('.cloud-page-status')?.textContent.includes('第 2 页'));
  assert.equal(await explorer.getByRole('checkbox',{name:columnName,exact:true}).isChecked(),false);await explorer.getByRole('button',{name:'显示全部字段',exact:true}).click();await fields.locator('summary').click();
  const next=JSON.parse(await page.locator('.cloud-json').filter({has:page.locator('summary',{hasText:'完整筛选与固定来源'})}).locator('pre').textContent());
  assert.deepEqual(next.selection,pinned.selection);await write('actual-page2-query-and-pin.json',next);
  const full=await exportData('export-telemetry-all');assert.deepEqual(full.header.selection,pinned.selection);
  assert(full.footer.row_count>100,'Full export must contain data beyond both read UI pages.');
  assert.equal(full.header.query.received_from,utcMicros(pinned.query.received_from));assert.equal(full.header.query.received_until,utcMicros(pinned.query.received_until));
  await check('Native field selection persists across pages without changing the pinned query or all-row export',{hidden_column:columnName,columns:columnCount});
  await check('Actual HTTP telemetry query uses local calendar controls, readable rows and stable pagination; full export includes unread pages',{rows:full.footer.row_count,pages:full.footer.page_count,package:full.package});

  await page.locator('.cloud-device-filter input[type=checkbox]').first().check();
  assert.equal(await page.getByRole('button',{name:'导出全部筛选结果',exact:true}).isDisabled(),true);
  const one=await query();assert.equal(one.query.device_ids.length,1);assert.deepEqual(one.selection.selected_device_ids,one.query.device_ids);
  await check('A named device checkbox sends the exact selected ID and invalidates the old export pin');
  await page.getByRole('button',{name:'清除设备筛选',exact:true}).click();
  await page.locator('.cloud-precise-time > summary').click();
  const from='2026-10-28T00:00:00.000001Z',untilUtc='2026-10-28T00:00:00.000002Z';
  await page.getByLabel('精确UTC起点',{exact:true}).fill(from);await page.getByLabel('精确UTC终点',{exact:true}).fill(untilUtc);
  await page.getByLabel('接收开始时间',{exact:true}).focus();
  assert.equal(await page.getByLabel('精确UTC起点',{exact:true}).inputValue(),from);
  const micro=await query();assert.equal(micro.query.received_from,from);assert.equal(micro.query.received_until,untilUtc);
  assert.equal(await page.locator('.cloud-query-table tbody tr').count(),0);
  const empty=await exportData('export-microsecond-empty');assert.equal(empty.footer.row_count,0);
  assert.equal(empty.header.query.received_from,from);assert.equal(empty.header.query.received_until,untilUtc);
  await check('Exact microsecond bounds survive calendar display, actual HTTP query and empty full export',{from,until:untilUtc,package:empty.package});

  await datasets.selectOption('@execution');await page.getByRole('combobox',{name:'查看内容',exact:true}).selectOption('accepted_quantity');
  await page.getByLabel('生产查询起点（秒）',{exact:true}).fill('0');await page.getByLabel('生产查询终点（秒）',{exact:true}).fill(String(source.T/1000));
  const prefix=await query();assert.equal(prefix.query.host_until_ms,source.T);assert.match(await page.locator('.cloud-query-table').innerText(),/11 件/);
  await page.getByRole('button',{name:'整段记录',exact:true}).click();
  const execution=await query();assert.equal(execution.query.host_until_ms,source.T+1);assert.equal(execution.selection.execution_revision,source.R);
  assert.match(await page.locator('.cloud-query-table').innerText(),/12 件/);
  await write('actual-execution-range-boundaries.json',{prefix,whole_record:execution,result_text:await page.locator('.cloud-query-table').innerText()});
  await page.locator('.cloud-query-table').scrollIntoViewIfNeeded();
  await capture('03-execution-quantity');
  await page.setViewportSize({width:1366,height:768});await page.evaluate(()=>document.querySelector('.cloud-production').scrollTo(0,0));
  assert.equal(await panel().evaluate(e=>e.scrollWidth>e.clientWidth+1),false);await capture('04-1366-query');
  await page.locator('.cloud-query-table').scrollIntoViewIfNeeded();await capture('05-1366-result');
  await check('Execution query preserves exclusive-end semantics; the whole-record button includes the final event and all twelve received pieces');
  await page.getByRole('tab',{name:'执行',exact:true}).click();
  for(const name of ['启动','暂停新派工','继续新派工','停止新派工'])assert(await page.getByRole('button',{name,exact:true}).isDisabled());
  await page.getByRole('button',{name:'导出生产报告',exact:true}).click();const reports=path.join(out,'export-report');await picker(reports,'选择执行报告导出目录');
  await page.waitForFunction(()=>{const p=document.querySelector('.cloud-task .cloud-json pre');return p&&JSON.parse(p.textContent).operation==='export_execution_report'&&JSON.parse(p.textContent).status==='completed';},{},{timeout:120000});
  const reportDir=(await fs.readdir(reports,{withFileTypes:true})).find(x=>x.isDirectory());assert(reportDir);
  const reportPath=path.join(reports,reportDir.name),report=await read(path.join(reportPath,'report.json'));
  assert.equal(report.sessions[0].snapshot.execution_revision,source.R);
  assert((await fs.readFile(path.join(reportPath,'report.pdf'))).subarray(0,5).equals(Buffer.from('%PDF-')));
  assert((await fs.stat(path.join(reportPath,'report.html'))).size>0);
  await check('Actual readonly historical source cannot control production and exports its same-source four-format report',{R:source.R,report:reportPath});
  assert.deepEqual(errors,[]);
  await write('receipt.json',{status:'actual_Native_named_forms_queries_pagination_and_exports_verified',pid:launch.pid,source:source.identity,
    checks,errors,warnings,report:reportPath,telemetry_export:full.package,microsecond_export:empty.package,
    source_linked_resources:true,standalone_package_verified:false,production_started:false,
    Browser:'Browser plugin not available; project installed Playwright connected to actual ordinary Native WebView2'});
  console.log('Data/form Native checks completed; keep this original process for actual production/phase verification.');
}catch(error){await write('failure.json',{error:String(error),stack:error.stack,checks,errors,warnings,pid:launch.pid});await page.screenshot({path:path.join(out,'failure.png')}).catch(()=>{});throw error;}
finally{await browser.close();}
