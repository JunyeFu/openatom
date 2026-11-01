import test from 'node:test';
import assert from 'node:assert/strict';
import { registerHooks } from 'node:module';

// Execute the real hook with only the desktop/React boundary replaced.
registerHooks({
  resolve(specifier, context, next) {
    const sources = {
      react: 'export const useRef=x=>globalThis.closeTest.ref={current:x}; export const useEffect=f=>{globalThis.closeTest.cleanup=f()}',
      '@tauri-apps/api/window': 'export const getCurrentWindow=()=>globalThis.closeTest.desktop',
      '@tauri-apps/api/core': 'export const invoke=(...args)=>globalThis.closeTest.invoke(...args)',
      '@tauri-apps/plugin-dialog': 'export const confirm=(...args)=>globalThis.closeTest.confirm(...args)',
    };
    return specifier in sources ? { url: `data:text/javascript,${encodeURIComponent(sources[specifier])}`, shortCircuit: true } : next(specifier, context);
  },
});
const { useWindowClose } = await import('../../apps/planner/src/services/windowClose.ts');

function setup({ local = false, unsaved = false, busy = false, tasks = [] } = {}) {
  const fixture = globalThis.closeTest = {
    errors: [], destroyed: 0, confirmations: 0,
    context: { project_id: 'P1', project_revision: 2, definition_revisions: [], unsaved, active_tasks: tasks },
    async invoke(command) { assert.equal(command, 'project_close_state'); return structuredClone(this.context); },
    async confirm() { this.confirmations++; return false; },
    desktop: {
      async destroy() { fixture.destroyed++; },
      async onCloseRequested(handler) { fixture.handler = () => handler({ preventDefault() {} }); return () => {}; },
    },
  };
  globalThis.window = { __TAURI_INTERNALS__: {} };
  useWindowClose({ hasUnappliedInput: local, workInProgress: busy, contextKey: 'P1:r2', onError: error => fixture.errors.push(error.message) });
  return fixture;
}

test('unapplied field close cancellation retains the window', async () => {
  const f = setup({ local: true }); await f.handler();
  assert.equal(f.confirmations, 1); assert.equal(f.destroyed, 0); assert.deepEqual(f.errors, []);
});
test('Host unsaved revision also requires a cancellable confirmation', async () => {
  const f = setup({ unsaved: true }); await f.handler();
  assert.equal(f.confirmations, 1); assert.equal(f.destroyed, 0);
});
test('saved clean session closes without a discard confirmation', async () => {
  const f = setup(); await f.handler();
  assert.equal(f.confirmations, 0); assert.equal(f.destroyed, 1);
});
test('busy operation or Host task blocks destruction', async () => {
  for (const options of [{ busy: true }, { tasks: ['task-1'] }]) {
    const f = setup(options); await f.handler();
    assert.equal(f.destroyed, 0); assert.equal(f.confirmations, 0); assert.equal(f.errors.length, 1);
  }
});
test('revision changing during a confirmation blocks destruction', async () => {
  const f = setup({ local: true });
  f.confirm = async () => { f.context.project_revision++; return true; };
  await f.handler(); assert.equal(f.destroyed, 0); assert.match(f.errors[0], /修订或任务状态已变化/);
});
test('editor context changing during a confirmation blocks destruction', async () => {
  const f = setup({ local: true });
  f.confirm = async () => { f.ref.current = { ...f.ref.current, contextKey: 'P2:r1' }; return true; };
  await f.handler(); assert.equal(f.destroyed, 0); assert.match(f.errors[0], /修订或任务状态已变化/);
});
test('overlapping close requests perform one check and one destruction', async () => {
  const f = setup(); let release;
  f.invoke = () => new Promise(resolve => { release = () => resolve(f.context); });
  const first = f.handler(); await f.handler(); release(); await first;
  assert.equal(f.destroyed, 1);
});
test('unresolved Native dialog is never treated as acceptance; async cancellation keeps window', async () => {
  const f = setup({ local: true }); let release;
  f.confirm = () => new Promise(resolve => { release = resolve; });
  const close = f.handler(); await Promise.resolve();
  assert.equal(f.destroyed, 0); release(false); await close;
  assert.equal(f.destroyed, 0);
});
test('Native dialog permission/error rejects close and reports the failure', async () => {
  const f = setup({ unsaved: true });
  f.confirm = async () => { throw new Error('dialog message not allowed'); };
  await f.handler(); assert.equal(f.destroyed, 0); assert.match(f.errors[0], /not allowed/);
});
