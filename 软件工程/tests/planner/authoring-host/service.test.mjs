import test from 'node:test';
import assert from 'node:assert/strict';
import { service, host } from './load-source.mjs';

const { createModuleAuthoringIO, createAuthoringSamples, unavailableAuthoringTemplates,
  createAuthoringTemplates, TemplateCompositionSaveFailure, readAuthoringTemplatePreview } = service;
const copy = value => structuredClone(value);
const definition = { id: 'fixture.rule', version: '0.1.0', payload: { category: 'operations', data: {} } };
const draft = { summary: { draft_id: 'opaque-host:draft:7', revision: 9, definition_ref: { id: definition.id, version: definition.version }, issues: [] }, definition, history: null };
const scope = { kind: 'definition_draft', data: draft.summary.draft_id };
const taskScope = { kind: 'edit', data: { kind: 'definition_draft', data: { id: draft.summary.draft_id, revision: 9 } } };
const task = { id: 'actual-task-reference-from-protocol-fixture', scope: taskScope, kind: 'simulation', state: 'queued', input_revision: 9, progress: null, result_ref: null, failure: null };
const scenario = { id: 'EXPLICIT', revision: 12, project_revision: 3, title: 'Unmodified explicit input', window: { start_ms: 0, end_ms: 240000 }, initial_conditions: { items: [] }, scope: [], orders: [] };
const output = { kind: 'run', data: { run_id: 'fixed-fixture-not-runtime-proof', computation_status: 'completed', accepted: null, issues: [] } };
const capability = draftId => ({ operation: 'run_scenario', allowed_scopes: [{ kind: 'project' }, ...(draftId ? [{ kind: 'definition_draft', data: draftId }] : [])] });
const catalog = draftId => ({ implemented_operations: [capability(draftId)] });
const snapshot = () => ({ project: { id: 'P1', revision: 3 }, context: { host: { session_id: 'desktop-main-ui' }, project: { id: 'P1', revision: 3 }, authorized_operations: ['run_scenario'] }, capabilities: catalog(draft.summary.draft_id) });
const state = (status, failure = null) => ({ task: { ...copy(task), state: status }, originating_operation: 'run_scenario', allowed_controls: [], result: status === 'completed' ? output : null, failure });
const apiError = (code, message = code) => ({ code, message, object_refs: [], field_errors: [], expected_revision: null, actual_revision: null });
const request = (operation, params, selectedScope = scope) => ({ api_version: '0.1', request_id: 'fixed-wire-request', project_id: selectedScope.kind === 'workspace' ? null : 'P1', scope: selectedScope, operation, params });

function native(handler) {
  const calls = [];
  globalThis.window = { setTimeout, __TAURI_INTERNALS__: { async invoke(command, args) {
    calls.push({ command, args });
    return handler(command, args, calls);
  } } };
  return calls;
}
function protocol(data) { return { outcome: { status: 'ok', data } }; }
function sampleTransport(states, onRequest = () => {}) {
  return native((command, args) => {
    assert.equal(command, 'project_api');
    onRequest(args.request);
    if (args.request.operation === 'describe_capabilities') return protocol(catalog(draft.summary.draft_id));
    if (args.request.operation === 'run_scenario') return protocol(task);
    if (args.request.operation === 'get_task') return protocol(states.shift());
    throw new Error(`Unexpected actual SDK operation ${args.request.operation}`);
  });
}

