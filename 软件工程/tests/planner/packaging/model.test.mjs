import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { acknowledgmentCommand, adoptArtifact, assertPackagingContext, assertTaskTarget, cancelPackagingTask, commitProfessionalDefinitionVersion, computeJob, exportEvidence, packagingStations, readTaskEvidence, recordAcknowledgment, sameInput, saveJob, saveProduct, scopeAvailable, selectStationJob } from '../../../apps/planner/src/features/packaging/model.ts';

// Protocol-consumer fixtures only. Existing core's type_fixture evidence is preserved.
const docs = JSON.parse(await readFile(new URL('../../../packages/commands/tests/fixtures/production/professional-documents.json', import.meta.url), 'utf8'));
const source = JSON.parse(await readFile(new URL('../../../modules/professional/paper-packaging-ui/sliding-sleeve-explicit-candidate.job.json', import.meta.url), 'utf8')).data;
const target = { projectId: 'P', scope: { kind: 'project' }, revision: 17 };

test('JSON object property order does not mark an unchanged professional input dirty, while values and array order remain significant', () => {
  const edited = { id: 'JOB', subject_ref: { id: 'PACKAGE', revision: 'r1' }, panels: [{ id: 'base', thickness_mm: .625 }, { id: 'side', thickness_mm: .5 }], budget: null };
  const returned = { budget: null, panels: [{ thickness_mm: .625, id: 'base' }, { thickness_mm: .5, id: 'side' }], subject_ref: { revision: 'r1', id: 'PACKAGE' }, id: 'JOB' };
  const before = structuredClone(edited);
  assert.equal(sameInput(edited, returned), true);
  assert.equal(sameInput(edited, { ...returned, subject_ref: { id: 'PACKAGE', revision: 'r2' } }), false);
  assert.equal(sameInput(edited, { ...returned, panels: [...returned.panels].reverse() }), false);
  assert.equal(sameInput(edited, { ...returned, panels: [{ ...returned.panels[0], thickness_mm: .5 }, returned.panels[1]] }), false);
  assert.equal(sameInput(edited, { ...returned, budget: {} }), false);
  assert.deepEqual(edited, before);
});

const draft = { projectId: 'P', scope: { kind: 'definition_draft', data: 'D' }, revision: 4 };
const result = structuredClone(docs.results[0]); const artifact = structuredClone(docs.artifacts.find(value => value.id === result.artifact_id));
const job = structuredClone(docs.jobs.find(value => value.id === result.job_ref.id));
const handle = (where = target) => ({ id: 'observed-task', kind: 'packaging', state: 'queued', input_revision: where.revision,
  scope: { kind: 'edit', data: where.scope.kind === 'project' ? { kind: 'project', data: where.projectId } : { kind: 'definition_draft', data: { id: where.scope.data, revision: where.revision } } }, progress: null, result_ref: null, failure: null });
