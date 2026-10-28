import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import { Vector3 } from 'three';

const root = new URL('../../', import.meta.url);
const output = new URL('.work/whole-packages/p201-ui/layout.mjs', root);
await mkdir(new URL('.', output), { recursive: true });
const source = await readFile(new URL('apps/planner/src/scene/layout.ts', root), 'utf8');
await writeFile(output, stripTypeScriptTypes(source));
const { identityPose, deviceHallPosition, devicePlacementCommands, withHallPosition, rotatePoseZ } = await import(output.href);
const close = (actual, expected) => actual.forEach((value, i) => assert.ok(Math.abs(value - expected[i]) < 1e-10, `${actual} != ${expected}`));

test('line-local placement displays in hall coordinates and an inspector edit inverts the actual group transform', () => {
  const transform = rotatePoseZ({ ...identityPose(), translation: { x: 10, y: 20, z: 0 } }, 90);
  const project = { lines: [{ id: 'L1', transform }] };
  const device = { line_id: 'L1', pose: { ...identityPose(), translation: { x: 2, y: 3, z: 4 } } };
  close(deviceHallPosition(project, device).toArray(), [7, 22, 4]);
  const edited = withHallPosition(project, device, new Vector3(8, 24, 4));
  close(Object.values(edited.pose.translation), [4, 2, 4]);
  close(deviceHallPosition(project, edited).toArray(), [8, 24, 4]);
  assert.deepEqual(device.pose.translation, { x: 2, y: 3, z: 4 });
});

test('moving only the line root moves its member once without mutating the member placement', () => {
  const device = { line_id: 'L1', pose: { ...identityPose(), translation: { x: 2, y: 3, z: 4 } } };
  const before = JSON.stringify(device);
  const project = { lines: [{ id: 'L1', transform: { ...identityPose(), translation: { x: 5, y: 0, z: 0 } } }] };
  close(deviceHallPosition(project, device).toArray(), [7, 3, 4]);
  project.lines[0].transform.translation.x = 12;
  close(deviceHallPosition(project, device).toArray(), [14, 3, 4]);
  assert.equal(JSON.stringify(device), before);
});

test('the Z shortcut composes with an existing tilted quaternion instead of discarding its other orientation', () => {
  const pose = { ...identityPose(), rotation: { x: Math.SQRT1_2, y: 0, z: 0, w: Math.SQRT1_2 } };
  const result = rotatePoseZ(pose, 90);
  close(Object.values(result.rotation), [0.5, 0.5, 0.5, 0.5]);
  assert.deepEqual(pose.rotation, { x: Math.SQRT1_2, y: 0, z: 0, w: Math.SQRT1_2 });
});

test('regrouping submits both member lists and the same physical hall pose together without mutating the source', () => {
  const device = { id: 'D1', hall_id: 'H1', line_id: 'A', pose: { ...identityPose(), translation: { x: 2, y: 3, z: 4 } } };
  const project = { devices: [device], lines: [
    { id: 'A', hall_id: 'H1', members: ['D1'], transform: rotatePoseZ({ ...identityPose(), translation: { x: 10, y: 20, z: 0 } }, 90) },
    { id: 'B', hall_id: 'H1', members: [], transform: identityPose() },
  ] };
  const before = JSON.stringify(project);
  const commands = devicePlacementCommands(project, device, 'B', new Vector3(7, 22, 4));
  assert.equal(commands.length, 3);
  assert.deepEqual(commands[0].data.data.members, []);
  assert.deepEqual(commands[1].data.data.members, ['D1']);
  const next = commands[2].data.data;
  assert.equal(next.id, 'D1'); assert.equal(next.line_id, 'B');
  close(Object.values(next.pose.translation), [7, 22, 4]);
  close(Object.values(next.pose.rotation), [0, 0, Math.SQRT1_2, Math.SQRT1_2]);
  assert.equal(JSON.stringify(project), before);
});

test('a missing line parent remains unknown rather than treating its local coordinates as hall coordinates', () => {
  const device = { line_id: 'missing', pose: { ...identityPose(), translation: { x: 2, y: 3, z: 4 } } };
  assert.equal(deviceHallPosition({ lines: [] }, device), null);
});
