import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import vm from 'node:vm';
import * as THREE from 'three';
import { root, out, json, bytes } from '../presentation/cu-consumer.mjs';
import '../presentation/pixel-decoder.mjs';

const media = await import(new URL('factoryMedia.mjs', out));
const { createFactoryItemLayer } = await import(new URL('factoryItems.mjs', out));
const factory = await import(new URL('factoryPresentation.mjs', out));
await writeFile(new URL('retained-layout.mjs', out), stripTypeScriptTypes(await readFile(new URL('apps/planner/src/scene/layout.ts', root), 'utf8')));
const { poseMatrix } = await import(new URL('retained-layout.mjs', out));
let resourceCode = stripTypeScriptTypes(await readFile(new URL('apps/planner/src/scene/factoryResources.ts', root), 'utf8'), { mode: 'transform' });
resourceCode = resourceCode.replace("from './factoryMedia'", "from './factoryMedia.mjs'");
await writeFile(new URL('factoryResources.mjs', out), resourceCode);
const { createFactoryResourceLayer } = await import(new URL('factoryResources.mjs', out));
const base = (await json('modules/media/p302-controlled/model.module.json')).definition.payload.data;
const pose = (x = 0, y = 0, z = 0) => ({ unit: 'meter', coordinate_frame: 'right_handed_y_up', translation: { x, y, z }, rotation: { x: 0, y: 0, z: 0, w: 1 }, scale: { x: 1, y: 1, z: 1 } });
const native = { kind: 'native_scene', data: 'retained-test' };
const presentation = { ...base, source: native, scale: { x: 1, y: 1, z: 1 },
  nodes: [{ ...base.nodes[0], name: 'Retained native box', transform: pose(), geometry: { kind: 'box', data: { x: 1, y: 1, z: 1 } }, material: { ...base.nodes[0].material, texture: null } }],
  poses: ['open', 'closed'].map((id, i) => ({ id, node_transforms: [{ node_index: 0, pose: pose(i * 2) }] })),
  timeline: ['open', 'closed'].map(id => ({ id, phase_ref: id, start_progress: 0, end_progress: 1, source: { kind: 'native_pose', data: id }, transition: 'hold' })),
  actions: [], tags: [], anchor_roles: { carry: { kind: 'scene_node', data: 0 } }, port_anchors: {}, lod: { detailed: native, simplified: native, region: null } };
const moduleRef = { id: 'retained.native', version: '1.0.0' };
function bound(at, options = {}) {
  return { presentation, rights: [], binding: { session_id: 'S', request: { project_id: 'P', scope: { kind: 'project' }, scope_revision: 1,
    source: { kind: 'run_module_version', data: { module_ref: moduleRef, run_id: 'R', at_ms: at } } } },
  async readAsset() { throw new Error('A native scene must not request asset bytes'); }, ...options };
}
function item(id, at, owner = { kind: 'location', id: 'A' }, closure = 'open') {
  return { item: { id, item_type_ref: { id: 'part', revision: 1 }, holder: owner.kind === 'item' ? { kind: 'container', id: owner.id } : owner,
      lifecycle: 'available', quantity: { value: '1', unit: 'piece' }, quality: 'qualified', attributes: { closure: { kind: 'enum', value: closure } }, contents: [] },
    media: bound(at), unavailable: null, anchor_ref: owner,
    attachment: { owner_ref: owner, slot: { item_type_ref: { id: 'part', revision: 1 }, anchor_role: owner.kind === 'location' ? null : 'carry', offset: pose(.25) } } };
}
const settled = async layer => {
  for (let i = 0; i < 1000; i++) { if (!layer.renderState(true).pending) return; await new Promise(r => setTimeout(r, 1)); }
  assert.fail('Owned layer did not settle');
};
const matrixValues = root => { root.updateMatrixWorld(true); const result = []; root.traverse(o => { if (o.isMesh) result.push(o.matrixWorld.toArray()); }); return result; };
const closeMatrices = (a, b) => { assert.equal(a.length, b.length); a.forEach((matrix, index) => matrix.forEach((value, element) => assert.ok(Math.abs(value - b[index][element]) < 1e-8))); };
const capturedSource = (data, point, packages, onRead = () => {}) => factory.projectFactoryPresentation(
  { session_id: 'captured-retained-test', project_id: data.project_id, scope: { kind: 'project' }, scope_revision: data.scope_revision, project: data.project, view: { run_id: data.run.id, at_ms: point.at_ms } },
  { project: data.project, run: data.run, observation: point, jobs: point.objects.flatMap(o => o.kind === 'job' ? [o.data] : []), items: point.objects.flatMap(o => o.kind === 'item' ? [o.data] : []), raw: { job_pages: [], item_pages: [] } },
  { module: async ref => packages.get(ref.id + '@' + ref.version), readAsset: request => { onRead(); const ref = request.source.data.module_ref; return bytes(`.work/cu-delivery/host-media/${ref.id}/${ref.version}/${request.asset.path}`); } }, () => {});

