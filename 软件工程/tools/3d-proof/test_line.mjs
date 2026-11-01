import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { LineSimulation } from './line-simulation.mjs';
const config = JSON.parse(readFileSync(new URL('./line-config.json', import.meta.url)));
const checks = [];

function consistent(run) {
  const s = run.snapshot();
  const holders = new Map();
  const add = (id, holder) => { assert(!holders.has(id), `duplicate holder: ${id}`); holders.set(id, holder); };
  for (const [q, ids] of Object.entries(s.queues)) {
    if (q !== 'quarantine') assert(ids.length <= config.queue_capacity);
    ids.forEach(id => add(id, 'Q-' + q));
  }
  s.pallet.forEach(id => add(id, 'PAL01'));
  for (const item of Object.values(s.items)) {
    if (item.type === 'package') item.contents.forEach(id => add(id, item.id));
    if (run.devices[item.holder]) add(item.id, item.holder);
  }
  for (const item of Object.values(s.items)) assert.equal(holders.get(item.id), item.holder, item.id);
  assert(s.pallet.length <= run.config.pallet.capacity);
  assert.equal(new Set(s.pallet).size, s.order.accepted);
  assert(Object.values(s.stock).every(q => q >= 0));
  for (const [role, owner] of Object.entries(s.resources)) {
    if (owner) assert(run.devices[owner].job && run.devices[owner].resources.includes(role));
  }
  const assemblies = Object.values(s.items).filter(i => i.type === 'assembly');
  const packages = Object.values(s.items).filter(i => i.type === 'package');
  assert(assemblies.length <= run.startedAssemblies);
  assert.equal(new Set(assemblies.flatMap(i => i.components)).size, assemblies.length * 6);
  assert.equal(new Set(packages.flatMap(i => i.contents)).size, packages.length);
  for (const d of Object.values(s.devices)) assert(d.effective_progress >= 0 && d.effective_progress <= 1);
}

