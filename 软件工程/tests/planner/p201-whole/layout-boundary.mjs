import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import { openCore, root, out } from './current-core.mjs';
const require = createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json');
const { chromium } = require('playwright'), suffix = process.argv[2] ?? 'before';
let core, browser, page;
const result = { kind: 'actual_current_Host_normal_Workbench_detailed_bounds_highlight', Native_app: false, status: 'started' };
try {
  core = await openCore(); const project = core.ready.snapshot.project, device = project.devices[0], hall = project.halls.find(value => value.id === device.hall_id);
  browser = await chromium.launch({ headless: true, executablePath: 'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe' }); page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
  await page.route('**/__p201_core*', async route => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(route.request().method() === 'GET' ? core.ready : await core.rpc(route.request().postDataJSON())) }));
  await page.goto('http://127.0.0.1:8922/@fs/' + fileURLToPath(root).replaceAll('\\', '/').replace(/\/$/, '') + '/tests/planner/p201-whole/harness.html');
  await page.getByLabel('工程对象树').getByRole('button', { name: '▣ ' + device.title, exact: true }).click(); await page.getByRole('button', { name: '打开详情编辑', exact: true }).click();
  const form = page.locator('.object-editor'); await form.getByRole('spinbutton', { name: 'X · 厂房坐标 m', exact: true }).fill(String(hall.width_m + 2)); await form.getByRole('spinbutton', { name: 'Y · 厂房坐标 m', exact: true }).fill(String(hall.depth_m / 2)); await form.getByRole('button', { name: '应用到工程', exact: true }).click(); await form.waitFor({ state: 'hidden' });
  await page.waitForFunction(title => [...document.querySelectorAll('.scene-label')].some(value => value.textContent.includes(title) && value.textContent.includes('静态素材')), device.title);
  result.visuals = await page.evaluate(id => {
    const values = [], scene = window.__p201.scene().scene;
    scene.traverse(object => { if (object.userData.ref?.kind !== 'device' || object.userData.ref.id !== id || !object.material?.color || object.material.color.getHex() !== 0xc84b4b) return; let visible = true, parent = object; while (parent) { visible &&= parent.visible; parent = parent.parent; } values.push({ type: object.type, visible }); }); return values;
  }, device.id);
  result.label = await page.locator('.scene-label').filter({ hasText: device.title }).first().textContent();
  await page.screenshot({ path: fileURLToPath(new URL('layout-boundary-' + suffix + '.png', out)), fullPage: true });
  assert.match(result.label, /占地超出厂房范围/); assert.ok(result.visuals.some(value => value.visible), 'The actual detailed media must not hide the declared red out-of-bounds footprint'); result.status = 'passed';
} catch (error) { result.status = 'failed'; result.error = String(error); process.exitCode = 1; }
finally { await writeFile(new URL('layout-boundary-' + suffix + '.json', out), JSON.stringify(result, null, 2)); await browser?.close(); await core?.close(); }
console.log(JSON.stringify(result));
