// Run only after the Native GUI lease and a real picker open of our L03 clone.
import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { Matrix4, Quaternion, Vector3 } from 'three';
import { attachNative } from './native-trace.mjs';
import { out } from './current-core.mjs';
const [endpoint, expectedHost, attempt] = process.argv.slice(2);
assert.ok(attempt);
const client = await attachNative(endpoint, expectedHost), { page } = client;
const result = { kind: 'actual_Native_P201_layout_history_result_invariants_consumer', expectedHost, attempt, actual_Native: true, groups: [], errors: [] };
page.on('pageerror', error => result.errors.push(String(error)));
page.setDefaultTimeout(20000);
async function api(operation, params, projectId = 'l03-producer:project:1') {
  return page.evaluate(async ({ operation, params, projectId }) => {
    const reply = await window.__TAURI_INTERNALS__.invoke('project_api', { request: { api_version: '0.1', request_id: 'P201-native-' + crypto.randomUUID(), project_id: projectId, scope: { kind: projectId ? 'project' : 'workspace' }, operation, params } });
    if (reply.outcome.status !== 'ok') throw new Error(JSON.stringify(reply)); return reply.outcome.data;
  }, { operation, params, projectId });
}
const project = async () => (await api('get_object', { object_id: { kind: 'project', id: 'l03-producer:project:1' }, source: { kind: 'draft' }, node_index: null })).data;
const run = async () => (await api('get_run', { run_id: 'l03-producer:run:1:1', query: { kind: 'full_result' } })).data;
const idle = () => page.waitForFunction(() => document.querySelector('.workbench')?.getAttribute('aria-busy') === 'false');
async function menu(menuName, label) { await page.getByText(menuName, { exact: true }).click(); await page.locator('nav details[open]').getByRole('button', { name: label, exact: true }).click(); await idle(); }
async function check(name, work) { try { const evidence = await work(); result.groups.push({ name, status: 'passed', evidence }); } catch (error) { result.groups.push({ name, status: 'failed', error: String(error) }); throw error; } }
function matrix(pose) { return new Matrix4().compose(new Vector3(pose.translation.x, pose.translation.y, pose.translation.z), new Quaternion(pose.rotation.x, pose.rotation.y, pose.rotation.z, pose.rotation.w), new Vector3(pose.scale.x, pose.scale.y, pose.scale.z)); }
function deviceWorld(project, id) { const device = project.devices.find(value => value.id === id), hall = project.halls.find(value => value.id === device.hall_id), line = project.lines.find(value => value.id === device.line_id); return matrix(hall.origin).multiply(line ? matrix(line.transform) : new Matrix4()).multiply(matrix(device.pose)).elements; }
function close(actual, expected) { assert.equal(actual.length, expected.length); actual.forEach((value, index) => assert.ok(Math.abs(value - expected[index]) < 1e-8)); }
try {
  const context = await api('get_context', { selection: [] }, null);
  assert.equal(context.project.id, 'l03-producer:project:1'); assert.equal(context.project.revision, 143);
  const baseline = await project(), frozen = await run(); result.version = client.version; result.identity = client.identity;
  assert.equal(baseline.devices.length, 8); assert.ok(baseline.resources.some(value => value.id === 'PAL01'));
  await check('actual Native two canvases three manual LODs and result-view camera preserve current draft and frozen Run', async () => {
    await page.getByRole('combobox', { name: '历史结果', exact: true }).selectOption(frozen.id); await idle();
    await page.waitForFunction(() => document.querySelector('.sf-timeline')?.getAttribute('data-view-time') === '360000');
    await page.getByRole('spinbutton', { name: '观察毫秒', exact: true }).fill('75000'); await page.getByRole('button', { name: '读取时点', exact: true }).click();
    await page.waitForFunction(() => document.querySelector('.sf-timeline')?.getAttribute('data-view-time') === '75000');
    for (const label of ['简化', '区域', '详细']) await menu('视图', label);
    await page.getByRole('button', { name: '多厂房画布', exact: true }).click(); await page.getByRole('button', { name: '流水线画布', exact: true }).click();
    const box = await page.locator('.factory-scene canvas').boundingBox(), x = box.x + box.width * .65, y = box.y + box.height * .55;
    await page.mouse.move(x,y); await page.mouse.down({button:'right'}); await page.mouse.move(x+60,y+25,{steps:8}); await page.mouse.up({button:'right'});
    await page.mouse.down({button:'middle'}); await page.mouse.move(x+85,y+35,{steps:6}); await page.mouse.up({button:'middle'}); await page.mouse.wheel(0,-200);
    assert.equal(await page.locator('.object-editor').count(), 0); await page.getByRole('button',{name:'⌂ 复位视图',exact:true}).click();
    assert.deepEqual(await project(), baseline); assert.deepEqual(await run(), frozen);
    return { revision: baseline.revision, run_id: frozen.id, at_ms:75000, current_draft_and_complete_Run_equal:true };
  });
  await page.getByRole('button', { name: '输入草稿', exact: true }).click(); await idle();
  await check('actual Native Shift selection group undo redo copy and branch invalidation use new common revisions and preserve all business identities', async () => {
    const tree = page.getByLabel('工程对象树');
    await tree.getByRole('button', { name: '▣ ASM01', exact: true }).click(); await tree.getByRole('button', { name: '▣ R01', exact: true }).click({ modifiers: ['Shift'] });
    await page.getByRole('button', { name: '编组所选', exact: true }).click(); const form = page.getByRole('form', { name: '编组所选设备' });
    await form.getByRole('textbox', { name: '新线组ID', exact: true }).fill('P201-native-group'); await form.getByRole('textbox', { name: '线组名称', exact: true }).fill('P201 Native pair'); await form.getByRole('button', { name: '应用编组', exact: true }).click(); await form.waitFor({ state: 'hidden' });
    const grouped = await project(); assert.equal(grouped.revision, baseline.revision + 1); assert.deepEqual(new Set(grouped.lines.find(line=>line.id==='P201-native-group').members), new Set(['ASM01','R01']));
    for (const id of ['ASM01','R01']) close(deviceWorld(grouped,id),deviceWorld(baseline,id));
    await menu('编辑','撤销'); const undone = await project(); assert.deepEqual(undone.devices,baseline.devices); assert.deepEqual(undone.lines,baseline.lines);
    await menu('编辑','重做'); const redone = await project(); assert.deepEqual(redone.devices,grouped.devices); assert.deepEqual(redone.lines,grouped.lines); assert.ok(redone.revision>undone.revision);
    await tree.getByRole('button',{name:'▤ P201 Native pair',exact:true}).click(); await page.getByRole('button',{name:'复制为新ID',exact:true}).click(); const copyForm=page.getByRole('form',{name:'复制布局为新ID'}); await copyForm.getByRole('button',{name:'复制为新ID',exact:true}).click(); await copyForm.waitFor({state:'hidden'});
    const copied=await project(); assert.deepEqual(new Set(copied.lines.find(line=>line.id==='P201-native-group-copy').members),new Set(['ASM01-copy','R01-copy']));
    for(const id of ['ASM01-copy','R01-copy']) assert.deepEqual(copied.devices.find(value=>value.id===id).role_bindings,{item_types:{},resources:{},targets:{},ports:{}});
    for(const field of ['connections','locations','acknowledgments'])assert.deepEqual(copied[field],baseline[field]); assert.deepEqual(await run(),frozen);
    await menu('编辑','撤销'); await tree.getByRole('button',{name:'▣ V01',exact:true}).click(); await page.getByRole('button',{name:'复制为新ID',exact:true}).click(); const branch=page.getByRole('form',{name:'复制布局为新ID'}); await branch.getByRole('button',{name:'复制为新ID',exact:true}).click(); await branch.waitFor({state:'hidden'});
    await page.getByText('编辑',{exact:true}).click(); assert.equal(await page.getByRole('button',{name:'重做',exact:true}).isDisabled(),true); await page.getByText('编辑',{exact:true}).click(); assert.deepEqual(await run(),frozen);
    return { baseline_revision:baseline.revision,group_revision:grouped.revision,undo_revision:undone.revision,redo_revision:redone.revision,copy_revision:copied.revision,redo_after_new_edit:false,old_Run_equal:true,acknowledgments_equal:true };
  });
  await check('actual Native numeric group transform and ungroup preserve full physical member poses', async () => {
    await page.getByLabel('工程对象树').getByRole('button',{name:'▤ P201 Native pair',exact:true}).click(); const before=await project(), line=before.lines.find(value=>value.id==='P201-native-group');
    await page.getByRole('button',{name:'打开详情编辑',exact:true}).click(); const form=page.locator('.object-editor');
    await form.getByRole('spinbutton',{name:'X · 厂房坐标 m',exact:true}).fill(String(line.transform.translation.x+1)); await form.getByRole('spinbutton',{name:'绕 Z 旋转增量 · °',exact:true}).fill('30'); await form.getByRole('button',{name:'应用到工程',exact:true}).click(); await form.waitFor({state:'hidden'});
    const moved=await project(); assert.deepEqual(moved.devices,before.devices); assert.equal(moved.lines.find(value=>value.id===line.id).transform.translation.x,line.transform.translation.x+1);
    await page.getByRole('button',{name:'解组',exact:true}).click(); await idle(); const ungrouped=await project(); assert.ok(!ungrouped.lines.some(value=>value.id===line.id));
    for(const id of line.members) {assert.equal(ungrouped.devices.find(value=>value.id===id).line_id,null);close(deviceWorld(ungrouped,id),deviceWorld(moved,id));}
    await menu('编辑','撤销'); await menu('编辑','撤销'); const restored=await project();assert.deepEqual(restored.devices,before.devices);assert.deepEqual(restored.lines,before.lines);assert.deepEqual(await run(),frozen);
    return {moved_revision:moved.revision,ungrouped_revision:ungrouped.revision,restored_revision:restored.revision,full_member_TRS_preserved:true};
  });
  await check('actual Native material connection form uses exact real endpoints unit and receiving store with common Undo', async () => {
    const before=await project(), existing=before.connections.find(value=>value.kind==='material'&&value.data.source.kind==='port'&&value.data.source.data.instance_id==='ASM01').data;
    await page.getByRole('button',{name:'连接物料',exact:true}).click(); const form=page.locator('.object-editor'); await form.getByRole('combobox',{name:'物品类型',exact:true}).selectOption(existing.item_type);
    const portKey=endpoint=>endpoint.kind==='location'?JSON.stringify(['location',endpoint.data]):JSON.stringify(['port',endpoint.data.module_ref.id,endpoint.data.module_ref.version,endpoint.data.instance_id,endpoint.data.port_id]);
    await form.getByRole('combobox',{name:'源端点 · 输出',exact:true}).selectOption(portKey(existing.source));await form.getByRole('combobox',{name:'目的端点 · 输入',exact:true}).selectOption(portKey(existing.destination));await form.getByRole('combobox',{name:'接收库位',exact:true}).selectOption(existing.destination_store);
    await form.getByRole('button',{name:'应用连接',exact:true}).click();await form.waitFor({state:'hidden'});const after=await project(),created=after.connections.find(value=>!before.connections.some(old=>old.data.id===value.data.id));assert.equal(after.connections.length,before.connections.length+1);assert.deepEqual({...created.data,id:existing.id},existing);
    await menu('编辑','撤销');assert.deepEqual((await project()).connections,before.connections);assert.deepEqual(await run(),frozen);return{revision:after.revision,connection:created.data};
  });
  await check('actual Native stale group input stays visible and only applies after explicit revision reread', async () => {
    await page.getByLabel('工程对象树').getByRole('button',{name:'▣ CV01',exact:true}).click();await page.getByRole('button',{name:'编组所选',exact:true}).click();const form=page.getByRole('form',{name:'编组所选设备'});
    await form.getByRole('textbox',{name:'新线组ID',exact:true}).fill('P201-native-conflict');await form.getByRole('textbox',{name:'线组名称',exact:true}).fill('P201 retained native input');const before=await project();
    await api('apply_commands',{base_revision:before.revision,commands:[{kind:'set_project_settings',data:{title:'P201 Native concurrent shared revision',time_reference:before.time_reference,generation_context:before.generation_context,sources:before.sources}}]});
    await form.getByRole('button',{name:'应用编组',exact:true}).click();await form.getByRole('alert').waitFor();assert.equal(await form.getByRole('textbox',{name:'线组名称',exact:true}).inputValue(),'P201 retained native input');const conflicted=await project();assert.ok(!conflicted.lines.some(value=>value.id==='P201-native-conflict'));
    await form.getByRole('button',{name:'重读基础并保留输入',exact:true}).click();await form.getByRole('button',{name:'应用编组',exact:true}).click();await form.waitFor({state:'hidden'});const applied=await project();assert.ok(applied.lines.some(value=>value.id==='P201-native-conflict'));assert.equal(applied.title,conflicted.title);assert.deepEqual(applied.acknowledgments,baseline.acknowledgments);assert.deepEqual(await run(),frozen);
    return{base_revision:before.revision,conflict_revision:conflicted.revision,applied_revision:applied.revision,retained_input:true,old_Run_equal:true,acknowledgments_equal:true};
  });
  result.status='passed'; assert.equal(result.errors.length,0,result.errors.join('\n'));
  await page.screenshot({path:fileURLToPath(new URL(`native-functional-${attempt}.png`,out))});
} catch(error) { result.status='failed';result.error=String(error);process.exitCode=1; }
finally { await writeFile(new URL(`native-functional-${attempt}.json`,out),JSON.stringify(result,null,2)); await client.browser.close(); }
console.log(JSON.stringify(result));
