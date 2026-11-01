import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import { openCore, root, out } from './current-core.mjs';
const require = createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json'), { chromium } = require('playwright');
let core, browser, page;
const result = { kind: 'actual_current_Host_empty_Project_File_New_and_start_card_are_distinct', Native_app: false, status: 'started' };
try {
  core = await openCore(); await core.rpc({ kind: 'new_project' }); const before = (await core.rpc({ kind: 'snapshot' })).project;
  browser = await chromium.launch({ headless: true, executablePath: 'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe' }); page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
  await page.route('**/__p201_core*', async route => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(route.request().method() === 'GET' ? core.ready : await core.rpc(route.request().postDataJSON())) }));
  await page.goto('http://127.0.0.1:8922/@fs/' + fileURLToPath(root).replaceAll('\\', '/').replace(/\/$/, '') + '/tests/planner/p201-whole/harness.html'); await page.getByLabel('工程开始方式').waitFor();
  await page.getByText('文件', { exact: true }).click(); await page.getByRole('button', { name: '新建工程…', exact: true }).click(); const modal = page.locator('.modal'); await modal.getByRole('textbox', { name: '工程名称', exact: true }).waitFor(); assert.equal(await page.locator('.object-editor').count(), 0);
  await modal.getByRole('textbox', { name: '工程名称', exact: true }).fill('P201 a distinct empty Project'); await modal.getByRole('combobox', { name: '离开当前工程', exact: true }).selectOption('discard_after_tasks_terminal'); await modal.getByRole('button', { name: '创建', exact: true }).click(); await modal.waitFor({ state: 'hidden' });
  const after = (await core.rpc({ kind: 'snapshot' })).project; assert.notEqual(after.id, before.id); assert.equal(after.halls.length, 0); assert.equal(after.devices.length, 0);
  await page.getByLabel('工程开始方式').getByRole('button', { name: /^空工程/ }).click(); const editor = page.locator('.object-editor'); await editor.getByRole('heading', { name: '新建厂房', exact: true }).waitFor(); assert.equal((await core.rpc({ kind: 'snapshot' })).project.id, after.id); await editor.getByRole('button', { name: '放弃更改', exact: true }).click();
  await page.screenshot({ path: fileURLToPath(new URL('empty-project-new-after.png', out)), fullPage: true }); result.status = 'passed'; result.previous_id = before.id; result.new_id = after.id; result.File_New_created_distinct_identity = true; result.start_card_reused_own_empty_identity = true;
} catch (error) { result.status = 'failed'; result.error = String(error); process.exitCode = 1; }
finally { await writeFile(new URL('empty-project-new-after.json', out), JSON.stringify({ ...result, records: core?.records }, null, 2)); await browser?.close(); await core?.close(); }
console.log(JSON.stringify(result));
