import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import * as THREE from 'three';

const root = new URL('../../../', import.meta.url);
const output = new URL('.work/single-window-20261018/p201-public-native23/connection-drawing-repair/compiled/', root);
await mkdir(output, { recursive: true });
await writeFile(new URL('connections.mjs', output), stripTypeScriptTypes(await readFile(new URL('apps/planner/src/scene/factoryConnections.ts', root), 'utf8')));
const { createFactoryConnectionLayer, factoryConnectionAt } = await import(new URL('connections.mjs', output));
const actual = JSON.parse(await readFile(new URL('tests/planner/fixtures/p201-branch-native.actual.json', root), 'utf8'));
const moduleKey = JSON.stringify([actual.module.definition.id, actual.module.definition.version]);
const modules = new Map([[moduleKey, actual.module]]);

function anchorScene(project, multi = false) {
  const physical = new THREE.Group(), anchors = new Map();
  physical.rotation.x = -Math.PI / 2;
  const place = (node, pose) => {
    node.position.set(pose.translation.x, pose.translation.y, pose.translation.z);
    node.quaternion.set(pose.rotation.x, pose.rotation.y, pose.rotation.z, pose.rotation.w);
    node.scale.set(pose.scale.x, pose.scale.y, pose.scale.z);
  };
  for (const hall of project.halls) {
    const hallRoot = new THREE.Group(), lines = new Map();
    if (multi) place(hallRoot, hall.origin);
    physical.add(hallRoot);
    for (const line of project.lines.filter(line => line.hall_id === hall.id)) {
      const node = new THREE.Group(); place(node, line.transform); hallRoot.add(node); lines.set(line.id, node);
    }
    for (const device of project.devices.filter(device => device.hall_id === hall.id)) {
      const node = new THREE.Group(); place(node, device.pose); (device.line_id ? lines.get(device.line_id) : hallRoot).add(node);
      anchors.set('device:' + device.id, node);
    }
    for (const location of project.locations.filter(location => location.hall_id === hall.id)) {
      const node = new THREE.Group(); place(node, location.position); hallRoot.add(node); anchors.set('location:' + location.id, node);
    }
  }
  return { physical, anchors };
}
function layer(project = structuredClone(actual.project), definitions = modules, multi = false, selection = null) {
  const scene = anchorScene(project, multi);
  const result = createFactoryConnectionLayer(project, definitions, scene.physical, scene.anchors, selection);
  scene.physical.add(result.root); scene.physical.updateMatrixWorld(true);
  return { ...result, ...scene };
}
const position = (value, index) => new THREE.Vector3().fromBufferAttribute(value.root.geometry.getAttribute('position'), index).toArray();
const close = (actual, expected) => actual.forEach((number, index) => assert.ok(Math.abs(number - expected[index]) < 1e-5, `${actual} != ${expected}`));

test('actual Native three-device branch draws three directed links from retained local and hall positions without mutating input', () => {
  const before = JSON.stringify(actual), value = layer(actual.project);
  assert.equal(value.drawn, 3);
  assert.equal(value.root.geometry.getAttribute('position').count, 18);
  assert.ok(value.rows.every(row => row.problem === null));
  assert.deepEqual(value.limitations, []);
  close(position(value, 0), [6, 4, .08]); close(position(value, 1), [10, 5, .08]);
  close(position(value, 6), [10, 5, .08]); close(position(value, 7), [12, 6, .08]);
  close(position(value, 12), [10, 5, .08]); close(position(value, 13), [12, 2, .08]);
  assert.match(value.rows[1].title, /共享交接缓存 → CNC-B \/ input.*piece/);
  assert.equal(JSON.stringify(actual), before);
  assert.equal(actual.project.locations[0].capacity, null);
});

test('root movement and multi-hall rotation are taken from the physical hierarchy, not the camera', () => {
  const project = structuredClone(actual.project); project.lines[0].transform.translation.x = 7;
  const moved = layer(project); close(position(moved, 0), [10, 4, .08]); close(position(moved, 7), [16, 6, .08]);
  project.lines[0].transform.translation.x = 3;
  project.halls[0].origin.translation = { x: 100, y: 200, z: 0 };
  project.halls[0].origin.rotation = { x: 0, y: 0, z: Math.SQRT1_2, w: Math.SQRT1_2 };
  const multi = layer(project, modules, true);
  close(position(multi, 0), [96, 206, .08]); close(position(multi, 1), [95, 210, .08]);
  close(position(multi, 7), [94, 212, .08]); close(position(multi, 13), [98, 212, .08]);
});

