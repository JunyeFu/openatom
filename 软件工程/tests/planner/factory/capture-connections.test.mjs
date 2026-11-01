import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { build } from 'vite';
import * as THREE from 'three';

const root = new URL('../../../', import.meta.url);
const out = new URL('.work/single-window-20261018/p201-public-native23/capture-connections/compiled/', root);
await build({ configFile: false, root: fileURLToPath(root), logLevel: 'warn', build: {
  ssr: true, outDir: fileURLToPath(out), emptyOutDir: false, minify: false,
  rolldownOptions: { input: {
    surface: fileURLToPath(new URL('apps/planner/src/features/render-surface/FactoryRenderSurface.tsx', root)),
    connections: fileURLToPath(new URL('apps/planner/src/scene/factoryConnections.ts', root)),
    visibility: fileURLToPath(new URL('apps/planner/src/features/render-surface/factoryCapture.ts', root)),
  }, output: { entryFileNames: '[name].mjs' } },
} });
const { projectFactoryCaptureSource } = await import(new URL('surface.mjs', out));
const { createFactoryConnectionLayer } = await import(new URL('connections.mjs', out));
const { visibleFactoryObjects } = await import(new URL('visibility.mjs', out));
const actual = JSON.parse(await readFile(new URL('tests/planner/fixtures/p201-branch-native.actual.json', root), 'utf8'));

function sourceWithOtherLine() {
  const project = structuredClone(actual.project), outside = structuredClone(project.devices[1]);
  outside.id = 'controlled-outside-device'; outside.line_id = 'controlled-outside-line';
  project.devices.push(outside);
  project.lines.push({ ...structuredClone(project.lines[0]), id: outside.line_id, members: [outside.id] });
  const connection = structuredClone(project.connections[1]);
  connection.data.id = 'controlled-outside-only-connection';
  connection.data.destination.data.instance_id = outside.id;
  project.connections.push(connection);
  return { key: 'frozen-source', project, run: null, observation: null, devices: [], modules: new Map([
    [JSON.stringify([actual.module.definition.id, actual.module.definition.version]), actual.module],
  ]), resources: [], items: [], raw: { job_pages: [], item_pages: [] } };
}

test('exact line capture excludes another line connection sharing its buffer without changing the frozen input', () => {
  const source = sourceWithOtherLine(), before = JSON.stringify(source.project);
  const selected = projectFactoryCaptureSource(source, { kind: 'line', id: actual.project.lines[0].id });
  assert.deepEqual(selected.project.devices, actual.project.devices);
  assert.deepEqual(selected.project.connections, actual.project.connections);
  assert.equal(selected.project.locations.length, 1);
  assert.equal(selected.modules, source.modules);
  assert.equal(JSON.stringify(source.project), before);
  assert.equal(projectFactoryCaptureSource(source, null), source);
});

test('a selected line boundary connection retains its real id and reports the missing outside endpoint without inventing a device', () => {
  const source = sourceWithOtherLine(), boundary = structuredClone(source.project.connections[1]);
  boundary.data.id = 'controlled-line-boundary-connection';
  boundary.data.source = structuredClone(actual.project.connections[0].data.source);
  boundary.data.destination.data.instance_id = 'controlled-outside-device';
  source.project.connections.push(boundary);
  const before = JSON.stringify(source.project);
  const selected = projectFactoryCaptureSource(source, { kind: 'line', id: actual.project.lines[0].id });
  const anchors = new Map([...selected.project.devices.map(device => ['device:' + device.id, new THREE.Group()]),
    ...selected.project.locations.map(location => ['location:' + location.id, new THREE.Group()])]);
  let x = 0; const parent = new THREE.Group();
  for (const anchor of anchors.values()) { anchor.position.x = ++x; parent.add(anchor); }
  const layer = createFactoryConnectionLayer(selected.project, selected.modules, parent, anchors, null);
  assert.equal(layer.drawn, 3);
  assert.equal(layer.limitations.length, 1);
  assert.deepEqual(layer.limitations[0].object_ref, { kind: 'connection', id: boundary.data.id });
  assert.match(layer.limitations[0].message, /未绘制/);
  assert.equal(selected.project.devices.some(device => device.id === 'controlled-outside-device'), false);
  assert.equal(JSON.stringify(source.project), before);
});

test('drawn logical arrows report their real connection identities; unavailable links remain separate limitations', () => {
  const scene = new THREE.Scene(), parent = new THREE.Group(), anchors = new Map();
  scene.add(parent);
  const selected = projectFactoryCaptureSource(sourceWithOtherLine(), { kind: 'line', id: actual.project.lines[0].id });
  for (const object of selected.project.devices) anchors.set('device:' + object.id, new THREE.Group());
  for (const object of selected.project.locations) anchors.set('location:' + object.id, new THREE.Group());
  let x = 0;
  for (const anchor of anchors.values()) { anchor.position.x = ++x; parent.add(anchor); }
  const value = createFactoryConnectionLayer(selected.project, selected.modules, parent, anchors, null);
  parent.add(value.root);
  const camera = new THREE.PerspectiveCamera(45, 1, .1, 100);
  camera.position.set(10, 10, 10); camera.lookAt(0, 0, 0); camera.updateProjectionMatrix();
  assert.equal(value.drawn, 3);
  assert.deepEqual(visibleFactoryObjects(scene, camera), actual.project.connections.map(connection => ({ kind: 'connection', id: connection.data.id })).sort((a, b) => a.id.localeCompare(b.id)));
  assert.deepEqual(value.limitations, []);
  const unavailable = createFactoryConnectionLayer(selected.project, new Map(), parent, anchors, null);
  assert.equal(unavailable.limitations.length, 3);
  assert.ok(unavailable.limitations.every(issue => issue.object_ref.kind === 'connection'));
});

test('connection visibility uses each arrow geometry, actual hierarchy, frustum, draw range and material visibility', () => {
  const source = projectFactoryCaptureSource(sourceWithOtherLine(), { kind: 'line', id: actual.project.lines[0].id });
  const scene = new THREE.Scene(), parent = new THREE.Group(), anchors = new Map(); scene.add(parent);
  const positions = [[-10, 0], [20, 0], [20, 20]];
  source.project.devices.forEach((device, index) => { const anchor = new THREE.Group(); anchor.position.set(...positions[index], 0); parent.add(anchor); anchors.set('device:' + device.id, anchor); });
  const buffer = new THREE.Group(); buffer.position.x = 10; parent.add(buffer); anchors.set('location:' + source.project.locations[0].id, buffer);
  const arrows = createFactoryConnectionLayer(source.project, source.modules, parent, anchors, null); parent.add(arrows.root);
  const camera = new THREE.OrthographicCamera(-2, 2, 2, -2, .1, 100); camera.position.z = 10; camera.lookAt(0, 0, 0); camera.updateProjectionMatrix();
  assert.deepEqual(visibleFactoryObjects(scene, camera), [{ kind: 'connection', id: actual.project.connections[0].data.id }]);
  parent.position.x = 100; assert.deepEqual(visibleFactoryObjects(scene, camera), []);
  parent.position.x = 0; arrows.root.geometry.setDrawRange(6, 12); assert.deepEqual(visibleFactoryObjects(scene, camera), []);
  arrows.root.geometry.setDrawRange(0, Infinity); parent.visible = false; assert.deepEqual(visibleFactoryObjects(scene, camera), []);
  parent.visible = true; arrows.root.material.opacity = 0; assert.deepEqual(visibleFactoryObjects(scene, camera), []);
});
