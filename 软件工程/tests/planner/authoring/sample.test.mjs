import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { source, rootUrl } from './load-source.mjs';
await source('modules', 'formSchema'); await source('modules', 'model');
const { readDefinitionSampleRun, sameSampleContext } = await source('modules', 'definitionSample');
const input = JSON.parse(await readFile(new URL('scenarios/自定义模块/runtime-authoring/sample-result-input.json', rootUrl), 'utf8'));
const task = input.task, run = input.run, projectId = run.input_ref.project_id;
test('completed actual Definition Task reads one original-scope full_result and keeps DraftR1 separate from Run ProjectR2', async () => {
  const original = structuredClone(input), calls = [];
  const result = await readDefinitionSampleRun({ dispatch: async request => { calls.push(request); return { kind: 'full', data: run }; } }, projectId, task, () => true);
  assert.equal(calls.length, 1); assert.equal(calls[0].project_id, projectId); assert.deepEqual(calls[0].scope, { kind: 'definition_draft', data: task.task.scope.data.data.id });
  assert.equal(calls[0].operation, 'get_run'); assert.deepEqual(calls[0].params, { run_id: task.result.data.run_id, query: { kind: 'full_result' } });
  assert.equal(task.task.input_revision, 1); assert.equal(result.input_ref.project_revision, 2); assert.equal(result, run);
  assert.deepEqual(result.acceptance_events.map(event => event.at_ms), [70000, 110000]); assert.equal(result.resource_claims.length, 12); assert.equal(result.waits.length, 1);
  assert.deepEqual(input, original); assert.equal(result.events, run.events); assert.equal(result.final_items, run.final_items);
});
test('failed/cancelled and completed-without-Run never read or manufacture business results', async () => {
  for (const state of ['failed', 'cancelled', 'running', 'paused']) {
    const actual = structuredClone(task); actual.task.state = state; actual.failure = { code: 'ComputationFailed', message: 'original failure', object_refs: [], field_errors: [], expected_revision: null, actual_revision: null };
    const original = structuredClone(actual); let calls = 0;
    assert.equal(await readDefinitionSampleRun({ dispatch: async () => { calls++; } }, projectId, actual, () => true), null);
    assert.equal(calls, 0); assert.deepEqual(actual, original);
  }
  const noResult = { ...task, result: null }; assert.equal(await readDefinitionSampleRun({ dispatch: async () => assert.fail('no Run means no read') }, projectId, noResult, () => true), null);
});
test('read error preserves original Host failure and completed Task; retry only repeats the exact readonly get_run', async () => {
  const failure = new Error('Actual permission denied'); failure.detail = { code: 'PermissionDenied', field_errors: [{ pointer: '/scope', reason: 'actual detail' }] };
  const original = structuredClone(task), calls = []; let attempt = 0;
  const io = { dispatch: async request => { calls.push(request); if (++attempt === 1) throw failure; return { kind: 'full', data: run }; } };
  await assert.rejects(readDefinitionSampleRun(io, projectId, task, () => true), error => error === failure && error.detail.field_errors[0].reason === 'actual detail');
  assert.deepEqual(task, original); assert.equal((await readDefinitionSampleRun(io, projectId, task, () => true)).id, run.id);
  assert.ok(calls.every(request => request.operation === 'get_run' && request.scope.kind === 'definition_draft' && request.params.run_id === run.id));
});
test('summary, foreign Run, changed frozen input or non-definition Task are rejected without rewriting scope', async () => {
  for (const view of [{ kind: 'summary', data: task.result.data }, { kind: 'full', data: { ...run, id: 'foreign' } }, { kind: 'full', data: { ...run, input_ref: { ...run.input_ref, scenario_revision: 999 } } }]) {
    await assert.rejects(readDefinitionSampleRun({ dispatch: async () => view }, projectId, task, () => true), /完整冻结 Run/);
  }
  const otherScope = structuredClone(task); otherScope.task.scope = { kind: 'edit', data: { kind: 'project', data: projectId } };
  await assert.rejects(readDefinitionSampleRun({ dispatch: async () => assert.fail('must not substitute Project scope') }, projectId, otherScope, () => true), /定义草稿输入/);
});
test('asynchronous project/session/definition switch discards the old full result; a stale read never starts', async () => {
  const captured = { project_id: projectId, session_id: 'host-1', draft_id: task.task.scope.data.data.id };
  for (const field of ['project_id', 'session_id', 'draft_id']) {
    let current = { ...captured }, resolve; const pending = new Promise(done => { resolve = done; }); let calls = 0;
    const read = readDefinitionSampleRun({ dispatch: async () => { calls++; return pending; } }, projectId, task, () => sameSampleContext(captured, current));
    current = { ...captured, [field]: 'changed' }; resolve({ kind: 'full', data: run });
    assert.equal(await read, null); assert.equal(calls, 1);
    assert.equal(await readDefinitionSampleRun({ dispatch: async () => assert.fail('stale context cannot read') }, projectId, task, () => sameSampleContext(captured, current)), null);
  }
});
test('unknown/full-window fields stay unchanged, and subsequent draft revision does not relabel the frozen Run', async () => {
  const partial = structuredClone(run); partial.goal_met = null; partial.computed_until_ms = null; partial.whole_window_computed = false;
  const current = { project_id: projectId, session_id: 'host-1', draft_id: task.task.scope.data.data.id };
  const captured = { ...current }; const laterDraftRevision = 8;
  const actual = await readDefinitionSampleRun({ dispatch: async () => ({ kind: 'full', data: partial }) }, projectId, task, () => sameSampleContext(captured, current));
  assert.equal(laterDraftRevision, 8); assert.equal(task.task.input_revision, 1); assert.equal(actual.goal_met, null); assert.equal(actual.computed_until_ms, null); assert.equal(actual.whole_window_computed, false);
  assert.deepEqual(actual.input_ref, run.input_ref); assert.deepEqual(actual.acceptance_events, partial.acceptance_events);
});
