import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile, writeFile, rename} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {root, source, out as models} from './load-source.mjs';
import {component, mount, text} from './noncu-hooks.mjs';
import {start, api} from './noncu-rpc.mjs';

const model = await source('model');
const authoring = await import(new URL('module-model.mjs', models));
const Component = await component();
const out = new URL('.work/delivery/', root);
const fixture = '.work/inputs/fixed.input.json';
const projectScope = {kind: 'project'};
const expected = {
  order: ['frame-b', 'frame-a', 'frame-c', 'frame-d'],
  ranges: [[0, .3], [.3, .65], [.65, .85], [.85, 1]],
  transitions: ['hold', 'crossfade', 'pan', 'zoom'],
  fields: ['device.completed_output', 'device.accepted_output'],
  unit: 'piece', initialAcceptedAtMs: 60000, initialAccepted: 1,
};
await writeFile(new URL('../inputs/closeout-expected.json', out), JSON.stringify(expected, null, 2));

test('P3-03 same live Host / two editor consumers / moved independent-process readback', async t => {
  let session, reopened, A, B;
  const completed = [];
  async function step(name, run) {
    let failure;
    await t.test(name, async () => {try {await run();} catch (error) {failure = error; throw error;}});
    if (failure) throw failure;
  }
  try {
    session = await start(fixture);
    const initial = session.ready;
    const admin = api(session, 'setup');
    const scope = {kind: 'definition_draft', data: initial.draft.summary.draft_id};
    const projectId = initial.snapshot.project.id;
    const definitionId = initial.package.definition.id;
    const oldRun = structuredClone(initial.formal_run);
    const oldModule = structuredClone(initial.package);
    const originalProject = structuredClone(initial.snapshot.project);
    const exact = version => ({id: definitionId, version});
    const request = (scope, operation, params) => ({scope, operation, params});
    const moduleAt = (dispatch, version) => dispatch(request(projectScope, 'get_object', {
      object_id: {kind: 'module', id: definitionId},
      source: {kind: 'module_version', data: {module_ref: exact(version)}}, node_index: null,
    })).then(value => value.data);
    const readDraft = (dispatch, commit) => authoring.readDraft({dispatch}, projectId, scope.data, commit?.history ?? null, commit?.new_revision ?? null);
    const change = (dispatch, revision, commands) => dispatch(request(scope, 'apply_commands', {base_revision: revision, commands}));
    const untouched = async () => {
      const currentRun = await admin(request(projectScope, 'get_run', {run_id: oldRun.id, query: {kind: 'full_result'}}));
      assert.deepEqual(currentRun.data, oldRun);
      assert.deepEqual(await moduleAt(admin, '0.1.0'), oldModule);
    };
    function consumer(name, draft) {
      const dispatch = api(session, name);
      const value = {dispatch, draft: structuredClone(draft), state: null, view: null, inspections: 0};
      value.h = mount(Component, {
        workspace: {project: initial.snapshot.project, context: initial.context},
        draft: value.draft, source: initial.package, io: {dispatch},
        samples: {status: 'unavailable', reason: 'same-Host conflict test; existing sample evidence reused'},
        scenario: null, actions: {inspect: async q => {
          const saved = await dispatch(request(scope, 'get_object', {object_id: {kind: 'definition_draft', id: q.draft_id}, source: {kind: 'draft'}, node_index: null}));
          value.inspections++; return {message: saved.data.title};
        }},
        media: {fields: [], render(view) {value.view = view; return null;}},
        onDraftChanged(draft) {value.draft = draft; value.h.props = {...value.h.props, draft};},
        onCommitted() {}, onClose() {}, onEditorStateChanged(state) {value.state = state;},
      });
      value.idle = () => value.h.until(() => !value.state.busy);
      value.click = label => {assert.equal(!!value.h.button(label).props.disabled, false, label); value.h.click(label);};
      return value;
    }
    let current;
    await step('freeze explicit four-image/typed-field/hotspot input through real Commands', async () => {
      assert.equal(oldRun.acceptance_events.length, expected.initialAccepted);
      assert.equal(oldRun.acceptance_events[0].at_ms, expected.initialAcceptedAtMs);
      const d = structuredClone(initial.draft.definition);
      const p = d.payload.data.presentation;
      p.timeline = expected.order.map((id, i) => ({...p.timeline.find(c => c.id === id), start_progress: expected.ranges[i][0], end_progress: expected.ranges[i][1], transition: expected.transitions[i]}));
      const port = {module_ref: exact('0.1.0'), instance_id: originalProject.devices[0].id, port_id: 'output'};
      for (const [i, field] of expected.fields.entries()) p.tags.push({
        id: `closeout-field-${i}`, label: field, anchor: {kind: 'image_normalized', data: {x: .1, y: .2 + i * .2}}, progress_range: null,
        payload: {kind: 'data', data: {source_field: field, object_ref: {kind: 'device', id: port.instance_id}, port_ref: port, value_type: 'quantity', unit: expected.unit}},
      });
      p.tags.push({id: 'closeout-inspect', label: 'Inspect exact saved draft', anchor: {kind: 'image_normalized', data: {x: .8, y: .2}}, progress_range: null, payload: {kind: 'action', data: 'inspect'}});
      assert.deepEqual(model.presentationIssues(p), []);
      const commit = await change(admin, 1, [{kind: 'put_definition', data: d}]);
      current = await readDraft(admin, commit);
      assert.deepEqual(current.definition, d);
      A = consumer('consumer-A', await readDraft(api(session, 'consumer-A-read'), commit));
      B = consumer('consumer-B', await readDraft(api(session, 'consumer-B-read'), commit));
      assert.equal(A.draft.summary.revision, B.draft.summary.revision);
      assert.equal(A.draft.summary.draft_id, B.draft.summary.draft_id);
      completed.push({check: 'two independent mounted consumers', host_pid: session.child.pid, scope, revision: current.summary.revision});
    });
    await step('A commits; B stale save is rejected without contaminating draft/library/adoption/old Run', async () => {
      const beforeProject = await session.rpc({kind: 'snapshot'});
      A.h.change('图片过渡', 'zoom'); A.click('保存到当前草稿'); await A.idle();
      const winning = structuredClone(A.draft);
      B.h.change('图片过渡', 'pan'); B.click('保存到当前草稿'); await B.idle();
      assert.match(text(B.h.tree), /RevisionConflict|revision|修订/i);
      assert.equal(B.view.presentation.timeline[0].transition, 'pan');
      assert.equal(B.state.dirty, true);
      assert.deepEqual((await readDraft(admin)).definition, winning.definition);
      assert.deepEqual(await session.rpc({kind: 'snapshot'}), beforeProject);
      await assert.rejects(moduleAt(admin, '0.9.0'));
      await untouched();
      B.click('重读保存载荷'); await B.idle();
      B.click('采用我的presentation'); B.click('接受核对基础');
      B.click('保存到当前草稿'); await B.idle();
      assert.equal(B.draft.definition.payload.data.presentation.timeline[0].transition, 'pan');
      completed.push({check: 'stale save rejected, local input retained, explicit reconcile saved', revision: B.draft.summary.revision});
    });
    await step('stale version/undo/redo and stale sample requests do not mutate any winning state', async () => {
      B.h.change('提交版本', '0.2.0'); B.click('明确提交精确版本'); await B.idle();
      assert.equal(B.draft.definition.version, '0.2.0');
      assert.deepEqual((await moduleAt(admin, '0.2.0')).definition, B.draft.definition);
      assert.deepEqual((await session.rpc({kind: 'snapshot'})).project, originalProject, 'freeze never adopts an instance');
      const winning = structuredClone(await readDraft(admin));
      A.h.change('提交版本', '0.9.0'); A.click('明确提交精确版本'); await A.idle();
      assert.match(text(A.h.tree), /RevisionConflict|revision|修订/i);
      assert.equal(A.h.control('提交版本').props.value, '0.9.0');
      const conflict = error => error.detail.code === 'RevisionConflict';
      for (const kind of ['undo', 'redo']) await assert.rejects(change(A.dispatch, A.draft.summary.revision, [{kind}]), conflict);
      const scenario = initial.snapshot.scenarios[0];
      await assert.rejects(A.dispatch(request(scope, 'run_scenario', {scenario_id: scenario.id, input_revision: A.draft.summary.revision, observation_window: scenario.window, intent: {kind: 'definition_cycle', data: {definition_revision: A.draft.summary.revision, test_input: scenario}}})), conflict);
      assert.deepEqual(await readDraft(admin), winning);
      await assert.rejects(moduleAt(admin, '0.9.0'));
      assert.deepEqual((await session.rpc({kind: 'snapshot'})).project, originalProject);
      await untouched();
      completed.push({check: 'stale freeze/history/sample rejected with no winning-state mutation', stale_revision: A.draft.summary.revision, current_revision: winning.summary.revision});
    });
    let adoptedProject, frozen02, frozen03;
    await step('explicit adoption has its own project history; Draft history leaves frozen versions/adoption/Run intact', async () => {
      frozen02 = await moduleAt(admin, '0.2.0');
      const device = originalProject.devices[0];
      await admin(request(projectScope, 'apply_commands', {base_revision: originalProject.revision, commands: [{kind: 'adopt_module', data: {module_ref: exact('0.2.0'), targets: [{kind: 'device', id: device.id}], bindings: device.role_bindings, new_instances: [], explicit_seed: null}}]}));
      let project = (await session.rpc({kind: 'snapshot'})).project;
      assert.deepEqual(project.devices[0].module_ref, exact('0.2.0'));
      await admin(request(projectScope, 'apply_commands', {base_revision: project.revision, commands: [{kind: 'undo'}]}));
      project = (await session.rpc({kind: 'snapshot'})).project;
      assert.deepEqual(project.devices[0].module_ref, exact('0.1.0'));
      await admin(request(projectScope, 'apply_commands', {base_revision: project.revision, commands: [{kind: 'redo'}]}));
      adoptedProject = (await session.rpc({kind: 'snapshot'})).project;
      const connections = structuredClone(adoptedProject.connections);
      for (const connection of connections) for (const side of ['source', 'destination']) {
        const endpoint = connection.data[side];
        if (endpoint.kind === 'port' && endpoint.data.instance_id === device.id) endpoint.data.module_ref = exact('0.2.0');
      }
      const route = structuredClone(adoptedProject.routes[0]);
      for (const item of route.steps) if (item.device_id === device.id) item.operation_ref = exact('0.2.0');
      await admin(request(projectScope, 'apply_commands', {base_revision: adoptedProject.revision, commands: [
        ...connections.map(data => ({kind: 'put_object', data: {kind: 'connection', data}})),
        {kind: 'put_object', data: {kind: 'route', data: route}},
      ]}));
      adoptedProject = (await session.rpc({kind: 'snapshot'})).project;
      assert.equal(B.h.button('撤销草稿').props.disabled, true, 'immutable freeze is a history boundary');
      assert.equal(B.h.button('重做草稿').props.disabled, true);
      B.h.change('图片过渡', 'zoom'); B.click('保存到当前草稿'); await B.idle();
      const firstRevision = B.draft.summary.revision;
      B.click('撤销草稿'); await B.idle(); assert.equal(B.draft.summary.revision, firstRevision + 1);
      B.click('重做草稿'); await B.idle(); assert.equal(B.draft.summary.revision, firstRevision + 2);
      B.click('撤销草稿'); await B.idle();
      B.h.change('图片过渡', 'hold'); B.click('保存到当前草稿'); await B.idle();
      assert.equal(B.draft.history.can_redo, false);
      const beforeRedo = await readDraft(admin);
      await assert.rejects(change(admin, B.draft.summary.revision, [{kind: 'redo'}]), /history/i);
      assert.deepEqual(await readDraft(admin), beforeRedo);
      B.h.change('提交版本', '0.3.0'); B.click('明确提交精确版本'); await B.idle();
      frozen03 = await moduleAt(admin, '0.3.0');
      assert.deepEqual(frozen03.definition, B.draft.definition);
      assert.deepEqual(await moduleAt(admin, '0.2.0'), frozen02);
      assert.deepEqual((await session.rpc({kind: 'snapshot'})).project, adoptedProject, '0.3 freeze remains unadopted; Draft undo does not undo project adoption');
      const p = frozen03.definition.payload.data.presentation;
      assert.deepEqual(p.timeline.map(c => c.id), expected.order);
      assert.deepEqual(p.timeline.map(c => [c.start_progress, c.end_progress]), expected.ranges);
      assert.deepEqual(p.timeline.map(c => c.transition), expected.transitions);
      const hotspot = p.tags.find(t => t.id === 'closeout-inspect');
      assert.equal(B.view.canAction(hotspot), true);
      B.view.onAction(hotspot); B.h.render(); await B.idle(); assert.equal(B.inspections, 1);
      assert.equal(B.view.canAction({...hotspot, id: 'not-saved'}), false);
      await untouched();
      completed.push({check: 'monotonic independent histories, redo branch, immutable0.2 and unadopted0.3', draft_revision: B.draft.summary.revision, project_revision: adoptedProject.revision});
    });
    await step('save, close Host, actually move own directory, independent new process reads all exact versions and original Run/media', async () => {
      const saved = fileURLToPath(new URL(`saved-${session.child.pid}/`, out));
      const moved = fileURLToPath(new URL(`moved-${session.child.pid}/`, out));
      const selection = await session.rpc({kind: 'select_directory', purpose: 'save_project', path: saved});
      const task = await admin(request(projectScope, 'save_project', {revision: adoptedProject.revision, selected_directory_ref: selection}));
      const reply = await session.rpc({kind: 'await_task', scope: projectScope, task_id: task.id});
      assert.equal(reply.outcome.data.task.state, 'completed');
      const firstPid = session.child.pid;
      A.h.unmount(); A = null; B.h.unmount(); B = null;
      await session.close('same-host-two-consumers'); session = null;
      assert.ok([saved, moved].every(path => path.startsWith(fileURLToPath(out))), 'both move endpoints are inside this test delivery');
      await rename(saved, moved);
      await assert.rejects(readFile(new URL(`saved-${firstPid}/project.json`, out)), {code: 'ENOENT'});
      reopened = await start(fixture, {reopen: moved});
      assert.notEqual(reopened.child.pid, firstPid);
      const read = api(reopened, 'independent-reader');
      assert.deepEqual(reopened.ready.formal_run, oldRun);
      assert.deepEqual(reopened.ready.snapshot.project, adoptedProject);
      assert.deepEqual(reopened.ready.package, frozen02);
      assert.equal(reopened.ready.draft.history.can_undo, false);
      assert.equal(reopened.ready.draft.history.can_redo, false);
      assert.deepEqual(await moduleAt(read, '0.1.0'), oldModule);
      assert.deepEqual(await moduleAt(read, '0.2.0'), frozen02);
      assert.deepEqual(await moduleAt(read, '0.3.0'), frozen03);
      await assert.rejects(moduleAt(read, '0.9.0'));
      const context = reopened.ready.context;
      const p = frozen03.definition.payload.data.presentation;
      const portResult = await reopened.rpc({kind: 'port_output', project_id: projectId, scope: projectScope,
        scope_revision: context.project.revision, run_id: oldRun.id, at_ms: 60000,
        port: p.tags.find(t => t.id === 'closeout-field-0').payload.data.port_ref});
      assert.equal(portResult.ok, true, JSON.stringify(portResult));
      assert.deepEqual(portResult.data.completed_output, {value: {coefficient: 1, decimal_places: 0}, unit: 'piece'});
      assert.deepEqual(portResult.data.accepted_output, {value: {coefficient: 1, decimal_places: 0}, unit: 'piece'});
      for (const clip of p.timeline) {
        const reply = await reopened.rpc({kind: 'media', project_id: projectId, scope: projectScope, scope_revision: context.project.revision, source: {kind: 'module_version', data: {module_ref: exact('0.3.0')}}, asset: clip.source.data});
        assert.equal(reply.ok, true, JSON.stringify(reply));
        assert.deepEqual(Buffer.from(reply.bytes), await readFile(new URL(`modules/devices/manual-assembly/presentation/p303-owned/${clip.source.data.path}`, root)));
      }
      assert.deepEqual(p.tags.filter(t => t.payload.kind === 'data').map(t => t.payload.data.source_field), expected.fields);
      for (const tag of p.tags.filter(t => t.payload.kind === 'data')) assert.equal(tag.payload.data.unit, expected.unit);
      completed.push({check: 'save/move/destroy/new-process exact readback', first_pid: firstPid, independent_pid: reopened.child.pid, moved, versions: ['0.1.0', '0.2.0', '0.3.0'], accepted_at_ms: oldRun.acceptance_events[0].at_ms});
    });
    await step('project switch rejects the captured old Draft scope and leaves the saved old project unchanged', async () => {
      const captured = reopened.ready.draft;
      const moved = completed.find(row => row.moved).moved;
      const savedManifest = await readFile(join(moved, 'project.json'));
      const switched = await reopened.rpc({kind: 'new_project'});
      assert.notEqual(switched.snapshot.project.id, projectId);
      const before = structuredClone(switched.snapshot);
      const old = api(reopened, 'captured-old-project');
      await assert.rejects(old(request({kind: 'definition_draft', data: captured.summary.draft_id}, 'apply_commands', {
        base_revision: captured.summary.revision, commands: [{kind: 'put_definition', data: captured.definition}],
      })), /project|工程/i);
      assert.deepEqual(await reopened.rpc({kind: 'snapshot'}), before);
      assert.deepEqual(await readFile(join(moved, 'project.json')), savedManifest);
      completed.push({check: 'old project/scope denied after switch; new project and old saved manifest unchanged', new_project: before.project.id});
    });
  } finally {
    A?.h.unmount(); B?.h.unmount();
    for (const [name, value] of [['same-host-failed', session], ['independent-reopened', reopened]]) if (value) await value.close(name);
    await writeFile(new URL('same-host-closeout-results.json', out), JSON.stringify({evidence_tier: 'actual ProjectHost/Commands/Engine plus two finite mounted production editor handlers; not GUI/Native', expected, completed}, null, 2));
  }
});