test('one owned native model updates actual finite item state without replacing nodes or reading bytes', async () => {
  const before = item('part', 0), after = item('part', 5000, undefined, 'closed');
  const instance = await media.createFactoryItemMedia(before, 'detailed'), root = instance.root, node = instance.nodes.get(0), geometry = node.geometry;
  assert.equal(await instance.update(after, 'detailed'), true);
  assert.equal(instance.root, root); assert.equal(instance.nodes.get(0), node); assert.equal(node.geometry, geometry);
  assert.equal(node.position.x, 2); assert.equal(instance.source.data.at_ms, 5000);
  const fresh = await media.createFactoryItemMedia(after, 'detailed'); closeMatrices(matrixValues(root), matrixValues(fresh.root));
  assert.equal(await instance.update(before, 'detailed'), true); assert.equal(node.position.x, 0);
  fresh.dispose(); instance.dispose();
});

for (const changed of ['session', 'project', 'scope', 'revision', 'run', 'module', 'presentation']) test(`media reuse rejects a different exact ${changed} owner without mutating the retained rig`, async () => {
  const instance = await media.createFactoryItemMedia(item('part', 0), 'detailed'), next = item('part', 5000, undefined, 'closed');
  if (changed === 'session') next.media.binding.session_id = 'OTHER';
  if (changed === 'project') next.media.binding.request.project_id = 'OTHER';
  if (changed === 'scope') next.media.binding.request.scope = { kind: 'definition_draft', data: 'OTHER' };
  if (changed === 'revision') next.media.binding.request.scope_revision++;
  if (changed === 'run') next.media.binding.request.source.data.run_id = 'OTHER';
  if (changed === 'module') next.media.binding.request.source.data.module_ref = { ...moduleRef, version: '2.0.0' };
  if (changed === 'presentation') next.media.presentation = structuredClone(presentation);
  assert.equal(await instance.update(next, 'detailed'), false); assert.equal(instance.nodes.get(0).position.x, 0); instance.dispose();
});

test('retained holders move across declared locations, nested containers and reverse seek without duplicate geometry', async () => {
  const locations = new Map(['A', 'B'].map((id, i) => { const group = new THREE.Group(); group.position.x = i * 10; return [id, group]; }));
  const parent = item('box', 0), child = item('child', 0, { kind: 'item', id: 'box' }); parent.item.contents = ['child'];
  const layer = createFactoryItemLayer([child, parent], 'detailed', () => null, id => locations.get(id), () => {});
  layer.reconcile(); await settled(layer);
  const first = new Map(layer.renderState(true).objects.map(o => [o.ref.id, o.root])); assert.equal(first.size, 2);
  const moved = item('box', 5000, { kind: 'location', id: 'B' }, 'closed'); moved.item.contents = ['child'];
  await layer.update([item('child', 5000, { kind: 'item', id: 'box' }), moved]); await settled(layer);
  assert.equal(layer.renderState(true).objects.length, 2);
  for (const value of layer.renderState(true).objects) assert.equal(value.root, first.get(value.ref.id));
  assert.ok(first.get('child').getWorldPosition(new THREE.Vector3()).x > 10);
  const missing = { ...moved, unavailable: 'unavailable · no declared parent slot', attachment: null };
  await layer.update([item('child', 6000, { kind: 'item', id: 'box' }), missing]);
  assert.equal(layer.renderState(true).objects.length, 0, 'a cached but unavailable parent cannot expose loose child stock');
  await layer.update([child, parent]); await settled(layer);
  assert.equal(layer.renderState(true).objects.length, 2); assert.equal(first.get('box'), layer.renderState(true).objects.find(o => o.ref.id === 'box').root);
  await layer.update([]); assert.equal(layer.renderState(true).objects.length, 0);
  assert.equal(locations.get('A').children.length, 0); assert.equal(locations.get('B').children.length, 0); layer.dispose();
});