function check(name, fn) { fn(); checks.push({ id: name, passed: true }); }
let baseline;
check('baseline_six_packages_identity_recipe_and_independent_expected_times', () => {
  baseline = new LineSimulation(config);
  for (let time = 0; time <= 350000; time += 1000) { baseline.advanceTo(time); consistent(baseline); }
  const s = baseline.snapshot();
  assert.equal(s.order.accepted, 6);
  assert.deepEqual(baseline.events.filter(e => e.type === 'order_accepted').map(e => e.at_ms), [90000, 130000, 240000, 255000, 290000, 330000]);
  assert.deepEqual(s.stock, { shell: 0, pcb: 0, fastener: 0, carton: 0, tape_meter: 0, tape_reserved_meter: 0 });
  assert.equal(s.items['A-003'].completed_reworks, 1);
  assert.equal(s.items['A-003'].holder, 'PK-003');
  assert.equal(s.items['PK-003'].holder, 'PAL01');
  assert.equal(s.items['PK-003'].closure, 'sealed');
  assert.equal(baseline.events.filter(e => e.type === 'rework_completed').length, 1);
  assert(Object.values(s.resources).every(owner => owner === null));
});
check('frame_step_size_does_not_change_business_events', () => {
  const coarse = new LineSimulation(config); coarse.advanceTo(350000);
  assert.deepEqual(coarse.result(), baseline.result());
  const autoStop = new LineSimulation(config); autoStop.advanceTo(1000000, true);
  assert.equal(autoStop.now, 330000); assert.equal(autoStop.order.accepted, 6);
  assert.deepEqual(autoStop.events, coarse.events); consistent(autoStop);
});
check('missing_pcb_waits_then_manual_supply_completes_same_order', () => {
  const run = new LineSimulation(config, 'missing_pcb'); run.advanceTo(400000); consistent(run);
  assert.equal(run.order.accepted, 2); assert.equal(run.devices.ASM01.state, 'waiting_inputs');
  assert.equal(run.stock.shell, 4); assert.equal(run.stock.fastener, 16);
  run.supply('pcb', 4); run.advanceTo(900000); consistent(run);
  assert.equal(run.order.accepted, 6);
});
check('missing_quality_holds_inspector_and_same_item_then_recovers', () => {
  const run = new LineSimulation(config, 'missing_quality'); run.advanceTo(400000); consistent(run);
  assert.equal(run.order.accepted, 1); assert.equal(run.devices.V01.state, 'waiting_quality');
  assert.equal(run.resources.inspector, 'V01'); assert.equal(run.items['A-002'].holder, 'V01');
  const progress = run.snapshot().devices.V01.effective_progress;
  run.advanceTo(500000); assert.equal(run.snapshot().devices.V01.effective_progress, progress);
  run.setQuality('A-002', 'pass'); run.advanceTo(1000000); consistent(run);
  assert.equal(run.order.accepted, 6);
});
check('second_failed_inspection_quarantines_same_identity_without_extra_rework', () => {
  const run = new LineSimulation(config, 'reject'); run.advanceTo(600000); consistent(run);
  assert.equal(run.order.accepted, 5); assert.deepEqual(run.queues.quarantine, ['A-005']);
  assert.equal(run.items['A-005'].completed_reworks, 1); assert.equal(run.items['A-005'].quality, 'fail');
  assert.equal(run.events.filter(e => e.type === 'rework_completed' && e.item_id === 'A-005').length, 1);
});
check('output_blocked_keeps_sealed_package_and_does_not_consume_tape_twice', () => {
  const run = new LineSimulation(config, 'output_blocked'); run.advanceTo(140000); consistent(run);
  assert.equal(run.order.accepted, 0); assert.equal(run.devices.CASE01.state, 'waiting_output');
  assert.equal(run.items['PK-001'].holder, 'CASE01'); assert.equal(run.items['PK-001'].closure, 'sealed');
  assert.equal(run.tapeMm, 3000);
  run.advanceTo(179000); assert.equal(run.tapeMm, 3000);
  run.advanceTo(350000); consistent(run); assert.equal(run.order.accepted, 6); assert.equal(run.tapeMm, 0);
  assert.equal(run.events.filter(e => e.type === 'order_accepted')[0].at_ms, 180000);
});
check('thirteenth_package_waits_at_full_twelve_slot_carrier', () => {
  const input = structuredClone(config); input.order.quantity = 13;
  input.supplies = { shell: 13, pcb: 13, fastener: 52, carton: 13, tape_meter: 7.8 }; input.arrivals = [];
  const run = new LineSimulation(input); run.advanceTo(1500000); consistent(run);
  assert.equal(run.order.accepted, 12); assert.equal(run.devices.CASE01.state, 'waiting_output');
  assert.equal(run.items['PK-013'].holder, 'CASE01'); assert.equal(run.items['PK-013'].closure, 'sealed');
  assert.equal(run.tapeMm, 0);
});
check('robot_grip_delivery_return_and_destination_capacity_are_distinct', () => {
  const run = new LineSimulation(config); run.advanceTo(42500);
  assert.equal(run.items['A-001'].holder, 'Q-asm');
  run.advanceTo(43500); assert.equal(run.items['A-001'].holder, 'R01');
  run.advanceTo(49000); assert.equal(run.items['A-001'].holder, 'CV01');
  assert.equal(run.devices.R01.state, 'working');
  run.advanceTo(52000); assert.equal(run.devices.R01.job, null); consistent(run);
});

const result = { scope: 'executed_headless_L03_business_checks', checks, baseline: baseline.result(), product_acceptance: 'not_claimed' };
writeFileSync(new URL('./output/line-business-checks.json', import.meta.url), JSON.stringify(result, null, 2));
console.log(JSON.stringify({ checks_passed: checks.length, accepted: baseline.order.accepted,
  last_acceptance_seconds: baseline.result().last_acceptance_ms / 1000,
  evidence: 'output/line-business-checks.json', scope: result.scope }, null, 2));
