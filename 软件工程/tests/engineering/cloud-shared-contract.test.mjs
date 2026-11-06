import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { rolldown } from 'rolldown';

// Exercise the real shared adapter. Only the Native transport is substituted;
// these checks do not claim a desktop, cloud database, or production acceptance.
const entry = '\0cloud-contract-entry';
const transport = '\0cloud-contract-transport';
const key = '__sfCloudContractTransport';
const source = fileURLToPath(new URL('../../apps/planner/src/services/cloudProduction.ts', import.meta.url));
const bundle = await rolldown({
  input: entry, platform: 'node',
  plugins: [{
    name: 'cloud-contract-host',
    resolveId(id, importer) {
      if (id === entry || id === transport) return id;
      if (id === './projectHost' && importer === source) return transport;
      if (id === '@tauri-apps/api/core') return transport;
    },
    load(id) {
      if (id === entry) return `export * from ${JSON.stringify(source.replaceAll('\\', '/'))};`;
      if (id === transport) return `
        export const nativeHostAvailable = () => true;
        export class HostFailure extends Error {}
        export const dispatch = request => globalThis.${key}(request);
        export const invoke = () => { throw new Error('Unexpected Native invocation'); };`;
    },
  }],
});
after(async () => { await bundle.close(); delete globalThis[key]; });
const output = await bundle.generate({ format: 'esm' });
const code = output.output.find(item => item.type === 'chunk').code;
const adapter = await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}`);
const wire = JSON.parse(await readFile(new URL('../../packages/domain/tests/fixtures/cloud_wire_cases.json', import.meta.url), 'utf8'));
const context = { projectId: 'project-A', projectRevision: 4, hostContextId: 'host-A' };
const identity = { project_id: context.projectId, host_context_session_id: context.hostContextId,
  session_id: 'session-A', execution_epoch_id: 'epoch-A' };
const params = { execution_contract_version: adapter.EXECUTION_CONTRACT, base_revision: 4,
  scenario_id: 'scenario-A', scenario_revision: 1, candidate_bindings: [],
  policy: wire.find(row => row.definition === 'ExecutionPolicy').value };
const task = kind => ({ id: 'task-A', kind, state: 'queued', input_revision: 4,
  scope: { kind: 'edit', data: { kind: 'project', data: context.projectId } },
  progress: null, result_ref: null, failure: null });
function host(reply) {
  const requests = [];
  globalThis[key] = async request => { requests.push(request); return reply(request); };
  return requests;
}
function completed(request, handle, kind, data) {
  return { task: { ...handle, state: 'completed' }, originating_operation: request.operation,
    originating_request_id: request.request_id, allowed_controls: [], failure: null,
    result: { kind, data } };
}

test('preparing an execution returns prepared without sending run or control', async () => {
  let original;
  const handle = task('execution_preparation');
  const calls = host(request => {
    if (request.operation === 'start_execution_session') { original = request; return handle; }
    assert.equal(request.operation, 'get_task');
    return completed(original, handle, 'execution_session', { state: 'prepared', identity });
  });
  const result = await adapter.prepareExecution(context, params);
  assert.equal(result.state, 'prepared');
  assert.deepEqual(calls.map(r => r.operation), ['start_execution_session', 'get_task']);
  assert.deepEqual(calls[0].params, params);
});

test('an ambiguous preparation is recovered by its original request without resubmitting', async () => {
  let original, record;
  const handle = task('execution_preparation');
  const calls = host(request => {
    if (request.operation === 'start_execution_session') { original = request; throw new Error('transport lost'); }
    if (request.operation === 'read_execution_view') {
      assert.equal(request.params.selection.originating_request_id, original.request_id);
      return { kind: 'request', data: { host_context_session_id: context.hostContextId,
        originating_request_id: original.request_id, operation: original.operation, state: 'registered', task: handle } };
    }
    assert.equal(request.operation, 'get_task');
    return completed(original, handle, 'execution_session', { state: 'prepared', identity });
  });
  await assert.rejects(adapter.prepareExecution(context, params, { onTask: r => { record = r; } }), /transport lost/);
  assert.equal(record.status, 'requesting');
  const result = await adapter.recoverRegisteredCloudTask(record);
  assert.equal(result.kind, 'execution_session');
  assert.deepEqual(calls.map(r => r.operation), ['start_execution_session', 'read_execution_view', 'get_task']);
});

test('leaving a cloud read cancels only its registered read task', async () => {
  const abort = new AbortController();
  let original;
  const handle = task('cloud_read');
  const calls = host(request => {
    if (request.operation === 'read_cloud_production') { original = request; return handle; }
    if (request.operation === 'cancel_task') return {};
    assert.equal(request.operation, 'get_task');
    abort.abort();
    return { task: { ...handle, state: 'running' }, originating_operation: original.operation,
      originating_request_id: original.request_id, allowed_controls: ['cancel'], result: null, failure: null };
  });
  await assert.rejects(adapter.readCloud(context, { kind: 'dataset_catalog' }, { signal: abort.signal }), { name: 'AbortError' });
  assert.deepEqual(calls.map(r => r.operation), ['read_cloud_production', 'get_task', 'get_task', 'cancel_task']);
  assert.deepEqual(calls.at(-1).params, { task_id: handle.id, action: 'cancel' });
});

test('a registered task from another project is rejected before polling', async () => {
  const handle = task('cloud_read');
  const calls = host(() => ({ ...handle, scope: { kind: 'edit', data: { kind: 'project', data: 'another-project' } } }));
  await assert.rejects(adapter.readCloud(context, { kind: 'dataset_catalog' }), { code: 'task_mismatch' });
  assert.equal(calls.length, 1);
});

test('history cannot send a production control request', async () => {
  const calls = host(() => assert.fail('History attempted a Host request'));
  const envelope = { snapshot: { identity, execution_revision: 8 }, delivery: { read_origin: 'host' },
    access: { ownership: 'current_host_owner', allowed_session_controls: ['run'], device_acceptance_control_ids: [] } };
  await assert.rejects(adapter.controlExecution(context, envelope, { kind: 'run' }, false), { code: 'read_only' });
  assert.equal(calls.length, 0);
});

const query = { dataset: 'telemetry', contract_version: 'sf.execution/2.0-draft11', cloud_project_id: 'cloud-A',
  dataset_id: 'load-A', device_ids: ['device-A'], aggregation: 'rows',
  received_from: '2026-11-06T00:00:00Z', received_until: '2026-11-06T01:00:00Z', page: { limit: 2, cursor: null } };
function page(position, frontier = 20) {
  return { dataset: 'telemetry', contract_version: query.contract_version, dataset_id: query.dataset_id,
    selected_epoch: null, selected_execution_revision: null, selected_visible_frontier: frontier,
    selected_device_ids: ['device-A'], next_cursor: null, data_quality_issues: [],
    rows: [{ kind: 'generated_load', ingress_id: position, dataset_id: query.dataset_id,
      cloud_received_at: query.received_from, event: { device_ref: 'device-A', device_sequence: position,
        source_mode: 'generated_load', observed_at: query.received_from, metric: 'status_sample', value: 1 } }] };
}
test('pagination rejects changed frontiers and repeated rows while retaining the valid selection', () => {
  const valid = adapter.validateQueryPage(query, page(1));
  const original = structuredClone(valid);
  assert.throws(() => adapter.validateQueryPage(query, page(2, 21), valid), { code: 'frontier_mismatch' });
  assert.throws(() => adapter.validateQueryPage(query, page(1), valid), { code: 'page_not_progressing' });
  assert.deepEqual(valid, original);
  assert.equal(adapter.validateQueryPage(query, page(2), valid).lastPosition, 2);
});

test('execution pagination cannot substitute an epoch or frozen revision', () => {
  const execution = { dataset: 'execution', contract_version: adapter.EXECUTION_CONTRACT, identity,
    selection: { kind: 'revision', execution_revision: 7 }, host_from_ms: 0, host_until_ms: 1000,
    aggregation: 'rows', page: { limit: 2, cursor: null } };
  const result = { dataset: 'execution', contract_version: adapter.EXECUTION_CONTRACT,
    selected_epoch: 'epoch-B', selected_execution_revision: 7, rows: [], next_cursor: null, data_quality_issues: [] };
  assert.throws(() => adapter.validateQueryPage(execution, result), { code: 'query_revision' });
  assert.throws(() => adapter.validateQueryPage(execution, { ...result, selected_epoch: 'epoch-A', selected_execution_revision: 8 }), { code: 'query_revision' });
});

test('a report cannot use a data-export directory purpose', async () => {
  const calls = host(() => assert.fail('Wrong directory purpose reached Host'));
  await assert.rejects(adapter.exportExecutionReport(context, { selected_destination_ref: { purpose: 'export_cloud_data' } }), { code: 'directory_purpose' });
  assert.equal(calls.length, 0);
});
