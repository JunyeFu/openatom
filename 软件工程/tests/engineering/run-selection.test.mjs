import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { rolldown } from 'rolldown';

const entry = fileURLToPath(new URL('../../apps/planner/src/features/factory/runSelection.ts', import.meta.url));
const bundle = await rolldown({ input: entry });
after(() => bundle.close());
const generated = await bundle.generate({ format: 'esm' });
const code = generated.output.find(output => output.type === 'chunk')?.code;
assert.ok(code);
const { selectRunForWorkspace, readRememberedRun, rememberRunSelection } =
  await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}`);

const run = (run_id, project_id, scenario_id, computed_until_ms = 1000) => ({
  run_id, input: { project_id, scenario_id }, computed_until_ms,
});
const request = (overrides = {}) => ({
  projectId: 'project-A', scenarioId: 'scenario-2', runs: [], selectedRunId: null,
  selectedProjectId: null, completedRunId: null, projectOpened: false, ...overrides,
});

test('opening a project chooses a readable saved Run for its current scenario', () => {
  const runs = [run('old-scenario', 'project-A', 'scenario-1'), run('matching-1', 'project-A', 'scenario-2'),
    run('other-project', 'project-B', 'scenario-2'), run('matching-unreadable', 'project-A', 'scenario-2', null),
    run('matching-2', 'project-A', 'scenario-2')];
  assert.equal(selectRunForWorkspace(request({ runs, projectOpened: true })), 'matching-2');
  assert.equal(selectRunForWorkspace(request({ runs, scenarioId: 'missing', projectOpened: true })), 'matching-2');
});

test('an explicit completed Run takes precedence only when it belongs to the current project and is readable', () => {
  const runs = [run('previous', 'project-A', 'scenario-2'), run('new', 'project-A', 'scenario-2'),
    run('foreign', 'project-B', 'scenario-2')];
  assert.equal(selectRunForWorkspace(request({ runs, selectedRunId: 'previous', selectedProjectId: 'project-A', completedRunId: 'new' })), 'new');
  assert.equal(selectRunForWorkspace(request({ runs, selectedRunId: 'previous', selectedProjectId: 'project-A', completedRunId: 'foreign' })), null);
});

test('ordinary refresh keeps the user-selected Run and never silently picks another', () => {
  const runs = [run('selected', 'project-A', 'scenario-1'), run('newer-in-list', 'project-A', 'scenario-2')];
  assert.equal(selectRunForWorkspace(request({ runs, selectedRunId: 'selected', selectedProjectId: 'project-A' })), 'selected');
  assert.equal(selectRunForWorkspace(request({ runs, selectedRunId: 'missing', selectedProjectId: 'project-A' })), null);
  assert.equal(selectRunForWorkspace(request({ runs })), null);
});

test('opening another project ignores the former project selection', () => {
  const runs = [run('former', 'project-A', 'scenario-2'), run('current', 'project-B', 'scenario-3')];
  assert.equal(selectRunForWorkspace(request({ projectId: 'project-B', scenarioId: 'scenario-3', runs,
    selectedRunId: 'former', selectedProjectId: 'project-A', projectOpened: true })), 'current');
  assert.equal(selectRunForWorkspace(request({ projectId: null, runs, projectOpened: true })), null);
});

test('a successful selection can be remembered per project across UI instances', () => {
  const values = new Map();
  const storage = { getItem: key => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) };
  rememberRunSelection('project-A', 'new-run', storage);
  assert.equal(readRememberedRun('project-A', storage), 'new-run');
  assert.equal(readRememberedRun('project-B', storage), null);
  rememberRunSelection('project-B', 'other-run', storage);
  assert.equal(readRememberedRun('project-A', storage), 'new-run');
  assert.equal(readRememberedRun('project-B', storage), 'other-run');
  assert.equal(JSON.parse([...values.values()][0]), 'new-run');
});

test('a remembered ID is still checked against readable runs before restoration', () => {
  const values = new Map();
  const storage = { getItem: key => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) };
  rememberRunSelection('project-A', 'removed-run', storage);
  const runs = [run('current', 'project-A', 'scenario-2'), run('foreign', 'project-B', 'scenario-2')];
  assert.equal(selectRunForWorkspace(request({ runs, selectedRunId: readRememberedRun('project-A', storage),
    selectedProjectId: 'project-A', projectOpened: true })), 'current');
  rememberRunSelection('project-A', 'foreign', storage);
  assert.equal(selectRunForWorkspace(request({ runs, selectedRunId: readRememberedRun('project-A', storage),
    selectedProjectId: 'project-A', projectOpened: true })), 'current');
});

test('storage denial and invalid data never block opening a project', () => {
  const denied = { getItem: () => { throw new Error('denied'); }, setItem: () => { throw new Error('denied'); } };
  assert.doesNotThrow(() => rememberRunSelection('project-A', 'new-run', denied));
  assert.equal(readRememberedRun('project-A', denied), null);
  assert.equal(readRememberedRun('project-A', { getItem: () => '{broken', setItem() {} }), null);
  assert.equal(readRememberedRun('project-A', { getItem: () => '42', setItem() {} }), null);
  assert.equal(readRememberedRun('project-A', null), null);
});
