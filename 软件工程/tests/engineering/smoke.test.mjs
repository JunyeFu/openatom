import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = fileURLToPath(new URL('../../', import.meta.url));
const builtDirectory = process.env.SUPERFACTORY_ACCEPTANCE_BIN_DIR ?? path.join(root, 'target/debug');
const binary = (name) => path.join(builtDirectory, name);

test('built desktop loads local React assets in WebView2 and completes native IPC', () => {
  const result = spawnSync(binary('superfactory.exe'), ['--s07-smoke'], {
    cwd: root, encoding: 'utf8', windowsHide: true, timeout: 60000,
  });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stderr);
  const record = result.stdout.split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line))
    .find((line) => line.kind === 'S07_engineering_boot');
  assert.deepEqual(record, {
    kind: 'S07_engineering_boot', phase: 'S07', react_version: '19.3.0',
    three_revision: '186', business_operations_ready: false, passed: true,
  });
});

test('built bridge has a working process entry and version command', () => {
  const result = spawnSync(binary('superfactory-mcp-bridge.exe'), ['--version'], {
    cwd: root, encoding: 'utf8', windowsHide: true,
  });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout.trim(), 'superfactory-mcp-bridge 0.1.0');
});

test('bridge requires an explicit existing Host instead of creating an Engine', () => {
  const result = spawnSync(binary('superfactory-mcp-bridge.exe'), [], {
    cwd: root, encoding: 'utf8', windowsHide: true,
  });
  assert.ifError(result.error);
  assert.equal(result.status, 2);
  assert.equal(result.stdout, '');
  assert.match(result.stderr, /Usage: superfactory-mcp-bridge --host-pipe <endpoint> --host-instance-id <desktop-host-PID>/);
});
