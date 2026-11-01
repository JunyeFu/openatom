import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

const file = new URL('../../../apps/planner/src/features/factory/Workbench.tsx', import.meta.url);
const workbench = readFileSync(file, 'utf8');
const start = workbench.indexOf('  async function refresh(resetResults = false)');
const end = workbench.indexOf('\n  const refreshLatest', start);
assert.ok(start >= 0 && end > start, 'actual Workbench refresh region');
const code = stripTypeScriptTypes(workbench.slice(start, end), { mode: 'transform' });

function workspace(id, revision = 1) {
  return { project: { id, revision, halls: [{ id: `${id}-hall` }] },
    context: { host: { session_id: 'same-native-host' }, run_summaries: [] },
    scenarios: [{ id: `${id}-scenario` }] };
}
function deferred() {
  let resolve, reject;
  const promise = new Promise((ok, fail) => { resolve = ok; reject = fail; });
  return { promise, resolve, reject };
}
function mountedRefresh(initial, reads) {
  const state = { snapshot: initial, cloudOpen: true, cloudDirty: true, hallId: initial.project.halls[0].id, scenarioId: initial.scenarios[0].id };
  const setters = ['CloudOpen', 'CloudDirty', 'CloudClosePrompt', 'Selections', 'LayoutMode', 'BusinessOpen', 'BusinessEditIntent', 'Authoring', 'AuthoringDraft',
    'AuthoringPresentation', 'PackagingScope', 'PackagingContext', 'DeviceProcess', 'Comparison', 'Selection',
    'TimelineSelection', 'Run', 'RunProject', 'RunInputs', 'ObservationPage', 'AtMs', 'Source', 'HallId',
    'CommittedHistory', 'Snapshot', 'ScenarioId'];
  const dependencies = {
    host: { readWorkspace: () => { assert.ok(reads.length, 'declared read only; no retry'); return reads.shift(); } },
    refreshEpoch: { current: 0 }, observation: { current: 0 }, pendingRun: { current: null },
    comparisonEpoch: { current: 0 }, primaryRun: { current: null },
    workspaceRef: { current: initial }, projectIdentity: { current: initial.project.id },
    nodeDirtyRef: { current: false }, run: null, comparison: null, source: 'draft', runProject: null,
  };
  for (const name of setters) {
    const key = name[0].toLowerCase() + name.slice(1);
    dependencies[`set${name}`] = value => { state[key] = typeof value === 'function' ? value(state[key]) : value; };
  }
  const names = Object.keys(dependencies);
  const refresh = new Function(...names, `${code}\nreturn refresh;`)(...names.map(name => dependencies[name]));
  return { refresh, state, dependencies };
}

test('late Project A refresh cannot replace the newer accepted Project B or its controls', async () => {
  const old = deferred(), a = workspace('A'), b = workspace('B');
  const view = mountedRefresh(a, [old.promise, Promise.resolve(b)]);
  const stale = view.refresh();
  assert.equal(await view.refresh(true), b);
  old.resolve(a);
  const [outcome] = await Promise.allSettled([stale]);
  assert.equal(view.state.snapshot, b);
  assert.equal(outcome.status, 'rejected');
  assert.match(outcome.reason.message, /旧.*读取|读取.*取代/);
  assert.equal(view.dependencies.projectIdentity.current, 'B');
  assert.equal(view.dependencies.workspaceRef.current, b);
  assert.equal(view.state.hallId, 'B-hall');
  assert.equal(view.state.scenarioId, 'B-scenario');
  assert.equal(view.state.cloudOpen, false);
  assert.equal(view.state.cloudDirty, false);
});

test('a late refresh of the same project cannot roll revision 2 back to revision 1', async () => {
  const old = deferred(), r1 = workspace('A'), r2 = workspace('A', 2);
  const view = mountedRefresh(r1, [old.promise, Promise.resolve(r2)]);
  const stale = view.refresh();
  assert.equal(await view.refresh(), r2);
  old.resolve(r1);
  const [outcome] = await Promise.allSettled([stale]);
  assert.equal(view.state.snapshot.project.revision, 2);
  assert.equal(view.state.cloudOpen, true);
  assert.equal(view.state.cloudDirty, true);
  assert.equal(outcome.status, 'rejected');
  assert.match(outcome.reason.message, /旧.*读取|读取.*取代/);
  assert.equal(view.dependencies.workspaceRef.current, r2);
});

test('a single current refresh still returns and publishes the actual next workspace', async () => {
  const a = workspace('A'), b = workspace('B');
  const view = mountedRefresh(a, [Promise.resolve(b)]);
  assert.equal(await view.refresh(true), b);
  assert.equal(view.state.snapshot, b);
  assert.equal(view.state.source, 'draft');
  assert.equal(view.state.run, null);
});

test('a newer failed read remains visible to its caller and does not revive an older response', async () => {
  const old = deferred(), latest = deferred(), initial = workspace('A');
  const view = mountedRefresh(initial, [old.promise, latest.promise]);
  const stale = view.refresh(), current = view.refresh();
  const actualFailure = new Error('actual Host read failed');
  latest.reject(actualFailure);
  await assert.rejects(current, error => error === actualFailure);
  old.resolve(workspace('old-other-project'));
  const [outcome] = await Promise.allSettled([stale]);
  assert.equal(view.state.snapshot, initial);
  assert.equal(outcome.status, 'rejected');
  assert.match(outcome.reason.message, /旧.*读取|读取.*取代/);
});
