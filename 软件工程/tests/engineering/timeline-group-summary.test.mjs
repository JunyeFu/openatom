import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rmdir, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { rolldown } from 'rolldown';

const entry = fileURLToPath(new URL('./__timeline-summary-render.mjs', import.meta.url));
const component = fileURLToPath(new URL('../../apps/planner/src/features/timeline/RunTimeline.tsx', import.meta.url)).replaceAll('\\', '/');
const projection = fileURLToPath(new URL('../../apps/planner/src/features/timeline/projection.ts', import.meta.url)).replaceAll('\\', '/');
const bundle = await rolldown({ input: entry, platform: 'node', transform: { jsx: { runtime: 'automatic' } },
  plugins: [{ name: 'summary-render-entry',
    resolveId(source) {
      if (source === entry) return entry;
      if (source.endsWith('.css')) return '\0timeline-summary-style';
    },
    load(id) {
      if (id === '\0timeline-summary-style') return 'export {};';
      if (id === entry) return `import {createElement} from 'react';
        import {renderToStaticMarkup} from 'react-dom/server';
        import {GroupSummaryBand,GroupSummaryDetails,groupSummaryKey,groupSummaryLabel,groupSummaryLayout,resolveGroupSummary} from ${JSON.stringify(component)};
        export {GroupSummaryBand,groupSummaryKey,groupSummaryLabel,groupSummaryLayout,resolveGroupSummary};
        export {summarizeGroup} from ${JSON.stringify(projection)};
        export const renderBand = (value,width) => renderToStaticMarkup(createElement(GroupSummaryBand,{value,left:0,width,selected:false,onInspect(){}}));
        export const renderDetails = value => renderToStaticMarkup(createElement(GroupSummaryDetails,{value}));`;
    },
  }],
});
const temporaryDirectory = await mkdtemp(path.join(tmpdir(), 'sf-timeline-summary-'));
const output = path.join(temporaryDirectory, 'summary.mjs');
after(async () => { await bundle.close(); await unlink(output); await rmdir(temporaryDirectory); });
const generated = await bundle.generate({ format: 'esm' });
const code = generated.output.find(value => value.type === 'chunk')?.code;
assert.ok(code);
await writeFile(output, code);
const { GroupSummaryBand, groupSummaryKey, groupSummaryLabel, groupSummaryLayout,
  resolveGroupSummary, summarizeGroup, renderBand, renderDetails } = await import(pathToFileURL(output).href);

const sample = { runId: 'frozen-run-A', groupId: 'line:packaging', groupTitle: '完整纸包装流水线名称（9台）',
  recordLabel: '当前记录', secondary: false, interval: { start_ms: 712600, end_ms: 717600 },
  busy: 1, waiting: 7, occupied: 1, total: 9 };
const buttonText = markup => markup.replace(/^<button[^>]*>/, '').replace(/<\/button>$/, '').replace(/<[^>]*>/g, '');

test('pixel-wide summaries render only bands while retaining every count in the accessible name', () => {
  for (const width of [1, 2, 6, 12, 32]) {
    const markup = renderBand(sample, width);
    assert.match(markup, /data-summary-layout="band"/);
    assert.equal(buttonText(markup), '', `width ${width} retained cropped B/W/O text`);
    assert.match(markup, /完整纸包装流水线名称（9台）/);
    assert.match(markup, /712\.6秒—717\.6秒/);
    assert.match(markup, /有效加工 1\/9；等待 7\/9；容量占用 1\/9/);
    assert.match(markup, /data-summary-run-id="frozen-run-A"/);
  }
});

test('text appears only when the available width fits all three complete counts', () => {
  assert.equal(groupSummaryLayout(sample, 90), 'counts');
  assert.equal(buttonText(renderBand(sample, 90)), 'B1/9W7/9O1/9');
  const largerCounts = { ...sample, total: 123456789, busy: 123456788 };
  assert.equal(groupSummaryLayout(largerCounts, 90), 'band');
  assert.equal(groupSummaryLayout(largerCounts, 240), 'counts');
});

test('focus and activation retain the exact source and true interval without rewriting the summary', () => {
  const original = JSON.stringify(sample), calls = [], target = { tagName: 'BUTTON' };
  const element = GroupSummaryBand({ value: sample, left: 15, width: 1, selected: false,
    onInspect: (value, node) => calls.push({ value, node }) });
  element.props.onFocus({ currentTarget: target });
  element.props.onClick({ currentTarget: target });
  assert.equal(element.type, 'button');
  assert.equal(element.props.type, 'button');
  assert.equal(calls.length, 2);
  for (const call of calls) { assert.equal(call.value, sample); assert.equal(call.node, target); }
  assert.equal(JSON.stringify(sample), original);
});

test('selection resolves by Run, group and true interval after clipping or changing records', () => {
  const other = { ...sample, runId: 'frozen-run-B', busy: 8, secondary: true, recordLabel: '对比记录 B' };
  const key = groupSummaryKey(sample), visible = { start_ms: 714000, end_ms: 715000 };
  assert.notEqual(key, groupSummaryKey(other));
  const resolved = resolveGroupSummary(key, [other, sample], visible);
  assert.equal(resolved, sample);
  assert.deepEqual(resolved.interval, { start_ms: 712600, end_ms: 717600 });
  assert.equal(resolveGroupSummary(key, [other], visible), null);
  assert.equal(resolveGroupSummary(key, [sample], { start_ms: 800000, end_ms: 900000 }), null);
  const updated = { ...sample, waiting: 6 };
  assert.equal(resolveGroupSummary(key, [updated], visible).waiting, 6);
});

test('full details retain overlapping counts from the real group projection', () => {
  const primitives = [
    { row: 'device:D1', kind: 'work', interval: { start_ms: 0, end_ms: 10000 } },
    { row: 'device:D2', kind: 'work', interval: { start_ms: 5000, end_ms: 15000 } },
    { row: 'device:D1', kind: 'wait', interval: { start_ms: 8000, end_ms: 12000 } },
    { row: 'device:D2', kind: 'claim', interval: { start_ms: 0, end_ms: 15000 } },
  ];
  const band = summarizeGroup(primitives, ['D1', 'D2']).find(value => value.interval.start_ms === 8000);
  assert.deepEqual(band, { interval: { start_ms: 8000, end_ms: 10000 }, busy: 2, waiting: 1, occupied: 1, total: 2 });
  const value = { ...sample, ...band, groupTitle: '完整两设备测试组' };
  assert.match(groupSummaryLabel(value), /有效加工 2\/2；等待 1\/2；容量占用 1\/2；计数允许重叠/);
  const markup = renderDetails(value);
  assert.match(markup, /aria-label="分组区间详情"/);
  assert.match(markup, /完整两设备测试组/);
  assert.match(markup, /区间 8秒—10秒/);
  assert.match(markup, /有效加工 2\/2 · 等待 1\/2 · 容量占用 1\/2/);
  assert.match(markup, /frozen-run-A/);
});
