import test from 'node:test';
import assert from 'node:assert/strict';
import { timelineProcessJob } from '../../../apps/planner/src/features/factory/timelineProcessJob.ts';

const active = { id: 'working', execution_ref: { kind: 'project_device', data: { device_id: 'D01' } } };
const queued = { id: 'queued', execution_ref: active.execution_ref };
const definitionCycle = { id: 'cycle', execution_ref: { kind: 'definition_cycle', data: { module_ref: { id: 'station', version: '1.0.4' } } } };
const run = { id: 'run-A', jobs: [active, queued, definitionCycle] };
const wait = { source: 'actual', run_id: 'run-A', object: { kind: 'job', id: 'queued' } };

test('waiting Job entry retains explicit identity instead of selecting the device active Job', () => {
  assert.equal(timelineProcessJob(wait, run), queued);
});
test('a resource claim opens its explicitly associated Job', () => {
  assert.equal(timelineProcessJob({ ...wait, object: { kind: 'resource', id: 'CAP' }, job: queued }, run), queued);
});
test('comparison or old Run, manual input and absent Job cannot open current Run process', () => {
  assert.equal(timelineProcessJob({ ...wait, run_id: 'run-B' }, run), null);
  assert.equal(timelineProcessJob({ ...wait, source: 'input' }, run), null);
  assert.equal(timelineProcessJob({ ...wait, object: { kind: 'job', id: 'absent' } }, run), null);
  assert.equal(timelineProcessJob(wait, null), null);
});
test('static device and definition-cycle identities do not become project-device Jobs', () => {
  assert.equal(timelineProcessJob({ ...wait, object: { kind: 'device', id: 'D01' } }, run), null);
  assert.equal(timelineProcessJob({ ...wait, object: { kind: 'job', id: 'cycle' } }, run), null);
});
