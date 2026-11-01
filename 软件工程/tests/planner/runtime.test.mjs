import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';

const root = new URL('../../', import.meta.url);
const output = new URL('.work/whole-packages/p201-ui/runtime.mjs', root);
await mkdir(new URL('.', output), { recursive: true });
await writeFile(output, stripTypeScriptTypes(await readFile(new URL('apps/planner/src/scene/runtime.ts', root), 'utf8')));
const { decimalText, itemAnchor, deviceActivity, objectHall, transportPhaseAt } = await import(output.href);

test('quantities preserve declared decimal places and safe integer precision', () => {
  assert.equal(decimalText({ coefficient: 7, decimal_places: 3 }), '0.007');
  assert.equal(decimalText({ coefficient: 0, decimal_places: 2 }), '0.00');
  assert.equal(decimalText({ coefficient: 9007199254740991, decimal_places: 6 }), '9007199254.740991');
});

test('actual nested holders resolve to recorded equipment; transport and missing holders remain unlocated', () => {
  const box = { id: 'BOX', holder: { kind: 'job', id: 'PACK' } };
  const part = { id: 'PART', holder: { kind: 'container', id: 'BOX' } };
  const run = { jobs: [{ id: 'PACK', execution_ref: { kind: 'project_device', data: { device_id: 'D1' } } }] };
  const before = JSON.stringify([box, part, run]);
  assert.deepEqual(itemAnchor(part, [box, part], run), { kind: 'device', id: 'D1' });
  assert.equal(itemAnchor({ ...part, holder: { kind: 'transport', id: 'T1' } }, [box, part], run), null);
  assert.equal(itemAnchor(part, [part], run), null);
  const cycle = { ...box, holder: { kind: 'container', id: 'PART' } };
  assert.equal(itemAnchor(part, [cycle, part], run), null);
  assert.equal(JSON.stringify([box, part, run]), before);
});

test('device state uses actual work and wait intervals with half-open boundaries, never nominal job duration', () => {
  const ref = { kind: 'project_device', data: { device_id: 'D1' } };
  const run = {
    jobs: [{ id: 'J1', execution_ref: ref }, { id: 'OTHER', execution_ref: { kind: 'project_device', data: { device_id: 'D2' } } }],
    phase_work_intervals: [
      { job_id: 'J1', interval: { start_ms: 10, end_ms: 20 } },
      { job_id: 'J1', interval: { start_ms: 10, end_ms: 20 } },
      { job_id: 'OTHER', interval: { start_ms: 0, end_ms: 100 } },
    ],
    waits: [{ execution_ref: ref, interval: { start_ms: 20, end_ms: 30 } }],
  };
  assert.deepEqual(deviceActivity(run, 9, 'D1'), { work: 0, waits: 0 });
  assert.deepEqual(deviceActivity(run, 10, 'D1'), { work: 1, waits: 0 });
  assert.deepEqual(deviceActivity(run, 20, 'D1'), { work: 0, waits: 1 });
  assert.deepEqual(deviceActivity(run, 30, 'D1'), { work: 0, waits: 0 });
});

test('actual B60 item and holder navigation resolves the frozen hall at each observed boundary', async () => {
  const actual = JSON.parse(await readFile(new URL('tests/planner/fixtures/p1-b60-navigation.actual.json', root)));
  const before = JSON.stringify(actual);
  assert.equal(objectHall(actual.project, { kind: 'location', id: 'L1-SOURCE' }, [], actual.run), 'H1');
  for (const frame of actual.observations) for (const item of frame.items) {
    assert.equal(objectHall(actual.project, { kind: 'item', id: item.id }, frame.items, actual.run), 'H3');
    assert.equal(objectHall(actual.project, { kind: item.holder.kind, id: item.holder.id }, frame.items, actual.run), 'H3');
  }
  assert.equal(objectHall(actual.project, { kind: 'line', id: 'L6' }, [], actual.run), 'H3');
  assert.equal(objectHall(actual.project, { kind: 'hall', id: 'H3' }, [], actual.run), 'H3');
  assert.equal(objectHall(actual.project, { kind: 'location', id: 'absent' }, [], actual.run), null);
  assert.equal(objectHall(actual.project, { kind: 'item', id: 'L6-ITEM-0001' }, [], actual.run), null);
  assert.equal(objectHall(actual.project, { kind: 'job', id: actual.run.jobs[0].id }, [], null), null);
  const moving = { id: 'moving', holder: { kind: 'transport', id: 'unlocated-trip' } };
  assert.equal(objectHall(actual.project, { kind: 'item', id: moving.id }, [moving], actual.run), null);
  assert.equal(JSON.stringify(actual), before);
});

test('a selected transport shows its actual phase at the observation, never its final returned phase', async () => {
  const actual = JSON.parse(await readFile(new URL('scenarios/从零搭建控制盒厂房/runtime-timeline/delayed-pcb.actual.json', root)));
  const trip = actual.run.transport.find(value => value.arrived_ms === 370000);
  const before = JSON.stringify(trip);
  assert.equal(trip.phase, 'returned');
  for (const [at, expected] of [[359999, '尚未发出'], [360000, '在途'], [369999, '在途'], [370000, '等待目的接收'], [419999, '等待目的接收'], [420000, '返程中'], [424999, '返程中'], [425000, '已返库']]) {
    assert.equal(transportPhaseAt(trip, at), expected, `phase at ${at}`);
  }
  assert.equal(transportPhaseAt(trip, null), '未知时点');
  assert.equal(transportPhaseAt({ ...trip, arrived_ms: null, received_ms: null, returned_ms: null }, 425000), '在途');
  assert.equal(JSON.stringify(trip), before);
});
