import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { source, rootUrl } from './load-source.mjs';
const {formSchemas} = await source('modules', 'formSchema'); const {shapeIssues} = await source('modules', 'model');
const helpers = await source('modules', 'templateComposition');
const fixture = async name => JSON.parse(await readFile(new URL(`tools/type-contract-proof/fixtures/${name}.json`, rootUrl), 'utf8')).data;
const scenario = await fixture('scenario');
const definition = (await fixture('module-templates')).definition;
const draft = { summary: { draft_id: 'D-source', revision: 11, definition_ref: { id: definition.id, version: definition.version }, issues: [] }, definition, history: null };
function fullSource() {
  const input = structuredClone(scenario); input.id = 'explicit-source'; input.revision = 4;
  input.orders = [structuredClone(input.orders[0]), { ...structuredClone(input.orders[0]), id: 'second-order' }];
  input.arrivals = [{ id: 'arrival-1', at_ms: 1000, destination: 'S1', release_policy: 'unchecked_allowed', items: structuredClone(input.initial_conditions.items) }, { id: 'arrival-2', at_ms: 2000, destination: 'S1', release_policy: 'unchecked_allowed', items: structuredClone(input.initial_conditions.items) }];
  return input;
}
test('explicit complete seed captures all three exact bodies and original Scenario id/revision, independent of Project/Draft revisions', () => {
  const selected = fullSource(), original = structuredClone(selected);
  const captured = helpers.captureCompleteSeed(helpers.emptyComposition(), selected);
  assert.deepEqual(captured.source_scenario, { scenario_id: 'explicit-source', revision: 4 });
  assert.deepEqual(shapeIssues(formSchemas.TemplateSubset,captured.subset),[]);
  for (const key of ['initial_conditions', 'arrivals', 'orders']) assert.deepEqual(captured.subset.seed[key], selected[key]);
  const input = helpers.compositionInput('P1', { ...captured, base_revision: 7 }, definition, draft);
  assert.equal(input.base_revision, 7); assert.equal(input.draft.summary.revision, 11); assert.equal(input.source_scenario.revision, 4);
  selected.revision = 9; selected.orders[0].id = 'newer-edited-order'; selected.arrivals.length = 0;
  assert.deepEqual(captured.subset.seed.orders, original.orders); assert.deepEqual(captured.subset.seed.arrivals, original.arrivals);
  assert.equal(helpers.compositionInput('P1', { ...captured, base_revision: 7 }, definition, draft).source_scenario.revision, 4);
});
test('a merely selected candidate creates neither seed nor outgoing source field; clearing removes both', () => {
  const candidate = fullSource(); // Selection alone is not the explicit capture action.
  const empty = { subset: helpers.emptyComposition(), base_revision: 7, source_scenario: null };
  const unselected = helpers.compositionInput('P1', empty, definition, draft);
  assert.equal(unselected.subset.seed, null); assert.equal(Object.hasOwn(unselected, 'source_scenario'), false); assert.equal(candidate.revision, 4);
  const captured = helpers.captureCompleteSeed(empty.subset, candidate), cleared = helpers.clearCompositionSeed(captured.subset);
  assert.equal(cleared.subset.seed, null); assert.equal(cleared.source_scenario, null);
  assert.equal(Object.hasOwn(helpers.compositionInput('P1', { ...cleared, base_revision: 7 }, definition, draft), 'source_scenario'), false);
  assert.ok(captured.subset.seed); assert.equal(captured.source_scenario.revision, 4);
});
test('old sidecar reference is preserved exactly; an absent reference is never guessed from matching bodies', () => {
  const captured = helpers.captureCompleteSeed(helpers.emptyComposition(), fullSource());
  const preview = { subset: captured.subset, base_revision: 13, source_scenario: { scenario_id: 'stored-old-source', revision: 2 }, issues: [{ code: 'template.unsupported_context', field_path: '/seed_context', message: 'Actual unsupported source condition', blocks: [] }] };
  const read = helpers.compositionFromPreview(preview);
  assert.deepEqual(read.source_scenario, { scenario_id: 'stored-old-source', revision: 2 }); assert.equal(read.base_revision, 13); assert.deepEqual(read.issues, preview.issues);
  preview.source_scenario.revision = 8; assert.equal(read.source_scenario.revision, 2);
  const historical = helpers.compositionFromPreview({ ...preview, source_scenario: undefined });
  assert.equal(historical.source_scenario, null); assert.deepEqual(historical.subset.seed, captured.subset.seed);
  assert.throws(() => helpers.compositionInput('P1', historical, definition, draft), /没有实际来源引用/);
  assert.equal(historical.source_scenario, null);
});
test('adoption supply and demand choices remain independent after complete creation capture', () => {
  const selected = fullSource(), complete = helpers.captureCompleteSeed(helpers.emptyComposition(), selected);
  assert.deepEqual(complete.subset.seed.orders, selected.orders); assert.deepEqual(complete.subset.seed.arrivals, selected.arrivals);
  const supply = helpers.declaredSeed(selected, true, false), demand = helpers.declaredSeed(selected, false, true);
  assert.deepEqual(supply.orders, []); assert.deepEqual(supply.arrivals, selected.arrivals); assert.deepEqual(supply.initial_conditions, selected.initial_conditions);
  assert.deepEqual(demand.orders, selected.orders); assert.deepEqual(demand.arrivals, []); assert.deepEqual(demand.initial_conditions.items, []);
  assert.equal(helpers.declaredSeed(selected, false, false), null);
});
