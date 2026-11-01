import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import { Box3, PerspectiveCamera, Vector3 } from 'three';

const root = new URL('../../../', import.meta.url);
const out = new URL('.work/p201-whole-20261017/model-tests/', root);
await mkdir(out, { recursive: true });
for (const name of ['layout', 'layoutEditing', 'runtime', 'regionSummary', 'defaultCamera']) {
  const source = await readFile(new URL('apps/planner/src/scene/' + name + '.ts', root), 'utf8');
  await writeFile(new URL(name + '.mjs', out), stripTypeScriptTypes(source).replace("from './layout'", "from './layout.mjs'").replace("from './runtime'", "from './runtime.mjs'"));
}
const { identityPose, rotatePoseZ, deviceHallPosition } = await import(new URL('layout.mjs', out));
const { groupLayoutSelection, ungroupLayoutSelection, copyLayoutSelection, outsideHallFloor } = await import(new URL('layoutEditing.mjs', out));
const { regionSummary } = await import(new URL('regionSummary.mjs', out));
const pose = (x, y, angle = 0) => rotatePoseZ({ ...identityPose(), translation: { x, y, z: 0 } }, angle);
function fixture() {
  const device = (id, x, y) => ({ id, title: id, hall_id: 'H', line_id: 'L', module_ref: { id: 'machine', version: '1.0.0' },
    pose: pose(x, y), footprint_m: [{ x: -1, y: -1 }, { x: 1, y: -1 }, { x: 1, y: 1 }, { x: -1, y: 1 }], height_m: 2,
    configuration: { duration: { kind: 'number', data: 5 } }, role_bindings: { item_types: { product: 'part' }, resources: { worker: 'W' }, targets: {}, ports: {} } });
  return { id: 'P', revision: 7, halls: [{ id: 'H', width_m: 50, depth_m: 50 }], resources: [], locations: [],
    lines: [{ id: 'L', title: '原线', hall_id: 'H', transform: pose(10, 20, 90), members: ['A', 'B', 'C'], boundary_ports: [{ instance_id: 'A' }], region_polygon_m: [] }],
    devices: [device('A', 2, 0), device('B', 4, 0), device('C', 6, 0)], connections: [{ id: 'M', source: 'A', target: 'B' }],
    acknowledgments: [{ id: 'ACK' }] };
}
const refs = ids => ids.map(id => ({ kind: 'device', id }));
function apply(project, commands) {
  const next = structuredClone(project);
  for (const command of commands) {
    const key = (command.kind === 'put_object' ? command.data.kind : command.data.kind) === 'line' ? 'lines' : 'devices';
    if (command.kind === 'remove_object') next[key] = next[key].filter(item => item.id !== command.data.id);
    else { const value = command.data.data, index = next[key].findIndex(item => item.id === value.id); if (index < 0) next[key].push(value); else next[key][index] = value; }
  }
  return next;
}
function close(actual, expected) { actual.forEach((value, i) => assert.ok(Math.abs(value - expected[i]) < 1e-9, String(actual) + ' != ' + String(expected))); }

test('group two members of a rotated line atomically preserves both world poses and the unselected member', () => {
  const project = fixture(), before = structuredClone(project);
  const commands = groupLayoutSelection(project, refs(['A', 'B']), 'G', '新线');
  assert.equal(commands.filter(command => command.kind === 'put_object' && command.data.kind === 'line' && command.data.data.id === 'L').length, 1);
  const next = apply(project, commands);
  assert.deepEqual(next.lines.find(line => line.id === 'L').members, ['C']);
  assert.deepEqual(next.lines.find(line => line.id === 'G').members, ['A', 'B']);
  for (const id of ['A', 'B', 'C']) close(deviceHallPosition(next, next.devices.find(device => device.id === id)).toArray(), deviceHallPosition(project, project.devices.find(device => device.id === id)).toArray());
  close(Object.values(next.devices[0].pose.rotation), [0, 0, Math.SQRT1_2, Math.SQRT1_2]);
  assert.equal(next.lines.find(line => line.id === 'G').region_polygon_m.length, 4);
  assert.deepEqual(project, before);
});

test('ungroup the actual line removes its root and keeps all declared hall poses, connections and acceptance records', () => {
  const project = fixture(), next = apply(project, ungroupLayoutSelection(project, [{ kind: 'line', id: 'L' }]));
  assert.equal(next.lines.length, 0);
  for (const device of next.devices) {
    assert.equal(device.line_id, null);
    close(deviceHallPosition(next, device).toArray(), deviceHallPosition(project, project.devices.find(value => value.id === device.id)).toArray());
  }
  assert.deepEqual(next.connections, project.connections); assert.deepEqual(next.acknowledgments, project.acknowledgments);
});

test('line copy gets explicit new identities, moves only the new root and requires new role/port bindings without stock or connection copies', () => {
  const project = fixture(), before = structuredClone(project);
  const commands = copyLayoutSelection(project, [{ kind: 'line', id: 'L' }], { L: 'L2', A: 'A2', B: 'B2', C: 'C2' }, { x: 5, y: 2, z: 0 });
  assert.equal(commands.length, 4);
  const next = apply(project, commands), line = next.lines.find(line => line.id === 'L2');
  assert.deepEqual(line.members, ['A2', 'B2', 'C2']); assert.deepEqual(line.boundary_ports, []);
  for (const id of ['A', 'B', 'C']) {
    const original = project.devices.find(device => device.id === id), copy = next.devices.find(device => device.id === id + '2');
    assert.deepEqual(copy.pose, original.pose); assert.deepEqual(copy.configuration, original.configuration);
    assert.deepEqual(copy.role_bindings, { item_types: {}, resources: {}, targets: {}, ports: {} });
    close(deviceHallPosition(next, copy).toArray(), deviceHallPosition(project, original).add(new Vector3(5, 2, 0)).toArray());
  }
  assert.deepEqual(next.connections, project.connections); assert.deepEqual(project, before);
});

