import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import * as THREE from 'three';
import { rootUrl } from './load-source.mjs';
const output = new URL('scenarios/自定义模块/runtime-authoring/scene-compiled/', rootUrl); await mkdir(output, { recursive: true });
for (const name of ['layout', 'modulePresentation']) {
  const source = await readFile(new URL(`apps/planner/src/scene/${name}.ts`, rootUrl), 'utf8');
  await writeFile(new URL(`${name}.mjs`, output), stripTypeScriptTypes(source).replace("from './layout'", "from './layout.mjs'"));
}
const { buildModulePresentation, presentationPoseMatrix } = await import(new URL('modulePresentation.mjs', output));
const identity = (unit = 'millimeter', frame = 'right_handed_z_up') => ({ translation: { x: 0, y: 0, z: 0 }, rotation: { x: 0, y: 0, z: 0, w: 1 }, scale: { x: 1, y: 1, z: 1 }, unit, coordinate_frame: frame });
const material = () => ({ color_rgba: [.2, .4, .6, 1], metallic: .3, roughness: .7 });
const node = (id = 1) => ({ id, name: `部件${id}`, transform: identity(), geometry: { kind: 'box', data: { x: 1000, y: 2000, z: 3000 } }, material: material() });
const presentation = (...nodes) => ({ kind: 'native_scene', source: { kind: 'native_scene', data: 'explicit.native' }, unit: 'millimeter', coordinate_frame: 'right_handed_z_up', scale: { x: 1, y: 1, z: 1 }, nodes, poses: [], actions: [], timeline: [], tags: [], anchor_roles: {}, port_anchors: {}, lod: { simplified: null, detailed: null }, production_effect: 'none' });
function near(actual, expected) { assert.ok(Math.abs(actual - expected) < 1e-6, `${actual} != ${expected}`); }
function vector(actual, expected) { ['x', 'y', 'z'].forEach((key, i) => near(actual[key], expected[i])); }
function bounds(object) { object.updateMatrixWorld(true); return new THREE.Box3().setFromObject(object); }