test('wrong exact version, absent port, incompatible direction, role and unit remain explicit unresolved links', () => {
  const changes = [
    p => { p.connections[1].data.destination.data.module_ref.version = '9.9.9'; },
    p => { p.connections[1].data.destination.data.port_id = 'absent'; },
    p => { p.connections[1].data.destination.data.port_id = 'output'; },
    p => { p.devices[1].role_bindings.item_types.input = 'other-item'; },
    p => { p.connections[1].data.unit = 'kilogram'; },
    p => { p.connections[1].data.destination_store = 'absent'; },
  ];
  for (const change of changes) {
    const project = structuredClone(actual.project); change(project);
    const value = layer(project); assert.equal(value.drawn, 2); assert.ok(value.rows[1].problem);
    assert.equal(value.limitations.length, 1);
    assert.deepEqual(value.limitations[0].object_ref, { kind: 'connection', id: project.connections[1].data.id });
    assert.match(value.limitations[0].message, /未绘制/);
  }
});

test('unread or mismatched frozen definitions do not borrow a current definition or draw apparently valid ports', () => {
  const missing = layer(structuredClone(actual.project), new Map());
  assert.equal(missing.drawn, 0); assert.ok(missing.rows.every(row => row.problem.includes('尚未读取')));
  assert.deepEqual(missing.limitations.map(value => value.object_ref.id), actual.project.connections.map(connection => connection.data.id));
  const counterfeit = structuredClone(actual.module); counterfeit.definition.version = '9.9.9';
  const changed = layer(structuredClone(actual.project), new Map([[moduleKey, counterfeit]]));
  assert.equal(changed.drawn, 0);
});

test('actual Three ray intersections map batched shafts and arrowheads to the correct connection identity', () => {
  const value = layer(actual.project, modules, false, { kind: 'connection', id: actual.project.connections[0].data.id });
  const ray = new THREE.Raycaster(new THREE.Vector3(8, 5, -4.5), new THREE.Vector3(0, -1, 0));
  ray.params.Line.threshold = .02;
  const hit = ray.intersectObject(value.root)[0]; assert.ok(hit);
  assert.deepEqual(factoryConnectionAt(hit.object, hit.index), { kind: 'connection', id: actual.project.connections[0].data.id });
  for (let row = 0; row < 3; row++) for (const offset of [0, 2, 4]) {
    assert.equal(factoryConnectionAt(value.root, row * 6 + offset).id, actual.project.connections[row].data.id);
  }
  assert.equal(factoryConnectionAt(new THREE.Group(), 0), null);
  assert.notEqual(value.root.geometry.getAttribute('color').getX(0), value.root.geometry.getAttribute('color').getX(6));
});

test('changing selection updates only the existing arrow colors and preserves geometry and ray identities', () => {
  const before = JSON.stringify(actual), value = layer(actual.project);
  const geometry = value.root.geometry, positions = [...geometry.getAttribute('position').array];
  const colors = geometry.getAttribute('color');
  const ids = actual.project.connections.map(connection => connection.data.id);
  for (const selected of [ids[0], ids[2], null]) {
    value.setSelection(selected ? { kind: 'connection', id: selected } : null);
    assert.equal(value.root.geometry, geometry);
    assert.equal(geometry.getAttribute('color'), colors);
    assert.deepEqual([...geometry.getAttribute('position').array], positions);
    for (let row = 0; row < ids.length; row++) {
      const expected = new THREE.Color(ids[row] === selected ? 0x176ca3 : 0x53788c);
      for (let vertex = row * 6; vertex < (row + 1) * 6; vertex++) {
        close([colors.getX(vertex), colors.getY(vertex), colors.getZ(vertex)], [expected.r, expected.g, expected.b]);
        assert.equal(factoryConnectionAt(value.root, vertex - vertex % 2).id, ids[row]);
      }
    }
  }
  assert.equal(JSON.stringify(actual), before);
});

test('coincident and unavailable anchors stay unresolved instead of drawing at a guessed origin', () => {
  const project = structuredClone(actual.project); project.locations[0].position.translation = { x: 6, y: 4, z: 0 };
  const coincident = layer(project); assert.equal(coincident.drawn, 2); assert.match(coincident.rows[0].problem, /重合/);
  const scene = anchorScene(actual.project); scene.anchors.delete('location:' + actual.project.locations[0].id);
  const missing = createFactoryConnectionLayer(actual.project, modules, scene.physical, scene.anchors, null);
  assert.equal(missing.drawn, 0); assert.ok(missing.rows.every(row => row.problem.includes('不在当前画布')));
  assert.equal(missing.limitations.length, 3);
  assert.equal(coincident.limitations.length, 1);
  assert.deepEqual(coincident.limitations[0].object_ref, { kind: 'connection', id: project.connections[0].data.id });
});