const taskState = (state, where = target) => ({ task: { ...handle(where), state }, originating_operation: 'compute_packaging', allowed_controls: ['cancel'], result: state === 'completed' ? { kind: 'packaging', data: structuredClone(result) } : null, failure: null });
function ioFixture({ where = target, latest = job, states = ['queued', 'running', 'completed'], terminalError = null, returned = result } = {}) {
  const calls = [];
  return { calls, async dispatch(request) { calls.push(structuredClone(request));
    if (request.operation === 'compute_packaging') return handle(where);
    if (request.operation === 'get_object') return request.params.object_id.kind === 'artifact' ? { kind: 'artifact', data: structuredClone(artifact) } : { kind: 'packaging_job', data: structuredClone(latest) };
    return { project_id: where.projectId, new_revision: where.revision + 1 };
  }, async awaitTask(task, publish) { assert.equal(task.id, 'observed-task'); for (const state of states) publish?.(taskState(state, where)); if (terminalError) throw terminalError; return { kind: 'packaging', data: structuredClone(returned) }; }, async pickDirectory() { return { token: 'actual-selection-fixture' }; } };
}
test('complete structured Job changes preserve canonical coordinates, product revision, panels, folds, relations and budget in one exact Scope save', async () => {
  const io = ioFixture(); const edited = structuredClone(source); const before = structuredClone(source);
  edited.revision += 1; edited.subject_ref.revision = 'r2'; edited.materials[0].basis = 'user_provided'; edited.panels[0].thickness_mm = .625;
  edited.changes = ['product_function', 'material_approved', 'part_relations_or_repeat_order'];
  await saveJob(io, target, edited); const request = io.calls[0];
  assert.equal(request.params.base_revision, 17); assert.deepEqual(request.params.commands[0], { kind: 'put_packaging_job', data: edited });
  assert.equal(request.params.commands[0].data.unit, 'millimeter'); assert.equal(request.params.commands[0].data.coordinate_frame, 'right_handed_z_up');
  assert.deepEqual(source, before); assert.ok(edited.panels.length && edited.fold_bands.length && edited.parts.length && edited.steps.length);
});
test('DraftR is the request revision and scope, independent of the Job revision and ProjectR', async () => {
  const io = ioFixture({ where: draft }); await saveJob(io, draft, source); await computeJob(io, draft, job, source.budget, () => {});
  assert.deepEqual(io.calls[0].scope, draft.scope); assert.equal(io.calls[0].project_id, 'P'); assert.equal(io.calls[0].params.base_revision, 4);
  assert.equal(io.calls[1].params.input_revision, 4); assert.equal(io.calls[1].params.packaging_job.revision, job.revision);
});
test('a real revision conflict object propagates unchanged and local input is preserved without retry', async () => {
  const conflict = Object.assign(new Error('RevisionConflict'), { detail: { code: 'RevisionConflict', expected_revision: 4, actual_revision: 5 } });
  const local = structuredClone(source); let calls = 0;
  await assert.rejects(saveJob({ dispatch: async () => { calls++; throw conflict; } }, draft, local), error => error === conflict);
  assert.deepEqual(local, source); assert.equal(calls, 1);
});
test('same observer preserves actual Task states and full three-state result; no acknowledgment or adoption is sent', async () => {
  for (const outcome of ['pass', 'fail', 'undetermined']) {
    const returned = { ...structuredClone(result), outcome }; const io = ioFixture({ returned }); const seen = [];
    const evidence = await computeJob(io, target, job, source.budget, state => seen.push(state.task.state));
    assert.deepEqual(seen, ['queued', 'running', 'completed']); assert.deepEqual(evidence.result, returned); assert.equal(evidence.result.evidence_origin, 'type_fixture');
    assert.deepEqual(io.calls.map(call => call.operation), ['compute_packaging', 'get_object', 'get_object']);
  }
});
test('failed/cancelled observations preserve terminal state and original error; no artifact is read or bound', async () => {
  for (const terminal of ['failed', 'cancelled']) {
    const failure = new Error(terminal); const io = ioFixture({ states: ['running', 'cancel_requested', terminal], terminalError: failure }); const seen = [];
    await assert.rejects(computeJob(io, target, job, source.budget, state => seen.push(state.task.state)), error => error === failure);
    assert.equal(seen.at(-1), terminal); assert.deepEqual(io.calls.map(call => call.operation), ['compute_packaging']);
  }
});
test('foreign Project/Draft/input revision admission is rejected before observation', () => {
  for (const wrong of [{ ...handle(), input_revision: 18 }, handle({ ...target, projectId: 'OTHER' }), handle(draft)]) assert.throws(() => assertTaskTarget(wrong, target), /Scope/);
  assert.throws(() => assertTaskTarget(handle({ ...draft, scope: { kind: 'definition_draft', data: 'OTHER' } }), draft), /Scope/);
});

test('every observed worker terminal refreshes the Scope once, including retained cancellation evidence, without replacing the original error or local input', async () => {
  for (const terminal of ['completed', 'failed', 'cancelled']) {
    const failure = terminal === 'completed' ? null : new Error('actual worker ' + terminal);
    const io = ioFixture({ states: ['running', terminal], terminalError: failure });
    const local = structuredClone(job); const before = structuredClone(local); const seen = [];
    const promise = computeJob(io, target, local, source.budget, state => seen.push(state.task.state), async () => {
      assert.equal(seen.at(-1), terminal);
      assert.equal(io.calls.filter(call => call.operation === 'compute_packaging').length, 1);
      if (terminal === 'completed') assert.equal(io.calls.at(-1).operation, 'get_object');
      seen.push('refreshed actual Scope');
    });
    if (failure) await assert.rejects(promise, error => error === failure); else await promise;
    assert.deepEqual(seen, ['running', terminal, 'refreshed actual Scope']);
    assert.deepEqual(local, before);
    assert.equal(io.calls.some(call => call.operation === 'apply_commands' || call.operation === 'adopt_artifact'), false);
  }
});

