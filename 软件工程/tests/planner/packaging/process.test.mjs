import test from 'node:test';
import assert from 'node:assert/strict';
import * as THREE from 'three';
import { readFile } from 'node:fs/promises';
import { connectionAnchors, panelGeometry, panelTransform } from '../../../apps/planner/src/features/packaging/process.ts';
const job = JSON.parse(await readFile(new URL('../../../modules/professional/paper-packaging-ui/sliding-sleeve-explicit-candidate.job.json', import.meta.url), 'utf8')).data;
test('actual polygon and finite panel thickness are preserved; renderer does not widen it to a bounding box', () => {
  const panel = structuredClone(job.panels[0]); panel.polygon_mm = [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 0, y: 10 }]; panel.thickness_mm = .5;
  const geometry = panelGeometry(panel); geometry.computeBoundingBox(); assert.equal(geometry.boundingBox.min.z, -.25); assert.equal(geometry.boundingBox.max.z, .25);
  const points = geometry.getAttribute('position'); for (let i = 0; i < points.count; i++) assert.ok(points.getX(i) + points.getY(i) <= 10.000001); geometry.dispose();
});
test('actual sample world pose is used directly; a missing sample panel has no flat-pose fallback', () => {
  const panel = job.panels[0]; const pose = structuredClone(panel.flat_pose); pose.translation = { x: 7, y: 11, z: 13 };
  const sample = { step_id: 'actual-protocol-step', progress: .375, panel_poses: [{ panel_id: panel.id, pose }] };
  const actual = new THREE.Vector3().applyMatrix4(panelTransform(job, panel, sample)); assert.deepEqual(actual.toArray(), [7, 11, 13]);
  assert.equal(panelTransform(job, panel, { ...sample, panel_poses: [] }), null);
});
test('input flat preview composes actual part placement and flat panel pose, preserving unsupported units explicitly', () => {
  const input = structuredClone(job); const panel = input.panels[0]; const part = input.parts.find(item => item.id === panel.part_id); part.placement.translation = { x: 100, y: 200, z: 300 };
  const actual = new THREE.Vector3().applyMatrix4(panelTransform(input, panel, null)); assert.deepEqual(actual.toArray(), [100 + panel.flat_pose.translation.x, 200 + panel.flat_pose.translation.y, 300 + panel.flat_pose.translation.z]);
  input.unit = 'meter'; assert.equal(panelTransform(input, panel, null), null);
});
test('connection reference endpoints are derived only from actual declared panel frames', () => {
  const input = structuredClone(job); const [a, b] = input.panels; const frame = structuredClone(a.flat_pose); frame.translation = { x: 0, y: 0, z: 0 };
  input.connections = [{ id: 'protocol-connection', panel_a: a.id, panel_b: b.id, frame_a: frame, frame_b: frame, kind: 'glue', allow_contact: false, contact_allowance_refs: [] }];
  assert.equal(connectionAnchors(input, null).length, 1); assert.equal(connectionAnchors(input, { step_id: 's', progress: 0, panel_poses: [] }).length, 0);
});
