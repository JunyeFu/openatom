import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { writeFile, readFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import { openCore, root, out } from './current-core.mjs';
const require = createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json');
const { chromium } = require('playwright');
const checks = [], errors = [], warnings = [];
let core, browser, page;
let delayObjectReads = false; const playbackQueries = [];
const snapshot = () => core.rpc({ kind: 'snapshot' });
async function request(operation, params) {
  const project = (await snapshot()).project;
  const reply = await core.rpc({ kind: 'dispatch', request: { api_version: '0.1', request_id: 'p201-' + Math.random(), project_id: project.id, scope: { kind: 'project' }, operation, params } });
  assert.equal(reply.outcome.status, 'ok', JSON.stringify(reply)); return reply.outcome.data;
}
async function apply(commands) { return request('apply_commands', { base_revision: (await snapshot()).project.revision, commands }); }
async function idle() { await page.waitForFunction(() => document.querySelector('.workbench')?.getAttribute('aria-busy') === 'false'); }
async function refresh() { await page.getByText('文件', { exact: true }).click(); await page.getByRole('button', { name: '刷新工程', exact: true }).click(); await idle(); await page.locator('.scene-view canvas').waitFor().catch(() => page.locator('.factory-scene canvas').waitFor()); }
async function menuEdit(name) { await page.getByText('编辑', { exact: true }).click(); await page.getByRole('button', { name, exact: true }).click(); await idle(); }
async function choose(title, additive = false) { await page.getByLabel('工程对象树').getByRole('button', { name: '▣ ' + title, exact: true }).click({ modifiers: additive ? ['Shift'] : [] }); }
async function check(name, action) {
  try { const evidence = await action(); checks.push({ name, status: 'passed', evidence }); console.log('PASS ' + name); }
  catch (error) { checks.push({ name, status: 'failed', error: String(error), stack: error.stack, body: await page?.locator('body').innerText() }); process.exitCode = 1; await page?.screenshot({ path: fileURLToPath(new URL('product-failure-' + checks.length + '.png', out)), fullPage: true }); throw error; }
}
try {
  core = await openCore();
  const initial = (await snapshot()).project, hall = initial.halls[0], original = initial.devices[0];
  const ids = [original.id, 'p201-B', 'p201-C'];
  const devices = ids.map((id, index) => ({ ...structuredClone(original), id, title: 'P201 ' + ['A', 'B', 'C'][index], hall_id: hall.id, line_id: null,
    pose: { ...structuredClone(original.pose), translation: { x: hall.width_m * (.2 + index * .2), y: hall.depth_m * .4, z: 0 } },
    footprint_m: [{ x: -.3, y: -.3 }, { x: .3, y: -.3 }, { x: .3, y: .3 }, { x: -.3, y: .3 }], role_bindings: index === 0 ? structuredClone(original.role_bindings) : { item_types: {}, resources: {}, targets: {}, ports: {} } }));
  await apply([...initial.lines.filter(line => line.members.includes(original.id)).map(line => ({ kind: 'put_object', data: { kind: 'line', data: { ...line, members: line.members.filter(id => id !== original.id) } } })), ...devices.map(device => ({ kind: 'put_object', data: { kind: 'device', data: device } }))]);
  browser = await chromium.launch({ headless: true, executablePath: 'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe' });
  page = await browser.newPage({ viewport: { width: 1920, height: 1080 } }); page.setDefaultTimeout(12000);
  page.on('pageerror', error => errors.push(String(error))); page.on('console', value => { if (value.type() === 'error') errors.push(value.text()); if (value.type() === 'warning') warnings.push(value.text()); });
  await page.route('**/__p201_core*', async route => {
    const query = route.request().method() === 'GET' ? null : route.request().postDataJSON(), view = query?.request?.params?.query;
    if (delayObjectReads && query?.request?.operation === 'get_run' && view?.kind === 'objects_at') { if (view.data.objects.length === 0 && view.data.page.limit === 500) playbackQueries.push({ at_ms: view.data.at_ms, wall_ms: performance.now() }); await new Promise(resolve => setTimeout(resolve, 250)); }
    return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(query ? await core.rpc(query) : core.ready) });
  });
  await page.goto('http://127.0.0.1:8922/@fs/' + fileURLToPath(root).replaceAll('\\', '/').replace(/\/$/, '') + '/tests/planner/p201-whole/harness.html');
  await page.getByRole('button', { name: '输入草稿', exact: true }).waitFor(); await page.locator('.factory-scene canvas').waitFor(); await idle();
  await check('normal Workbench loads two canvases with no error overlay or console errors', async () => {
    assert.match(await page.title(), /P2-01/); assert.equal(await page.locator('vite-error-overlay').count(), 0); assert.equal(errors.length, 0, errors.join('\n'));
    await page.getByRole('button', { name: '多厂房画布', exact: true }).click(); await page.getByRole('button', { name: '流水线画布', exact: true }).click();
    await page.screenshot({ path: fileURLToPath(new URL('product-initial.png', out)), fullPage: true });
    return { viewport: '1920x1080', native: false, real_host: true, title: await page.title() };
  });
  await check('real capability catalog and read-only checker reach all six check kinds with truthful per-object outcomes in the normal product form', async () => {
    const before = (await snapshot()).project;
    assert.ok(core.ready.capabilities.implemented_operations.some(value => value.operation === 'check_factory_draft'));
    await page.getByRole('button', { name: '草稿检查', exact: true }).click(); const form = page.getByRole('form', { name: '工程草稿检查', exact: true });
    await form.getByRole('textbox', { name: '行业', exact: true }).fill('自建接口验证'); await form.getByRole('textbox', { name: '产品族', exact: true }).fill('总装');
    const product = before.products[0]; await form.getByRole('combobox', { name: '产品修订', exact: true }).selectOption(product.id + '@' + product.revision);
    await form.getByRole('combobox', { name: '验收目的库位', exact: true }).selectOption('finished'); await form.getByRole('spinbutton', { name: '观察开始 / ms', exact: true }).fill('0'); await form.getByRole('spinbutton', { name: '观察结束 / ms', exact: true }).fill('120000');
    await form.getByRole('combobox', { name: '场地参照', exact: true }).selectOption(before.halls[0].id); await form.getByRole('combobox', { name: '生产日历', exact: true }).selectOption(before.calendars[0].id);
    await form.getByRole('button', { name: '执行只读检查', exact: true }).click(); const result = form.getByLabel('实际草稿检查结果'); await result.waitFor(); const outcomes = await result.locator('[data-check-outcome]').evaluateAll(elements => elements.map(element => ({ outcome: element.getAttribute('data-check-outcome'), text: element.textContent })));
    const actualCheck = core.records.findLast(value => value.request.request?.operation === 'check_factory_draft').reply.outcome.data; assert.equal(outcomes.length, actualCheck.checks.length); assert.deepEqual(new Set(actualCheck.checks.map(value => value.kind)), new Set(['references_and_units', 'material_process_connectivity', 'site_bounds', 'footprint_overlap', 'height', 'clearance_and_static_lanes'])); assert.ok(outcomes.every(value => ['pass', 'fail', 'undetermined'].includes(value.outcome))); assert.ok(outcomes.some(value => value.outcome === 'undetermined'), 'Unknown clearance/target inputs cannot become a fabricated pass');
    assert.deepEqual((await snapshot()).project, before);
    await page.screenshot({ path: fileURLToPath(new URL('product-current-checker.png', out)), fullPage: true }); await form.getByRole('button', { name: '返回画布', exact: true }).click(); await form.getByRole('button', { name: '明确放弃检查输入', exact: true }).click(); await form.waitFor({ state: 'hidden' });
    return { revision: before.revision, outcomes, actual_same_Host: true, Native_app: false };
  });
  await check('Shift multiselect, real atomic grouping, Undo and Redo keep physical poses', async () => {
    const before = (await snapshot()).project;
    await choose('P201 A'); await choose('P201 B', true);
    await page.getByRole('button', { name: '编组所选', exact: true }).click();
    const form = page.getByRole('form', { name: '编组所选设备' });
    await form.getByRole('textbox', { name: '新线组ID', exact: true }).fill('p201-group'); await form.getByRole('textbox', { name: '线组名称', exact: true }).fill('P201 grouped pair');
    await form.getByRole('button', { name: '应用编组', exact: true }).click(); await form.waitFor({ state: 'hidden' });
    const grouped = (await snapshot()).project, line = grouped.lines.find(value => value.id === 'p201-group');
    assert.deepEqual(new Set(line.members), new Set(ids.slice(0, 2))); assert.equal(grouped.devices.find(value => value.id === ids[2]).line_id, null);
    await menuEdit('撤销'); const undone = (await snapshot()).project; assert.deepEqual(undone.devices, before.devices); assert.deepEqual(undone.lines, before.lines); assert.ok(undone.revision > grouped.revision);
    await menuEdit('重做'); const redone = (await snapshot()).project; assert.deepEqual(redone.devices, grouped.devices); assert.deepEqual(redone.lines, grouped.lines); assert.ok(redone.revision > undone.revision);
    return { before_revision: before.revision, grouped_revision: grouped.revision, undo_revision: undone.revision, redo_revision: redone.revision, members: line.members };
  });
  await check('copy group requires explicit new IDs and does not clone business quantities or connections', async () => {
    const before = (await snapshot()).project;
    await page.getByLabel('工程对象树').getByRole('button', { name: '▤ P201 grouped pair', exact: true }).click();
    await page.getByRole('button', { name: '复制为新ID', exact: true }).click();
    const form = page.getByRole('form', { name: '复制布局为新ID' }); await form.getByRole('button', { name: '复制为新ID', exact: true }).click(); await form.waitFor({ state: 'hidden' });
    const after = (await snapshot()).project, copy = after.lines.find(value => value.id === 'p201-group-copy');
    assert.deepEqual(copy.members.sort(), ids.slice(0, 2).map(id => id + '-copy').sort()); assert.deepEqual(after.connections, before.connections); assert.deepEqual(after.locations, before.locations); assert.deepEqual(after.acknowledgments, before.acknowledgments);
    assert.deepEqual(after.devices.find(value => value.id === ids[0] + '-copy').role_bindings, { item_types: {}, resources: {}, targets: {}, ports: {} });
    return { new_line: copy.id, new_members: copy.members, revision: after.revision };
  });
  await check('stale group form retains input, rereads actual revision, and discards redo branch after new edit', async () => {
    await choose('P201 C'); await page.getByRole('button', { name: '编组所选', exact: true }).click();
    const form = page.getByRole('form', { name: '编组所选设备' }); await form.getByRole('textbox', { name: '新线组ID', exact: true }).fill('p201-conflict'); await form.getByRole('textbox', { name: '线组名称', exact: true }).fill('retained conflict input');
    const base = (await snapshot()).project; await apply([{ kind: 'put_object', data: { kind: 'hall', data: { ...base.halls[0], title: 'actual concurrent hall edit' } } }]);
    await form.getByRole('button', { name: '应用编组', exact: true }).click(); await form.getByRole('alert').waitFor(); assert.equal(await form.getByRole('textbox', { name: '线组名称', exact: true }).inputValue(), 'retained conflict input');
    assert.ok(!(await snapshot()).project.lines.some(value => value.id === 'p201-conflict'));
    await form.getByRole('button', { name: '重读基础并保留输入', exact: true }).click(); await form.getByRole('button', { name: '应用编组', exact: true }).click(); await form.waitFor({ state: 'hidden' });
    await menuEdit('撤销'); await choose('P201 C'); await page.getByRole('button', { name: '复制为新ID', exact: true }).click(); const copy = page.getByRole('form', { name: '复制布局为新ID' }); await copy.getByRole('button', { name: '复制为新ID', exact: true }).click(); await copy.waitFor({ state: 'hidden' });
    await page.getByText('编辑', { exact: true }).click(); assert.equal(await page.getByRole('button', { name: '重做', exact: true }).isDisabled(), true); await page.getByText('编辑', { exact: true }).click();
    return { conflicted_revision: base.revision, retained_input: 'retained conflict input', redo_after_new_edit: false };
  });
  await check('real 3D orbit, pan, zoom and reset do not edit project or switch detail', async () => {
    await choose('P201 C'); const before = (await snapshot()).project;
    const camera = () => page.evaluate(() => ({ p: window.__p201.scene().camera.position.toArray(), q: window.__p201.scene().camera.quaternion.toArray() }));
    const initialCamera = await camera(), box = await page.locator('.factory-scene canvas').boundingBox(), x = box.x + box.width * .6, y = box.y + box.height * .5;
    await page.mouse.move(x, y); await page.mouse.down({ button: 'right' }); await page.mouse.move(x + 80, y + 30, { steps: 8 }); await page.mouse.up({ button: 'right' }); const orbited = await camera(); assert.notDeepEqual(orbited.q, initialCamera.q);
    await page.mouse.move(x, y); await page.mouse.down({ button: 'middle' }); await page.mouse.move(x + 50, y + 20, { steps: 6 }); await page.mouse.up({ button: 'middle' }); const panned = await camera(); assert.notDeepEqual(panned.p, orbited.p);
    await page.mouse.wheel(0, -250); const zoomed = await camera(); assert.notDeepEqual(zoomed.p, panned.p); assert.equal(await page.locator('.object-editor').count(), 0);
    await page.getByRole('button', { name: '⌂ 复位视图', exact: true }).click(); const reset = await camera(); assert.deepEqual(reset, initialCamera); assert.deepEqual((await snapshot()).project, before);
    await page.screenshot({ path: fileURLToPath(new URL('product-camera.png', out)), fullPage: true }); return { initialCamera, orbited, panned, zoomed, reset, project_revision: before.revision };
  });
  await check('screen drag of explicit move handle snaps the declared pose through real common commands', async () => {
    await choose('P201 C'); await page.getByRole('checkbox', { name: '网格吸附', exact: true }).check(); await page.getByRole('button', { name: '↔ 移动手柄', exact: true }).click();
    const before = (await snapshot()).project, device = before.devices.find(value => value.id === ids[2]);
    const handle = await page.evaluate(() => {
      const { scene, camera, renderer } = window.__p201.scene(); let gizmo;
      scene.traverse(value => { if (value.type === 'TransformControlsGizmo') gizmo = value; });
      if (!gizmo) throw new Error('Actual TransformControlsGizmo missing');
      const candidates = gizmo.gizmo.translate.children.filter(value => value.name === 'X' && value.isMesh && value.visible);
      const object = candidates.find(value => value.geometry.type === 'CylinderGeometry') ?? candidates[0];
      const center = object.geometry.boundingSphere?.center.clone(); if (!center) object.geometry.computeBoundingSphere();
      const point = (center ?? object.geometry.boundingSphere.center.clone()).applyMatrix4(object.matrixWorld);
      const origin = gizmo.worldPosition.clone(); point.project(camera); origin.project(camera);
      const rect = renderer.domElement.getBoundingClientRect(), x = rect.x + (point.x + 1) / 2 * rect.width, y = rect.y + (1 - point.y) / 2 * rect.height;
      const ox = rect.x + (origin.x + 1) / 2 * rect.width, oy = rect.y + (1 - origin.y) / 2 * rect.height, length = Math.hypot(x - ox, y - oy);
      return { x, y, dx: (x - ox) / length * 90, dy: (y - oy) / length * 90, candidates: candidates.map(value => ({ name: value.name, geometry: value.geometry.type })) };
    });
    await page.screenshot({ path: fileURLToPath(new URL('product-move-handle.png', out)), fullPage: true });
    await page.mouse.move(handle.x, handle.y); await page.mouse.down(); await page.mouse.move(handle.x + handle.dx, handle.y + handle.dy, { steps: 12 }); await page.mouse.up(); await idle();
    const after = (await snapshot()).project, moved = after.devices.find(value => value.id === ids[2]);
    assert.ok(after.revision > before.revision, JSON.stringify({ handle, before: device.pose, after: moved.pose })); assert.notDeepEqual(moved.pose.translation, device.pose.translation); assert.ok(Math.abs(moved.pose.translation.x / .5 - Math.round(moved.pose.translation.x / .5)) < 1e-8);
    assert.deepEqual(moved.pose.rotation, device.pose.rotation); assert.deepEqual(after.connections, before.connections); await menuEdit('撤销'); assert.deepEqual((await snapshot()).project.devices.find(value => value.id === ids[2]), device);
    await page.getByRole('button', { name: '↖ 选择', exact: true }).click(); return { handle, before: device.pose, moved: moved.pose, committed_revision: after.revision };
  });
  await check('screen drag of rotation handle preserves translation and commits snapped quaternion', async () => {
    await choose('P201 C'); await page.getByRole('button', { name: '⟳ 旋转手柄', exact: true }).click();
    const before = (await snapshot()).project, device = before.devices.find(value => value.id === ids[2]);
    const drag = await page.evaluate(() => {
      const { scene, camera, renderer } = window.__p201.scene(); let gizmo; scene.traverse(value => { if (value.type === 'TransformControlsGizmo') gizmo = value; });
      const ring = gizmo.gizmo.rotate.children.find(value => value.name === 'Z' && value.isMesh && value.visible);
      if (!ring) throw new Error('Visible actual Z rotation ring missing');
      const rect = renderer.domElement.getBoundingClientRect();
      const geometry = ring.geometry, position = geometry.attributes.position, tubular = geometry.parameters.tubularSegments, radial = geometry.parameters.radialSegments;
      const points = Array.from({ length: 13 }, (_, index) => {
        const at = Math.round(tubular * (.25 + index * .025)), point = ring.position.clone().set(0, 0, 0);
        for (let cross = 0; cross < radial; cross++) point.add(ring.position.clone().fromBufferAttribute(position, at + cross * (tubular + 1)));
        point.divideScalar(radial).applyMatrix4(ring.matrixWorld).project(camera);
        return { x: rect.x + (point.x + 1) / 2 * rect.width, y: rect.y + (1 - point.y) / 2 * rect.height };
      }); return { points, geometry: geometry.type, parameters: geometry.parameters };
    });
    await page.screenshot({ path: fileURLToPath(new URL('product-rotation-handle.png', out)), fullPage: true });
    await page.mouse.move(drag.points[0].x, drag.points[0].y); await page.mouse.down(); for (const point of drag.points.slice(1)) await page.mouse.move(point.x, point.y); await page.mouse.up(); await idle();
    const after = (await snapshot()).project, rotated = after.devices.find(value => value.id === ids[2]);
    assert.ok(after.revision > before.revision, JSON.stringify({ drag, before: device.pose, after: rotated.pose })); assert.deepEqual(rotated.pose.translation, device.pose.translation); assert.notDeepEqual(rotated.pose.rotation, device.pose.rotation);
    const angle = 2 * Math.atan2(rotated.pose.rotation.z, rotated.pose.rotation.w); assert.ok(Math.abs(angle / (Math.PI / 12) - Math.round(angle / (Math.PI / 12))) < 1e-6);
    await menuEdit('撤销'); await page.getByRole('button', { name: '↖ 选择', exact: true }).click(); return { drag, before: device.pose, after: rotated.pose, angle };
  });
  await check('numeric group root move and rotation keep member local poses, then explicit ungroup keeps physical positions', async () => {
    await page.getByLabel('工程对象树').getByRole('button', { name: '▤ P201 grouped pair', exact: true }).click();
    const before = (await snapshot()).project, line = before.lines.find(value => value.id === 'p201-group');
    await page.getByRole('button', { name: '打开详情编辑', exact: true }).click(); const form = page.locator('.object-editor');
    await form.getByRole('spinbutton', { name: 'X · 厂房坐标 m', exact: true }).fill(String(line.transform.translation.x + 1)); await form.getByRole('spinbutton', { name: '绕 Z 旋转增量 · °', exact: true }).fill('30'); await form.getByRole('button', { name: '应用到工程', exact: true }).click(); await form.waitFor({ state: 'hidden' });
    const moved = (await snapshot()).project, movedLine = moved.lines.find(value => value.id === line.id); assert.equal(movedLine.transform.translation.x, line.transform.translation.x + 1); assert.notDeepEqual(movedLine.transform.rotation, line.transform.rotation);
    assert.deepEqual(moved.devices, before.devices); await page.getByRole('button', { name: '解组', exact: true }).click(); await idle();
    const ungrouped = (await snapshot()).project; assert.ok(!ungrouped.lines.some(value => value.id === line.id));
    for (const id of line.members) { const local = moved.devices.find(value => value.id === id), actual = ungrouped.devices.find(value => value.id === id), angle = Math.PI / 6;
      assert.equal(actual.line_id, null); assert.ok(Math.abs(actual.pose.translation.x - (movedLine.transform.translation.x + Math.cos(angle) * local.pose.translation.x - Math.sin(angle) * local.pose.translation.y)) < 1e-8); assert.ok(Math.abs(actual.pose.translation.y - (movedLine.transform.translation.y + Math.sin(angle) * local.pose.translation.x + Math.cos(angle) * local.pose.translation.y)) < 1e-8);
    }
    await menuEdit('撤销'); await menuEdit('撤销'); assert.deepEqual((await snapshot()).project.devices, before.devices); return { moved_revision: moved.revision, ungrouped_revision: ungrouped.revision, member_local_poses_preserved: true };
  });
  await check('material connection UI uses actual exact ports and common Undo', async () => {
    const before = (await snapshot()).project, existing = before.connections.find(value => value.kind === 'material' && value.data.source.kind === 'port' && value.data.source.data.instance_id === ids[0]).data;
    await page.getByRole('button', { name: '连接物料', exact: true }).click(); const form = page.locator('.object-editor');
    await form.getByRole('combobox', { name: '物品类型', exact: true }).selectOption(existing.item_type);
    const portKey = endpoint => endpoint.kind === 'location' ? JSON.stringify(['location', endpoint.data]) : JSON.stringify(['port', endpoint.data.module_ref.id, endpoint.data.module_ref.version, endpoint.data.instance_id, endpoint.data.port_id]);
    await form.getByRole('combobox', { name: '源端点 · 输出', exact: true }).selectOption(portKey(existing.source)); await form.getByRole('combobox', { name: '目的端点 · 输入', exact: true }).selectOption(portKey(existing.destination)); await form.getByRole('combobox', { name: '接收库位', exact: true }).selectOption(existing.destination_store);
    await form.getByRole('button', { name: '应用连接', exact: true }).click(); await form.waitFor({ state: 'hidden' }); const after = (await snapshot()).project, created = after.connections.find(value => !before.connections.some(original => original.data.id === value.data.id));
    assert.equal(after.connections.length, before.connections.length + 1); assert.deepEqual({ ...created.data, id: existing.id }, existing); await menuEdit('撤销'); assert.deepEqual((await snapshot()).project.connections, before.connections);
    return { created_id: created.data.id, exact_source: created.data.source, exact_destination: created.data.destination, item_type: created.data.item_type, unit: created.data.unit };
  });
  await check('real new Run region summarizes explicit members and exact boundary port; zoom cannot change inventory or frozen result', async () => {
    const before = (await snapshot()).project, line = before.lines.find(value => value.id === 'p201-group');
    await apply([{ kind: 'put_object', data: { kind: 'line', data: { ...line, boundary_ports: [{ instance_id: ids[0], module_ref: before.devices.find(value => value.id === ids[0]).module_ref, port_id: 'output' }] } } }]);
    const current = await snapshot(), scenario = current.scenarios[0];
    const task = await request('run_scenario', { scenario_id: scenario.id, input_revision: current.project.revision, observation_window: scenario.window, intent: { kind: 'project_scenario' } });
    const reply = await core.rpc({ kind: 'await_task', scope: { kind: 'project' }, task_id: task.id }); assert.equal(reply.outcome.status, 'ok'); assert.equal(reply.outcome.data.task.state, 'completed', JSON.stringify(reply));
    const runId = reply.outcome.data.result.data.run_id, run = (await request('get_run', { run_id: runId, query: { kind: 'full_result' } })).data;
    await refresh(); await page.getByRole('combobox', { name: '历史结果', exact: true }).selectOption(runId); await page.waitForFunction(id => document.querySelector('.sf-timeline')?.getAttribute('data-run-id') === id, runId);
    await page.getByText('视图', { exact: true }).click(); await page.getByRole('button', { name: '区域', exact: true }).click();
    const panel = page.getByLabel('区域实际汇总'); await panel.waitFor(); await panel.locator('summary').click(); await panel.getByText(/已读作业/).first().waitFor();
    const region = panel.locator('section').filter({ has: page.getByRole('button', { name: 'P201 grouped pair · 2台设备', exact: true }) });
    assert.match(await region.innerText(), /边界口 .*output/); assert.match(await region.innerText(), /流入|流出/); assert.equal(await region.getByRole('button', { name: 'P201 A', exact: true }).count(), 1); assert.equal(await region.getByRole('button', { name: 'P201 B', exact: true }).count(), 1);
    const projectAtView = (await snapshot()).project, canvas = await page.locator('.factory-scene canvas').boundingBox(); await page.mouse.move(canvas.x + canvas.width * .7, canvas.y + canvas.height * .7); await page.mouse.wheel(0, -300);
    assert.deepEqual((await snapshot()).project, projectAtView); assert.deepEqual((await request('get_run', { run_id: runId, query: { kind: 'full_result' } })).data, run);
    await region.getByRole('button', { name: 'P201 A', exact: true }).click(); assert.match(await page.getByLabel('对象属性').innerText(), /P201 A/);
    await page.screenshot({ path: fileURLToPath(new URL('product-region-run.png', out)), fullPage: true }); return { run_id: runId, view: await region.innerText(), frozen_revision: run.project_revision, current_revision: projectAtView.revision };
  });
  await check('stale screen-handle submission retains full proposed pose and can explicitly reread before applying', async () => {
    await page.getByRole('button', { name: '输入草稿', exact: true }).click(); await choose('P201 C'); await page.getByRole('button', { name: '↔ 移动手柄', exact: true }).click();
    const before = (await snapshot()).project, original = before.devices.find(value => value.id === ids[2]);
    const point = await page.evaluate(() => {
      const { scene, camera, renderer } = window.__p201.scene(); let gizmo; scene.traverse(value => { if (value.type === 'TransformControlsGizmo') gizmo = value; });
      const mesh = gizmo.gizmo.translate.children.find(value => value.name === 'X' && value.isMesh && value.visible && value.geometry.type === 'CylinderGeometry'); mesh.geometry.computeBoundingSphere();
      const p = mesh.geometry.boundingSphere.center.clone().applyMatrix4(mesh.matrixWorld).project(camera), o = gizmo.worldPosition.clone().project(camera), rect = renderer.domElement.getBoundingClientRect();
      const x = rect.x + (p.x + 1) * rect.width / 2, y = rect.y + (1 - p.y) * rect.height / 2, dx = (p.x - o.x) * rect.width / 2, dy = (o.y - p.y) * rect.height / 2, length = Math.hypot(dx, dy); return { x, y, dx: dx / length * 70, dy: dy / length * 70 };
    });
    await apply([{ kind: 'put_object', data: { kind: 'hall', data: { ...before.halls[0], title: 'new actual concurrent revision for handle' } } }]);
    await page.mouse.move(point.x, point.y); await page.mouse.down(); await page.mouse.move(point.x + point.dx, point.y + point.dy, { steps: 10 }); await page.mouse.up();
    const form = page.getByLabel('保留的布局变换'); await form.waitFor(); const proposed = JSON.parse(await form.locator('pre').innerText()); assert.notDeepEqual(proposed.translation, original.pose.translation); assert.deepEqual((await snapshot()).project.devices.find(value => value.id === ids[2]), original); assert.match(await form.innerText(), /RevisionConflict|修订/);
    await form.getByRole('button', { name: '重读基础并保留变换', exact: true }).click(); await form.getByRole('button', { name: '应用保留的变换', exact: true }).click(); await form.waitFor({ state: 'hidden' }); assert.deepEqual((await snapshot()).project.devices.find(value => value.id === ids[2]).pose, proposed);
    await menuEdit('撤销'); assert.deepEqual((await snapshot()).project.devices.find(value => value.id === ids[2]), original); assert.equal((await snapshot()).project.halls[0].title, 'new actual concurrent revision for handle'); await page.getByRole('button', { name: '↖ 选择', exact: true }).click();
    return { base_revision: before.revision, proposed, external_change_retained: true };
  });
  await check('keyboard layout, copy/group, local input isolation, shared Undo/Redo and Home use the normal workbench', async () => {
    await page.getByRole('button', { name: '输入草稿', exact: true }).click(); await choose('P201 C'); await page.locator('.titlebar').click();
    await page.keyboard.press('m'); assert.match(await page.getByRole('button', { name: '↔ 移动手柄', exact: true }).getAttribute('class'), /active/);
    await page.keyboard.press('r'); assert.match(await page.getByRole('button', { name: '⟳ 旋转手柄', exact: true }).getAttribute('class'), /active/);
    await page.keyboard.press('v'); assert.match(await page.getByRole('button', { name: '↖ 选择', exact: true }).getAttribute('class'), /active/);
    await page.keyboard.press('g'); const group = page.getByRole('form', { name: '编组所选设备' }); await group.waitFor(); const beforeTyping = (await snapshot()).project;
    await group.getByRole('textbox', { name: '线组名称', exact: true }).fill('local keyboard input'); await page.keyboard.press('Control+z'); assert.deepEqual((await snapshot()).project, beforeTyping);
    await group.getByRole('textbox', { name: '线组名称', exact: true }).fill('explicit retained keyboard input'); await group.getByRole('button', { name: '返回画布', exact: true }).click(); await group.getByRole('button', { name: '明确放弃本地输入', exact: true }).click();
    await page.locator('.titlebar').click(); await page.keyboard.press('Control+d'); const copy = page.getByRole('form', { name: '复制布局为新ID' }); await copy.waitFor(); await copy.getByRole('button', { name: '返回画布', exact: true }).click();
    const before = (await snapshot()).project; await page.locator('.titlebar').click(); await page.keyboard.press('Control+z'); await idle(); const undone = (await snapshot()).project; assert.ok(undone.revision > before.revision);
    await page.keyboard.press('Control+y'); await idle(); const redone = (await snapshot()).project; assert.ok(redone.revision > undone.revision); assert.deepEqual(redone.lines, before.lines); assert.deepEqual(redone.devices, before.devices);
    await page.keyboard.press('Home'); assert.equal(await page.locator('.object-editor').count(), 0);
    return { move: 'M', rotate: 'R', select: 'V', group: 'G', copy: 'Ctrl+D', undo: 'Ctrl+Z', redo: 'Ctrl+Y', reset: 'Home', original_revision: before.revision, new_revision: redone.revision, text_edit_did_not_dispatch: true };
  });
  await check('property text 100/150/200 percent, focus and normal manual LOD remain available', async () => {
    await choose('P201 C'); const before = (await snapshot()).project, samples = [];
    for (const scale of [100, 150, 200]) { await page.getByText('视图', { exact: true }).click(); await page.getByRole('button', { name: new RegExp('属性文字 ' + scale + '%$') }).click(); samples.push(await page.locator('.inspector-scroll').evaluate(element => ({ zoom: getComputedStyle(element).zoom, width: element.clientWidth, scroll_width: element.scrollWidth }))); }
    assert.deepEqual(samples.map(value => Number(value.zoom)), [1, 1.5, 2]);
    for (const lod of ['详细', '简化', '区域']) { await page.getByText('视图', { exact: true }).click(); await page.getByRole('button', { name: new RegExp('^(✓ )?' + lod + '$') }).click(); assert.match(await page.locator('.scene-caption').innerText(), new RegExp(lod)); }
    const button = page.getByRole('button', { name: '打开详情编辑', exact: true }); await button.focus(); await page.keyboard.press('Enter'); await page.locator('.object-editor').waitFor(); await page.getByRole('button', { name: '返回画布', exact: true }).click(); assert.equal(await button.evaluate(element => document.activeElement === element), true);
    await page.screenshot({ path: fileURLToPath(new URL('product-text-200.png', out)), fullPage: true }); assert.deepEqual((await snapshot()).project, before);
    return { samples, keyboard_opened_details: true, manual_lods: ['detailed', 'simplified', 'region'] };
  });
  await check('keyboard Save writes the real own package and Undo cannot rewrite saved bytes', async () => {
    await page.locator('.titlebar').click(); await page.keyboard.press('Control+s'); await idle(); await page.getByText(/^已保存 r/).waitFor();
    const manifestUrl = new URL('saved-' + core.pid + '/project.json', out), manifestBytes = await readFile(manifestUrl, 'utf8'), manifest = JSON.parse(manifestBytes).data;
    const entry = manifest.documents.find(value => value.role === 'active_project'), projectUrl = new URL(entry.file_ref.path, manifestUrl), savedBytes = await readFile(projectUrl, 'utf8');
    const before = (await snapshot()).project; await page.keyboard.press('Control+z'); await idle(); assert.ok((await snapshot()).project.revision > before.revision); assert.equal(await readFile(manifestUrl, 'utf8'), manifestBytes); assert.equal(await readFile(projectUrl, 'utf8'), savedBytes); await page.keyboard.press('Control+y'); await idle();
    return { manifest: fileURLToPath(manifestUrl), active_document: entry.file_ref.path, saved_revision: manifest.project_revision, undo_changed_disk: false };
  });
  await check('saved Project title/footer track actual local field changes and clear when values are restored', async () => {
    await page.locator('.titlebar').click(); await page.keyboard.press('Control+s'); await idle(); await page.getByText(/^已保存 r/).waitFor(); assert.doesNotMatch(await page.locator('.titlebar').innerText(), /\*/); await choose('P201 C'); const actual = (await snapshot()).project.devices.find(value => value.id === ids[2]);
    await page.getByRole('button', { name: '打开详情编辑', exact: true }).click(); const form = page.locator('.object-editor'); assert.doesNotMatch(await page.locator('.titlebar').innerText(), /\*/);
    await form.getByRole('textbox', { name: '名称', exact: true }).fill(actual.title + ' local'); await page.getByText('本地输入未应用', { exact: true }).waitFor(); assert.match(await page.locator('.titlebar').innerText(), /\*/);
    await form.getByRole('textbox', { name: '名称', exact: true }).fill(actual.title); await page.waitForFunction(() => !document.querySelector('.app-status')?.textContent.includes('本地输入未应用')); assert.doesNotMatch(await page.locator('.titlebar').innerText(), /\*/); await form.getByRole('button', { name: '返回画布', exact: true }).click();
    return { clean_panel_open: true, actual_change_dirty: true, restored_value_clean: true };
  });
  await check('tenfold result playback follows one wall-clock origin across delayed real Host observations', async () => {
    const runId = core.ready.formal_run.id; await page.getByRole('combobox', { name: '历史结果', exact: true }).selectOption(runId); await idle();
    await page.getByRole('spinbutton', { name: '观察毫秒', exact: true }).fill('0'); await page.getByRole('button', { name: '读取时点', exact: true }).click(); await idle();
    const speeds = await page.getByRole('combobox', { name: '回看速度', exact: true }).locator('option').evaluateAll(options => options.map(value => value.value));
    assert.ok(speeds.includes('10'), 'The frozen playback measurement requires an explicit 10x result-view speed');
    await page.getByRole('combobox', { name: '回看速度', exact: true }).selectOption('10'); delayObjectReads = true;
    await page.getByRole('button', { name: '播放回看', exact: true }).click();
    await page.waitForFunction(() => Number(document.querySelector('.sf-timeline')?.getAttribute('data-view-time')) >= 22000, null, { timeout: 12000 });
    await page.getByRole('button', { name: '暂停回看', exact: true }).click(); delayObjectReads = false; await idle();
    const distinct = playbackQueries.filter((value, index, list) => index === 0 || list[index-1].at_ms !== value.at_ms), first = distinct[0], last = distinct.at(-1);
    assert.ok(distinct.length >= 3); const ratio = (last.at_ms - first.at_ms) / (last.wall_ms - first.wall_ms); assert.ok(ratio > 9 && ratio < 11, JSON.stringify({ ratio, distinct }));
    assert.deepEqual((await request('get_run', { run_id: runId, query: { kind: 'full_result' } })).data, core.ready.formal_run);
    await page.getByRole('button', { name: '输入草稿', exact: true }).click(); return { ratio, real_query_delays_ms: 250, observations: distinct, Native_app: false, performance_acceptance: false };
  });
  await check('switching actual Project clears selection and cannot undo the old project; three start entries use the new empty project', async () => {
    const prior = (await snapshot()).project; await core.rpc({ kind: 'new_project' }); await refresh(); const empty = (await snapshot()).project; assert.notEqual(empty.id, prior.id); assert.equal(empty.halls.length, 0); assert.equal(empty.devices.length, 0);
    const start = page.getByLabel('工程开始方式'); await start.waitFor(); for (const name of ['展示任务', '从模板开始', '空工程']) assert.equal(await start.getByRole('button', { name: new RegExp('^' + name) }).count(), 1);
    await start.getByRole('button', { name: /^从模板开始/ }).click(); const library = page.getByLabel('模块与模板库'); await library.waitFor(); assert.equal(await library.getByRole('combobox', { name: '类别', exact: true }).inputValue(), 'templates'); assert.equal((await snapshot()).project.id, empty.id); await library.getByRole('button', { name: '返回画布', exact: true }).click();
    await start.getByRole('button', { name: /^空工程/ }).click(); const form = page.locator('.object-editor'); await form.getByRole('textbox', { name: '名称', exact: true }).fill('new project hall only'); await form.getByRole('spinbutton', { name: '宽度 · m', exact: true }).fill('20'); await form.getByRole('spinbutton', { name: '进深 · m', exact: true }).fill('10'); await form.getByRole('spinbutton', { name: '厂房净高 · m', exact: true }).fill('8'); await form.getByRole('button', { name: '应用到工程', exact: true }).click(); await form.waitFor({ state: 'hidden' });
    const created = (await snapshot()).project; assert.equal(created.id, empty.id); assert.equal(created.halls.length, 1); await menuEdit('撤销'); const undone = (await snapshot()).project; assert.equal(undone.id, empty.id); assert.equal(undone.halls.length, 0); assert.equal(undone.devices.length, 0); assert.ok(!undone.lines.some(value => value.id === 'p201-group'));
    await page.screenshot({ path: fileURLToPath(new URL('product-start-entries.png', out)), fullPage: true }); return { prior_id: prior.id, current_id: empty.id, three_entries: true, template_category: 'templates', cross_project_history: false };
  });
  await check('actual template preparation publishes local dirty and preserves input until explicit discard', async () => {
    await core.rpc({ kind: 'register', directory: fileURLToPath(new URL('modules/templates/purchased-assembly/candidate/', root)) });
    await page.getByLabel('工程开始方式').getByRole('button', { name: /^从模板开始/ }).click(); const library = page.getByLabel('模块与模板库'); await library.getByRole('button', { name: /外购件齐套总装与包装/ }).click(); const input = library.getByRole('textbox', { name: '新建对象前缀', exact: true }); await input.waitFor();
    await input.fill('p201-explicit-prefix'); await page.getByText(/^本地输入未应用/).waitFor(); assert.equal(await library.getByRole('combobox', { name: '类别', exact: true }).isDisabled(), true);
    await input.fill(''); await page.waitForFunction(() => !document.querySelector('.app-status')?.textContent.includes('本地输入未应用')); assert.equal(await library.getByRole('combobox', { name: '类别', exact: true }).isDisabled(), false);
    await input.fill('p201-retain-template'); await library.getByRole('button', { name: '返回画布', exact: true }).click(); const guard = library.getByRole('dialog', { name: '放弃模板准备输入', exact: true }); await guard.getByRole('button', { name: '返回保留模板输入', exact: true }).click(); assert.equal(await input.inputValue(), 'p201-retain-template');
    await library.getByRole('button', { name: '返回画布', exact: true }).click(); await guard.getByRole('button', { name: '明确放弃模板输入', exact: true }).click(); await library.waitFor({ state: 'hidden' }); return { actual_template: 'community.template-purchased-assembly@1.0.0', restored_fields_clean: true, explicit_discard: true };
  });
  assert.equal(errors.length, 0, errors.join('\n'));
} catch (error) { console.error(error); process.exitCode = 1; }
finally {
  if (core) await writeFile(new URL('product-current-host-kinds-fixed.json', out), JSON.stringify({ checks, errors, warnings, native: false, transport: 'rustc-linked actual current Commands/Host from explicit P001 frontier', records: core.records, stderr: core.stderr() }, null, 2));
  await browser?.close(); await core?.close();
}
