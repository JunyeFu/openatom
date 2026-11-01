import test from 'node:test';
import assert from 'node:assert/strict';
import { PerspectiveCamera, Vector2, Vector3, Plane, Raycaster } from 'three';
import { viewportMetersPerPixel, chooseGridStep, viewportScaleFor, createAdaptiveViewportGrid } from '../../../apps/planner/src/scene/AdaptiveViewportGrid.ts';

const near = (actual, expected) => assert.ok(Math.abs(actual - expected) <= Math.max(1e-12, Math.abs(expected) * 1e-10), `${actual} vs ${expected}`);

test('viewport scale follows actual camera FOV, zoom and CSS height', () => {
  const target = new Vector3(1, 2, 3), view = new Vector2();
  for (const fov of [30, 42, 65]) for (const zoom of [1, 2]) for (const height of [350, 700]) {
    const camera = new PerspectiveCamera(fov, 16 / 9, .01, 100000);
    camera.position.set(10, 8, 6); camera.zoom = zoom; camera.updateProjectionMatrix();
    near(viewportMetersPerPixel(camera, target, height, view), 2 * camera.position.distanceTo(target) * Math.tan(fov * Math.PI / 360) / zoom / height);
  }
});

test('binary grid levels retain their spacing inside the hysteresis band', () => {
  assert.equal(chooseGridStep(.25 / 64, null), .25);
  assert.equal(chooseGridStep(1 / 40, 1), 1);
  assert.equal(chooseGridStep(1 / 96, 1), 1);
  assert.equal(chooseGridStep(1 / 39.99, 1), 2);
  assert.equal(chooseGridStep(1 / 40.01, 2), 2);
  assert.equal(chooseGridStep(2 / 96.01, 2), 1);
  assert.equal(chooseGridStep(2 / 95.99, 1), 1);
});

test('continuous zoom stays in the grid screen band and ruler is physically calibrated', () => {
  let step = null;
  for (let power = -8; power <= 4; power += .02) {
    const metersPerPixel = 10 ** power;
    step = chooseGridStep(metersPerPixel, step);
    near(Math.log2(step), Math.round(Math.log2(step)));
    assert.ok(step / metersPerPixel >= 40 - 1e-9 && step / metersPerPixel <= 96 + 1e-9);
    const scale = viewportScaleFor(metersPerPixel);
    assert.ok(scale.widthPx >= 40 - 1e-8 && scale.widthPx <= 100 + 1e-8);
    near(scale.widthPx * metersPerPixel, scale.totalM);
  }
});

test('half-size ruler halves the physical length and formats metric units', () => {
  for (const [reference, width, mid, end] of [[.5, 64, '0.125', '0.25 m'], [2, 64, '0.5', '1 m'], [.1, 64, '2.5', '5 cm'], [.01, 64, '2.5', '5 mm'], [2000, 64, '0.5', '1 km']]) {
    const value = viewportScaleFor(reference / 128);
    near(value.widthPx, width); near(value.totalM, reference / 2);
    assert.equal(value.midLabel, mid); assert.equal(value.endLabel, end);
  }
});

test('ruler remains valid at the inclusive 80/200px reference boundaries', () => {
  for (let exponent = -9; exponent < 9; exponent++) for (const multiple of [1, 2, 5]) for (const pixels of [80, 200]) {
    const mpp = multiple * 10 ** exponent / pixels, value = viewportScaleFor(mpp);
    assert.ok(value.widthPx >= 40 - 1e-8 && value.widthPx <= 100 + 1e-8);
    near(value.widthPx * mpp, value.totalM);
  }
});

test('grid covers the visible ground intersection and stays aligned to engineering coordinates', () => {
  const grid = createAdaptiveViewportGrid(), camera = new PerspectiveCamera(42, 16 / 9, .01, 100000), target = new Vector3(2, 0, -3);
  camera.position.set(12, 9, 10); camera.lookAt(target);
  const state = grid.update(camera, target, 700), bounds = state.bounds;
  assert.ok(bounds); assert.equal(grid.mesh.userData.ref, undefined);
  for (const value of Object.values(bounds)) near(value / state.stepM, Math.round(value / state.stepM));
  const plane = new Plane(new Vector3(0, 1, 0), -.001), ray = new Raycaster(), hit = new Vector3();
  for (const x of [-.8, 0, .8]) for (const y of [-.8, 0, .8]) {
    ray.setFromCamera(new Vector2(x, y), camera);
    if (!ray.ray.intersectPlane(plane, hit)) continue;
    assert.ok(hit.x >= bounds.minX && hit.x <= bounds.maxX && hit.z >= bounds.minZ && hit.z <= bounds.maxZ);
  }
  const geometry = grid.mesh.geometry, material = grid.mesh.material;
  camera.position.add(new Vector3(8, 0, -4)); target.add(new Vector3(8, 0, -4)); camera.lookAt(target);
  const panned = grid.update(camera, target, 700);
  near(panned.metersPerPixel, state.metersPerPixel); assert.equal(panned.stepM, state.stepM);
  camera.zoom = 2; camera.updateProjectionMatrix(); grid.update(camera, target, 350);
  assert.equal(grid.mesh.geometry, geometry); assert.equal(grid.mesh.material, material);
  assert.match(material.vertexShader, /ground = world\.xz/);
  geometry.dispose(); material.dispose();
});

test('top/bottom views use the ground, while a view entirely above the ground hides it', () => {
  const grid = createAdaptiveViewportGrid(), camera = new PerspectiveCamera(42, 1, .01, 1000), target = new Vector3();
  for (const y of [10, -10]) {
    camera.position.set(0, y, .0001); camera.lookAt(target);
    assert.ok(grid.update(camera, target, 700).bounds);
  }
  camera.position.set(0, 4, 0); target.set(0, 8, 0); camera.lookAt(target);
  assert.equal(grid.update(camera, target, 700).bounds, null); assert.equal(grid.mesh.visible, false);
  grid.mesh.geometry.dispose(); grid.mesh.material.dispose();
});