test('native package purposes select ZIP-file commands, other purposes select directories; cancellation is null and other failures remain typed', async () => {
  let current = snapshot();
  const io = createModuleAuthoringIO(() => current);
  const calls = native((_command, args) => ({ selection_id: `native-${args.purpose}`, purpose: args.purpose }));
  for (const purpose of ['import_package', 'export_package', 'save_project', 'export_evaluation']) {
    const value = await io.pickDirectory(purpose);
    assert.equal(value.purpose, purpose);
  }
  assert.deepEqual(calls.map(call => call.command), ['project_select_file', 'project_select_file', 'project_select_directory', 'project_select_directory']);
  const cancelled = apiError('Cancelled', 'actual native cancelled fixture');
  native(() => { throw cancelled; });
  assert.equal(await io.pickDirectory('import_package'), null);
  assert.equal(await io.pickDirectory('export_package'), null);
  assert.equal(await io.pickDirectory('save_project'), null);
  const denied = apiError('PermissionDenied', 'original selection denial');
  native(() => { throw denied; });
  await assert.rejects(io.pickDirectory('export_package'), error => error instanceof host.HostFailure && error.detail === denied);
});

test('existing awaitTask observes the original Definition Task through actual ProjectId and returns its original output', async () => {
  const current = snapshot();
  const io = createModuleAuthoringIO(() => current);
  const queued = state('queued');
  const complete = state('completed');
  // JSON object key order is not Scope identity.
  complete.task.scope = { data: { data: { revision: 9, id: draft.summary.draft_id }, kind: 'definition_draft' }, kind: 'edit' };
  const states = [queued, complete];
  const calls = sampleTransport(states);
  const seen = [];
  const result = await io.awaitTask(task, value => seen.push(value));
  assert.equal(result, output);
  assert.deepEqual(seen, [queued, complete]);
  assert.equal(calls.length, 2);
  for (const call of calls) {
    assert.equal(call.args.request.project_id, 'P1');
    assert.deepEqual(call.args.request.scope, scope);
    assert.equal(call.args.request.params.task_id, task.id);
  }
  assert.equal(task.scope.data.data.revision, 9);
});

test('sample readiness consumes exact advertised Draft Scope, not an operation-only flag or a foreign draft', () => {
  let current = snapshot();
  current.capabilities = catalog(null);
  assert.equal(createAuthoringSamples(() => current, draft).status, 'unavailable');
  current.capabilities = catalog('another-draft');
  assert.equal(createAuthoringSamples(() => current, draft).status, 'unavailable');
  current.capabilities = catalog(draft.summary.draft_id);
  assert.equal(createAuthoringSamples(() => current, draft).status, 'available');
  assert.equal(createAuthoringSamples(() => null, draft).status, 'unavailable');
  assert.equal(createAuthoringSamples(() => current, null).status, 'unavailable');
});

test('sample request freshly checks capabilities, preserves explicit Scenario and both independent revision spaces, then returns original terminal State', async () => {
  const current = snapshot();
  const io = createModuleAuthoringIO(() => current);
  const provider = createAuthoringSamples(() => current, draft, io);
  const before = JSON.stringify({ current, draft, scenario });
  const complete = state('completed');
  const calls = sampleTransport([state('queued'), complete]);
  const actual = await provider.run(draft, scenario);
  assert.equal(actual, complete);
  assert.equal(calls[0].args.request.operation, 'describe_capabilities');
  const sent = calls.find(call => call.args.request.operation === 'run_scenario').args.request;
  assert.deepEqual(sent.scope, scope);
  assert.equal(sent.project_id, 'P1');
  assert.equal(sent.params.input_revision, 9);
  assert.equal(sent.params.intent.data.definition_revision, 9);
  assert.equal(sent.params.intent.data.test_input, scenario);
  assert.equal(sent.params.intent.data.test_input.project_revision, 3);
  assert.deepEqual(sent.params.observation_window, scenario.window);
  assert.equal(JSON.stringify({ current, draft, scenario }), before);
});