test('late exact-media updates cannot overwrite a newer frame or mutate a disposed rig', async () => {
  const original = item('part', 0), instance = await media.createFactoryItemMedia(original, 'detailed');
  let release;
  const slow = item('part', 1000, undefined, 'open');
  const proxy = { ...presentation, lod: { ...presentation.lod, detailed: { kind: 'media_ref', data: moduleRef } } };
  slow.media = bound(1000, { presentation: proxy, resolveModule: () => new Promise(resolve => { release = resolve; }) });
  const pending = instance.update(slow, 'detailed');
  assert.equal(await instance.update(item('part', 5000, undefined, 'closed'), 'detailed'), true);
  release({ definition: { ...moduleRef, payload: { category: 'media', data: presentation } }, rights: [] });
  assert.equal(await pending, false); assert.equal(instance.nodes.get(0).position.x, 2);
  const last = instance.update(slow, 'detailed'); instance.dispose();
  release({ definition: { ...moduleRef, payload: { category: 'media', data: presentation } }, rights: [] });
  assert.equal(await last, false); assert.equal(instance.nodes.get(0).position.x, 2);
});

test('personnel markers retain their sprite and texture while actual occupation text changes', async () => {
  const priorDocument = globalThis.document, text = [];
  const ctx = { clearRect() {}, fillRect() {}, beginPath() {}, arc() {}, ellipse() {}, fill() {}, fillText(value) { text.push(value); } };
  globalThis.document = { createElement() { return { getContext: () => ctx }; } };
  const location = new THREE.Group(), input = { resource: { id: 'W', kind: 'worker', role: 'operator', home_location: 'A' }, worker_status: { kind: 'unobserved', job_ids: [] }, media: null, unavailable: null };
  const layer = createFactoryResourceLayer([input], 'detailed', id => id === 'A' ? location : null, () => {});
  try {
    layer.reconcile(); const sprite = layer.renderState().objects[0].root, texture = sprite.material.map;
    await layer.update([{ ...input, worker_status: { kind: 'claimed', job_ids: ['J1', 'J2'] } }]);
    assert.equal(layer.renderState().objects[0].root, sprite); assert.equal(sprite.material.map, texture); assert.equal(text.at(-1), '占用 · 2作业');
    await layer.update([{ ...input, worker_status: { kind: 'available', job_ids: [] } }]);
    assert.equal(text.at(-1), '本时点未占用'); assert.equal(layer.renderState().objects[0].root, sprite);
    await layer.update([{ ...input, resource: { ...input.resource, home_location: 'UNKNOWN' } }]);
    assert.equal(layer.renderState().objects.length, 0); assert.match(layer.renderState().unavailable[0].message, /home_location/);
  } finally { layer.dispose(); layer.dispose(); globalThis.document = priorDocument; }
  assert.equal(location.children.length, 0);
});

test('owned resource models reuse their geometry and replace a changed exact source rather than retarget old bytes', async () => {
  const location = new THREE.Group(), input = { resource: { id: 'C', kind: 'carrier', home_location: 'A' }, media: bound(0), unavailable: null };
  const layer = createFactoryResourceLayer([input], 'detailed', () => location, () => {});
  layer.reconcile(); await settled(layer); const first = layer.renderState().objects[0].root;
  await layer.update([{ ...input, media: bound(5000) }]); await settled(layer); assert.equal(layer.renderState().objects[0].root, first);
  const different = bound(6000); different.binding.request.source.data.module_ref = { ...moduleRef, version: '2.0.0' };
  await layer.update([{ ...input, media: different }]); await settled(layer); assert.notEqual(layer.renderState().objects[0].root, first);
  await layer.update([]); assert.equal(location.children.length, 0); layer.dispose();
});

