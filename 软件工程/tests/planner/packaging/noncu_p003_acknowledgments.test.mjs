// Actual component SSR; these protocol records are not human or Host evidence.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { renderFixture } from '../../../.work/noncu-p003-view/view-fixture.mjs';
import { artifactAcknowledgments } from '../../../apps/planner/src/features/packaging/model.ts';
const json = async path => JSON.parse(await readFile(new URL(`../../../${path}`, import.meta.url), 'utf8'));
const docs = await json('packages/commands/tests/fixtures/production/professional-documents.json');
const sourceProject = (await json('packages/commands/tests/fixtures/comparison/actual-frozen-inputs.json'))[0][0];
const input = (await json('modules/professional/paper-packaging-ui/sliding-sleeve-explicit-candidate.job.json')).data;

test('saved acknowledgments require an explicit exact-version selection, never the first matching Artifact record', () => {
  const project = structuredClone(sourceProject);
  const artifact = docs.artifacts.find(a => a.id === docs.results[0].artifact_id);
  const acknowledgment = (id, subject) => ({ id, artifact_ref: artifact.id, accepted_subject: subject,
    accepted_scope: ['design_reference'], decision_source: { kind: 'human_statement', data: 'Finite selection fixture only' }, conditions: [] });
  project.acknowledgments = [
    acknowledgment('ACK-FIRST', artifact.subject_ref),
    acknowledgment('ACK-OTHER-CHOICE', artifact.subject_ref),
    acknowledgment('ACK-WRONG-REVISION', { ...artifact.subject_ref, revision: 'r999' }),
  ];
  const html = renderFixture('project', { project, docs, input });
  assert.match(html, /选择已保存的用途认可/);
  assert.match(html, /option value="ACK-FIRST"/);
  assert.match(html, /option value="ACK-OTHER-CHOICE"/);
  assert.doesNotMatch(html, /option value="ACK-WRONG-REVISION"/);
  assert.match(html, /尚未明确选择此成果的用途认可/);
  assert.doesNotMatch(html, /实际认可：ACK-FIRST/);
});

test('recognition choices belong to the exact Artifact and subject revision and preserve conditional records for Host review', () => {
  const artifact = docs.artifacts.find(a => a.id === docs.results[0].artifact_id);
  const make = (id, artifactRef, revision) => ({ id, artifact_ref: artifactRef,
    accepted_subject: { ...artifact.subject_ref, revision }, accepted_scope: ['design_reference'],
    decision_source: { kind: 'human_statement', data: 'Finite choice only' },
    conditions: [{ code: 'needs_review', message: 'Conditional acknowledgment remains conditional', blocks: ['execution'] }] });
  const project = { ...sourceProject, acknowledgments: [
    make('EXACT', artifact.id, artifact.subject_ref.revision),
    make('WRONG-ARTIFACT', 'another-artifact', artifact.subject_ref.revision),
    make('WRONG-REVISION', artifact.id, 'r999'),
  ] };
  const before = structuredClone(project);
  assert.deepEqual(artifactAcknowledgments(project, { artifact }).map(a => a.id), ['EXACT']);
  assert.deepEqual(artifactAcknowledgments(project, { artifact })[0].conditions, before.acknowledgments[0].conditions);
  assert.deepEqual(project, before);
});

test('a cached acknowledgment is unavailable when the refreshed Project no longer contains that saved record', () => {
  const artifact = docs.artifacts.find(a => a.id === docs.results[0].artifact_id);
  const acknowledgment = { id: 'REMOVED', artifact_ref: artifact.id, accepted_subject: artifact.subject_ref,
    accepted_scope: ['design_reference'], decision_source: { kind: 'human_statement', data: 'Finite refreshed-project fixture' }, conditions: [] };
  const project = { ...sourceProject, acknowledgments: [acknowledgment] };
  assert.deepEqual(artifactAcknowledgments(project, { artifact }), [acknowledgment]);
  const afterRefresh = { ...project, acknowledgments: [] };
  assert.deepEqual(artifactAcknowledgments(afterRefresh, { artifact }), []);
  assert.deepEqual(project.acknowledgments, [acknowledgment]);
});
