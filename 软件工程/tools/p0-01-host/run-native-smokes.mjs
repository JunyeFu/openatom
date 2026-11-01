// Executes the existing product startup consumers against one exact staged candidate.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdir, copyFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const [candidate, output, python] = process.argv.slice(2).map(value => path.resolve(value));
assert.ok(candidate && output && python);
await mkdir(output);
const results = [];
async function run(name, flag, cwd) {
  const result = spawnSync(path.join(candidate, 'superfactory.exe'), [flag], {
    cwd, encoding: 'utf8', windowsHide: true, timeout: 60000,
    env: { ...process.env, SUPERFACTORY_TEST_PYTHON: python,
      WEBVIEW2_USER_DATA_FOLDER: path.join(output, name + '-webview-profile') },
  });
  await writeFile(path.join(output, name + '.stdout.txt'), result.stdout ?? '');
  await writeFile(path.join(output, name + '.stderr.txt'), result.stderr ?? '');
  const records = (result.stdout ?? '').split(/\r?\n/).filter(line => line.startsWith('{')).map(line => JSON.parse(line));
  results.push({ name, flag, candidate, cwd, exit_code: result.status,
    process_error: result.error ? String(result.error) : null,
    receipts: records.map(record => ({ kind: record.kind, passed: record.passed,
      check_count: Array.isArray(record.checks) ? record.checks.length : null })),
    actual_visible_GUI: false, human_acceptance: false });
  await writeFile(path.join(output, 'result.json'), JSON.stringify({ results, whole_task_done: false }, null, 2));
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stderr);
  assert.ok(records.some(record => record.passed === true), 'Actual startup consumer must report pass');
  if (name === 'host') assert.ok(records.some(record => record.checks?.some(check => check.name === 'registered_native_renderer_admits_real_task' && check.passed === true)), 'The actual desktop must exercise its registered renderer, not the unregistered adapter branch');
  console.log(JSON.stringify(results.at(-1)));
}

await run('host', '--p0-01-host-smoke', root);
// The existing MCP startup consumer expects this test layout under cwd.
// Populate only our new owned folder with the exact staged bridge and unchanged client.
const mcpRoot = path.join(output, 'mcp-owned-root');
await mkdir(path.join(mcpRoot, 'target/debug'), { recursive: true });
await mkdir(path.join(mcpRoot, 'tests/delivery/mcp'), { recursive: true });
await copyFile(path.join(candidate, 'superfactory-mcp-bridge.exe'), path.join(mcpRoot, 'target/debug/superfactory-mcp-bridge.exe'));
await copyFile(path.join(root, 'tests/delivery/mcp/process_client.py'), path.join(mcpRoot, 'tests/delivery/mcp/process_client.py'));
await writeFile(path.join(mcpRoot, 'exact-staged-bridge.json'), JSON.stringify({
  source: path.join(candidate, 'superfactory-mcp-bridge.exe'),
  test_layout_copy: path.join(mcpRoot, 'target/debug/superfactory-mcp-bridge.exe'),
  global_target_untouched: true, client_assertions_unchanged: true,
}, null, 2));
await run('mcp', '--p3-04-mcp-smoke', mcpRoot);