test('freshly removed Scope prevents admission, and real API revision errors retain exact failure details', async () => {
  const current = snapshot();
  const io = createModuleAuthoringIO(() => current);
  const provider = createAuthoringSamples(() => current, draft, io);
  let calls = native((command, args) => { assert.equal(args.request.operation, 'describe_capabilities'); return protocol(catalog(null)); });
  await assert.rejects(provider.run(draft, scenario), /精确 DefinitionDraft Scope/);
  assert.equal(calls.length, 1);
  const conflict = { ...apiError('RevisionConflict', 'exact actual Host draft conflict fixture'), expected_revision: 9, actual_revision: 10, field_errors: [{ pointer: '/params/input_revision', reason: 'current exact draft changed' }] };
  calls = native((_command, args) => args.request.operation === 'describe_capabilities' ? protocol(catalog(draft.summary.draft_id)) : { outcome: { status: 'error', data: conflict } });
  await assert.rejects(provider.run(draft, scenario), error => error instanceof host.HostFailure && error.detail === conflict);
  assert.equal(calls.length, 2);
  assert.equal(scenario.project_revision, 3);
  assert.equal(draft.summary.revision, 9);
});

test('actual failed and cancelled terminal TaskStates remain failed/cancelled, retaining original failure and no synthetic result', async () => {
  const current = snapshot();
  for (const status of ['failed', 'cancelled']) {
    const io = createModuleAuthoringIO(() => current);
    const provider = createAuthoringSamples(() => current, draft, io);
    const failure = status === 'failed' ? apiError('ComputationFailed', 'original terminal failure') : null;
    const terminal = state(status, failure);
    sampleTransport([terminal]);
    const actual = await provider.run(draft, scenario);
    assert.equal(actual, terminal);
    assert.equal(actual.task.state, status);
    assert.equal(actual.failure, failure);
    assert.equal(actual.result, null);
  }
});

test('session/project changes prevent publishing old requests, native selections or task results without cancelling', async () => {
  for (const field of ['session', 'project']) {
    let current = snapshot();
    const io = createModuleAuthoringIO(() => current);
    const calls = native(() => { throw new Error('No dispatch expected'); });
    current = field === 'session' ? { ...current, context: { ...current.context, host: { session_id: 'another-session' } } } : { ...current, project: { id: 'P2', revision: 1 } };
    await assert.rejects(io.pickDirectory('import_package'), /已切换/);
    await assert.rejects(io.dispatch(request('get_context', { selection: [] })), /已切换/);
    assert.equal(calls.length, 0);
  }
  let current = snapshot();
  let io = createModuleAuthoringIO(() => current);
  native((_command, args) => { current = { ...current, project: { id: 'P2', revision: 1 } }; return { selection_id: 'old-project-token', purpose: args.purpose }; });
  await assert.rejects(io.pickDirectory('export_package'), /已切换/);
  current = snapshot(); io = createModuleAuthoringIO(() => current);
  const provider = createAuthoringSamples(() => current, draft, io);
  const calls = sampleTransport([state('completed')], req => { if (req.operation === 'get_task') current = { ...current, project: { id: 'P2', revision: 1 } }; });
  await assert.rejects(provider.run(draft, scenario), /已切换/);
  assert.ok(calls.every(call => call.args.request.operation !== 'cancel_task'));
});

test('wrong Context, foreign task and mismatched returned identity/scope/revision stop at their actual boundary', async () => {
  const current = snapshot();
  const io = createModuleAuthoringIO(() => current);
  native(() => protocol({ ...current.context, host: { session_id: 'wrong-context' } }));
  await assert.rejects(io.dispatch(request('get_context', { selection: [] })), /Host 返回的会话或工程已切换/);
  native(() => { throw new Error('No foreign task dispatch'); });
  await assert.rejects(io.awaitTask({ ...task, scope: { kind: 'edit', data: { kind: 'project', data: 'P2' } } }), /不属于/);
  for (const change of [value => { value.task.id = 'different-task'; }, value => { value.task.scope.data.data.revision = 10; }, value => { value.task.input_revision = 10; }]) {
    const actual = state('completed'); change(actual);
    sampleTransport([actual]);
    await assert.rejects(io.awaitTask(task), /不同任务、Scope 或输入修订/);
  }
});

