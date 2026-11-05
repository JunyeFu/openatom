import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { rolldown } from 'rolldown';

const entry = fileURLToPath(new URL('../../apps/planner/src/features/factory/hallLabels.ts', import.meta.url));
const bundle = await rolldown({ input: entry });
after(() => bundle.close());
const generated = await bundle.generate({ format: 'esm' });
const code = generated.output.find(output => output.type === 'chunk')?.code;
assert.ok(code);
const { hallOptionLabel } = await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}`);
const fixture = JSON.parse(await readFile(fileURLToPath(new URL(
  '../../packages/commands/tests/fixtures/production/p202-halls-B.json', import.meta.url)), 'utf8'));

test('distinct hall names are primary labels without exposing internal IDs', () => {
  const halls = fixture.project.halls;
  assert.deepEqual(halls.map(hall => hallOptionLabel(halls, hall)), ['Assembly hall', 'Packing hall']);
  assert.deepEqual(halls.map(hall => hall.id), ['H-A', 'H-B']);
});

test('same names remain distinguishable while selection identity stays unchanged', () => {
  const halls = fixture.project.halls.map(hall => ({ ...hall, title: '包装厂房' }));
  const original = JSON.stringify(halls);
  assert.deepEqual(halls.map(hall => hallOptionLabel(halls, hall)), ['包装厂房（同名 1/2）', '包装厂房（同名 2/2）']);
  assert.notEqual(halls[0].id, halls[1].id);
  assert.equal(JSON.stringify(halls), original);
});

test('blank legacy names get a readable fallback and still distinguish duplicates', () => {
  const halls = fixture.project.halls.map(hall => ({ ...hall, title: '  ' }));
  assert.deepEqual(halls.map(hall => hallOptionLabel(halls, hall)), ['未命名厂房（同名 1/2）', '未命名厂房（同名 2/2）']);
});