test('request admission and observation failures without a verified terminal do not trigger a Scope refresh', async () => {
  for (const admission of [true, false]) {
    const failure = new Error(admission ? 'request rejected' : 'observer failed');
    const io = ioFixture({ states: ['running'], terminalError: failure });
    if (admission) io.dispatch = async () => { throw failure; };
    let refreshed = false;
    await assert.rejects(computeJob(io, target, job, source.budget, () => {}, () => { refreshed = true; }), error => error === failure);
    assert.equal(refreshed, false);
  }
});

test('retained cancelled output reaches the terminal consumer with its exact input while the cancellation error remains', async () => {
  const cancelled = taskState('cancelled');
  const partial = { ...structuredClone(result), outcome: 'undetermined', budget_used: { ...result.budget_used, stopped_by: 'cancelled' } };
  cancelled.result = { kind: 'packaging', data: partial };
  const failure = new Error('actual worker cancelled');
  const io = ioFixture();
  io.awaitTask = async (_task, publish) => { publish(taskState('running')); publish(cancelled); throw failure; };
  let shown;
  await assert.rejects(computeJob(io, target, job, source.budget, () => {}, async state => {
    assert.equal(state.task.state, 'cancelled');
    assert.equal(state.task.id, 'observed-task');
    shown = await readTaskEvidence(io, target, state.result.data);
  }), error => error === failure);
  assert.equal(shown.result.outcome, 'undetermined');
  assert.equal(shown.result.budget_used.stopped_by, 'cancelled');
  assert.deepEqual(shown.input, job);
  assert.equal(io.calls.some(call => call.operation === 'apply_commands' || call.operation === 'adopt_artifact'), false);
});
test('searched execution Job is read by returned job_ref and does not overwrite the edited Job', async () => {
  const local = structuredClone(source); const before = structuredClone(local); const searched = { ...structuredClone(job), revision: job.revision + 3 };
  const searchedResult = { ...structuredClone(result), job_ref: { id: searched.id, revision: searched.revision } };
  const io = ioFixture({ latest: searched, returned: searchedResult }); const evidence = await computeJob(io, target, local, source.budget, () => {});
  assert.deepEqual(evidence.input, searched); assert.deepEqual(local, before); assert.equal(evidence.result.job_ref.revision, searched.revision);
});
test('highest Job revision cannot substitute the exact old result input; full result and actual file refs remain', async () => {
  const io = ioFixture({ latest: { ...job, revision: job.revision + 1 } }); const evidence = await readTaskEvidence(io, target, result);
  assert.equal(evidence.input, null); assert.match(evidence.inputReadIssue, /不以其他修订/); assert.deepEqual(evidence.result, result); assert.deepEqual(evidence.artifact.input_ref, artifact.input_ref);
});
test('explicit acknowledgment requires human statement and selected actual supported purposes; draft cannot acknowledge or bind', async () => {
  const evidence = { result, artifact: { ...artifact, supported_scope: ['forming'] }, input: job, inputReadIssue: null };
  assert.throws(() => acknowledgmentCommand(evidence, [], 'agree'), /认可用途/); assert.throws(() => acknowledgmentCommand(evidence, ['forming'], ''), /认可用途/);
  assert.throws(() => acknowledgmentCommand(evidence, ['manufacturing'], 'agree'), /认可用途/);
  const ack = acknowledgmentCommand(evidence, ['forming'], '明确只认可成型用途'); assert.deepEqual(ack.decision_source, { kind: 'human_statement', data: '明确只认可成型用途' });
  const io = ioFixture(); await recordAcknowledgment(io, target, ack); assert.equal(io.calls[0].params.commands[0].kind, 'record_artifact_acknowledgment');
  assert.throws(() => recordAcknowledgment(io, draft, ack), /实际 Project/); assert.throws(() => adoptArtifact(io, draft, {}), /实际 Project/); assert.equal(io.calls.length, 1);
});
test('separate binding preserves exact operation, actual acknowledgment, device, purposes and current Project base_revision', async () => {
  const io = ioFixture(); const params = { artifact_id: artifact.id, acceptance_ref: 'actual-ack', consumer_ref: { kind: 'device', id: 'PACK' }, operation_ref: { id: 'PACK-OP', version: '1.2.0' }, requested_scope: ['forming'] };
  // The public Host returns BindingCommit, without EditCommit.scope/history.
  const bindingCommit = { project_id: 'P', previous_revision: 17, new_revision: 18, binding: { id: 'BINDING', artifact_ref: artifact.id }, issues: [] };
  io.dispatch = async request => { io.calls.push(structuredClone(request)); return bindingCommit; };
  assert.equal(await adoptArtifact(io, target, params), bindingCommit);
  assert.equal('scope' in bindingCommit, false); assert.equal('history' in bindingCommit, false);
  assert.equal(io.calls[0].operation, 'adopt_artifact'); assert.deepEqual(io.calls[0].params, { ...params, base_revision: 17 });
});
test('exact draft allowed_scopes are required; operation-only availability and planned providers do not widen scope', () => {
  const catalog = { implemented_operations: [{ operation: 'compute_packaging', allowed_scopes: [{ kind: 'project' }, { kind: 'definition_draft', data: 'D' }] }], providers: [{ producer_status: 'not_implemented' }] };
  assert.equal(scopeAvailable(catalog, 'compute_packaging', draft.scope), true); assert.equal(scopeAvailable(catalog, 'compute_packaging', { kind: 'definition_draft', data: 'OTHER' }), false);
  assert.equal(scopeAvailable(catalog, 'adopt_artifact', target.scope), false);
});
test('real selected ZIP export includes explicit artifact and preserves all missing/file information; picker cancellation starts no Task', async () => {
  const io = ioFixture(); let starts = 0; io.pickDirectory = async purpose => { assert.equal(purpose, 'export_package'); return null; };
  assert.equal(await exportEvidence(io, target, { artifact }, () => {}), null); assert.equal(io.calls.length, 0);
  io.pickDirectory = async () => ({ token: 'selected' }); io.dispatch = async request => { starts++; assert.deepEqual(request.params.package_scope.artifact_ids, [artifact.id]); assert.equal(request.params.revision, 17); return handle(); };
  const exported = { package: { path: 'actual.zip', kind: 'asset' }, included_files: [{ path: 'cut.svg', kind: 'asset' }], missing: [{ code: 'actual_missing', message: 'original message' }] };
  io.awaitTask = async () => ({ kind: 'package', data: exported }); assert.deepEqual(await exportEvidence(io, target, { artifact }, () => {}), exported); assert.equal(starts, 1);
});
test('Project product editing is a real put_object; Draft cannot silently edit the factory product', async () => {
  const io = ioFixture(); const product = { id: 'PRODUCT', revision: 'r2', item_type_ref: { id: 'ITEM', revision: 'r1' }, bom_ref: { id: 'BOM', revision: 'r1' }, routing_ref: { id: 'ROUTE', revision: 'r1' }, artifact_refs: [] };
  await saveProduct(io, target, product); assert.deepEqual(io.calls[0].params.commands[0], { kind: 'put_object', data: { kind: 'product', data: product } });
  assert.throws(() => saveProduct(io, draft, product), /不能改工程产品/);
});
test('cross-project/session/scope late response and unmount cannot be published', async () => {
  for (const current of ['other-project', 'other-session', 'other-draft']) {
    let publish = 0; await assert.rejects(Promise.resolve(result).then(value => { assertPackagingContext('captured', current, true); publish++; return value; }), /停止展示旧响应/); assert.equal(publish, 0);
  }
  assert.throws(() => assertPackagingContext('same', 'same', false), /停止展示旧响应/); assert.doesNotThrow(() => assertPackagingContext('same', 'same', true));
});
test('stations come from actual precise pack Operation refs, not title or planned module names', () => {
  const pack = { id: 'OP', version: '1.0.0' }; const device = { id: 'PACK', title: 'station', module_ref: { id: 'D', version: '1.0.0' }, route_operation_refs: [pack] };
  const project = { devices: [device] }; const modules = new Map([[JSON.stringify(['OP', '1.0.0']), { definition: { payload: { category: 'operations', data: { builtin: 'pack' } } } }],
    [JSON.stringify(['D', '1.0.0']), { definition: { parameters: [], payload: { category: 'devices', data: { behavior: { selected: 'operation_ref', operation_ref: pack } } } } }]]);
  assert.equal(packagingStations(project, modules).length, 1); assert.equal(packagingStations(project, new Map()).length, 0);
  modules.get(JSON.stringify(['D', '1.0.0'])).definition.payload.data.behavior = { selected: 'inline', inline: { builtin: 'pack' } };
  assert.deepEqual(packagingStations(project, modules)[0].operation, device.module_ref); assert.equal(packagingStations(project, modules)[0].jobSelectionAvailable, false);
  assert.equal(sameInput(source, structuredClone(source)), true);
});
test('explicit station Job selection uses the actual two declared parameters atomically before separate binding', async () => {
  const io = ioFixture(); await selectStationJob(io, target, 'PACK', result.job_ref); const request = io.calls[0];
  assert.equal(request.operation, 'apply_commands'); assert.equal(request.params.base_revision, 17); assert.equal(request.params.commands.length, 2);
  assert.equal(request.params.commands[0].data.parameter_id, 'packaging_job_id'); assert.deepEqual(request.params.commands[0].data.value, { kind: 'string', value: result.job_ref.id });
  assert.equal(request.params.commands[1].data.parameter_id, 'packaging_job_revision'); assert.deepEqual(request.params.commands[1].data.value, { kind: 'integer', value: result.job_ref.revision });
  assert.ok(request.params.commands.every(command => command.data.basis === 'user_provided')); assert.throws(() => selectStationJob(io, draft, 'PACK', result.job_ref), /实际 Project/);
});
test('cancel retains actual Task identity and reports actual terminal-won result without changing its input revision', async () => {
  const io = ioFixture(); const cancellation = { task: { ...handle(), state: 'completed' }, action_accepted: false, terminal_won: true, issues: [] };
  io.dispatch = async request => { assert.equal(request.operation, 'cancel_task'); assert.deepEqual(request.params, { task_id: 'observed-task', action: 'cancel' }); return cancellation; };
  assert.equal(await cancelPackagingTask(io, { ...target, revision: 18 }, handle()), cancellation);
  assert.throws(() => cancelPackagingTask(io, target, handle({ ...target, projectId: 'OTHER' })), /Scope/);
});
test('Draft export selects the actual committed exact module ref; dirty draft is refused before picker or Task', async () => {
  const io = ioFixture({ where: draft }); let picked = 0; let dirty = true; const exported = { package: { path: 'module.zip' }, included_files: [], missing: [] };
  io.pickDirectory = async () => { picked++; return { token: 'selection' }; };
  io.dispatch = async request => {
    if (request.operation === 'get_context') return { definition: { draft_id: 'D', revision: 4, definition_ref: { id: 'D', version: '2.0.0' }, issues: dirty ? [{ code: 'definition.unsaved_changes' }] : [] } };
    assert.equal(request.operation, 'export_package'); assert.deepEqual(request.params.package_scope.module_refs, [{ id: 'D', version: '2.0.0' }]); assert.equal(request.params.package_scope.purpose, 'private_module'); return handle(draft);
  };
  io.awaitTask = async () => ({ kind: 'package', data: exported });
  await assert.rejects(exportEvidence(io, draft, { artifact }, () => {}), /先明确提交定义版本/); assert.equal(picked, 0);
  dirty = false; assert.equal(await exportEvidence(io, draft, { artifact }, () => {}), exported); assert.equal(picked, 1);
});
test('Definition commit is an explicit separate command with captured DraftR and chosen version, never automatic export acknowledgment', async () => {
  const io = ioFixture({ where: draft }); await commitProfessionalDefinitionVersion(io, draft, '2.0.0');
  assert.deepEqual(io.calls[0].params, { base_revision: 4, commands: [{ kind: 'commit_definition_version', data: { version: '2.0.0' } }] }); assert.throws(() => commitProfessionalDefinitionVersion(io, target, '2.0.0'), /DefinitionDraft/);
});
