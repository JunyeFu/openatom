import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import vm from 'node:vm';

const source = readFileSync(process.env.SF_WORKBENCH_SOURCE ?? new URL('../../../apps/planner/src/features/factory/Workbench.tsx', import.meta.url), 'utf8');
const observations = source.slice(source.indexOf('  function ownsObservation('), source.indexOf('  async function commitTimeline('));
const choosing = source.slice(source.indexOf('  async function chooseRun('), source.indexOf('  async function chooseComparison('));
const switching = source.slice(source.indexOf('  function switchSource('), source.indexOf('  function revealObjectHall('));
const key = c => JSON.stringify([c.session_id, c.project_id, c.scope, c.scope_revision, c.project.id, c.project.revision, c.view]);
const tick = () => new Promise(resolve => setImmediate(resolve));

function consumer() {
  const project = { id: 'P', revision: 11, halls: [{ id: 'H' }] };
  const frozenProject = { ...project, revision: 7 };
  const run = { id: 'R', computed_until_ms: 1200000, jobs: [{ id: 'final-job', state: 'completed' }] };
  const frozen = { project: frozenProject, modules: new Map(), scenario: { id: 'S', revision: 7 } };
  const baseline = JSON.stringify({ project, run, frozenProject });
  const writes = [], reads = [], protocol = [];
  const oldPage = { run_id: 'R', at_ms: 0, objects: [], next_index: null };
  const env = {
    Error, project, run, runProject: frozenProject, runInputs: frozen,
    observation: { current: 0 }, pendingRun: { current: 'R' }, primaryRun: { current: 'R' },
    projectIdentity: { current: 'P' }, comparisonEpoch: { current: 0 }, comparison: null,
    factoryCurrent: { current: { session_id: 'session-A', project_id: 'P', scope: { kind: 'project' }, scope_revision: 11,
      project: frozenProject, view: { run_id: 'R', at_ms: 0 }, preparedRun: { run, modules: frozen.modules } } },
    factoryContextKey: key,
    createFactoryPresentation(current) {
      return { read(context) {
        return new Promise((resolve, reject) => reads.push({ current, context, resolve, reject }));
      } };
    },
    host: {
      async getRun(projectId, runId, query) {
        protocol.push({ projectId, runId, query });
        return query.kind === 'full_result' ? { kind: 'full', data: run }
          : { kind: 'objects', data: { ...oldPage, at_ms: query.data.at_ms } };
      },
      async readRunInputs() { return frozen; },
    },
  };
  env.factoryReadCurrent = env.factoryCurrent;
  for (const name of ['FactorySource', 'AtMs', 'ObservationPage', 'Source', 'Run', 'RunProject', 'RunInputs', 'TimelineSelection', 'HallId', 'Comparison', 'Selections', 'Selection', 'LayoutMode']) {
    env['set' + name] = value => writes.push([name, value]);
  }
  vm.runInNewContext(stripTypeScriptTypes(choosing + observations + switching) + '\nglobalThis.choose=chooseRun;globalThis.seek=seekRun;globalThis.switch=switchSource;', env);
  function complete(read = reads.at(-1)) {
    const page = { run_id: read.context.view.run_id, at_ms: read.context.view.at_ms, next_index: null,
      objects: [{ kind: 'job', data: { id: 'actual-job', state: 'running' } }, { kind: 'item', data: { id: 'actual-item', holder: { kind: 'job', id: 'actual-job' } } }] };
    const presentation = { key: key(read.context), project: frozenProject, run, observation: page, devices: [], items: [] };
    read.resolve(presentation);
    return presentation;
  }
  return { env, project, run, frozenProject, frozen, baseline, writes, reads, protocol, complete };
}

test('seek keeps the displayed time unchanged until its actual object and presentation source are ready', async () => {
  const t = consumer(), pending = t.env.seek('R', 600000);
  await tick();
  assert.equal(t.writes.length, 0, 'do not publish a time that still lacks its presentation');
  assert.equal(t.reads.length, 1);
  assert.equal(t.protocol.length, 0, 'do not also read the same objects through the old public pager');
  const presentation = t.complete();
  await pending;
  assert.deepEqual(t.writes.map(([name]) => name), ['FactorySource', 'AtMs', 'ObservationPage', 'Source']);
  assert.equal(t.writes[0][1], presentation);
  assert.equal(t.writes[1][1], 600000);
  assert.equal(t.writes[2][1], presentation.observation);
  assert.equal(t.writes[2][1].objects[0].data.state, 'running', 'actual JobsAt, not final Run.jobs');
  assert.equal(JSON.stringify({ project: t.project, run: t.run, frozenProject: t.frozenProject }), t.baseline);
});