test('millimeter geometry and pose translation convert separately; declared root scale applies once', () => {
  const part = node(); part.transform.translation.x = 1000;
  const value = presentation(part); value.scale = { x: 2, y: 3, z: 4 }; const original = structuredClone(value);
  const scene = buildModulePresentation(value), box = bounds(scene.root);
  vector(box.min, [1, -3, -6]); vector(box.max, [3, 3, 6]); assert.equal(scene.nativeMeshCount, 1);
  assert.deepEqual(value, original); scene.dispose();
});
test('actual parent hierarchy combines mixed units, rotation and scale without flattening children', () => {
  const parent = node(1), child = node(2); parent.transform = identity('meter'); parent.transform.translation.x = 1;
  const q = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 0, 1), Math.PI / 2);
  parent.transform.rotation = { x: q.x, y: q.y, z: q.z, w: q.w }; parent.transform.scale.x = 2;
  child.parent_index = 0; child.transform.translation.x = 500;
  const scene = buildModulePresentation(presentation(parent, child)), object = scene.nodes.get(2);
  assert.equal(object.parent, scene.nodes.get(1)); vector(new THREE.Vector3().setFromMatrixPosition(object.matrixWorld), [1, 1, 0]);
  vector(new THREE.Vector3(1, 0, 0).applyMatrix4(object.matrixWorld), [1, 3, 0]); scene.dispose();
});
test('Y-up cylinder height and horizontal plane dimensions become physical Z-up with no double conversion', () => {
  const cylinder = node(); cylinder.transform = identity('meter', 'right_handed_y_up'); cylinder.geometry = { kind: 'cylinder', data: { radius: 500, height: 2000 } };
  const value = presentation(cylinder); value.coordinate_frame = 'right_handed_y_up';
  const scene = buildModulePresentation(value); vector(bounds(scene.root).getSize(new THREE.Vector3()), [1, 1, 2]); scene.dispose();
  cylinder.geometry = { kind: 'plane', data: { x: 4000, y: 2000 } };
  const plane = buildModulePresentation(value); vector(bounds(plane.root).getSize(new THREE.Vector3()), [4, 2, 0]); plane.dispose();
});
test('a differently declared Pose basis converts translation and rotation together', () => {
  const pose = identity('meter', 'right_handed_y_up'); pose.translation.y = 1;
  const q = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), Math.PI / 2); pose.rotation = { x: q.x, y: q.y, z: q.z, w: q.w };
  const matrix = presentationPoseMatrix(pose, 'right_handed_z_up'); vector(new THREE.Vector3().applyMatrix4(matrix), [0, 0, 1]); vector(new THREE.Vector3(1, 0, 0).applyMatrix4(matrix), [0, 1, 1]);
});
test('material opacity, metallic and roughness use the actual declaration; missing texture is explicit', () => {
  const part = node(); part.material.color_rgba[3] = .35; part.material.texture = { id: 'real-texture', path: 'textures/body.png', kind: 'png' };
  const scene = buildModulePresentation(presentation(part)), mesh = scene.nodes.get(1).children[0];
  near(mesh.material.opacity, .35); near(mesh.material.metalness, .3); near(mesh.material.roughness, .7); assert.equal(mesh.material.transparent, true); assert.equal(mesh.material.map, null);
  assert.ok(scene.diagnostics.some(issue => issue.asset?.id === 'real-texture')); scene.dispose();
});
test('unavailable GLB/image bytes never become invented replacement meshes or arbitrary path loads', () => {
  const part = node(); const asset = { id: 'legal-declared-model', path: 'media/source.glb', kind: 'glb' };
  part.geometry = { kind: 'imported_node', data: { asset_ref: asset, node_index: 4, unsupported_extensions: ['KHR_draco_mesh_compression'] } };
  const value = presentation(part); value.kind = 'model'; value.source = { kind: 'asset', data: asset };
  const scene = buildModulePresentation(value); assert.equal(scene.nativeMeshCount, 0); assert.equal(scene.nodes.get(1).children.length, 0); assert.equal(scene.diagnostics.filter(issue => issue.code === 'missing_asset').length, 2); scene.dispose();
  value.kind = 'image'; value.nodes = []; value.source = { kind: 'asset', data: { id: 'real-image', path: 'media/front.png', kind: 'png' } };
  const image = buildModulePresentation(value); assert.equal(image.nativeMeshCount, 0); assert.equal(image.diagnostics[0].asset.path, 'media/front.png'); image.dispose();
});
test('bad parents/cycles/nonfinite geometry stay visibly invalid without relocating them to root', () => {
  const a = node(1), b = node(2), valid = node(3); a.parent_index = 1; b.parent_index = 0;
  const scene = buildModulePresentation(presentation(a, b, valid)); assert.equal(scene.nativeMeshCount, 1); assert.equal(scene.nodes.has(1), false); assert.equal(scene.nodes.has(2), false); assert.ok(scene.diagnostics.some(issue => issue.code === 'invalid_parent')); scene.dispose();
  a.parent_index = 99; b.parent_index = null; b.geometry.data.x = NaN;
  const invalid = buildModulePresentation(presentation(a, b)); assert.equal(invalid.nativeMeshCount, 0); assert.ok(invalid.diagnostics.some(issue => issue.code === 'invalid_geometry')); invalid.dispose();
});
test('mesh resources are disposed and declared actions do not create a clock or change input', () => {
  const value = presentation(node()); value.actions = [{ id: 'declared-only' }]; const original = structuredClone(value);
  const scene = buildModulePresentation(value), mesh = scene.nodes.get(1).children[0]; let geometry = 0, surface = 0;
  mesh.geometry.addEventListener('dispose', () => geometry++); mesh.material.addEventListener('dispose', () => surface++);
  assert.ok(scene.diagnostics.some(issue => issue.code === 'static_preview')); assert.deepEqual(value, original);
  scene.dispose(); assert.equal(geometry, 1); assert.equal(surface, 1); assert.equal(scene.root.children.length, 0);
});

test('Y-up part grouping and mixed-basis ancestor movement agree with the same rendered Presentation', async () => {
  const {source}=await import('./load-source.mjs');const parts=await source('device-authoring','parts');
  const a=node(1),b=node(2);a.transform=identity('millimeter','right_handed_y_up');b.transform=identity('millimeter','right_handed_y_up');a.transform.translation={x:500,y:1000,z:0};b.transform.translation={x:1500,y:2000,z:0};
  const value=presentation(a,b);value.coordinate_frame='right_handed_y_up';
  const initial=buildModulePresentation(value);const before=initial.nodes.get(2).matrixWorld.clone();
  const grouped=buildModulePresentation({...value,nodes:parts.groupParts(value.nodes,[0,1])});
  before.elements.forEach((number,index)=>near(grouped.nodes.get(2).matrixWorld.elements[index],number));
  const mixed=presentation(node(1),node(2));mixed.nodes[0].transform=identity('meter','right_handed_z_up');mixed.nodes[0].transform.translation.x=1;mixed.nodes[1].transform=identity('millimeter','right_handed_y_up');mixed.nodes[1].parent_index=0;mixed.nodes[1].transform.translation.y=1000;
  const original=buildModulePresentation(mixed),moved=buildModulePresentation({...mixed,nodes:parts.moveParts(mixed.nodes,[1],{x:250,y:0,z:100})});
  const delta=new THREE.Vector3().setFromMatrixPosition(moved.nodes.get(2).matrixWorld).sub(new THREE.Vector3().setFromMatrixPosition(original.nodes.get(2).matrixWorld));vector(delta,[.25,0,.1]);
  [initial,grouped,original,moved].forEach(scene=>scene.dispose());
});
