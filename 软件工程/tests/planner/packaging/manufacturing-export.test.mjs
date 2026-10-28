// Finite consumer protocol fixtures; no actual Host, Native or human permission proof.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as model from '../../../apps/planner/src/features/packaging/model.ts';
const target = { projectId: 'P', scope: { kind: 'project' }, revision: 7 };
const evidence = { artifact: { id: 'ART-EXACT', input_ref: { path: 'job-r1.json', kind: 'packaging_job' }, result_ref: { path: 'result-r1.json', kind: 'packaging_result' } }, result: { artifact_id: 'ART-EXACT', job_ref: { id: 'JOB', revision: 1 }, manufacturing_files: [
  { part_id: 'PART-A', file: { path: 'manufacturing/part-a.svg', kind: 'asset' }, layers: ['cut', 'fold', 'glue'], unit: 'millimeter' },
  { part_id: 'PART-B', file: { path: 'manufacturing/part-b.svg', kind: 'asset' }, layers: ['cut', 'fold', 'glue'], unit: 'millimeter' },
] } };
const rows = () => evidence.result.manufacturing_files.map(({ file }, index) => ({ artifact_id: evidence.artifact.id, file: structuredClone(file), source: `Explicit fixture owner ${index}`, license: 'Explicit fixture permission for this delivery' }));
const handle = where => ({ id: 'actual-export-handle', kind: 'exporting', state: 'queued', input_revision: where.revision, scope: { kind: 'edit', data: where.scope.kind === 'project' ? { kind: 'project', data: where.projectId } : { kind: 'definition_draft', data: { id: where.scope.data, revision: where.revision } } } });
function fixture(where = target) {
  const requests = []; let picks = 0;
  const output = { package: { path: 'selected.zip', kind: 'project' }, included_files: evidence.result.manufacturing_files.map(value => value.file), missing: [{ code: 'original-third-party-missing', message: 'Original Host detail', blocks: ['preview'] }] };
  const io = { dispatch: async () => { throw new Error('No public export or automatic acknowledgment'); }, pickDirectory: async purpose => { assert.equal(purpose, 'export_package'); picks++; return { selection_id: 'chosen-native-fixture', purpose: 'export_package' }; }, awaitTask: async (task, onState) => { onState({ task: { ...task, state: 'completed' }, originating_operation: 'export_package' }); return { kind: 'package', data: output }; } };
  return { io, requests, output, picks: () => picks, producer: { status: 'available', start: async request => { requests.push(request); return handle(where); } } };
}
const run = (f, declarations = rows(), confirmed = true, guard = () => {}, where = target, item = evidence, publish = () => {}) => model.exportDeclaredManufacturingEvidence(f.io, where, item, declarations, confirmed, f.producer, publish, guard);

