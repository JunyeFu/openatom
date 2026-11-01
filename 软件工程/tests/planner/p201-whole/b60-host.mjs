import { readFile, writeFile, cp } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import { openCore, root, out } from './current-core.mjs';
const attempt = process.argv[3] ?? 'capacity-explicit';
const variant = process.argv[2]; assert.ok(['balanced', 'dense'].includes(variant));
const executable = process.argv[4] ?? 'current-core.exe';
const fixtureUrl = new URL(process.argv[6] ?? 'tests/planner/p201-whole/fixtures/b60/' + variant + '.input.json', root);
const fixture = JSON.parse(await readFile(fixtureUrl, 'utf8'));
let core;
const result = { kind: 'actual_current_frozen_Commands_Host_Engine_B60_input_admission_and_independent_expected_counts', variant, Native_app: false,
  fixture: fileURLToPath(fixtureUrl), frontier: process.argv[5] ?? '.work/whole-packages/p001-20261017/current-core-libraries.json', executable, actual_result: null };
try {
  core = await openCore(fileURLToPath(fixtureUrl), executable);
  assert.ok(core.ready.capabilities.implemented_operations.some(value => value.operation === 'check_factory_draft'));
  const run = core.ready.formal_run, sinks = new Set(fixture.project.lines.map(line => line.id + '-SINK'));
  const accepted = run.acceptance_events.filter(event => sinks.has(event.destination));
  const quantity = events => events.reduce((total, event) => total + event.quantity.value.coefficient / 10 ** event.quantity.value.decimal_places, 0);
  const acceptedIds = new Set(accepted.flatMap(event => event.item_refs));
  const initialIds = new Set(fixture.scenario.initial_conditions.items.map(item => item.id));
  const counts = { first_ms: Math.min(...accepted.map(event => event.at_ms)), last_ms: Math.max(...accepted.map(event => event.at_ms)), at_600s: quantity(accepted.filter(event => event.at_ms <= 600000)), at_1200s: quantity(accepted), distinct_accepted_identities: acceptedIds.size, final_item_identities: run.final_items.length, jobs: run.jobs.length };
  await writeFile(new URL('b60-' + variant + '.current.' + attempt + '.run.json', out), JSON.stringify(run));
  result.run_id = run.id; result.counts = counts; result.computation_status = run.computation_status; result.limitations = run.limitations;
  assert.equal(run.computation_status, 'completed'); assert.equal(run.computed_until_ms, 1200000);
  assert.equal(counts.jobs, 30000); assert.equal(counts.final_item_identities, 3000);
  assert.equal(counts.first_ms, 20000); assert.equal(counts.last_ms, 1018000); assert.equal(counts.at_600s, 1746); assert.equal(counts.at_1200s, 3000);
  assert.equal(acceptedIds.size, 3000); assert.deepEqual(acceptedIds, initialIds); assert.deepEqual(new Set(run.final_items.map(item => item.id)), initialIds);
  assert.ok(run.final_items.every(item => item.holder.kind === 'location' && sinks.has(item.holder.id)));
  const request = async (operation, params) => {
    const reply = await core.rpc({ kind: 'dispatch', request: { api_version: '0.1', request_id: `b60-${variant}-${attempt}-${Math.random()}`, project_id: fixture.project.id, scope: { kind: 'project' }, operation, params } });
    assert.equal(reply.outcome.status, 'ok'); return reply.outcome.data;
  };
  result.identity_observations = [];
  for (const at_ms of [0, 600000, 1200000]) {
    const pages = []; let after_index = null;
    do {
      const view = await request('get_run', { run_id: run.id, query: { kind: 'objects_at', data: { at_ms, objects: [], page: { after_index, limit: 500 } } } });
      assert.equal(view.kind, 'objects'); assert.equal(view.data.at_ms, at_ms); pages.push(view.data); after_index = view.data.next_index;
    } while (after_index !== null);
    await writeFile(new URL(`b60-${variant}.${attempt}.items-${at_ms}.json`, out), JSON.stringify(pages));
    const items = pages.flatMap(page => page.objects).filter(object => object.kind === 'item').map(object => object.data);
    assert.equal(items.length, 3000); assert.deepEqual(new Set(items.map(item => item.id)), initialIds);
    const representatives = [];
    for (const line of fixture.project.lines) for (const ordinal of [1, 250, 500]) {
      const id = `${line.id}-ITEM-${String(ordinal).padStart(4, '0')}`, item = items.find(value => value.id === id);
      assert.ok(item); assert.deepEqual(item.quantity, { value: { coefficient: 1, decimal_places: 0 }, unit: 'piece' });
      const completed = at_ms >= (ordinal + 9) * 2000;
      const notReleased = at_ms < (ordinal - 1) * 2000;
      if (completed || notReleased) assert.deepEqual(item.holder, { kind: 'location', id: line.id + (completed ? '-SINK' : '-SOURCE') });
      else {
        assert.equal(item.holder.kind, 'job');
        const job = run.jobs.find(value => value.id === item.holder.id);
        const station = Math.floor(at_ms / 2000) - (ordinal - 1) + 1;
        assert.equal(job.execution_ref.data.device_id, `${line.id}-D${String(station).padStart(2, '0')}`);
      }
      representatives.push({ id, holder: item.holder, quantity: item.quantity });
    }
    result.identity_observations.push({ at_ms, distinct_items: items.length, representatives });
  }
  const selected = await core.rpc({ kind: 'select_save' });
  const task = await request('save_project', { revision: core.ready.snapshot.project.revision, selected_directory_ref: selected });
  const saved = await core.rpc({ kind: 'await_task', scope: { kind: 'project' }, task_id: task.id });
  assert.equal(saved.outcome.status, 'ok'); assert.equal(saved.outcome.data.task.state, 'completed');
  const savedDirectory = new URL(`saved-${core.pid}/`, out), archive = new URL(`b60-${variant}-${attempt}-archive/`, out);
  await cp(savedDirectory, archive, { recursive: true, errorOnExist: true, force: false });
  result.saved_archive = fileURLToPath(archive); result.save_task = saved.outcome.data;
  result.actual_result = 'passed_current_Host_Engine_counts_and_identity_not_Native_performance';
} catch (error) { result.actual_result = 'failed'; result.error = String(error); result.stack = error.stack; process.exitCode = 1; }
finally {
  result.stderr = core?.stderr() ?? null;
  await writeFile(new URL('b60-' + variant + '.current.' + attempt + '.result.json', out), JSON.stringify(result, null, 2));
  await core?.close();
}
console.log(JSON.stringify({ variant, actual_result: result.actual_result, counts: result.counts, error: result.error?.slice(0, 1400), result_ref: 'b60-' + variant + '.current.' + attempt + '.result.json' }));
