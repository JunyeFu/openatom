import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import * as THREE from 'three';
const root = new URL('../../../', import.meta.url), out = new URL('.work/p201-whole-20261017/model-tests/', root);
await mkdir(out, { recursive: true });
await writeFile(new URL('factoryCapture.mjs', out), stripTypeScriptTypes(await readFile(new URL('apps/planner/src/features/render-surface/factoryCapture.ts', root), 'utf8')));
const { visibleFactoryObjects } = await import(new URL('factoryCapture.mjs', out));
function fixture() { const scene = new THREE.Scene(), camera = new THREE.PerspectiveCamera(60, 1, .1, 100); camera.position.set(0, 0, 10); camera.lookAt(0, 0, 0); camera.updateMatrixWorld(true); return { scene, camera }; }
function marker(id) { const object = new THREE.Sprite(new THREE.SpriteMaterial({ color: 'white' })); object.userData.ref = { kind: 'resource', id }; return object; }
test('a real visible worker Sprite contributes its actual resource ID beside Mesh, Line and Points', () => {
  const { scene, camera } = fixture(); scene.add(marker('worker'));
  const mesh = new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshBasicMaterial()); mesh.userData.ref = { kind: 'device', id: 'machine' }; scene.add(mesh);
  const geometry = new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(-1, 0, 0), new THREE.Vector3(1, 0, 0)]);
  const line = new THREE.Line(geometry, new THREE.LineBasicMaterial()); line.userData.ref = { kind: 'line', id: 'line' }; scene.add(line);
  const point = new THREE.Points(geometry, new THREE.PointsMaterial()); point.userData.ref = { kind: 'location', id: 'location' }; scene.add(point);
  assert.deepEqual(new Set(visibleFactoryObjects(scene, camera).map(value => value.kind + ':' + value.id)), new Set(['resource:worker', 'device:machine', 'line:line', 'location:location']));
});
test('Sprite inclusion respects the real frustum, layers, material and complete visible parent chain', () => {
  const { scene, camera } = fixture(); const visible = marker('visible'); scene.add(visible);
  const outside = marker('outside'); outside.position.x = 100; scene.add(outside);
  const invisible = marker('invisible'); invisible.visible = false; scene.add(invisible);
  const masked = marker('masked'); masked.layers.set(1); scene.add(masked);
  const transparent = marker('transparent'); transparent.material.opacity = 0; scene.add(transparent);
  const materialHidden = marker('material-hidden'); materialHidden.material.visible = false; scene.add(materialHidden);
  const parent = new THREE.Group(); parent.visible = false; parent.add(marker('parent-hidden')); scene.add(parent);
  assert.deepEqual(visibleFactoryObjects(scene, camera), [{ kind: 'resource', id: 'visible' }]);
});