test('five category previews and two deferred task kinds are shown unavailable, composition has no invented command', () => {
  const current = snapshot(); current.capabilities = catalog(null);
  for (const category of ['items', 'resources', 'templates', 'professional', 'media']) {
    const candidate = { ...draft, definition: { ...definition, payload: { category, data: {} } } };
    const provider = createAuthoringSamples(() => current, candidate);
    assert.equal(provider.status, 'unavailable'); assert.ok(provider.reason.length > 10);
  }
  for (const kind of ['deferred_maintenance', 'deferred_construction']) {
    const candidate = { ...draft, definition: { ...definition, payload: { category: 'tasks', data: { kind } } } };
    assert.match(createAuthoringSamples(() => current, candidate).reason, /执行后置/);
  }
  const templates = unavailableAuthoringTemplates();
  assert.equal(templates.status, 'unavailable'); assert.equal(templates.save, undefined);
});

test('a completed task without its original output keeps the existing SDK error instead of manufacturing sample completion', async () => {
  const current = snapshot();
  const io = createModuleAuthoringIO(() => current);
  const provider = createAuthoringSamples(() => current, draft, io);
  const missing = state('completed'); missing.result = null;
  sampleTransport([missing]);
  await assert.rejects(provider.run(draft, scenario), /尚无可读取结果/);
});

const templateDefinition = { ...definition, id: 'actual.template', payload: { category: 'templates', data: { project: 'client-placeholder.json', generation_profile: null } } };
const templateDraft = { ...draft, definition: templateDefinition, summary: { ...draft.summary, definition_ref: { id: templateDefinition.id, version: templateDefinition.version } } };
const subset = { layout_scope: 'region', halls: [], lines: [], devices: [], connections: [], module_refs: [], resource_requirements: [], item_types: [], role_requirements: [], suggested_role_bindings: { item_types: {}, resources: {}, targets: {}, ports: {} }, locations: [], seed: null };
const templateCatalog = available => ({ ...catalog(null), available_command_kinds: available ? ['create_definition_draft', 'put_template_composition'] : [] });
const templateSnapshot = () => ({ ...snapshot(), capabilities: templateCatalog(true) });
function commit(id, before, after) {
  return { project_id: 'P1', scope: { kind: 'definition_draft', data: { id, revision: after } }, previous_revision: before, new_revision: after, changed_objects: [{ kind: 'definition_draft', id }], issues: [], history: { can_undo: before !== null, can_redo: false, boundary_reason: null } };
}
function draftReplies(current, id, revision, data) {
  const context = { ...current.context, definition: { draft_id: id, revision, definition_ref: { id: data.id, version: data.version }, issues: [] } };
  return [['get_context', context], ['get_object', { kind: 'definition', data }], ['get_context', context]];
}
function sequence(replies) {
  return native((command, args) => {
    assert.equal(command, 'project_api');
    const next = replies.shift(); assert.ok(next, `Unexpected ${args.request.operation}`);
    assert.equal(args.request.operation, next[0]);
    return next[1]?.outcome ? next[1] : protocol(next[1]);
  });
}

