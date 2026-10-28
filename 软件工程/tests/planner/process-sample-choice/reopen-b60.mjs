// Manual current-Host reception of the preserved Native18 B60 work copies.
// Run after preparing native-inputs/b60 and build-current-host.py; no Native UI.
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { openCore } from '../node-inspector/core.mjs';
const root = new URL('../../../', import.meta.url);
const folder = new URL('.work/full-closeout-20261021/native-inputs/b60/', root);
assert.ok(process.env.SF_TEST_HOST_CORE, 'Set SF_TEST_HOST_CORE to the current Native-linked test consumer');
const json = async path => JSON.parse(await readFile(path, 'utf8'));
const prep = await json(new URL('preparation.json', folder));
const results = [];
for (const input of prep.inputs) {
  const core = await openCore('scenarios/图片时间轴/runtime/phase.input.json');
  try {
    const directory = fileURLToPath(new URL(input.test_copy + '/', root));
    const opened = await core.rpc({ kind: 'open_saved', directory });
    assert.equal(opened.project_id, input.project_id);
    const frozenInput = await json(new URL(`tests/planner/p201-whole/fixtures/b60/cut-r2/${input.variant}.input.json`, root));
    // The shared Domain contract owns absent/null/empty serialization. Compare
    // model values, not an ad-hoc JS removal of optional fields in either input.
    const expected = await core.rpc({ kind: 'canonical_input', input: frozenInput });
    async function request(operation, params) {
      const reply = await core.rpc({ kind: 'dispatch', request: { api_version: '0.1', request_id: crypto.randomUUID(),
        project_id: input.project_id, scope: { kind: 'project' }, operation, params } });
      assert.equal(reply.outcome.status, 'ok', JSON.stringify(reply)); return reply.outcome.data;
    }
    const active = await request('get_object', { object_id: { kind: 'project', id: input.project_id }, source: { kind: 'draft' }, node_index: null });
    assert.equal(active.kind, 'project'); assert.deepEqual(active.data, expected.project);
    const frozen = await request('get_object', { object_id: { kind: 'project', id: input.project_id },
      source: { kind: 'run_snapshot', data: { run_id: input.original_run_id, at_ms: 0 } }, node_index: null });
    assert.equal(frozen.kind, 'project'); assert.deepEqual(frozen.data, expected.project);
    const scenario = await request('get_object', { object_id: { kind: 'scenario', id: expected.scenario.id }, source: { kind: 'draft' }, node_index: null });
    assert.equal(scenario.kind, 'scenario'); assert.deepEqual(scenario.data, expected.scenario);
    for (const reference of expected.project.module_refs) {
      const module = await request('get_object', { object_id: { kind: 'module', id: reference.id },
        source: { kind: 'run_module_version', data: { run_id: input.original_run_id, at_ms: 0, module_ref: reference } }, node_index: null });
      assert.equal(module.kind, 'module');
      assert.deepEqual(module.data.definition, expected.definitions.find(d => d.id === reference.id && d.version === reference.version));
    }
    const seeds = new Map(expected.scenario.initial_conditions.items.map(item => [item.id, item]));
    assert.equal(seeds.size, 3000);
    const historyPoints = [];
    for (const at of [0, 600000, 1200000]) {
      const items = []; let cursor = null, pages = 0;
      do {
        const value = await request('get_run', { run_id: input.original_run_id,
          query: { kind: 'objects_at', data: { at_ms: at, objects: [], page: { after_index: cursor, limit: 200 } } } });
        assert.equal(value.kind, 'objects'); assert.equal(value.data.run_id, input.original_run_id); assert.equal(value.data.at_ms, at);
        if (value.data.next_index !== null) assert.ok(value.data.next_index > (cursor ?? -1), 'History page cursor must advance');
        items.push(...value.data.objects.filter(v => v.kind === 'item').map(v => v.data)); cursor = value.data.next_index; pages++;
      } while (cursor !== null);
      assert.deepEqual(items.map(v => v.id).sort(), [...seeds.keys()].sort());
      for (const item of items) {
        assert.deepEqual(item.quantity, seeds.get(item.id).quantity);
      }
      // ObjectsAt is right-continuous: jobs can already hold inputs at0ms.
      // Initial seeds are not a holder oracle. This reception checks all IDs
      // and quantities; independent holder/phase/visible checks remain separate.
      historyPoints.push({ at_ms: at, pages, identities: items.length, exact_initial_IDs_and_quantities: true });
    }
    const result = { status: 'pass', variant: input.variant, pid: core.pid, project_id: input.project_id,
      current_Host_canonical_active_Project_exact: true, current_Host_canonical_frozen_Run_Project_exact: true,
      Scenario_and_exact_frozen_definitions_match: true, original_run_id: input.original_run_id,
      history_points: historyPoints, Native_UI: false, formal_performance: false, records: core.records };
    await writeFile(new URL(input.variant + '-current-host-reopen.json', folder), JSON.stringify(result, null, 2) + '\n');
    results.push({ ...result, records: undefined });
  } catch (error) {
    await writeFile(new URL(input.variant + '-current-host-reopen-failure.json', folder), JSON.stringify({ status: 'failed', error: String(error), stack: error.stack, stderr: core.stderr(), records: core.records }, null, 2));
    throw error;
  } finally { await core.close(); }
}
await writeFile(new URL('current-host-reopen.json', folder), JSON.stringify({ status: 'pass', Native_UI: false, results }, null, 2) + '\n');
console.log(JSON.stringify({ status: 'pass', results }));
