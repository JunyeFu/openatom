import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { openCore } from '../node-inspector/core.mjs';
import { root, authoring, sampleReceiver } from '../node-static/guard/load-source.mjs';
const output = new URL('.work/full-closeout-20261021/process-sample-choice/', root);
await mkdir(output, { recursive: true });
assert.ok(process.env.SF_TEST_HOST_CORE, 'Build current-host-core with build-current-host.py and set SF_TEST_HOST_CORE before running current Host acceptance');
async function setup() {
  const path = 'scenarios/图片时间轴/runtime/fixed.input.json';
  const input = JSON.parse(await readFile(new URL(path, root), 'utf8')), core = await openCore(path);
  const project = core.ready.snapshot.project, pid = project.id;
  const io = { async dispatch(request) {
    const response = await core.rpc({ kind: 'dispatch', request });
    if (response.outcome.status !== 'ok') throw new Error(response.outcome.data.message);
    return response.outcome.data;
  } };
  const request = (operation, params, scope = { kind: 'project' }) => io.dispatch({ api_version: '0.1', request_id: crypto.randomUUID(), project_id: pid, scope, operation, params });
  const definition = input.definitions.find(v => v.id === 'p303.owned.manual-assembly');
  const created = await request('apply_commands', { base_revision: project.revision, commands: [{ kind: 'create_definition_draft', data: definition }] });
  const scope = { kind: 'definition_draft', data: created.scope.data.id }, draft = await authoring.readDraft(io, pid, scope.data);
  const task = await request('run_scenario', { scenario_id: input.scenario.id, input_revision: 1, observation_window: input.scenario.window,
    intent: { kind: 'definition_cycle', data: { definition_revision: draft.summary.revision, test_input: input.scenario } } }, scope);
  const done = (await core.rpc({ kind: 'await_task', scope, task_id: task.id })).outcome.data;
  assert.equal(done.task.state, 'completed');
  const run = (await request('get_run', { run_id: done.result.data.run_id, query: { kind: 'full_result' } }, scope)).data;
  const active = { current: { id: 'fixed-two-job-process', scope, draft, selection: { kind: 'definition', module_ref: { id: definition.id, version: definition.version } } } };
  const workspace = { current: { project, context: core.ready.context } }, changes = [], choices = [];
  const receive = await sampleReceiver({ deviceProcessRef: active, workspaceRef: workspace, authoringIO: io,
    authoringRequest: authoring.authoringRequest, setDeviceProcess: value => changes.push(value),
    setProcessSampleChoice: value => choices.push(value) });
  return { core, request, scope, draft, active, receive, changes, choices, run };
}

test('actual two-job fixed sample requires a deliberate Job choice and opens the chosen second Job without changing the Run', async () => {
  const s = await setup();
  try {
    const original = structuredClone(s.run);
    assert.equal(original.jobs.length, 2, 'The original fixed stock case has two real jobs');
    await s.receive(s.run);
    assert.equal(s.changes.length, 0, 'No first-job default navigation');
    const choice = s.choices.at(-1);
    assert.deepEqual(choice.jobs.map(v => v.id), original.jobs.map(v => v.id));
    await choice.select(original.jobs[1].id);
    assert.equal(s.changes.length, 1);
    assert.deepEqual(s.changes[0].selection, { kind: 'actual_run', run_id: original.id, job_id: original.jobs[1].id, at_ms: original.computed_until_ms });
    assert.equal(s.choices.at(-1), null, 'Successful explicit choice closes the selector');
    const after = (await s.request('get_run', { run_id: original.id, query: { kind: 'full_result' } }, s.scope)).data;
    assert.deepEqual(after, original);
    await writeFile(new URL('actual-two-job-choice.json', output), JSON.stringify({ selected: s.changes[0].selection, original_Run: original, after_Run: after, records: s.core.records, ordinaryNative: false }, null, 2));
  } finally { await s.core.close(); }
});

test('delayed choice rejects a nonmatching Job and real Host Draft revision change, retaining the actual Run', async () => {
  const s = await setup();
  try {
    const before = structuredClone(s.run); await s.receive(s.run); const choice = s.choices.at(-1);
    await assert.rejects(choice.select('not-in-this-sample'), /作业/); assert.equal(s.changes.length, 0);
    await s.request('apply_commands', { base_revision: s.draft.summary.revision, commands: [{ kind: 'put_definition', data: { ...s.draft.definition, title: 'Actual revised draft while choice is open' } }] }, s.scope);
    await assert.rejects(choice.select(before.jobs[0].id), /修订已变化/); assert.equal(s.changes.length, 0);
    assert.deepEqual((await s.request('get_run', { run_id: before.id, query: { kind: 'full_result' } }, s.scope)).data, before);
  } finally { await s.core.close(); }
});

test('delayed explicit selection cannot navigate after its original process entry closes', async () => {
  const s = await setup();
  try {
    await s.receive(s.run); const choice = s.choices.at(-1); s.active.current = null;
    await assert.rejects(choice.select(s.run.jobs[1].id), /来源已切换/); assert.equal(s.changes.length, 0);
  } finally { await s.core.close(); }
});