test('existing template Draft atomically saves real carrier with separate revisions and reads actual Host path/history', async () => {
  const current = templateSnapshot();
  const io = createModuleAuthoringIO(() => current);
  const provider = createAuthoringTemplates(() => current, io);
  assert.equal(provider.status, 'available');
  const saved = commit(templateDraft.summary.draft_id, 9, 10);
  const actualDefinition = { ...templateDefinition, payload: { category: 'templates', data: { project: 'actual-host-generated/subset.json', generation_profile: null } } };
  const calls = sequence([['describe_capabilities', templateCatalog(true)], ['apply_commands', saved], ...draftReplies(current, templateDraft.summary.draft_id, 10, actualDefinition)]);
  const input = { project_id: 'P1', base_revision: 3, definition: templateDefinition, draft: templateDraft, subset };
  const before = JSON.stringify({ current, input });
  const result = await provider.save(input);
  const mutations = calls.filter(call => call.args.request.operation === 'apply_commands');
  assert.equal(mutations.length, 1);
  assert.deepEqual(mutations[0].args.request.scope, scope);
  assert.equal(mutations[0].args.request.params.base_revision, 9);
  assert.equal(mutations[0].args.request.params.commands[0].kind, 'put_template_composition');
  const payload = mutations[0].args.request.params.commands[0].data;
  assert.equal(payload.project_revision, 3); assert.equal(payload.definition, templateDefinition); assert.equal(payload.subset, subset);
  assert.equal(result.commit, saved);
  assert.equal(result.draft.definition, actualDefinition);
  assert.equal(result.draft.definition.payload.data.project, 'actual-host-generated/subset.json');
  assert.equal(result.draft.history, saved.history);
  assert.equal(JSON.stringify({ current, input }), before);
});

test('new template is created and composed in two original calls, never represented as one atomic new definition', async () => {
  const current = templateSnapshot(); const io = createModuleAuthoringIO(() => current);
  const provider = createAuthoringTemplates(() => current, io);
  const createdId = 'opaque-host:new-draft:8'; const creation = commit(createdId, null, 1); const saved = commit(createdId, 1, 2);
  const actualDefinition = { ...templateDefinition, payload: { category: 'templates', data: { project: 'actual-new-sidecar/subset.json', generation_profile: null } } };
  const calls = sequence([['describe_capabilities', templateCatalog(true)], ['apply_commands', creation], ...draftReplies(current, createdId, 1, templateDefinition), ['apply_commands', saved], ...draftReplies(current, createdId, 2, actualDefinition)]);
  const result = await provider.save({ project_id: 'P1', base_revision: 3, definition: templateDefinition, draft: null, subset });
  const mutations = calls.filter(call => call.args.request.operation === 'apply_commands').map(call => call.args.request);
  assert.equal(mutations.length, 2);
  assert.deepEqual(mutations[0].scope, { kind: 'project' }); assert.equal(mutations[0].params.base_revision, 3);
  assert.equal(mutations[0].params.commands[0].kind, 'create_definition_draft');
  assert.deepEqual(mutations[1].scope, { kind: 'definition_draft', data: createdId }); assert.equal(mutations[1].params.base_revision, 1);
  assert.equal(mutations[1].params.commands[0].kind, 'put_template_composition'); assert.equal(mutations[1].params.commands[0].data.project_revision, 3);
  assert.equal(result.draft.summary.draft_id, createdId); assert.equal(result.commit, saved); assert.equal(result.draft.history, saved.history);
});

test('second-call failure carries real created Draft and original cause, preserves local composition, and explicit retry uses same Draft', async () => {
  const current = templateSnapshot(); const io = createModuleAuthoringIO(() => current);
  const provider = createAuthoringTemplates(() => current, io);
  const id = 'opaque-host:new-draft:9'; const creation = commit(id, null, 1);
  const missing = apiError('MissingReference', 'actual selected source role missing fixture');
  const input = { project_id: 'P1', base_revision: 3, definition: templateDefinition, draft: null, subset };
  const before = JSON.stringify(input);
  const calls = sequence([['describe_capabilities', templateCatalog(true)], ['apply_commands', creation], ...draftReplies(current, id, 1, templateDefinition), ['apply_commands', { outcome: { status: 'error', data: missing } }]]);
  let partial;
  await assert.rejects(provider.save(input), failure => {
    partial = failure;
    return failure instanceof TemplateCompositionSaveFailure && failure.cause instanceof host.HostFailure && failure.cause.detail === missing;
  });
  assert.equal(partial.creationCommit, creation); assert.equal(partial.createdDraft.summary.draft_id, id); assert.equal(partial.createdDraft.definition, templateDefinition);
  assert.equal(partial.createdDraft.history, creation.history); assert.equal(JSON.stringify(input), before);
  assert.equal(calls.filter(call => call.args.request.operation === 'apply_commands').length, 2);
  const saved = commit(id, 1, 2);
  const retryCalls = sequence([['describe_capabilities', templateCatalog(true)], ['apply_commands', saved], ...draftReplies(current, id, 2, templateDefinition)]);
  const retried = await provider.save({ ...input, draft: partial.createdDraft });
  assert.equal(retried.draft.summary.draft_id, id);
  const retryMutations = retryCalls.filter(call => call.args.request.operation === 'apply_commands');
  assert.equal(retryMutations.length, 1); assert.equal(retryMutations[0].args.request.params.commands[0].kind, 'put_template_composition');
});

