import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
const require = createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json');
const { chromium } = require('playwright'), root = new URL('../../../', import.meta.url), out = new URL('.work/p201-whole-20261017/b60-geometry/', root);
const browser = await chromium.launch({ headless: true, executablePath: 'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe' });
const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } }), errors = [], frames = [];
page.on('pageerror', error => errors.push(String(error))); page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
try {
  await page.goto('http://127.0.0.1:8922/@fs/' + fileURLToPath(root).replaceAll('\\', '/').replace(/\/$/, '') + '/tests/planner/p201-whole/b60-preview.html');
  await page.getByText(/105 个声明原生网格/).waitFor(); assert.equal(await page.locator('vite-error-overlay').count(), 0);
  for (const view of ['正视', '俯视', '适配全部']) { await page.getByRole('button', { name: view, exact: true }).click(); const path = fileURLToPath(new URL('station-' + ({ 正视: 'front', 俯视: 'top', 适配全部: 'oblique' })[view] + '.png', out)); await page.screenshot({ path, fullPage: true }); frames.push({ view, path }); }
  await page.getByRole('button', { name: '正视', exact: true }).click(); const box = await page.locator('canvas').boundingBox(), x = box.x + box.width / 2, y = box.y + box.height / 2;
  await page.mouse.move(x - box.height / 4, y); await page.mouse.down(); await page.mouse.move(x + box.height / 4, y, { steps: 20 }); await page.mouse.up(); await page.screenshot({ path: fileURLToPath(new URL('station-rear.png', out)), fullPage: true }); frames.push({ view: 'rear_by_actual_orbit' });
  await page.getByRole('button', { name: '正视', exact: true }).click(); await page.mouse.move(x, y); await page.mouse.down(); await page.mouse.move(x, y - box.height / 5, { steps: 20 }); await page.mouse.up(); await page.screenshot({ path: fileURLToPath(new URL('station-bottom.png', out)), fullPage: true }); frames.push({ view: 'bottom_by_actual_orbit' });
  assert.equal(errors.length, 0, errors.join('\n'));
} finally { await writeFile(new URL('browser-preview.json', out), JSON.stringify({ frames, errors, Native_app: false, Engine: false, kind: 'declared_geometry_preview_only', quality_acceptance: null }, null, 2)); await browser.close(); }
console.log(JSON.stringify({ frames: frames.length, errors, Native_app: false, Engine: false }));