test('choosing a saved Run publishes its exact frozen inputs and requested observation in the same completion', async () => {
  const t = consumer(), pending = t.env.choose('R', 35000);
  await tick();
  assert.equal(t.writes.length, 0);
  assert.equal(t.reads.length, 1);
  assert.equal(t.reads[0].context.project, t.frozenProject);
  assert.equal(t.reads[0].context.preparedRun.run, t.run);
  assert.equal(t.reads[0].context.preparedRun.modules, t.frozen.modules);
  const presentation = t.complete();
  await pending;
  assert.equal(t.protocol.length, 1);
  assert.equal(t.protocol[0].query.kind, 'full_result');
  assert.equal(t.writes.find(([name]) => name === 'FactorySource')[1], presentation);
  assert.equal(t.writes.find(([name]) => name === 'AtMs')[1], 35000);
  assert.equal(t.writes.find(([name]) => name === 'ObservationPage')[1], presentation.observation);
});

test('business record location reads the requested actual time while the scene is hidden by the editor', async () => {
  const t = consumer();
  t.env.factoryCurrent = { current: null };
  const pending = t.env.seek('R', 40000);
  await tick();
  assert.equal(t.reads.length, 1);
  assert.equal(t.reads[0].context.view.at_ms, 40000);
  const presentation = t.complete();
  await pending;
  assert.equal(t.writes.find(([name]) => name === 'AtMs')[1], 40000);
  assert.equal(t.writes.find(([name]) => name === 'ObservationPage')[1], presentation.observation);
  assert.equal(JSON.stringify({ project: t.project, run: t.run, frozenProject: t.frozenProject }), t.baseline);
});

test('an older completed seek cannot overwrite a newer observation', async () => {
  const t = consumer(), old = t.env.seek('R', 35000), latest = t.env.seek('R', 40000);
  assert.equal(t.reads.length, 2);
  const presentation = t.complete(t.reads[1]);
  await latest;
  const accepted = t.writes.length;
  t.complete(t.reads[0]);
  await old;
  assert.equal(t.writes.length, accepted);
  assert.equal(t.writes.find(([name]) => name === 'ObservationPage')[1], presentation.observation);
});

for (const changed of ['session', 'project', 'revision', 'closed']) test(`pending actual source cannot publish after workspace ${changed} changes`, async () => {
  const t = consumer(), pending = t.env.seek('R', 600000);
  if (changed === 'session') t.env.factoryCurrent.current = { ...t.env.factoryCurrent.current, session_id: 'session-B' };
  if (changed === 'project') { t.env.projectIdentity.current = 'OTHER'; t.env.factoryCurrent.current = { ...t.env.factoryCurrent.current, project_id: 'OTHER' }; }
  if (changed === 'revision') t.env.factoryCurrent.current = { ...t.env.factoryCurrent.current, scope_revision: 12 };
  if (changed === 'closed') t.env.factoryCurrent.current = null;
  t.complete();
  await pending;
  assert.equal(t.writes.length, 0);
});

test('source read failure preserves the prior displayed observation and reports the actual error', async () => {
  const t = consumer(), failure = new Error('actual port output read failed'), pending = t.env.seek('R', 600000);
  t.reads[0].reject(failure);
  await assert.rejects(pending, error => error === failure);
  assert.equal(t.writes.length, 0);
});

test('explicitly returning to the draft cancels a pending seek without switching back on its late completion', async () => {
  const t = consumer(), pending = t.env.seek('R', 600000);
  t.env.switch('draft');
  const accepted = t.writes.length;
  t.complete(); await pending;
  assert.equal(t.writes.length, accepted);
  assert.equal(t.writes.find(([name]) => name === 'Source')[1], 'draft');
});

test('displayed-frame media stays owned while a replacement reads, then loses ownership when it commits', async () => {
  const t = consumer(), first = t.env.seek('R', 35000);
  t.complete(); await first;
  t.env.factoryCurrent.current = t.reads[0].context;
  const next = t.env.seek('R', 40000);
  assert.equal(t.reads[0].current(), t.reads[0].context);
  t.complete(t.reads[1]); await next;
  t.env.factoryCurrent.current = t.reads[1].context;
  assert.equal(t.reads[0].current(), null);
  assert.equal(t.reads[1].current(), t.reads[1].context);
});

test('a render of an already paired observation neither clears it nor starts a second factory read', async () => {
  const start = source.indexOf('  useEffect(() => {', source.indexOf('  const visibleFactorySource'));
  const end = source.indexOf('\n  async function refresh(', start);
  const context = { session_id: 'session', project_id: 'P', scope: { kind: 'project' }, scope_revision: 3, project: { id: 'P', revision: 3 }, view: { run_id: 'R', at_ms: 600000 } };
  const value = { key: key(context), observation: { run_id: 'R', at_ms: 600000 } }, writes = [], reads = [];
  let effect;
  vm.runInNewContext(stripTypeScriptTypes(source.slice(start, end)), {
    factoryCurrent: { current: context }, factorySource: value, factoryKey: value.key,
    factoryContextKey: key, factoryReader: { async read(c) { reads.push(c); return value; } },
    setFactorySource: next => writes.push(next), setFailure: error => { throw error; },
    useEffect: callback => { effect = callback; },
  });
  effect(); await tick();
  assert.equal(writes.length, 0);
  assert.equal(reads.length, 0);
});