test('existing-Draft conflict is original HostFailure; actual capability removal queues no create/composition', async () => {
  const current = templateSnapshot(); const io = createModuleAuthoringIO(() => current);
  const provider = createAuthoringTemplates(() => current, io);
  const conflict = { ...apiError('RevisionConflict'), expected_revision: 9, actual_revision: 10 };
  const input = { project_id: 'P1', base_revision: 3, definition: templateDefinition, draft: templateDraft, subset };
  const before = JSON.stringify(input);
  sequence([['describe_capabilities', templateCatalog(true)], ['apply_commands', { outcome: { status: 'error', data: conflict } }]]);
  await assert.rejects(provider.save(input), failure => failure instanceof host.HostFailure && failure.detail === conflict && !(failure instanceof TemplateCompositionSaveFailure));
  assert.equal(JSON.stringify(input), before);
  const calls = sequence([['describe_capabilities', templateCatalog(false)]]);
  await assert.rejects(provider.save(input), /未提供实际模板组合保存命令/);
  assert.equal(calls.length, 1);
  current.capabilities = templateCatalog(false);
  assert.equal(createAuthoringTemplates(() => current).status, 'unavailable');
});

test('draft sidecar preview reads exact DefinitionDraft and DraftR but preserves actual Host ProjectR and paths', async () => {
  const current = templateSnapshot(); const io = createModuleAuthoringIO(() => current);
  const preview = { project_id: 'P1', module_ref: templateDraft.summary.definition_ref, base_revision: 3, source_project: 'actual-host-generated/source.json', subset, resources: [], command: null, issues: [] };
  const calls = sequence([['get_object', { kind: 'template_preview', data: preview }]]);
  const result = await readAuthoringTemplatePreview(() => current, templateDraft, null, null, io);
  assert.equal(result, preview); assert.equal(result.base_revision, 3);
  const sent = calls[0].args.request; assert.deepEqual(sent.scope, scope); assert.equal(sent.project_id, 'P1');
  assert.equal(sent.params.source.kind, 'template_preview'); assert.equal(sent.params.source.data.base_revision, 9);
  assert.equal(sent.params.source.data.module_ref, templateDraft.summary.definition_ref);
  assert.equal(sent.params.source.data.choices, null); assert.equal(sent.params.source.data.seed, null);
  assert.equal(result.source_project, 'actual-host-generated/source.json');
});

test('context switch after the composition reply does not falsely claim that the second atomic command failed', async () => {
  let current = templateSnapshot(); const io = createModuleAuthoringIO(() => current);
  const provider = createAuthoringTemplates(() => current, io);
  const id = 'opaque-host:new-draft:10'; const creation = commit(id, null, 1); const saved = commit(id, 1, 2);
  const replies = [['describe_capabilities', templateCatalog(true)], ['apply_commands', creation], ...draftReplies(current, id, 1, templateDefinition), ['apply_commands', saved]];
  native((_command, args) => {
    const next = replies.shift(); assert.equal(args.request.operation, next[0]);
    if (args.request.operation === 'apply_commands' && args.request.params.commands[0].kind === 'put_template_composition') current = { ...current, project: { id: 'P2', revision: 1 } };
    return protocol(next[1]);
  });
  await assert.rejects(provider.save({ project_id: 'P1', base_revision: 3, definition: templateDefinition, draft: null, subset }), error => !(error instanceof TemplateCompositionSaveFailure) && /已切换/.test(error.message));
});

