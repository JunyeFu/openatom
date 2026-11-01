// Actual consumer list loader + finite producer responses, not Native/Host proof.
import test from 'node:test';
import assert from 'node:assert/strict';
import { loadStoredArtifactList, HostFailure } from '../../../scenarios/专业桌面/runtime/view-build/view-fixture.mjs';

test('actual one-shot list consumer uses only the provider IDs, including an explicitly empty Scope', async () => {
  const states = []; let calls = 0;
  await loadStoredArtifactList(async () => { calls++; return ['DRAFT-ONLY-F18']; }, () => true, state => states.push(state));
  assert.equal(calls, 1);
  assert.deepEqual(states, [{ status: 'loading', ids: [] }, { status: 'ready', ids: ['DRAFT-ONLY-F18'] }]);
  await loadStoredArtifactList(async () => [], () => true, state => states.push(state));
  assert.deepEqual(states.at(-1), { status: 'ready', ids: [] });
});

test('actual list failure clears IDs and preserves original Host conflict detail for explicit display', async () => {
  const detail = { code: 'RevisionConflict', message: 'finite exact Scope conflict', expected_revision: 3, actual_revision: 4, object_refs: [], field_errors: [{ pointer: '/scope_revision', reason: 'finite exact Scope conflict' }] };
  const failure = new HostFailure(detail);
  const states = [];
  await loadStoredArtifactList(async () => { throw failure; }, () => true, state => states.push(state));
  assert.equal(states.at(-1).status, 'failed');
  assert.deepEqual(states.at(-1).ids, []);
  assert.ok(states.at(-1).error.includes(JSON.stringify(detail, null, 2)));
  assert.equal(failure.detail, detail);
});

test('late successful and failed reads cannot publish after Scope/revision switch or unmount', async () => {
  for (const rejectLate of [false, true]) {
    let current = true; let finish;
    const states = [];
    const pending = loadStoredArtifactList(() => new Promise((resolve, reject) => { finish = () => rejectLate ? reject(new Error('old Scope denied')) : resolve(['OLD-SCOPE']); }), () => current, state => states.push(state));
    current = false; finish(); await pending;
    assert.deepEqual(states, [{ status: 'loading', ids: [] }]);
  }
  let called = false;
  await loadStoredArtifactList(async () => { called = true; return []; }, () => false, () => assert.fail('unmounted publication'));
  assert.equal(called, false);
});
