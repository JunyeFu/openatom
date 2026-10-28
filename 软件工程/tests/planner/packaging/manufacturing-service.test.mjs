// Actual service -> installed Tauri SDK -> finite invoke fixture. Not Native/Host execution.
import test from 'node:test';
import assert from 'node:assert/strict';
import { startManufacturingPackageExport, HostFailure, manufacturingRequestFixture as request, manufacturingTaskFixture } from '../../../scenarios/专业桌面/runtime/view-build/view-fixture.mjs';
test('actual private service invokes the named Native command with the complete exact typed request and returns its original Task', async () => {
  const previous = globalThis.window; const calls = []; const task = structuredClone(manufacturingTaskFixture);
  globalThis.window = { __TAURI_INTERNALS__: { invoke: async (command, args) => { calls.push({ command, args }); return task; } } };
  try { assert.equal(await startManufacturingPackageExport(request), task); assert.deepEqual(calls, [{ command: 'project_export_manufacturing_package', args: { request } }]); }
  finally { if (previous === undefined) delete globalThis.window; else globalThis.window = previous; }
});
test('private service preserves full original ApiError as HostFailure and unrelated invoke failure unchanged', async () => {
  const previous = globalThis.window;
  const detail = { code: 'RevisionConflict', message: 'finite Host conflict', object_refs: [{ kind: 'artifact', id: 'A' }], field_errors: [{ pointer: '/scope_revision', reason: 'exact mismatch' }], expected_revision: 3, actual_revision: 4 };
  try {
    globalThis.window = { __TAURI_INTERNALS__: { invoke: async () => { throw detail; } } };
    await assert.rejects(startManufacturingPackageExport(request), error => error instanceof HostFailure && error.detail === detail);
    const failure = new Error('finite transport disconnected'); globalThis.window.__TAURI_INTERNALS__.invoke = async () => { throw failure; };
    await assert.rejects(startManufacturingPackageExport(request), error => error === failure);
  } finally { if (previous === undefined) delete globalThis.window; else globalThis.window = previous; }
});