const completeSeed = { initial_conditions: { items: [], initial_setup_families: {}, declared_quality_decisions: [], material_allocations: [] }, arrivals: [], orders: [] };

test('composition forwards the explicitly captured Scenario id/revision with the complete declared seed, never selecting another live Scenario', async () => {
  const current = { ...templateSnapshot(), scenarios: [{ id: 'SOURCE', revision: 4 }, { id: 'OTHER', revision: 99 }] };
  const io = createModuleAuthoringIO(() => current); const provider = createAuthoringTemplates(() => current, io);
  const captured = { scenario_id: 'SOURCE', revision: 4 };
  const selectedSubset = { ...subset, seed: completeSeed };
  const saved = commit(templateDraft.summary.draft_id, 9, 10);
  const calls = sequence([['describe_capabilities', templateCatalog(true)], ['apply_commands', saved], ...draftReplies(current, templateDraft.summary.draft_id, 10, templateDefinition)]);
  const input = { project_id: 'P1', base_revision: 3, definition: templateDefinition, draft: templateDraft, subset: selectedSubset, source_scenario: captured };
  const before = JSON.stringify(input);
  await provider.save(input);
  const sent = calls.find(call => call.args.request.operation === 'apply_commands').args.request.params.commands[0].data;
  assert.equal(sent.source_scenario, captured); assert.deepEqual(sent.source_scenario, { scenario_id: 'SOURCE', revision: 4 });
  assert.equal(sent.subset.seed, completeSeed); assert.deepEqual(Object.keys(sent.subset.seed).sort(), ['arrivals', 'initial_conditions', 'orders']);
  assert.equal(sent.project_revision, 3); assert.equal(JSON.stringify(input), before);
});

test('composition without seed/ref does not invent Scenario authority; explicit null is preserved', async () => {
  const current = { ...templateSnapshot(), scenarios: [{ id: 'CURRENT', revision: 77 }] };
  const io = createModuleAuthoringIO(() => current); const provider = createAuthoringTemplates(() => current, io);
  for (const extra of [{}, { source_scenario: null }]) {
    const saved = commit(templateDraft.summary.draft_id, 9, 10);
    const calls = sequence([['describe_capabilities', templateCatalog(true)], ['apply_commands', saved], ...draftReplies(current, templateDraft.summary.draft_id, 10, templateDefinition)]);
    await provider.save({ project_id: 'P1', base_revision: 3, definition: templateDefinition, draft: templateDraft, subset, ...extra });
    const sent = calls.find(call => call.args.request.operation === 'apply_commands').args.request.params.commands[0].data;
    assert.equal(sent.subset.seed, null);
    if (Object.hasOwn(extra, 'source_scenario')) { assert.equal(sent.source_scenario, null); assert.ok(Object.hasOwn(sent, 'source_scenario')); }
    else assert.ok(!Object.hasOwn(sent, 'source_scenario'));
    assert.ok(calls.every(call => call.args.request.operation !== 'get_object' || call.args.request.params.object_id.kind === 'definition_draft'));
  }
});

