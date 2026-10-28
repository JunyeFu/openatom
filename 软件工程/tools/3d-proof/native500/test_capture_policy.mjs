import test from 'node:test';
import assert from 'node:assert/strict';
import {capturePlan} from './capture-policy.mjs';

const entries = ['a', 'b'].flatMap(id => ['detailed', 'simple'].map(suffix => ({id, suffix, batch: 'B02'})));
entries.push({id: 'other', suffix: 'detailed', batch: 'B01'});

test('changed-device selection retains both LODs without recapturing another device', () => {
  const plan = capturePlan(entries, 'B02', ['b']);
  assert.deepEqual(plan.entries.map(e => [e.id, e.suffix]), [['b', 'detailed'], ['b', 'simple']]);
});
test('an empty device selector captures the current batch only', () => {
  assert.equal(capturePlan(entries, 'B02').entries.length, 4);
});
test('lean capture uses one mechanism view and full capture remains explicit', () => {
  assert.deepEqual(capturePlan(entries, 'B02').closeupProgress, [.5]);
  assert.deepEqual(capturePlan(entries, 'B02', [], true).closeupProgress, [0, .5, 1]);
});
test('a misspelled or out-of-batch selector cannot silently capture everything', () => {
  assert.throws(() => capturePlan(entries, 'B02', ['other']), /outside current batch/);
  assert.throws(() => capturePlan(entries, 'B02', ['typo']), /outside current batch/);
  assert.throws(() => capturePlan(entries, 'unknown'), /No models selected/);
});
