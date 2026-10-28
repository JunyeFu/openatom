import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { openCore, root } from '../../../tests/planner/factory-actual-items/core.mjs';

const out = new URL('scenarios/Agent共建/runtime-factory-render-surface/labels-regression/', root);
await mkdir(out, { recursive: true });
const require = createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json');
const { chromium } = require('playwright');
const browser = await chromium.launch({ headless: true, executablePath: 'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe' });
let core, page;
const checks = [], errors = [], warnings = [];
async function settle() {
  await page.waitForFunction(() => window.__items?.read().source && document.querySelector('[data-scene-label-key]'));
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}
async function visibleBoxes() {
  return page.locator('.scene-annotations button:visible').evaluateAll(elements => elements.map(element => {
    const { left, top, right, bottom } = element.getBoundingClientRect();
    return { key: element.dataset.sceneLabelKey, text: element.textContent, left, top, right, bottom };
  }));
}
async function clearLabels(name) {
  await settle();
  const boxes = await visibleBoxes();
  assert.ok(boxes.length, 'scene keeps readable labels');
  for (let i = 0; i < boxes.length; i++) for (let j = i + 1; j < boxes.length; j++) {
    const a = boxes[i], b = boxes[j];
    assert.ok(a.right <= b.left || b.right <= a.left || a.bottom <= b.top || b.bottom <= a.top, `${a.key} overlaps ${b.key}`);
  }
  const rows = await page.locator('[data-scene-label-key]').evaluateAll(elements => elements.map(element => ({ key: element.dataset.sceneLabelKey, hidden: getComputedStyle(element).visibility === 'hidden', text: element.textContent })));
  const sideKeys = await page.locator('[data-collapsed-label-key]').evaluateAll(elements => elements.map(element => element.dataset.collapsedLabelKey));
  assert.ok(rows.filter(row => row.hidden).every(row => sideKeys.includes(row.key)), 'every suppressed label remains in the side list');
  const state = await page.evaluate(() => window.__items.read());
  assert.equal(state.runUnchanged, true);
  checks.push({ name, visible: boxes, suppressed: rows.filter(row => row.hidden), actual_items: state.source.items.map(value => value.item.id) });
}
try {
  core = await openCore('scenarios/媒体表现/runtime-factory-items/root-input.json');
  page = await browser.newPage({ viewport: { width: 1320, height: 920 } });
  page.on('pageerror', error => errors.push(String(error)));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); if (message.type() === 'warning') warnings.push(message.text()); });
  await page.route('**/__factory_items_core*', async route => {
    const value = route.request().method() === 'GET' ? core.ready : await core.rpc(route.request().postDataJSON());
    await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(value) });
  });
  await page.goto('http://127.0.0.1:8922/@fs/' + fileURLToPath(root).replaceAll('\\', '/').replace(/\/$/, '') + '/tests/planner/factory-actual-items/harness.html');
  await page.waitForFunction(() => window.__items);
  await page.evaluate(() => window.__items.set({ at: 110000 }));
  await page.waitForFunction(() => window.__items.read().source?.observation?.at_ms === 110000);
  await clearLabels('actual multi-history scene: readable labels and complete collapsed list');
  const details = page.locator('details[aria-label="实际物品与共享标签"]');
  await details.locator('summary').click();
  await clearLabels('expanded actual item list: labels remain clear');
  const state = await page.evaluate(() => window.__items.read());
  for (const { item } of state.source.items) await page.getByRole('button', { name: item.id, exact: true }).waitFor({ state: 'visible' });
  const collapsed = page.locator('[data-collapsed-label-key^="item:"]').first();
  const key = await collapsed.getAttribute('data-collapsed-label-key');
  await collapsed.click();
  assert.deepEqual((await page.evaluate(() => window.__items.read())).selected, { kind: 'item', id: key.slice(5) });
  checks.push({ name: 'collapsed actual item label selects the original item identity', key });
  await page.screenshot({ path: fileURLToPath(new URL('actual-side-list.png', out)), fullPage: true });
  await details.locator('summary').click();
  await page.setViewportSize({ width: 920, height: 720 });
  await page.locator('.factory-scene').evaluate(element => { element.style.width = '880px'; element.style.height = '580px'; });
  await clearLabels('resized actual canvas: no overlapping projected labels');
  assert.equal(await page.locator('.factory-scene').evaluate(element => element.clientWidth), 880);
  checks.at(-1).canvas_width = 880;
  await page.locator('.factory-scene').screenshot({ path: fileURLToPath(new URL('actual-resized.png', out)) });
  await page.evaluate(() => window.__items.set({ at: 40000 }));
  await page.waitForFunction(() => window.__items.read().source?.observation?.at_ms === 40000);
  await clearLabels('reverse actual seek: new labels stay readable without changing the Run');
  assert.deepEqual(errors, []);
} catch (error) {
  checks.push({ name: 'browser execution', status: 'failed', error: String(error), stack: error.stack });
  process.exitCode = 1;
} finally {
  if (page) await page.close();
  if (core) await core.close();
  await browser.close();
  await writeFile(new URL('browser-result.json', out), JSON.stringify({ checks, errors, warnings, actual_Host: true, Native: false, Host_cut: 'existing runtime-factory-items core_consumer.exe frozen before Media-adoption successor; source/Run behavior under test unchanged', prior_observed_overlap: 'scenarios/媒体表现/runtime-factory-items/real-multi-history-quantity.png' }, null, 2) + '\n');
}
console.log(JSON.stringify({ groups: checks.length, failed: checks.filter(row => row.status === 'failed').length, errors, warnings }));