test('current exact files start the private producer with captured ScopeR and original Task result, leaving source declarations untouched', async () => {
  const f = fixture(); const source = structuredClone(evidence); const declarations = rows();
  assert.deepEqual(await run(f, declarations), f.output);
  assert.equal(f.requests.length, 1);
  assert.deepEqual(f.requests[0], { project_id: 'P', scope: { kind: 'project' }, scope_revision: 7, params: { revision: 7, selected_destination_ref: { selection_id: 'chosen-native-fixture', purpose: 'export_package' }, package_scope: { purpose: 'private_project', scenarios: [], run_ids: [], artifact_ids: ['ART-EXACT'], module_refs: [], include_redistributable_assets: true } }, declarations, confirm_redistribution: true });
  assert.deepEqual(evidence, source);
});
test('empty default declarations have no invented license and unconfirmed/mismatched/missing/duplicate files never open a picker', async () => {
  assert.ok(model.emptyManufacturingDeclarations(evidence).every(row => row.source === '' && row.license === ''));
  for (const [declarations, confirmed] of [[rows(), false], [[], true], [rows().slice(0, 1), true], [[...rows(), rows()[0]], true], [rows().map(row => ({ ...row, license: '' })), true], [rows().map(row => ({ ...row, artifact_id: 'OTHER' })), true], [rows().map(row => ({ ...row, file: { ...row.file, kind: 'project' } })), true]]) {
    const f = fixture(); await assert.rejects(run(f, declarations, confirmed)); assert.equal(f.picks(), 0); assert.equal(f.requests.length, 0);
  }
});
test('unwired producer and cancelled real selection start no Task and do not grant permission', async () => {
  const f = fixture(); f.producer = { status: 'unavailable', reason: 'Native not connected' };
  await assert.rejects(run(f), /Native not connected/); assert.equal(f.picks(), 0);
  const cancelled = fixture(); cancelled.io.pickDirectory = async () => null;
  assert.equal(await run(cancelled), null); assert.equal(cancelled.requests.length, 0);
});
test('Scope/revision/Artifact/File identity changes before picker completion cannot apply old declarations', async () => {
  for (const change of ['session', 'Project', 'Scope', 'R', 'Artifact', 'File', 'unmount']) {
    const f = fixture(); let current = true;
    f.io.pickDirectory = async () => { current = false; return { selection_id: 'late-picker', purpose: 'export_package' }; };
    await assert.rejects(run(f, rows(), true, () => { if (!current) throw new Error(`changed ${change}`); }), /changed/);
    assert.equal(f.requests.length, 0);
  }
  const baseline = model.manufacturingExportIdentity('session', target, evidence);
  assert.notEqual(baseline, model.manufacturingExportIdentity('other-session', target, evidence));
  for (const next of [{ ...target, projectId: 'OTHER' }, { ...target, revision: 8 }, { ...target, scope: { kind: 'definition_draft', data: 'D' } }]) assert.notEqual(baseline, model.manufacturingExportIdentity('session', next, evidence));
  const changed = structuredClone(evidence); changed.result.manufacturing_files[0].file.path = 'other.svg';
  assert.notEqual(baseline, model.manufacturingExportIdentity('session', target, changed));
});
test('declarations and target are frozen before asynchronous picker returns', async () => {
  const f = fixture(); const declarations = rows(); const original = structuredClone(declarations); const where = structuredClone(target);
  f.io.pickDirectory = async () => { declarations[0].license = 'later edit'; where.revision = 999; return { selection_id: 'captured-choice', purpose: 'export_package' }; };
  await run(f, declarations, true, () => {}, where);
  assert.deepEqual(f.requests[0].declarations, original); assert.equal(f.requests[0].scope_revision, 7);
});
test('original Host failure/terminal cancellation and wrong Task Scope propagate without fake success or another public operation', async () => {
  const f = fixture(); const failure = { code: 'RevisionConflict', message: 'Exact Host conflict', expected_revision: 7, actual_revision: 8 };
  f.producer.start = async () => { throw failure; }; await assert.rejects(run(f), error => error === failure);
  const cancelled = fixture(); cancelled.io.awaitTask = async () => { throw failure; }; await assert.rejects(run(cancelled), error => error === failure);
  const wrong = fixture(); wrong.producer.start = async () => handle({ ...target, projectId: 'OTHER' }); await assert.rejects(run(wrong), /Scope/);
  const late = fixture(); let active = true; let published = 0;
  late.io.awaitTask = async (task, publish) => { active = false; publish({ task, originating_operation: 'export_package' }); return { kind: 'package', data: late.output }; };
  await assert.rejects(run(late, rows(), true, () => { if (!active) throw new Error('old response'); }, target, evidence, () => published++), /old response/); assert.equal(published, 0);
});
test('committed DefinitionDraft uses exact actual module/version and DraftR; dirty or changed Draft stops before selection', async () => {
  const where = { ...target, scope: { kind: 'definition_draft', data: 'DRAFT' }, revision: 3 };
  for (const state of ['saved', 'dirty', 'changed']) {
    const f = fixture(where);
    f.io.dispatch = async request => { assert.equal(request.operation, 'get_context'); return { definition: { draft_id: 'DRAFT', revision: state === 'changed' ? 4 : 3, definition_ref: { id: 'module', version: '2.0.0' }, issues: state === 'dirty' ? [{ code: 'definition.unsaved_changes' }] : [] } }; };
    if (state === 'saved') { await run(f, rows(), true, () => {}, where); assert.deepEqual(f.requests[0].params.package_scope.module_refs, [{ id: 'module', version: '2.0.0' }]); assert.equal(f.requests[0].params.package_scope.purpose, 'private_module'); assert.equal(f.requests[0].scope_revision, 3); }
    else { await assert.rejects(run(f, rows(), true, () => {}, where)); assert.equal(f.picks(), 0); }
  }
});
test('original unlicensed export still sends include=false and preserves the actual missing-file receipt', async () => {
  const f = fixture(); const requests = [];
  f.io.dispatch = async request => { requests.push(request); return handle(target); };
  f.io.awaitTask = async () => ({ kind: 'package', data: f.output });
  assert.equal(await model.exportEvidence(f.io, target, evidence, () => {}), f.output);
  assert.equal(requests[0].operation, 'export_package');
  assert.equal(requests[0].params.package_scope.include_redistributable_assets, false);
  assert.equal(f.requests.length, 0);
});