test('a device copied out of a rotated group keeps its full hall orientation and uses an independent ID', () => {
  const project = fixture(), commands = copyLayoutSelection(project, refs(['A']), { A: 'A2' }, { x: 3, y: -2, z: 1 });
  const copy = commands[0].data.data;
  assert.equal(copy.line_id, null); close(Object.values(copy.pose.translation), [13, 20, 1]);
  close(Object.values(copy.pose.rotation), [0, 0, Math.SQRT1_2, Math.SQRT1_2]);
  assert.throws(() => copyLayoutSelection(project, refs(['A']), { A: 'A' }, { x: 0, y: 0, z: 0 }), /ID/);
});

test('known floor breach uses the actual group transform; unknown footprint is not a pass', () => {
  const project = fixture(); assert.equal(outsideHallFloor(project, project.devices[0]), false);
  project.lines[0].transform.translation.y = 49;
  assert.equal(outsideHallFloor(project, project.devices[0]), true);
  assert.equal(outsideHallFloor(project, { ...project.devices[0], footprint_m: [] }), null);
});

test('cross-hall grouping refuses instead of relocating another hall through a guessed parent', () => {
  const project = fixture(); project.devices[1].hall_id = 'H2';
  assert.throws(() => groupLayoutSelection(project, refs(['A', 'B']), 'G', '新线'), /跨厂房/);
});

test('region state uses real JobsAt and claims at the requested boundary, never final Jobs or a running clock', () => {
  const project = fixture(); project.resources = [{ id: 'W', title: '装配工' }];
  const job = { id: 'J', state: 'waiting_resource', execution_ref: { kind: 'project_device', data: { device_id: 'A' } } };
  const source = { run: { jobs: [{ ...job, state: 'completed' }], resource_claims: [
    { job_id: 'J', resource_id: 'W', intervals: [{ start_ms: 70, end_ms: 75 }] },
    { job_id: 'OTHER', resource_id: 'W', intervals: [{ start_ms: 0, end_ms: 100 }] }] },
    raw: { job_pages: [{ objects: [{ kind: 'job', data: job }] }] }, observation: { at_ms: 75 }, devices: [] };
  const before = structuredClone(source), value = regionSummary(project, project.lines[0], source);
  assert.match(value.state, /等资源1/); assert.doesNotMatch(value.state, /完成/); assert.deepEqual(value.resources, []);
  source.observation.at_ms = 74; assert.deepEqual(regionSummary(project, project.lines[0], source).resources, [{ id: 'W', title: '装配工' }]);
  source.observation.at_ms = 75; assert.deepEqual(source, before);
});

test('regional output shows only explicit boundary-port quantities; it does not add intermediate device output', () => {
  const project = fixture(), port = { instance_id: 'A', module_ref: { id: 'machine', version: '1.0.0' }, port_id: 'out' };
  project.lines[0].boundary_ports = [port];
  const quantity = value => ({ value: { coefficient: value, decimal_places: 0 }, unit: 'piece' });
  const source = { run: { resource_claims: [] }, observation: { at_ms: 75, objects: [] }, raw: { job_pages: [] },
    devices: ['A', 'B'].map((id, i) => ({ device_id: id, port_outputs: { ports: [{ status: 'available', port: { ...port, instance_id: id }, completed_output: quantity(i ? 20 : 1), accepted_output: quantity(0) }] } })) };
  const value = regionSummary(project, project.lines[0], source);
  assert.equal(value.ports.length, 1); assert.equal(value.ports[0].label, '做完 1 piece · 移交 0 piece');
  assert.equal(regionSummary(project, project.lines[0], null).state, '声明区域 · 无实际运行数据');
});

test('regional loading distinguishes a selected actual observation from a draft without inventing Job states', () => {
  const project = fixture(), before = structuredClone(project);
  const loading = regionSummary(project, project.lines[0], null, true);
  assert.equal(loading.actual, false);
  assert.equal(loading.state, '正在读取当前时点实际作业 · 暂不汇总');
  assert.deepEqual(loading.resources, []);
  assert.doesNotMatch(loading.state, /完成|执行\d|已读作业/);
  assert.equal(regionSummary(project, project.lines[0], null, false).state, '声明区域 · 无实际运行数据');
  assert.deepEqual(project, before);
});

test('default oblique camera fits complete bounds with five percent on each edge for wide and narrow canvases', async () => {
  const { defaultObliqueCamera } = await import(new URL('defaultCamera.mjs', out));
  const bounds = new Box3(new Vector3(-12, 0, -80), new Vector3(60, 8, 0));
  for (const aspect of [.5, 1, 2.5]) {
    const camera = new PerspectiveCamera(42, aspect, .01, 100000), fit = defaultObliqueCamera(bounds, 42, aspect);
    camera.position.copy(fit.eye); camera.lookAt(fit.target); camera.updateMatrixWorld(true); camera.updateProjectionMatrix();
    let edge = 0;
    for (const x of [bounds.min.x, bounds.max.x]) for (const y of [bounds.min.y, bounds.max.y]) for (const z of [bounds.min.z, bounds.max.z]) {
      const corner = new Vector3(x, y, z).project(camera); assert.ok(Math.abs(corner.x) <= .90000001 && Math.abs(corner.y) <= .90000001); assert.ok(corner.z > -1 && corner.z < 1); edge = Math.max(edge, Math.abs(corner.x), Math.abs(corner.y));
    }
    assert.ok(Math.abs(edge - .9) < 1e-8); assert.deepEqual(fit.target.toArray(), bounds.getCenter(new Vector3()).toArray());
  }
});
