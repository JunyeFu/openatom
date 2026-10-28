import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';

// Invoke the actual UI action at its IO boundary; ordinary Native verifies the button.
const text = await readFile(new URL('../../../apps/planner/src/features/modules/ModuleAuthoringWorkspace.tsx', import.meta.url), 'utf8');
const action = text.match(/async function exportContribution\(\) \{[\s\S]+?\n  \}/)?.[0];
assert.ok(action, 'Actual contribution-export action must exist');
const compiled = stripTypeScriptTypes(action, { mode: 'transform' });
function setup(overrides = {}) {
  const calls = [], messages = [];
  const context = { committedRef: { id: 'test.saved-device', version: '0.6.90' }, draft: null, projectId: 'P1', dirty: false,
    workspace: { project: { id: 'P1', revision: 13 } }, wrongProject: false, unreadModule: false,
    definition: { contribution: { license: 'MIT' } }, redistribute: true,
    io: { async pickDirectory(purpose) { calls.push({ pick: purpose }); return { selection_id: 'explicit-destination' }; },
      async dispatch(request) { calls.push(request); return { id: 'actual-IO-boundary-task' }; },
      async awaitTask() { return { kind: 'package', data: { missing: [], included_files: ['module.json', 'actual.svg'] } }; } },
    authoringRequest: (project_id, scope, operation, params) => ({ project_id, scope, operation, params }),
    setMessage: value => messages.push(value), ...overrides };
  const run = new Function(...Object.keys(context), `${compiled}; return exportContribution;`)(...Object.values(context));
  return { run, calls, messages };
}
test('reopened committed library version exports exact module with Project revision and no instances, scenarios or Runs', async () => {
  const state = setup(); await state.run();
  const request = state.calls.find(value => value.operation === 'export_package');
  assert.deepEqual(request.scope, { kind: 'project' }); assert.equal(request.params.revision, 13);
  assert.deepEqual(request.params.package_scope, { purpose: 'community_module', module_refs: [{ id: 'test.saved-device', version: '0.6.90' }], scenarios: [], run_ids: [], artifact_ids: [], include_redistributable_assets: true });
  assert.equal(state.calls.filter(value => value.operation === 'apply_commands').length, 0);
});
test('committed live Draft retains its own Scope and revision for export', async () => {
  const state = setup({ draft: { summary: { draft_id: 'opaque-draft', revision: 2 } } }); await state.run();
  const request = state.calls.find(value => value.operation === 'export_package');
  assert.deepEqual(request.scope, { kind: 'definition_draft', data: 'opaque-draft' }); assert.equal(request.params.revision, 2);
});
test('dirty, switched or unread source cannot export, and destination cancellation writes nothing', async () => {
  for (const override of [{ dirty: true }, { wrongProject: true }, { unreadModule: true }, { definition: { contribution: { license: null } } }]) {
    const state = setup(override); await assert.rejects(state.run); assert.equal(state.calls.length, 0);
  }
  const state = setup({ io: { async pickDirectory() { return null; }, async dispatch() { assert.fail('cancelled destination must not dispatch'); } } });
  await state.run(); assert.equal(state.calls.length, 0);
});
test('destination selection retains the captured revision for Host conflict checking', async () => {
  const workspace = { project: { id: 'P1', revision: 13 } }, requests = [];
  const state = setup({ workspace, io: {
    async pickDirectory() { workspace.project.revision = 14; return { selection_id: 'selected-before-concurrent-edit' }; },
    async dispatch(request) { requests.push(request); throw new Error('Conflict from real revision contract'); },
  } });
  await assert.rejects(state.run, /Conflict/); assert.equal(requests.length, 1); assert.equal(requests[0].params.revision, 13);
});
