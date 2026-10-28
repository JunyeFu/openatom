// Reads and edits only the explicitly selected P0 acceptance project on a real Native Host.
// Windows directory selection and save/open actions remain in the actual application UI.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const [playwrightPath, endpoint, expectedHost, mode = 'inspect', diskManifest] = process.argv.slice(2);
assert.ok(playwrightPath && endpoint && expectedHost);
const { chromium } = createRequire(import.meta.url)(playwrightPath);
const root = new URL('../../', import.meta.url);
const out = new URL(`.work/whole-packages/p001-20261017/native/${mode}/`, root);
await mkdir(out, { recursive: true });
const browser = await chromium.connectOverCDP(endpoint);
const page = browser.contexts().flatMap(context => context.pages()).find(candidate => !candidate.url().startsWith('devtools:'));
assert.ok(page, 'Actual Native main WebView must exist');
const identity = await page.evaluate(() => window.__TAURI_INTERNALS__.invoke('agent_connection_info'));
assert.equal(identity.host_instance_id, expectedHost);
const errors = [], checks = [];
page.on('pageerror', error => errors.push(String(error)));
const save = (name, value) => writeFile(new URL(name, out), JSON.stringify(value, null, 2));
async function raw(operation, params, projectId = null) {
  return page.evaluate(({ operation, params, projectId }) => window.__TAURI_INTERNALS__.invoke('project_api', {
    request: { api_version: '0.1', request_id: 'p001-native-' + crypto.randomUUID(), project_id: projectId,
      scope: { kind: projectId ? 'project' : 'workspace' }, operation, params },
  }), { operation, params, projectId });
}
async function api(operation, params, projectId = null) {
  const response = await raw(operation, params, projectId);
  assert.equal(response.outcome.status, 'ok', JSON.stringify(response));
  return response.outcome.data;
}
const context = () => api('get_context', { selection: [] });
const initial = await context();
const projectId = initial.project?.id;
const object = async (kind, id) => (await api('get_object', { object_id: { kind, id }, source: { kind: 'draft' }, node_index: null }, projectId)).data;
const project = () => object('project', projectId);
const edit = (base_revision, command) => api('apply_commands', { base_revision, commands: [command] }, projectId);
const record = async (name, action) => {
  try { checks.push({ name, passed: true, details: await action() }); }
  catch (error) { checks.push({ name, passed: false, error: String(error), stack: error.stack }); throw error; }
  finally { await save('result.json', { identity, mode, initial, checks, errors, whole_task_done: false, human_acceptance: false }); }
};

let exitCode = 0;
try {
  await record('actual_main_Renderer_and_private_close_snapshot_use_same_Host', async () => {
    const close = await page.evaluate(() => window.__TAURI_INTERNALS__.invoke('project_close_state'));
    assert.equal(close.project_id, projectId ?? null);
    assert.equal(close.project_revision, initial.project?.revision ?? null);
    await save('initial-context.json', initial);
    if (projectId) await save('initial-project.json', await project());
    return { url: page.url(), close };
  });
  if (mode === 'typed-history') {
    const expected = JSON.parse(await readFile(new URL('.work/whole-packages/p001-20261017/complete-input/payload/project-state.json', root), 'utf8')).data;
    const original = await project();
    const diskBefore = diskManifest ? await readFile(diskManifest, 'utf8') : null;
    const frozen = [];
    for (const run of initial.run_summaries ?? []) frozen.push(await api('get_run', { run_id: run.run_id, query: { kind: 'full_result' } }, projectId));
    await record('actual_typed_roundtrip_preserves_two_halls_branches_ports_units_goals_and_sources', async () => {
      const keys = ['halls', 'lines', 'devices', 'connections', 'locations', 'item_types', 'generation_context', 'sources', 'parameter_sources', 'module_refs', 'resources', 'calendars', 'products', 'boms', 'routes', 'scenario_refs', 'artifact_refs', 'acknowledgments', 'bindings', 'setup_policies', 'template_adoptions'];
      for (const key of keys) {
        assert.deepEqual(original[key], expected[key], key);
      }
      assert.equal(original.halls.length, 2);
      assert.equal(original.lines.length, 2);
      assert.equal(original.devices.length, 4);
      assert.equal(original.connections.filter(connection => connection.kind === 'material').length, 3);
      assert.equal(original.connections.filter(connection => connection.kind === 'signal').length, 1);
      assert.equal(original.connections.filter(connection => connection.kind === 'artifact').length, 1);
      assert.equal(original.issues.length, 1);
      assert.equal(original.issues[0].code, 'professional_inputs.nonhuman_acknowledgment');
      return { project_id: projectId, revision: original.revision, exact_sections: keys.length, expected_professional_issue_retained: true };
    });
    const hall = original.halls[0], first = { ...hall, title: hall.title + ' — edited in Native' };
    let revision = original.revision;
    await record('shared_Rust_apply_undo_redo_create_new_revisions_and_restore_typed_object', async () => {
      await edit(revision++, { kind: 'put_object', data: { kind: 'hall', data: first } });
      assert.deepEqual((await project()).halls[0], first);
      await edit(revision++, { kind: 'undo' });
      assert.deepEqual((await project()).halls[0], hall);
      await edit(revision++, { kind: 'redo' });
      const after = await project();
      assert.equal(after.revision, revision);
      assert.deepEqual(after.halls[0], first);
      return { revisions: [original.revision + 1, original.revision + 2, revision] };
    });
    await record('same_revision_conflict_retains_current_project', async () => {
      const before = await project();
      const response = await raw('apply_commands', { base_revision: original.revision, commands: [{ kind: 'put_object', data: { kind: 'hall', data: hall } }] }, projectId);
      assert.notEqual(response.outcome.status, 'ok');
      assert.deepEqual(await project(), before);
      await save('stale-revision-response.json', response);
      return response.outcome;
    });
    await record('new_edit_after_undo_invalidates_redo_branch', async () => {
      await edit(revision++, { kind: 'undo' });
      const branch = { ...hall, title: hall.title + ' — new branch' };
      await edit(revision++, { kind: 'put_object', data: { kind: 'hall', data: branch } });
      const before = await project();
      const response = await raw('apply_commands', { base_revision: revision, commands: [{ kind: 'redo' }] }, projectId);
      assert.notEqual(response.outcome.status, 'ok');
      assert.deepEqual(await project(), before);
      assert.deepEqual(before.halls[0], branch);
      await save('invalid-redo-response.json', response);
      return { revision, rejected: response.outcome };
    });
    await record('draft_history_leaves_saved_manifest_and_existing_Run_results_unchanged', async () => {
      if (diskManifest) assert.equal(await readFile(diskManifest, 'utf8'), diskBefore);
      for (const before of frozen) assert.deepEqual(await api('get_run', { run_id: before.data.id, query: { kind: 'full_result' } }, projectId), before);
      await save('final-project.json', await project());
      return { manifest_checked: Boolean(diskManifest), retained_run_records: frozen.length, run_origin_not_inferred_from_input: true };
    });
  }
  await page.screenshot({ path: fileURLToPath(new URL('actual-main.png', out)), fullPage: true });
  console.log(JSON.stringify({ mode, passed: checks.length, errors, host: identity.host_instance_id }));
  assert.deepEqual(errors, []);
} catch (error) {
  exitCode = 1;
  console.error(error);
} finally {
  // End this client process without asking the actual Native browser to close.
  process.exit(exitCode);
}
