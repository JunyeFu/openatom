import { readFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import test from 'node:test';
import assert from 'node:assert/strict';

const source = await readFile(new URL('../../../apps/planner/src/services/windowClose.ts', import.meta.url), 'utf8');
const body = stripTypeScriptTypes(source, { mode: 'transform' })
  .replace(/^import .*;\r?\n/gm, '')
  .replace('export function useWindowClose', 'function useWindowClose');

function harness({ afterSnapshot = () => {}, unsaved = false, local = false, confirmResult = 'stay', confirmAction = () => {}, saveAction = state => { state.unsaved = false; } } = {}) {
  const state = { project_id: 'close-test', project_revision: 1, definition_revisions: [], active_tasks: [], unsaved };
  const errors = [];
  let callback;
  let destroyed = 0;
  let confirmations = 0;
  let finalAdmissions = 0;
  let saves = 0;
  const desktop = {
    onCloseRequested(handler) { callback = handler; return Promise.resolve(() => {}); },
    async destroy() { destroyed++; },
  };
  let reads = 0;
  const invoke = async (command, args) => {
    if (command === 'project_close_state') {
      const snapshot = structuredClone(state);
      if (++reads === 1) afterSnapshot(state);
      return snapshot;
    }
    if (command === 'project_finish_close') {
      finalAdmissions++;
      // The native boundary is isolated here; its real shared-Host admission is
      // tested in Rust. State changes deliberately precede the final request.
      if (JSON.stringify(state) !== JSON.stringify(args.expected) || state.active_tasks.length) {
        throw new Error('Closing state changed; current data retained');
      }
      if (state.unsaved && !args.discardConfirmed) throw new Error('Unconfirmed unsaved data');
      await desktop.destroy();
      return;
    }
    throw new Error(`Unexpected command: ${command}`);
  };
  const useWindowClose = new Function('useEffect', 'useRef', 'getCurrentWindow', 'invoke', 'confirm', 'window',
    `${body}\nreturn useWindowClose;`)(
      effect => effect(), value => ({ current: value }), () => desktop, invoke,
      async () => { confirmations++; confirmAction(state); return confirmResult; },
      { __TAURI_INTERNALS__: {} },
    );
  useWindowClose({ hasUnappliedInput: local, workInProgress: false, contextKey: 'close-test:1',
    confirmClose: async () => { confirmations++; confirmAction(state); return confirmResult; },
    save: async () => { saves++; await saveAction(state); },
    onError: error => errors.push(error.message) });
  return { state, errors, async close() { await callback({ preventDefault() {} }); },
    get destroyed() { return destroyed; }, get confirmations() { return confirmations; }, get saves() { return saves; }, get finalAdmissions() { return finalAdmissions; } };
}

test('an external edit after a clean close snapshot must not be lost', async () => {
  const h = harness({ afterSnapshot(state) { state.project_revision = 2; state.unsaved = true; } });
  await h.close();
  assert.equal(h.destroyed, 0, 'new Host revision must keep the actual window alive');
  assert.equal(h.state.project_revision, 2);
  assert.equal(h.state.unsaved, true);
  assert.equal(h.errors.length, 1);
});

test('an import admitted after a clean snapshot must keep the window alive', async () => {
  const h = harness({ afterSnapshot(state) { state.active_tasks.push('actual-import-task'); } });
  await h.close();
  assert.equal(h.destroyed, 0, 'new active task must be checked at the native close boundary');
  assert.equal(h.errors.length, 1);
});

test('an unchanged clean window closes normally', async () => {
  const h = harness();
  await h.close();
  assert.equal(h.destroyed, 1);
  assert.equal(h.confirmations, 0);
  assert.deepEqual(h.errors, []);
});

test('canceling dirty close preserves the window and unsaved data', async () => {
  const h = harness({ unsaved: true });
  await h.close();
  assert.equal(h.destroyed, 0);
  assert.equal(h.state.unsaved, true);
  assert.equal(h.confirmations, 1);
});

test('explicit dirty close succeeds only for the unchanged context', async () => {
  const h = harness({ unsaved: true, confirmResult: 'discard' });
  await h.close();
  assert.equal(h.destroyed, 1);
  assert.equal(h.confirmations, 1);
  assert.deepEqual(h.errors, []);
});

test('a revision change during dirty confirmation invalidates that confirmation', async () => {
  const h = harness({ unsaved: true, confirmResult: 'discard', confirmAction(state) { state.project_revision = 2; } });
  await h.close();
  assert.equal(h.destroyed, 0);
  assert.equal(h.errors.length, 1);
});

test('save choice persists before a clean atomic close', async () => {
  const h = harness({ unsaved: true, confirmResult: 'save' });
  await h.close();
  assert.equal(h.saves, 1);
  assert.equal(h.state.unsaved, false);
  assert.equal(h.destroyed, 1);
  assert.deepEqual(h.errors, []);
});

test('failed or canceled save keeps the window and input', async () => {
  const h = harness({ unsaved: true, confirmResult: 'save', saveAction() { throw new Error('save canceled'); } });
  await h.close();
  assert.equal(h.destroyed, 0);
  assert.equal(h.state.unsaved, true);
  assert.match(h.errors[0], /save canceled/);
});

test('save cannot discard unapplied local input', async () => {
  const h = harness({ local: true, confirmResult: 'save' });
  await h.close();
  assert.equal(h.saves, 0);
  assert.equal(h.destroyed, 0);
  assert.equal(h.errors.length, 1);
});

test('remaining dirty definitions after save keep the window', async () => {
  const h = harness({ unsaved: true, confirmResult: 'save', saveAction() {} });
  await h.close();
  assert.equal(h.saves, 1);
  assert.equal(h.destroyed, 0);
  assert.equal(h.state.unsaved, true);
});

test('a concurrent revision change during save cannot be closed by its earlier choice', async () => {
  const h = harness({ unsaved: true, confirmResult: 'save', saveAction(state) { state.project_revision = 2; state.unsaved = false; } });
  await h.close();
  assert.equal(h.destroyed, 0);
  assert.equal(h.errors.length, 1);
});