test('captured real Host GLB frames retain exact nodes and match freshly created poses through forward and reverse seeks', async () => {
  const data = await json('.work/cu-delivery/host/l03-complete.json'), original = JSON.stringify(data), packages = new Map(data.modules.map(pkg => [pkg.definition.id + '@' + pkg.definition.version, pkg]));
  const points = [43000, 49000, 83000, 88000].map(at => data.points.find(point => point.at_ms === at)); assert.ok(points.every(Boolean));
  let instance, deviceId, kept = 0, reads = 0;
  for (const point of [...points, ...points.toReversed()]) {
    const source = await capturedSource(data, point, packages, () => { reads++; });
    deviceId ??= source.devices.find(device => device.frame && device.media).device_id;
    const input = source.devices.find(device => device.device_id === deviceId), device = source.project.devices.find(device => device.id === deviceId);
    if (!instance) instance = await media.createFactoryMedia(device, input, 'detailed');
    else {
      const root = instance.root, node = instance.nodes.get(0), priorReads = reads;
      assert.equal(await instance.update(input, 'detailed'), true); assert.equal(instance.root, root); assert.equal(instance.nodes.get(0), node); assert.equal(reads, priorReads); kept++;
    }
    const fresh = await media.createFactoryMedia(device, input, 'detailed'); closeMatrices(matrixValues(instance.root), matrixValues(fresh.root)); fresh.dispose();
    assert.equal(instance.source.data.at_ms, point.at_ms);
  }
  assert.equal(kept, 7); instance.dispose(); assert.equal(JSON.stringify(data), original, 'view updates never mutate recorded business facts');
});

test('captured real holder/contents transitions retain the same world geometry as fresh scene construction', async () => {
  const data = await json('.work/cu-delivery/host/l03-complete.json'), original = JSON.stringify(data), packages = new Map(data.modules.map(pkg => [pkg.definition.id + '@' + pkg.definition.version, pkg]));
  const points = [43000, 49000, 83000, 88000, 120000].map(at => data.points.find(point => point.at_ms === at)); assert.ok(points.every(Boolean));
  const place = (group, p) => { group.matrix.copy(poseMatrix(p)); group.matrixAutoUpdate = false; };
  async function world(source) {
    const root = new THREE.Group(), halls = new Map(), lines = new Map(), owners = new Map(), groups = new Map(), locations = new Map(); root.rotation.x = -Math.PI / 2;
    for (const hall of source.project.halls) { const group = new THREE.Group(); place(group, hall.origin); root.add(group); halls.set(hall.id, group); }
    for (const line of source.project.lines) { const group = new THREE.Group(); place(group, line.transform); halls.get(line.hall_id).add(group); lines.set(line.id, group); }
    for (const location of source.project.locations) { const group = new THREE.Group(); place(group, location.position); halls.get(location.hall_id).add(group); locations.set(location.id, group); }
    for (const device of source.project.devices) { const group = new THREE.Group(); place(group, device.pose); (device.line_id ? lines.get(device.line_id) : halls.get(device.hall_id)).add(group); groups.set(device.id, group); }
    async function devices(source) {
      for (const input of source.devices) {
        let instance = owners.get(input.device_id);
        if (instance && await instance.update(input, 'detailed')) continue;
        if (instance) { instance.root.removeFromParent(); instance.dispose(); owners.delete(input.device_id); }
        if (!input.media) continue;
        instance = await media.createFactoryMedia(source.project.devices.find(d => d.id === input.device_id), input, 'detailed');
        groups.get(input.device_id).add(instance.root); owners.set(input.device_id, instance);
      }
    }
    await devices(source);
    const layer = createFactoryItemLayer(source.items, 'detailed', id => owners.get(id), id => locations.get(id), () => {}); layer.reconcile(); await settled(layer);
    return { root, layer, async update(source) { layer.detach(); await devices(source); await layer.update(source.items); await settled(layer); }, dispose() { layer.dispose(); for (const owner of owners.values()) owner.dispose(); } };
  }
  let retained, unchanged = 0, nested = 0;
  try {
    for (const point of [...points, ...points.toReversed()]) {
      const source = await capturedSource(data, point, packages), prior = new Map(retained?.layer.renderState(true).objects.map(o => [o.ref.id, o.root]) ?? []);
      if (!retained) retained = await world(source); else await retained.update(source);
      const fresh = await world(source);
      try {
        retained.root.updateMatrixWorld(true); fresh.root.updateMatrixWorld(true);
        const actual = retained.layer.renderState(true), expected = fresh.layer.renderState(true), baseline = new Map(expected.objects.map(o => [o.ref.id, o.root]));
        assert.deepEqual(actual.failures, []); assert.deepEqual(actual.objects.map(o => o.ref.id).sort(), [...baseline.keys()].sort());
        for (const object of actual.objects) {
          const own = root => { const values = []; root.traverse(node => { if (node.isMesh && node.userData.ref?.id === object.ref.id) values.push(node.matrixWorld.toArray()); }); return values; };
          closeMatrices(own(object.root), own(baseline.get(object.ref.id)));
          if (prior.get(object.ref.id) === object.root) unchanged++;
          if (source.items.find(item => item.item.id === object.ref.id).attachment.owner_ref.kind === 'item') nested++;
        }
      } finally { fresh.dispose(); }
    }
  } finally { retained?.dispose(); }
  assert.ok(unchanged > 0); assert.ok(nested > 0); assert.equal(JSON.stringify(data), original);
});

