import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { rolldown } from 'rolldown';

// Bundle the real TSX dependency graph in memory; the test creates no app artifact.
const entry = fileURLToPath(new URL('../../apps/planner/src/features/business/preview.ts', import.meta.url));
const bundle = await rolldown({ input: entry });
after(() => bundle.close());
const generated = await bundle.generate({ format: 'esm' });
const code = generated.output.find(output => output.type === 'chunk')?.code;
assert.ok(code);
const { previewCommands } = await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}`);

// The production fixture supplies actual ProjectState and ScenarioInput shapes.
const fixture = JSON.parse(await readFile(fileURLToPath(new URL(
  '../../packages/commands/tests/fixtures/production/p202-halls-B.json', import.meta.url)), 'utf8'));
const base = { project: fixture.project, scenarios: [fixture.scenario] };
const original = fixture.scenario.orders[0];
const second = { ...structuredClone(original), id: 'ORDER-02', priority: 40 };
const scenario = orders => ({ ...structuredClone(fixture.scenario), orders });
const changesFor = updated => previewCommands(base, { project: fixture.project, scenarios: [updated] },
  [{ kind: 'put_object', data: { kind: 'scenario', data: updated } }])[0].changes;

test('priority 10 to 90 names the exact order and shows original and proposed values', () => {
  const before = scenario([{ ...structuredClone(original), priority: 10 }, second]);
  const after = scenario([{ ...structuredClone(original), priority: 90 }, second]);
  const snapshot = { project: fixture.project, scenarios: [before] };
  const originalBytes = JSON.stringify(snapshot);
  const draftBytes = JSON.stringify(after);
  const group = previewCommands(snapshot, { project: fixture.project, scenarios: [after] },
    [{ kind: 'put_object', data: { kind: 'scenario', data: after } }])[0];
  assert.match(group.title, /方案.*Original two-hall B/);
  assert(group.changes.some(change => change.field.includes('ORDER-01') && change.field.includes('优先序')
    && change.before === '10' && change.after === '90'));
  assert(!group.changes.some(change => change.field.includes('ORDER-02')));
  assert.equal(JSON.stringify(snapshot), originalBytes, 'preview changed the original frozen input');
  assert.equal(JSON.stringify(after), draftBytes, 'preview changed the proposed input');
});

test('same product with another order ID remains a separate change target', () => {
  const initial = scenario([original, second]);
  const updated = scenario([original, { ...second, priority: 90 }]);
  const changes = previewCommands({ project: fixture.project, scenarios: [initial] },
    { project: fixture.project, scenarios: [updated] },
    [{ kind: 'put_object', data: { kind: 'scenario', data: updated } }])[0].changes;
  assert(changes.some(change => change.field.includes('ORDER-02') && change.before === '40' && change.after === '90'));
  assert(!changes.some(change => change.field.includes('ORDER-01')));
});

test('added and removed orders show their identities and missing side', () => {
  const added = changesFor(scenario([original, second]));
  assert(added.some(change => change.field.includes('ORDER-02') && change.before === '未填写'));
  const removed = changesFor(scenario([]));
  assert(removed.some(change => change.field.includes('ORDER-01') && change.after === '未填写'));
});

test('reordering existing orders reports order without inventing field edits', () => {
  const initial = scenario([original, second]);
  const reordered = scenario([second, original]);
  const changes = previewCommands({ project: fixture.project, scenarios: [initial] },
    { project: fixture.project, scenarios: [reordered] },
    [{ kind: 'put_object', data: { kind: 'scenario', data: reordered } }])[0].changes;
  assert.deepEqual(changes, [{ field: '订单 / 顺序', before: '原顺序', after: '新顺序' }]);
});

test('decimal quantity 2.5 to 3 is one readable change with its unchanged unit', () => {
  const before = scenario([{ ...structuredClone(original), quantity: { value: { coefficient: 25, decimal_places: 1 }, unit: 'piece' } }]);
  const after = scenario([{ ...structuredClone(original), quantity: { value: { coefficient: 3, decimal_places: 0 }, unit: 'piece' } }]);
  const changes = previewCommands({ project: fixture.project, scenarios: [before] },
    { project: fixture.project, scenarios: [after] },
    [{ kind: 'put_object', data: { kind: 'scenario', data: after } }])[0].changes;
  assert.equal(changes.length, 1);
  assert.match(changes[0].field, /^订单 1 .*ORDER-01.*\/ 数量$/);
  assert.equal(changes[0].before, '2.5 件');
  assert.equal(changes[0].after, '3 件');
});

test('quantity unit change stays visible alongside the exact decimal amount', () => {
  const before = scenario([{ ...structuredClone(original), quantity: { value: { coefficient: 25, decimal_places: 1 }, unit: 'piece' } }]);
  const after = scenario([{ ...structuredClone(original), quantity: { value: { coefficient: 25, decimal_places: 1 }, unit: 'kg' } }]);
  const changes = previewCommands({ project: fixture.project, scenarios: [before] },
    { project: fixture.project, scenarios: [after] },
    [{ kind: 'put_object', data: { kind: 'scenario', data: after } }])[0].changes;
  assert.equal(changes.length, 1);
  assert.equal(changes[0].before, '2.5 件');
  assert.equal(changes[0].after, '2.5 千克');
});

test('large safe coefficients retain all decimal digits', () => {
  const before = scenario([{ ...structuredClone(original), quantity: { value: { coefficient: 9007199254740990, decimal_places: 9 }, unit: 'piece' } }]);
  const after = scenario([{ ...structuredClone(original), quantity: { value: { coefficient: 9007199254740991, decimal_places: 9 }, unit: 'piece' } }]);
  const changes = previewCommands({ project: fixture.project, scenarios: [before] },
    { project: fixture.project, scenarios: [after] },
    [{ kind: 'put_object', data: { kind: 'scenario', data: after } }])[0].changes;
  assert.equal(changes.length, 1);
  assert.equal(changes[0].before, '9007199.254740990 件');
  assert.equal(changes[0].after, '9007199.254740991 件');
});

test('a direct Decimal cost rate is displayed as a value rather than coefficient fields', () => {
  const cost = { id: 'COST-01', object_ref: { kind: 'order', id: 'ORDER-01' }, component: '材料',
    basis: 'consumed_material', rate: { coefficient: 25, decimal_places: 1 }, currency: 'CNY',
    rate_quantity: { value: { coefficient: 1, decimal_places: 0 }, unit: 'piece' }, includes: [],
    source: { target: { kind: 'order', id: 'ORDER-01' }, field_path: 'cost_inputs', basis: 'user_provided' } };
  const before = { ...structuredClone(fixture.scenario), cost_inputs: [cost] };
  const after = { ...structuredClone(before), cost_inputs: [{ ...cost, rate: { coefficient: 3, decimal_places: 0 } }] };
  const changes = previewCommands({ project: fixture.project, scenarios: [before] },
    { project: fixture.project, scenarios: [after] },
    [{ kind: 'put_object', data: { kind: 'scenario', data: after } }])[0].changes;
  assert.equal(changes.length, 1);
  assert.match(changes[0].field, /单价/);
  assert.equal(changes[0].before, '2.5');
  assert.equal(changes[0].after, '3');
});

test('negative fractional Decimal keeps its sign and leading zero', () => {
  const cost = { id: 'COST-NEGATIVE', object_ref: { kind: 'order', id: 'ORDER-01' }, component: '调整',
    basis: 'consumed_material', rate: { coefficient: -5, decimal_places: 2 }, currency: 'CNY',
    rate_quantity: { value: { coefficient: 1, decimal_places: 0 }, unit: 'piece' }, includes: [],
    source: { target: { kind: 'order', id: 'ORDER-01' }, field_path: 'cost_inputs', basis: 'user_provided' } };
  const before = { ...structuredClone(fixture.scenario), cost_inputs: [cost] };
  const after = { ...structuredClone(before), cost_inputs: [{ ...cost, rate: { coefficient: 3, decimal_places: 2 } }] };
  const changes = previewCommands({ project: fixture.project, scenarios: [before] },
    { project: fixture.project, scenarios: [after] },
    [{ kind: 'put_object', data: { kind: 'scenario', data: after } }])[0].changes;
  assert.equal(changes.length, 1);
  assert.equal(changes[0].before, '-0.05');
  assert.equal(changes[0].after, '0.03');
});

test('a provenance-only parameter edit shows the old and new source basis', () => {
  const parameterId = 'cycle/~duration';
  const device = { ...structuredClone(fixture.project.devices[0]),
    configuration: { [parameterId]: { kind: 'integer', value: 5 } } };
  const project = { ...structuredClone(fixture.project), devices: [device], parameter_sources: [
    { target: { kind: 'device', id: device.id }, field_path: '/configuration/cycle~1~0duration/value',
      value: device.configuration[parameterId], basis: 'module_declared' },
    { target: { kind: 'device', id: device.id }, field_path: '/configuration/other/value', basis: 'unknown' },
  ] };
  const snapshot = { project, scenarios: [] };
  const original = JSON.stringify(snapshot);
  const group = previewCommands(snapshot, snapshot, [{ kind: 'set_declared_parameter', data: {
    target: { kind: 'device', id: device.id }, parameter_id: parameterId,
    value: device.configuration[parameterId], basis: 'user_provided',
  } }])[0];
  assert(group.changes.some(change => change.field === '来源性质'
    && change.before === '模块声明' && change.after === '人工/企业声明'));
  assert.equal(JSON.stringify(snapshot), original, 'preview rewrote the stored source basis');
});

test('an unset stored parameter source remains unknown in the preview', () => {
  const device = { ...structuredClone(fixture.project.devices[0]),
    configuration: { cycle: { kind: 'integer', value: 5 } } };
  const project = { ...structuredClone(fixture.project), devices: [device], parameter_sources: [] };
  const snapshot = { project, scenarios: [] };
  const group = previewCommands(snapshot, snapshot, [{ kind: 'set_declared_parameter', data: {
    target: { kind: 'device', id: device.id }, parameter_id: 'cycle',
    value: { kind: 'integer', value: 6 }, basis: 'template_declared_example',
  } }])[0];
  assert(group.changes.some(change => change.field === '来源性质'
    && change.before === '待补' && change.after === '模板示例'));
});