test('live Project/Scenario revision changes remain original Host conflicts with captured source and local seed untouched', async () => {
  let current = { ...templateSnapshot(), scenarios: [{ id: 'SOURCE', revision: 4 }] };
  const io = createModuleAuthoringIO(() => current); const provider = createAuthoringTemplates(() => current, io);
  const captured = { scenario_id: 'SOURCE', revision: 4 };
  const input = { project_id: 'P1', base_revision: 3, definition: templateDefinition, draft: templateDraft, subset: { ...subset, seed: completeSeed }, source_scenario: captured };
  const before = JSON.stringify(input);
  current = { ...current, project: { id: 'P1', revision: 4 }, scenarios: [{ id: 'SOURCE', revision: 5 }] };
  const conflict = { ...apiError('RevisionConflict', 'exact captured Scenario reference is stale'), expected_revision: 4, actual_revision: 5, field_errors: [{ pointer: '/params/commands/data/source_scenario/revision', reason: 'SOURCE revision changed' }] };
  const calls = sequence([['describe_capabilities', templateCatalog(true)], ['apply_commands', { outcome: { status: 'error', data: conflict } }]]);
  await assert.rejects(provider.save(input), failure => failure instanceof host.HostFailure && failure.detail === conflict);
  const sent = calls[1].args.request;
  assert.equal(sent.params.base_revision, 9); assert.equal(sent.params.commands[0].data.project_revision, 3);
  assert.equal(sent.params.commands[0].data.source_scenario, captured); assert.equal(sent.params.commands[0].data.source_scenario.revision, 4);
  assert.equal(JSON.stringify(input), before); assert.equal(calls.length, 2);
});

test('creation then rejected seed-context save preserves exact source and full original partial-failure evidence', async () => {
  const current = { ...templateSnapshot(), scenarios: [{ id: 'SOURCE', revision: 5 }] };
  const io = createModuleAuthoringIO(() => current); const provider = createAuthoringTemplates(() => current, io);
  const id = 'opaque-host:new-draft:seed'; const creation = commit(id, null, 1);
  const captured = { scenario_id: 'SOURCE', revision: 4 };
  const input = { project_id: 'P1', base_revision: 3, definition: templateDefinition, draft: null, subset: { ...subset, seed: completeSeed }, source_scenario: captured };
  const before = JSON.stringify(input);
  const conflict = { ...apiError('RevisionConflict', 'actual seed-source revision conflict'), expected_revision: 4, actual_revision: 5 };
  const calls = sequence([['describe_capabilities', templateCatalog(true)], ['apply_commands', creation], ...draftReplies(current, id, 1, templateDefinition), ['apply_commands', { outcome: { status: 'error', data: conflict } }]]);
  await assert.rejects(provider.save(input), failure => failure instanceof TemplateCompositionSaveFailure && failure.creationCommit === creation && failure.createdDraft.summary.draft_id === id && failure.cause instanceof host.HostFailure && failure.cause.detail === conflict);
  const sent = calls.filter(call => call.args.request.operation === 'apply_commands');
  assert.equal(sent.length, 2); assert.equal(sent[1].args.request.params.commands[0].data.source_scenario, captured);
  assert.equal(sent[1].args.request.params.commands[0].data.subset.seed, completeSeed); assert.equal(JSON.stringify(input), before);
});

test('Draft preview preserves actual old source_scenario and never infers an absent reference from seed body or current Workspace', async () => {
  const current = { ...templateSnapshot(), scenarios: [{ id: 'SOURCE', revision: 100 }] };
  const io = createModuleAuthoringIO(() => current);
  for (const extra of [{ source_scenario: { scenario_id: 'SOURCE', revision: 4 } }, {}]) {
    const preview = { project_id: 'P1', module_ref: templateDraft.summary.definition_ref, base_revision: 3, source_project: 'actual-source-scenario-sidecar/source.json', subset: { ...subset, seed: completeSeed }, resources: [], command: null, issues: [], ...extra };
    const calls = sequence([['get_object', { kind: 'template_preview', data: preview }]]);
    const actual = await readAuthoringTemplatePreview(() => current, templateDraft, null, null, io);
    assert.equal(actual, preview); assert.equal(actual.source_scenario, extra.source_scenario);
    assert.equal(Object.hasOwn(actual, 'source_scenario'), Object.hasOwn(extra, 'source_scenario'));
    assert.equal(calls.length, 1); assert.equal(calls[0].args.request.params.source.data.base_revision, 9);
    assert.deepEqual(calls[0].args.request.scope, scope);
  }
});