async function componentHarness() {
  const source = await readFile(process.env.SF_FACTORY_SCENE_SOURCE ?? new URL('apps/planner/src/scene/FactoryScene.tsx', root), 'utf8');
  const prefix = source.slice(0, source.indexOf('  return <div className="factory-scene"')).replace(/^import .*;\r?\n/gm, '').replace('export function FactoryScene', 'function FactoryScene');
  const camera = stripTypeScriptTypes(await readFile(new URL('apps/planner/src/scene/defaultCamera.ts', root), 'utf8')).replace(/^import .*;\r?\n/m, '').replace('export function', 'function');
  const cells = [], renderers = [], pendingEffects = []; let cursor = 0;
  const node = { clientWidth: 1920, clientHeight: 1080, children: [], getBoundingClientRect: () => ({ width: 1920, height: 1080 }), appendChild(canvas) { this.children.push(canvas); } };
  class Renderer {
    constructor() { this.info = { render: { frame: 0 }, memory: {} }; this.domElement = { addEventListener() {}, removeEventListener() {}, remove: () => node.children.splice(node.children.indexOf(this.domElement), 1) }; this.frames = []; renderers.push(this); }
    setPixelRatio(v) { this.ratio = v; } getPixelRatio() { return this.ratio; } setClearColor() {} setSize(w, h) { this.width = w; this.height = h; }
    getContext() { return {}; } getDrawingBufferSize(vector) { return vector.set(this.width, this.height); }
    render(scene, camera) { scene.updateMatrixWorld(true); camera.updateMatrixWorld(true); this.scene = scene; this.info.render.frame++; this.frames.push(this.domElement.readFactoryDiagnostics().observation_ms); }
    dispose() { this.disposed = true; } forceContextLoss() { this.contextLost = true; }
  }
  class Controls { constructor(camera) { this.camera = camera; this.target = new THREE.Vector3(); } update() { this.camera.lookAt(this.target); this.camera.updateMatrixWorld(true); } addEventListener() {} dispose() {} }
  const useRef = value => { const index = cursor++; cells[index] ??= { current: value }; return cells[index]; };
  const useState = value => { const index = cursor++; cells[index] ??= { value: typeof value === 'function' ? value() : value }; return [cells[index].value, next => { cells[index].value = typeof next === 'function' ? next(cells[index].value) : next; }]; };
  const useEffect = (callback, deps) => { const index = cursor++, previous = cells[index]; if (!previous || deps.some((value, i) => !Object.is(value, previous.deps[i]))) pendingEffects.push(() => { previous?.cleanup?.(); cells[index] = { deps, cleanup: callback() }; }); };
  const env = { ...THREE, THREE: { ...THREE, WebGLRenderer: Renderer }, useRef, useState, useEffect, useLayoutEffect() {},
    window: { devicePixelRatio: 1 }, WebGL2RenderingContext: class {}, ResizeObserver: class { constructor(callback) { this.callback = callback; } observe() { this.callback(); } disconnect() {} },
    requestAnimationFrame: callback => setTimeout(callback, 0), cancelAnimationFrame: clearTimeout,
    OrbitControls: Controls, poseMatrix: p => new THREE.Matrix4().compose(new THREE.Vector3(p.translation.x, p.translation.y, p.translation.z), new THREE.Quaternion(p.rotation.x, p.rotation.y, p.rotation.z, p.rotation.w), new THREE.Vector3(p.scale.x, p.scale.y, p.scale.z)),
    outsideHallFloor: () => false, deviceActivity: () => null, decimalText: value => String(value),
    regionSummary: () => ({ members: [], state: 'fixture', resources: [], ports: [], boundary: [] }),
    attachLayoutControls: () => { throw new Error('Readonly Run cannot create editing handles'); },
    createFactoryConnectionLayer: () => null, factoryConnectionAt: () => null, captureCamera: () => { throw new Error('This program check is not capture acceptance'); }, visibleFactoryObjects: () => [],
    ...factory, ...media, createFactoryItemLayer, createFactoryResourceLayer,
  };
  vm.runInNewContext(camera + '\n' + stripTypeScriptTypes(prefix + '\n  return {host};\n}') + '\nglobalThis.component=FactoryScene;', env);
  return { renderers, render(props) { cursor = 0; const result = env.component(props); result.host.current = node; while (pendingEffects.length) pendingEffects.shift()(); }, dispose() { for (const cell of cells) cell?.cleanup?.(); } };
}

