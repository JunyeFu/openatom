import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, readFile, access, rm, rmdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const runner = fileURLToPath(new URL('./verify.mjs', import.meta.url));
const run = (...args) => spawnSync(process.execPath, [runner, ...args], { encoding: 'utf8' });

test('docs实际拒绝损坏JSON并接受可解析的明确文件', async () => {
  const dir = await mkdtemp(path.join(tmpdir(), 'sf-verify-docs-'));
  try {
    const file = path.join(dir, 'input.json');
    await writeFile(file, '{"value":1}');
    assert.equal(run('--kind', 'docs', '--file', file, '--run').status, 0);
    await writeFile(file, '{broken');
    assert.notEqual(run('--kind', 'docs', '--file', file, '--run').status, 0);
  } finally { await rm(path.join(dir, 'input.json'), { force: true }); await rmdir(dir); }
});

test('默认计划不执行测试；显式执行才产生实际行为', async () => {
  const dir = await mkdtemp(path.join(tmpdir(), 'sf-verify-plan-'));
  try {
    const sentinel = path.join(dir, 'executed');
    const file = path.join(dir, 'behavior.test.mjs');
    await writeFile(file, `import { writeFileSync } from 'node:fs'; writeFileSync(${JSON.stringify(sentinel)}, 'done');`);
    assert.equal(run('--kind', 'node', '--test', file).status, 0);
    await assert.rejects(access(sentinel));
    assert.equal(run('--kind', 'node', '--test', file, '--run').status, 0);
    assert.equal(await readFile(sentinel, 'utf8'), 'done');
  } finally {
    await rm(path.join(dir, 'behavior.test.mjs'), { force: true });
    await rm(path.join(dir, 'executed'), { force: true });
    await rmdir(dir);
  }
});

test('实际测试失败保留非零退出，不被包装为成功', async () => {
  const dir = await mkdtemp(path.join(tmpdir(), 'sf-verify-fail-'));
  try {
    const file = path.join(dir, 'failure.test.mjs');
    await writeFile(file, 'process.exit(7);');
    assert.notEqual(run('--kind', 'node', '--test', file, '--run').status, 0);
  } finally { await rm(path.join(dir, 'failure.test.mjs'), { force: true }); await rmdir(dir); }
});

test('Rust必须明确包；命令保留筛选，不能静默退化为全workspace', () => {
  assert.notEqual(run('--kind', 'rust').status, 0);
  const result = run('--kind', 'rust', '--package', 'sf-domain', '--filter', 'one_case');
  assert.equal(result.status, 0);
  assert.deepEqual(JSON.parse(result.stdout).commands[0].args, ['test', '--locked', '-p', 'sf-domain', 'one_case']);
  assert.notEqual(run('--kind', 'docs', '--file', 'AGENTS.md', '--test', 'ignored').status, 0);
});
