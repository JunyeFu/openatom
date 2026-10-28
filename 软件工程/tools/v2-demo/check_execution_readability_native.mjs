import fs from 'node:fs/promises';
import path from 'node:path';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {pathToFileURL} from 'node:url';
import assert from 'node:assert/strict';

const [nativeRoot, previousRoot, attempt = 'native-01', mode] = process.argv.slice(2);
const resume = mode === 'resume-current-owner';
const finish = mode === 'finish-save';
const continued = resume || finish;
assert(nativeRoot && previousRoot, 'Provide current owned Native root and the preserved multistage evidence root.');
const native = path.join(nativeRoot, attempt), output = path.join(native, 'readability');
if (!continued) await fs.mkdir(output);
const read = async file => JSON.parse((await fs.readFile(file, 'utf8')).replace(/^\uFEFF/, ''));
const write = (name, value) => fs.writeFile(path.join(output, name), JSON.stringify(value, null, 2));
const launch = await read(path.join(native, 'launch.json'));
const previous = await read(path.join(previousRoot, 'completion.json'));
const entry = previous.executions[0];
const pwsh = 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe';
const exec = promisify(execFile), sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const {chromium} = await import(pathToFileURL('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs').href);
const browser = await chromium.connectOverCDP(launch.cdp);
const page = browser.contexts().flatMap(c => c.pages()).find(p => p.url() === 'http://tauri.localhost/');
assert(page, 'The actual owned Native page is missing.');
page.setDefaultTimeout(20000);
const errors = [], warnings = [], checks = continued ? await read(path.join(output,'progress.json')) : [], screenshots = continued
  ? (await fs.readdir(output)).filter(name=>name.endsWith('.png') && name!=='failure.png').map(name=>path.join(output,name)) : [];
page.on('pageerror', error => errors.push(String(error)));
page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); if (message.type() === 'warning') warnings.push(message.text()); });
const invoke = (command, args) => page.evaluate(({command, args}) => window.__TAURI_INTERNALS__.invoke(command, args), {command, args});
let projectId = null, initial, original;
const api = async (operation, params = {}, id = projectId) => {
  const result = await invoke('project_api', {request: {api_version: '0.1', request_id: 'readability-'+crypto.randomUUID(),
    project_id: id, scope: {kind: id ? 'project' : 'workspace'}, operation, params}});
  assert.equal(result.outcome.status, 'ok', JSON.stringify(result.outcome));
  return result.outcome.data;
};
const snapshot = (identity, revision) => api('get_execution_snapshot', {execution_contract_version: 'sf.execution/2.0-draft11', identity,
  selection: revision === undefined ? {kind: 'latest'} : {kind: 'revision', execution_revision: revision}});
