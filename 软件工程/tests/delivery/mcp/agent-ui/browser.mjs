import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const root = new URL('../../../../', import.meta.url);
const out = new URL('tests/delivery/mcp/output/full-package-20261017/agent-ui/', root);
await mkdir(out, { recursive: true });
const require = createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json');
const { chromium } = require('playwright');
const browser = await chromium.launch({ headless: true, executablePath: 'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe' });
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
page.setDefaultTimeout(12000);
const errors = [], warnings = [], checks = [];
page.on('pageerror', value => errors.push(String(value)));
page.on('console', value => { if (value.type() === 'error') errors.push(value.text()); if (value.type() === 'warning') warnings.push(value.text()); });
const check = async (name, run) => { await run(); checks.push({ name, passed: true }); };
try {
  const url = 'http://127.0.0.1:8922/@fs/' + fileURLToPath(root).replaceAll('\\', '/') + 'tests/delivery/mcp/agent-ui/index.html';
  await page.goto(url);
  await page.getByRole('heading', { name: '外部 Agent · 同一工程' }).waitFor();
  await check('identity, meaningful screen, no framework overlay, initial clean', async () => {
    assert.equal(await page.title(), 'P3-04 Agent工作台受控交互');
    assert.equal(await page.locator('vite-error-overlay').count(), 0);
    await page.waitForFunction(() => window.__agentUi?.state.states.length);
    assert.deepEqual(await page.evaluate(() => window.__agentUi.state.states.at(-1)), { identity: '["UI-SESSION","UI-P1","agent"]', dirty: false, busy: false });
  });
  const goal = page.getByLabel('用一句话说明产品、目标和场地');
  await check('actual field dirty and return to initial clean', async () => {
    await goal.fill('厂房24×16×6米，日产1万个纸箱');
    await page.waitForFunction(() => window.__agentUi.state.states.at(-1).dirty);
    await goal.fill('');
    await page.waitForFunction(() => !window.__agentUi.state.states.at(-1).dirty);
  });
  await check('current Host configuration is displayed and never installs a global client config', async () => {
    await page.getByLabel('实际桥程序完整路径').fill('D:/release/superfactory-mcp-bridge.exe');
    const config = await page.getByLabel('Codex MCP连接配置').inputValue();
    assert.match(config, /D:\/release\/superfactory-mcp-bridge.exe/);
    assert.match(config, /UI-HOST/); assert.match(config, /--host-pipe/);
    assert.equal(await page.evaluate(() => window.__agentUi.state.calls.some(value => /config|launch|spawn/.test(value.command))), false);
    await page.getByLabel('实际桥程序完整路径').fill('');
  });
  await page.getByLabel('Windows 已验证的连接').selectOption('PEER-1');
  await check('explicit first authorization is read-only and bound to the actual current project', async () => {
    await page.getByRole('button', { name: '确认并授权此连接' }).click();
    await page.getByRole('button', { name: '提交新的授权范围' }).waitFor();
    const call = await page.evaluate(() => window.__agentUi.state.calls.find(value => value.command === 'agent_authorize_connection'));
    assert.equal(call.args.sessionId, 'PEER-1'); assert.equal(call.args.choice.project_id, 'UI-P1');
    assert.deepEqual(call.args.choice.edit, []); assert.equal(call.args.choice.allow_view_control, false);
    assert.equal(call.args.choice.operations.includes('apply_commands'), false);
  });
  await page.getByText('对象、字段、新增数量与复用资源权限', { exact: true }).click();
  const grants = page.getByLabel('编辑权限JSON');
  await check('malformed local permissions are retained and not sent to Native', async () => {
    await grants.fill('{broken');
    await page.getByRole('button', { name: '提交新的授权范围' }).click();
    await page.getByRole('alert').waitFor(); assert.equal(await grants.inputValue(), '{broken');
    assert.equal(await page.evaluate(() => window.__agentUi.state.calls.filter(value => value.command === 'agent_update_connection_authority').length), 0);
  });
  await check('explicit denied authority preserves local scope and permission proposal', async () => {
    await grants.fill('[]');
    await page.evaluate(() => { window.__agentUi.state.failure = 'PermissionDenied: scope changed'; });
    await page.getByRole('button', { name: '提交新的授权范围' }).click();
    await page.getByRole('alert').filter({ hasText: 'PermissionDenied' }).waitFor();
    assert.equal(await grants.inputValue(), '[]');
    await page.evaluate(() => { window.__agentUi.state.failure = ''; });
  });
  await check('authorization does not mark an unsaved business goal as saved', async () => {
    await goal.fill('保留的真实用户目标');
    await page.getByRole('button', { name: '提交新的授权范围' }).click();
    await page.getByRole('status').filter({ hasText: '所选范围已提交' }).waitFor();
    assert.equal(await page.evaluate(() => window.__agentUi.state.states.at(-1).dirty), true);
    await page.getByRole('button', { name: '关闭工作台' }).click();
    await page.getByRole('alertdialog', { name: '未提交Agent输入' }).waitFor();
    await page.getByRole('button', { name: '保留输入，返回' }).click();
    assert.equal(await goal.inputValue(), '保留的真实用户目标');
  });
  await check('Native directory token is scoped to selected peer and purpose', async () => {
    await page.getByLabel('为此连接选择目录用途').selectOption('export_evaluation');
    await page.getByRole('button', { name: '在 Native 对话框选择目录' }).click();
    await page.getByRole('status').filter({ hasText: '目录令牌仅授予' }).waitFor();
    const call = await page.evaluate(() => window.__agentUi.state.calls.find(value => value.command === 'agent_select_directory'));
    assert.deepEqual(call.args, { sessionId: 'PEER-1', purpose: 'export_evaluation' });
  });
  await check('real current-project revision observation refreshes the parent without clearing local goal', async () => {
    const before = await page.evaluate(() => window.__agentUi.state.refreshes);
    await page.evaluate(() => { window.__agentUi.state.revision = 4; });
    await page.waitForFunction(count => window.__agentUi.state.refreshes > count, before);
    assert.equal(await goal.inputValue(), '保留的真实用户目标');
    assert.equal(await page.evaluate(() => window.__agentUi.state.states.at(-1).dirty), true);
    await page.evaluate(() => { window.__agentUi.state.revision = 3; });
  });
  await page.screenshot({ path: fileURLToPath(new URL('desktop.png', out)), fullPage: true });
  await check('explicit refresh publishes busy; stale completion cannot dirty a replacement identity', async () => {
    await page.evaluate(() => { window.__agentUi.state.delayedRefresh = true; });
    await page.getByRole('button', { name: '刷新 Agent 已提交的工程变化' }).click();
    await page.waitForFunction(() => window.__agentUi.state.states.at(-1).busy);
    assert.equal(await page.getByRole('button', { name: '关闭工作台' }).isDisabled(), true);
    await page.evaluate(() => window.__agentUi.render('UI-SESSION-NEW'));
    await page.waitForFunction(() => window.__agentUi.state.states.at(-1).identity.includes('UI-SESSION-NEW'));
    await page.evaluate(() => window.__agentUi.state.resolveRefresh());
    await page.waitForTimeout(50);
    const last = await page.evaluate(() => window.__agentUi.state.states.at(-1));
    assert.equal(last.identity, '["UI-SESSION-NEW","UI-P1","agent"]'); assert.equal(last.dirty, false); assert.equal(last.busy, false);
  });
  assert.deepEqual(errors, []); assert.deepEqual(warnings, []);
  await writeFile(new URL('result.json', out), JSON.stringify({ method: 'actual product component; controlled private Native invocation boundary; not real Host/Agent proof', Browser: 'Browser plugin not available; existing Playwright runtime used', url, viewport: { width: 1440, height: 1000 }, checks, errors, warnings, actual: await page.evaluate(() => window.__agentUi.state) }, null, 2));
  console.log(JSON.stringify({ passed: checks.length, failed: 0, errors, warnings }));
} catch (error) {
  await page.screenshot({ path: fileURLToPath(new URL('failure.png', out)), fullPage: true });
  await writeFile(new URL('failure.json', out), JSON.stringify({ checks, error: String(error), errors, warnings }, null, 2));
  throw error;
} finally { await browser.close(); }
