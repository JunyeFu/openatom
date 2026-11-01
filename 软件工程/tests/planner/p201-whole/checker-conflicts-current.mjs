import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import { openCore, root, out } from './current-core.mjs';
const require = createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json'), { chromium } = require('playwright');
const result = { kind: 'actual_current_Host_checker_conflicts_and_product_location', Native_app: false, errors: [] };
let core, browser, page;
try {
  core = await openCore(); const initial = core.ready.snapshot.project, original = initial.devices[0], hall = initial.halls[0];
  const request = async (operation, params) => {
    const reply = await core.rpc({ kind: 'dispatch', request: { api_version: '0.1', request_id: 'checker-conflicts-' + Math.random(), project_id: initial.id, scope: { kind: 'project' }, operation, params } });
    assert.equal(reply.outcome.status, 'ok'); return reply.outcome.data;
  };
  const first = { ...structuredClone(original), title: 'P201 known site conflict', line_id: null, height_m: hall.height_m + 1,
    pose: { ...structuredClone(original.pose), translation: { x: 0, y: hall.depth_m / 2, z: 0 } }, footprint_m: [{ x: -.5, y: -.5 }, { x: .5, y: -.5 }, { x: .5, y: .5 }, { x: -.5, y: .5 }] };
  const second = { ...structuredClone(first), id: 'P201-known-overlap', title: 'P201 known overlap', role_bindings: { item_types: {}, resources: {}, targets: {}, ports: {} } };
  await request('apply_commands', { base_revision: initial.revision, commands: [
    ...initial.lines.filter(line => line.members.includes(original.id)).map(line => ({ kind: 'put_object', data: { kind: 'line', data: { ...line, members: line.members.filter(id => id !== original.id) } } })),
    ...[first, second].map(data => ({ kind: 'put_object', data: { kind: 'device', data } }))] });
  const before = (await core.rpc({ kind: 'snapshot' })).project;
  browser = await chromium.launch({ headless: true, executablePath: 'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe' });
  page = await browser.newPage({ viewport: { width: 1920, height: 1080 } }); page.setDefaultTimeout(15000);
  page.on('pageerror', error => result.errors.push(String(error))); page.on('console', message => { if (message.type() === 'error') result.errors.push(message.text()); });
  await page.route('**/__p201_core*', async route => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(route.request().method() === 'GET' ? core.ready : await core.rpc(route.request().postDataJSON())) }));
  await page.goto('http://127.0.0.1:8922/@fs/' + fileURLToPath(root).replaceAll('\\', '/').replace(/\/$/, '') + '/tests/planner/p201-whole/harness.html');
  await page.getByRole('button', { name: '草稿检查', exact: true }).click(); const form = page.getByRole('form', { name: '工程草稿检查', exact: true });
  await form.getByRole('textbox', { name: '行业', exact: true }).fill('自建静态冲突验证'); await form.getByRole('textbox', { name: '产品族', exact: true }).fill('组装');
  const product = before.products[0]; await form.getByRole('combobox', { name: '产品修订', exact: true }).selectOption(product.id + '@' + product.revision);
  await form.getByRole('combobox', { name: '验收目的库位', exact: true }).selectOption('finished'); await form.getByRole('spinbutton', { name: '观察开始 / ms', exact: true }).fill('0'); await form.getByRole('spinbutton', { name: '观察结束 / ms', exact: true }).fill('120000');
  await form.getByRole('combobox', { name: '场地参照', exact: true }).selectOption(hall.id); await form.getByRole('combobox', { name: '生产日历', exact: true }).selectOption(before.calendars[0].id);
  await form.getByRole('button', { name: '执行只读检查', exact: true }).click(); const actual = form.getByLabel('实际草稿检查结果'); await actual.waitFor();
  const report = core.records.findLast(row => row.request.request?.operation === 'check_factory_draft').reply.outcome.data;
  result.report = report;
  for (const kind of ['site_bounds', 'footprint_overlap', 'height']) assert.ok(report.checks.some(check => check.kind === kind && check.outcome === 'fail'), 'Missing actual failure: ' + kind);
  assert.ok(report.checks.some(check => check.kind === 'clearance_and_static_lanes' && check.outcome === 'undetermined'));
  await actual.locator('article').filter({ hasText: '失败 · 设备高度' }).first().getByRole('button', { name: 'device / ' + original.id, exact: true }).click();
  assert.match(await page.getByLabel('对象属性').innerText(), /P201 known site conflict/);
  assert.deepEqual((await core.rpc({ kind: 'snapshot' })).project, before);
  await page.screenshot({ path: fileURLToPath(new URL('checker-conflicts-current.png', out)), fullPage: true });
  await form.getByRole('button', { name: '返回画布', exact: true }).click(); await form.getByRole('button', { name: '明确放弃检查输入', exact: true }).click(); await form.waitFor({ state: 'hidden' });
  await page.locator('.scene-label.device').filter({ hasText: 'P201 known site conflict · 占地超出厂房范围' }).waitFor();
  assert.equal(result.errors.length, 0, result.errors.join('\n')); result.status = 'passed'; result.revision_unchanged = before.revision;
} catch (error) { result.status = 'failed'; result.error = String(error); process.exitCode = 1; await page?.screenshot({ path: fileURLToPath(new URL('checker-conflicts-current-failure.png', out)), fullPage: true }); }
finally { await writeFile(new URL('checker-conflicts-current.json', out), JSON.stringify(result, null, 2)); await browser?.close(); await core?.close(); }
console.log(JSON.stringify({ status: result.status, error: result.error, errors: result.errors, checks: result.report?.checks.length, Native_app: false }));
