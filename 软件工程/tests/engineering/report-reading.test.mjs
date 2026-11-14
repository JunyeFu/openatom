import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { rolldown } from 'rolldown';

const reading = fileURLToPath(new URL('../../apps/planner/src/features/comparison/issueReading.ts', import.meta.url));
const model = fileURLToPath(new URL('../../apps/planner/src/features/comparison/model.ts', import.meta.url));
const entry = '\0report-reading-check';
const bundle = await rolldown({ input: entry, plugins: [{ name: 'reading-entry', resolveId(id) { if (id === entry) return id; },
  load(id) { if (id === entry) return `export * from ${JSON.stringify(reading)}; export {metricText,errorText,errorDetail} from ${JSON.stringify(model)};`; } }] });
after(() => bundle.close());
const generated = await bundle.generate({ format: 'esm' });
const code = generated.output.find(output => output.type === 'chunk').code;
const { issueMessage, issueGroups, metricText, errorText, errorDetail } = await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}`);
const issue = (code, message, extra = {}) => ({ code, message, blocks: [], ...extra });

test('unknown completion is not relabelled as missing cost because it shares an issue code', () => {
  const reason = issue('comparison_metric_unknown', '声明的接受目标尚未完成，完整接受时刻未知');
  assert.equal(metricText({ kind: 'unknown', data: reason }), `未知：${reason.message}`);
  assert(!issueMessage(reason).includes('成本'));
});

test('unknown English reasons stay unresolved, with the exact original issue retained', () => {
  const original = Object.freeze(issue('future_contract_condition', 'Additional frozen condition unavailable', {
    blocks: ['execution'], field_path: '/orders/7/quantity', object_ref: { kind: 'order', id: 'ORIGINAL-7' },
  }));
  const before = JSON.stringify(original);
  const groups = issueGroups([original]);
  assert.match(groups[0].message, /限制.*核对/);
  assert.equal(groups[0].issues[0], original);
  assert.equal(JSON.stringify(original), before);
  assert(!/通过|已就绪|等于零/.test(groups[0].message));
});

test('shared presentation codes preserve different causes rather than inventing a timeline restriction', () => {
  for (const message of ['设备未声明详细素材', '冻结ItemType无精确Items模块来源', '二维视图不适用三维相机']) {
    assert.equal(issueMessage(issue('render_view.presentation_limitation', message)), message);
  }
  const unknown = issue('render_view.presentation_limitation', 'Future presentation condition');
  assert.match(issueMessage(unknown), /核对/);
  assert(!issueMessage(unknown).includes('时间轴'));
  assert.equal(issueGroups([unknown])[0].issues[0], unknown);
});

test('grouping repeated descriptions preserves every distinct object and field source', () => {
  const first = issue('evaluation.cost_rate', '费率未填写；未知不补零', { field_path: '/costs/0', object_ref: { kind: 'resource', id: 'R0' } });
  const second = issue('evaluation.cost_rate', first.message, { field_path: '/costs/1', object_ref: { kind: 'resource', id: 'R1' } });
  const original = [first, second]; const before = JSON.stringify(original);
  assert.deepEqual(issueGroups(original).flatMap(group => group.issues), original);
  assert.equal(issueGroups(original).length, 1);
  assert.equal(JSON.stringify(original), before);
});

test('empty issues do not manufacture a pass or a zero metric', () => {
  assert.deepEqual(issueGroups([]), []);
  assert.equal(metricText({ kind: 'quantity', data: { value: { coefficient: 0, decimal_places: 0 }, unit: 'piece' } }), '0 piece');
  assert.match(metricText({ kind: 'unknown', data: issue('new_unknown', 'not supplied') }), /^未知：/);
});

test('English request errors have a readable failure and retain their complete technical detail', () => {
  const original = Object.assign(new Error('Permission denied for original path'), { detail: { code: 'PermissionDenied', field_errors: [{ pointer: '/destination', reason: 'original source' }] } });
  assert.match(errorText(original), /未完成/);
  assert.equal(errorDetail(original), original.detail);
  const simple = new Error('Original English failure');
  assert.deepEqual(errorDetail(simple), { name: simple.name, message: simple.message });
});
