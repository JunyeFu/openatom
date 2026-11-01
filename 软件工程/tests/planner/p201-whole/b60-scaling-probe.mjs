import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import { openCore, root, out } from './current-core.mjs';

const perLine = Number(process.argv[2] ?? 10);
const suffix = process.argv[3] ?? 'inspect';
const executable = process.argv[4] ?? 'optimized-core.exe';
assert.ok(Number.isInteger(perLine) && perLine > 0 && perLine < 500);
const input = JSON.parse(await readFile(new URL('tests/planner/p201-whole/fixtures/b60/balanced.input.json', root), 'utf8'));
input.scenario.initial_conditions.items = input.scenario.initial_conditions.items.filter(item => Number(item.id.slice(-4)) <= perLine);
for (const task of input.scenario.production_tasks) task.planned_quantity.value.coefficient = perLine;
const fixture = new URL(`b60-scaling-${perLine}.${suffix}.input.json`, out);
await writeFile(fixture, JSON.stringify(input));
const result = { kind: 'reduced_diagnostic_for_observed_long_full_B60_compute', original_B60: false, Native_performance: false, per_line_items: perLine, total_items: perLine * 6, expected_first_ms: 20000, expected_last_ms: (perLine + 9) * 2000 };
let core; const started = performance.now();
try {
  core = await openCore(fileURLToPath(fixture), executable);
  result.elapsed_diagnostic_ms = performance.now() - started;
  const run = core.ready.formal_run, sinks = new Set(input.project.lines.map(line => `${line.id}-SINK`));
  const accepted = run.acceptance_events.filter(event => sinks.has(event.destination));
  result.computation_status = run.computation_status;
  result.acceptance_event_count = run.acceptance_events.length; result.first_event = run.acceptance_events[0]; result.final_holder_sample = run.final_items.slice(0, 3).map(item => ({id:item.id,holder:item.holder}));
  result.jobs = run.jobs.length; result.accepted = accepted.length;
  result.first_ms = Math.min(...accepted.map(event => event.at_ms)); result.last_ms = Math.max(...accepted.map(event => event.at_ms));
  await writeFile(new URL(`b60-scaling-${perLine}.${suffix}.run.json`, out), JSON.stringify(run));
  assert.equal(run.computation_status, 'completed'); assert.equal(result.jobs, perLine * 60); assert.equal(result.accepted, perLine * 6);
  assert.equal(result.first_ms, result.expected_first_ms); assert.equal(result.last_ms, result.expected_last_ms);
  assert.deepEqual(new Set(run.final_items.map(item => item.id)), new Set(input.scenario.initial_conditions.items.map(item => item.id)));
  result.status = 'diagnostic_passed_full_B60_result_still_separate';
} catch (error) { result.status = 'failed'; result.error = String(error); process.exitCode = 1; }
finally { await core?.close(); await writeFile(new URL(`b60-scaling-${perLine}.${suffix}.result.json`, out), JSON.stringify(result, null, 2)); }
console.log(JSON.stringify(result));