const capture = async name => { const file = path.join(output, name+'.png'); await page.screenshot({path: file}); screenshots.push(file); };
const check = async (name, facts = {}) => { checks.push({name, ...facts}); await write('progress.json', checks); console.log(name); };
const loaded = async () => {
  await page.locator('.execution-view-scene[aria-busy="false"]').waitFor({timeout: 60000});
  await page.locator('.execution-view-complete').filter({hasText: /^已读取 .*全部/}).waitFor({timeout: 60000});
  assert.equal(await page.locator('.execution-view-error,.scene-error').count(), 0);
};
const openView = async () => {
  await page.getByRole('tab', {name: '执行', exact: true}).click();
  await page.getByRole('button', {name: '回看执行画布与时间轴', exact: true}).click();
  await loaded();
};
const back = async () => {
  await page.locator('.execution-observation').getByRole('button', {name: '返回云端生产', exact: true}).click();
  await page.getByRole('heading', {name: '云端生产', exact: true}).waitFor();
};
const sourceDetails = async () => {
  const details = page.locator('.execution-source-technical');
  if (!await details.evaluate(element => element.open)) await details.locator('summary').click();
  return JSON.parse(await details.locator('pre').innerText());
};
const closeDetails = () => page.locator('.execution-source-technical > summary').click();
const assertReadable = async () => {
  const text = await page.locator('.execution-observation').innerText();
  assert.doesNotMatch(text, /read_origin|ownership|clock_started|record_sequence|current_host_owner|other_host|\bStarted\b|\bholder\b/);
  const sourceText = await page.locator('.execution-view-source').innerText();
  assert.doesNotMatch(sourceText, /\bR\d+|\bF\d+|\bT=|Host/);
  for (const name of ['启动', '暂停新派工', '继续新派工', '停止新派工']) {
    assert.equal(await page.locator('.execution-observation').getByRole('button', {name, exact: true}).count(), 0);
  }
};
try {
  await page.setViewportSize(continued ? {width:1366,height:768} : {width:1920,height:1080});
  const rectangle = JSON.parse((await exec(pwsh, ['-NoProfile', '-NonInteractive', '-File', path.join(nativeRoot, 'owned-rectangle.ps1'),
    '-NativeProcessId', String(launch.pid), '-ExpectedExe', launch.exe], {windowsHide: true})).stdout);
  assert(launch.monitor.connected === 1 || !rectangle.primary);
  if (resume) {
    const priorClose = await invoke('project_close_state');
    assert.deepEqual(priorClose.active_tasks,[]);
    assert.deepEqual(priorClose.execution_blockers,[]);
    const taskText = await page.locator('.cloud-task').innerText();
    assert.match(taskText,/start_execution_session · failed/);
    assert.match(await page.locator('[role=alert]').innerText(),/Frozen terminal unit count must equal the explicit target/);
    await fs.writeFile(path.join(output,'retained-target-mismatch.json'),await fs.readFile(path.join(output,'failure.json')),{flag:'wx'});
    projectId=entry.identity.project_id;
    initial=await api('get_context',{selection:[]},null);
    original=await read(path.join(output,'actual-restored-envelope.json'));
    await check('The previous one-piece preparation was terminally rejected against the actual twelve-piece frozen plan; no production obligations remained before correction');
  } else if (finish) {
    projectId=entry.identity.project_id;
    original=await read(path.join(output,'actual-restored-envelope.json'));
    initial=await api('get_context',{selection:[]},null);
    const stoppedProof=await read(path.join(output,'actual-prepared-stopped-cleanly.json'));
    assert.equal(stoppedProof.snapshot.coverage.clock_started,false);
    assert.equal(stoppedProof.snapshot.attempts.length,0);
    assert.deepEqual((await snapshot(stoppedProof.snapshot.identity)).snapshot,stoppedProof.snapshot);
    assert(checks.some(c=>c.name.includes('current-owner prepared')) && checks.some(c=>c.name.includes('cloud archive')));
    const state=await invoke('project_close_state');
    assert.deepEqual(state.active_tasks,[]);assert.deepEqual(state.execution_blockers,[]);
  } else {
  await page.getByText('文件', {exact: true}).click();
  await page.getByRole('button', {name: '打开工程目录…', exact: true}).click();
  let dialog;
  const deadline = Date.now()+15000;
  do {
    const result = await exec(pwsh, ['-NoProfile', '-NonInteractive', '-File', path.join(nativeRoot, 'owned-dialog.ps1'),
      '-NativeProcessId', String(launch.pid), '-ExpectedExe', launch.exe], {windowsHide: true});
    if (result.stdout.trim()) { dialog = JSON.parse(result.stdout); break; }
    await sleep(100);
  } while (Date.now() < deadline);
  assert(dialog); assert.equal(dialog.title, '选择要打开的工程目录');
  const edit = dialog.controls.find(c => c.class === 'Edit' && c.id === 1152);
  const button = dialog.controls.find(c => c.class === 'Button' && c.id === 1);
  assert(edit && button);
  await exec(pwsh, ['-NoProfile', '-NonInteractive', '-File', path.join(nativeRoot, 'owned-dialog.ps1'),
    '-NativeProcessId', String(launch.pid), '-ExpectedExe', launch.exe,
    '-SelectDirectory', path.join(previousRoot, 'project-final'), '-EditHandle', String(edit.handle), '-ButtonHandle', String(button.handle)], {windowsHide: true});
  await page.getByRole('button', {name: '确认打开', exact: true}).click();
  await page.waitForFunction(() => document.querySelector('main.workbench')?.getAttribute('aria-busy') !== 'true');
  initial = await api('get_context', {selection: []}, null);
  projectId = initial.project.id;
  assert.equal(projectId, entry.identity.project_id);
  original = await snapshot(entry.identity, entry.R);
  assert.equal(original.access.ownership, 'other_host');
  assert.deepEqual(original.access.allowed_session_controls, []);
  await write('actual-restored-envelope.json', original);
  await page.getByRole('button', {name: '云端生产', exact: true}).click();
  await page.getByRole('heading', {name: '云端生产', exact: true}).waitFor();
  const picker = page.getByRole('combobox', {name: '本机执行来源', exact: true});
  const option = picker.locator('option').filter({hasText: entry.identity.session_id+' /'});
  await option.waitFor({state: 'attached'});
  await picker.selectOption(await option.getAttribute('value'));
  await page.waitForFunction(r => document.querySelector('.cloud-source')?.textContent.includes('R'+r), entry.R);
  await openView();
  await assertReadable();
  assert.match(await page.locator('.execution-read-only').innerText(), /不是这条记录的控制方/);
  const details = await sourceDetails();
  assert.equal(details.access.ownership, 'other_host'); assert.equal(details.execution_revision, entry.R);
  assert.equal(details.read_origin, 'host'); assert(details.object_pages > 0 && details.history_pages > 0);
  await capture('01-other-session-technical'); await closeDetails();
  await capture('02-other-session-overview');
  await check('Restored other-session execution shows readable source and keeps raw identity in closed technical details', {R: entry.R, T: entry.T});

  const at = entry.T-1;
  await page.getByLabel('观察时间（秒）', {exact: true}).fill(String(at/1000));
  await page.getByRole('button', {name: '读取时点', exact: true}).click();
  await loaded();
  assert.equal((await sourceDetails()).at_host_ms, at); await closeDetails();
  await page.getByLabel('观察对象类别', {exact: true}).selectOption('device');
  assert.equal(await page.locator('.execution-object-table tbody tr').count(), 4);
  const deviceText = await page.locator('.execution-object-table').innerText();
  assert.doesNotMatch(deviceText, /\b(idle|busy|failed|fresh|stale|unknown)\b/);
  await page.locator('.execution-object-table').scrollIntoViewIfNeeded();
  await capture('03-device-states');
  await page.locator('.execution-object-table tbody tr button').first().click();
  assert.match(await page.locator('.notice').innerText(), /已选中执行记录中的对象/);
  await page.getByRole('button', {name: '清除筛选', exact: true}).click();
  await page.getByLabel('观察时间（秒）', {exact: true}).fill('0.0001');
  await page.getByRole('button', {name: '读取时点', exact: true}).click();
  assert.match(await page.locator('.execution-view-error').innerText(), /最多保留三位小数/);
  await page.getByLabel('观察时间（秒）', {exact: true}).fill(String(at/1000));
  await page.getByRole('button', {name: '读取时点', exact: true}).click();
  assert.equal(await page.locator('.execution-view-error').count(), 0, 'A valid seek to the same time clears the form error.');
  await page.getByLabel('观察时间（秒）', {exact: true}).fill('0');
  await page.getByRole('button', {name: '读取时点', exact: true}).click(); await loaded();
  assert.equal((await sourceDetails()).at_host_ms, 0); await closeDetails();
  await assertReadable();
  await check('Seconds input seeks to the exact millisecond, rejects sub-millisecond values, and returns to the real initialization', {seek_ms: at});

  await page.evaluate(() => document.querySelector('.execution-observation').scrollTo(0,0));
  await page.setViewportSize({width:1366,height:768});
  await capture('04-1366-initialization');
  const overflow = await page.locator('.execution-observation').evaluate(e => e.scrollWidth > e.clientWidth+1);
  assert.equal(overflow, false);
  assert.equal(await page.locator('.object-tree').isVisible(), false);
  assert.equal(await page.locator('.bottom-dock').isVisible(), false);
  const sceneBounds = await page.locator('.execution-view-scene').boundingBox();
  assert(sceneBounds.height >= 300 && sceneBounds.y+sceneBounds.height <= 768-15, 'The actual factory view must fit in the desktop first viewport.');
  const checkboxBounds = await page.getByRole('checkbox',{name:'物料连线',exact:true}).boundingBox();
  assert(checkboxBounds.width<=24 && checkboxBounds.height<=24, 'Scene checkboxes must keep their normal control size.');
  await check('Actual Native view fits the 1366 by 768 emulated desktop viewport', {viewport: {width:1366,height:768}});
  await back();
  await page.getByLabel('指定修订R', {exact: true}).fill(String(entry.R-1));
  await page.getByRole('button', {name: '读取指定R', exact: true}).click();
  await page.waitForFunction(r => document.querySelector('.cloud-source')?.textContent.includes('R'+r), entry.R-1);
  await openView(); await assertReadable();
  assert.equal((await sourceDetails()).execution_revision, entry.R-1); await closeDetails();
  await capture('05-historical-revision');
  await check('Historical revision keeps its own source and remains read-only', {R: entry.R-1});
  await back();
  await page.getByRole('button', {name: '云端同R只读', exact: true}).click();
  await page.waitForFunction(() => document.querySelector('.cloud-source')?.textContent.includes('云只读'));
  await openView(); await assertReadable();
  assert.match(await page.locator('.execution-read-only').innerText(), /云端存档/);
  assert.equal((await sourceDetails()).read_origin, 'cloud'); await closeDetails();
  await capture('06-cloud-read-only');
  await check('Actual HTTPS cloud archive uses the cloud read-only reason and preserves the selected revision', {R:entry.R-1});
  await back();
  }
  if (!finish) {
  await page.getByRole('tab', {name: '执行', exact: true}).click();
  await page.getByLabel('目标数量', {exact: true}).fill('12');
  await page.getByLabel('目标单位', {exact: true}).fill('piece');
  await page.getByLabel('报告截止ms', {exact: true}).fill('240000');
  await page.getByLabel('观测有效期ms', {exact: true}).fill('5000');
  const bindings = page.locator('.cloud-bindings input[type=checkbox]');
  assert.equal(await bindings.count(), 4);
  for (const binding of await bindings.all()) await binding.check();
  const prepare = page.getByRole('button', {name:'准备冻结输入',exact:true});
  assert.equal(await prepare.isEnabled(), true);
  await prepare.click();
  await page.waitForFunction(() => {
    const run = [...document.querySelectorAll('.cloud-controls button')].find(e => e.textContent === '启动');
    return run && !run.disabled;
  }, {}, {timeout:60000});
  const preparedIdentity = await page.locator('.cloud-source dl').evaluate(e => Object.fromEntries([...e.querySelectorAll(':scope > div')].map(row => [row.querySelector('dt').textContent,row.querySelector('dd').textContent])));
  const prepared = await snapshot(preparedIdentity);
  assert.equal(prepared.access.ownership,'current_host_owner');
  assert.equal(prepared.snapshot.coverage.clock_started,false);
  assert.equal(prepared.snapshot.attempts.length,0);
  await write('actual-current-owner-prepared.json',prepared);
  await openView(); await assertReadable();
  assert.match(await page.locator('.execution-read-only').innerText(),/当前本机会话维护/);
  assert.match(await page.locator('.execution-view-clock').innerText(),/尚未开始生产/);
  assert.equal(await page.getByLabel('执行观察时间游标',{exact:true}).isDisabled(),true);
  assert.equal((await sourceDetails()).access.ownership,'current_host_owner'); await closeDetails();
  await capture('07-current-owner-prepared');
  await check('Actual current-owner prepared execution is explicitly read-only and shows its real unstarted zero-time record', {identity:preparedIdentity});
  await back();
  const stop = page.getByRole('button',{name:'停止新派工',exact:true});
  assert.equal(await stop.isEnabled(),true);
  await stop.click();
  await page.waitForFunction(async () => {
    const state = await window.__TAURI_INTERNALS__.invoke('project_close_state');
    return state.active_tasks.length===0 && state.execution_blockers.length===0;
  }, {}, {timeout:60000});
  const stopped = await snapshot(preparedIdentity);
  assert.equal(stopped.snapshot.coverage.clock_started,false);
  assert.equal(stopped.snapshot.attempts.length,0);
  await write('actual-prepared-stopped-cleanly.json',stopped);
  await page.getByRole('button',{name:'返回画布',exact:true}).click();
  await page.locator('.cloud-production').waitFor({state:'hidden'});
  assert.equal(await page.locator('.object-tree').isVisible(),true);
  assert.equal(await page.locator('.bottom-dock').isVisible(),true);
  await check('Returning to the factory restores its editor panels; the prepared-only test issued no production commands and stopped cleanly');
  }
  const after = await snapshot(entry.identity, entry.R);
  assert.deepEqual(after.snapshot, original.snapshot);
  const finalContext = await api('get_context',{selection:[]},null);
  assert.deepEqual(finalContext.run_summaries,initial.run_summaries);
  let close = await invoke('project_close_state');
  const saved = path.join(native,'readability-saved-project');
  if (close.unsaved) {
    await fs.mkdir(saved);
    await page.getByText('文件',{exact:true}).click();
    await page.getByRole('button',{name:'保存工程…',exact:true}).click();
    let saveDialog;const until=Date.now()+15000;
    do {
      const result=await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(nativeRoot,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe],{windowsHide:true});
      if(result.stdout.trim()){saveDialog=JSON.parse(result.stdout);break;}
      await sleep(100);
    }while(Date.now()<until);
    assert(saveDialog);assert.match(saveDialog.title,/保存/);
    const edit=saveDialog.controls.find(c=>c.class==='Edit'&&c.id===1152),button=saveDialog.controls.find(c=>c.class==='Button'&&c.id===1);assert(edit&&button);
    await exec(pwsh,['-NoProfile','-NonInteractive','-File',path.join(nativeRoot,'owned-dialog.ps1'),'-NativeProcessId',String(launch.pid),'-ExpectedExe',launch.exe,'-SelectDirectory',saved,'-EditHandle',String(edit.handle),'-ButtonHandle',String(button.handle)],{windowsHide:true});
    await page.getByRole('status').filter({hasText:'已保存'}).waitFor({timeout:120000});
    close=await invoke('project_close_state');
    await check('New execution archive is saved through the actual OS directory picker to a separate project; the source archive is retained',{saved});
  }
  assert.equal(close.unsaved, false); assert.deepEqual(close.active_tasks, []); assert.deepEqual(close.execution_blockers, []);
  assert.deepEqual(errors, []);
  assert.equal(await page.locator('vite-error-overlay,#webpack-dev-server-client-overlay').count(),0);
  await write('receipt.json', {status:'current_owner_other_session_historical_and_actual_cloud_readability_verified',
    Browser:'Browser plugin not available; project installed Playwright connected to actual ordinary Native WebView2',
    page_identity:{url:page.url(),title:await page.title()},viewports:[{width:1920,height:1080},{width:1366,height:768}],
    pid:launch.pid,project_id:projectId,checks,screenshots,errors,warnings,rectangle,original_snapshot_unchanged:true,
    current_owner_prepared_verified:true,source_linked_resources:true,standalone_package_verified:false,saved,
    whole_factory_accepted:false,close});
  console.log('Readability checks passed for all four source/access cases in actual ordinary Native.');
} catch (error) {
  await write(resume ? 'resume-failure.json' : 'failure.json', {error:String(error),stack:error.stack,checks,errors,warnings,pid:launch.pid});
  await page.screenshot({path:path.join(output,resume ? 'resume-failure.png' : 'failure.png')}).catch(()=>{});
  throw error;
} finally { await browser.close(); }