test('actual FactoryScene keeps one renderer and exact device/item instances across paired frozen-Run observations', async () => {
  const h = await componentHarness(), run = { id: 'R', window: { start_ms: 0, end_ms: 10000 }, computed_until_ms: 10000 }, modules = new Map();
  const p = { id: 'P', revision: 1, halls: [{ id: 'H', width_m: 20, depth_m: 20, origin: pose() }], lines: [], devices: [{ id: 'D', title: 'D', hall_id: 'H', line_id: null, pose: pose(3, 3), module_ref: moduleRef, footprint_m: [{ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 1, y: 1 }], height_m: 1 }], resources: [], locations: ['A', 'B'].map((id, i) => ({ id, title: id, hall_id: 'H', position: pose(5 + i * 5, 5) })) };
  const frame = at => { const value = item('part', at, { kind: 'location', id: at ? 'B' : 'A' }, at ? 'closed' : 'open'); return { key: 'frame-' + at, project: p, run, observation: { run_id: 'R', at_ms: at, next_index: null, objects: [{ kind: 'item', data: value.item }] }, modules, devices: [{ device_id: 'D', media: bound(at), frame: null, unavailable: null, projection: 'static' }], items: [value], resources: [], raw: { job_pages: [], item_pages: [] } }; };
  const props = source => ({ project: p, canvas: 'line_canvas', hallId: 'H', selection: null, onSelect() {}, lod: 'detailed', autoLod: false, resetKey: 0, run, observation: source.observation, mediaSource: source, modules });
  const wait = async (at, minimum = 1) => { for (let i = 0; i < 1000; i++) { const renderer = h.renderers.at(-1); if (renderer?.frames.filter(value => value === at).length >= minimum) { const items = []; renderer.scene.traverse(o => { if (o.isMesh && o.userData.ref?.kind === 'item') items.push(o); }); if (items.length === 1) return items[0]; } await new Promise(resolve => setTimeout(resolve, 1)); } assert.fail('Scene frame did not settle'); };
  try {
    h.render(props(frame(0))); const initial = await wait(0), renderer = h.renderers[0];
    h.render(props(frame(5000))); const moved = await wait(5000);
    assert.equal(h.renderers.length, 1, 'seeking must not destroy and recreate WebGLRenderer'); assert.equal(moved, initial);
    assert.equal(renderer.disposed, undefined); assert.equal(moved.position.x, 2);
    const before = renderer.frames.filter(value => value === 0).length;
    h.render(props(frame(0))); const reversed = await wait(0, before + 1);
    assert.equal(reversed, initial); assert.equal(reversed.position.x, 0); assert.equal(h.renderers.length, 1);
  } finally { h.dispose(); }
  assert.equal(h.renderers[0].disposed, true); assert.equal(h.renderers[0].contextLost, true);
});
