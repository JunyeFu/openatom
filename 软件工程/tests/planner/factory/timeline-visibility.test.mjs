import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { transformSync } from 'rolldown/utils';

const root = new URL('../../../', import.meta.url);
const workbench = readFileSync(new URL('apps/planner/src/features/factory/Workbench.tsx', root), 'utf8');
const timeline = readFileSync(new URL('apps/planner/src/features/timeline/RunTimeline.tsx', root), 'utf8');
const dock = workbench.slice(workbench.indexOf('    {(timelineVisible || descriptionVisible) &&'), workbench.indexOf('    <footer className="app-status"'));
const React = { Fragment: Symbol('fragment'), createElement: (type, props, ...children) => ({ type, props: props ?? {}, children }) };
const RunTimeline = () => {};
function render(jsx, environment) {
  const code = transformSync('timeline-visibility.tsx', `globalThis.tree = (${jsx});`, { jsx: { runtime: 'classic', pragma: 'React.createElement', pragmaFrag: 'React.Fragment' } }).code;
  const context = { React, ...environment };
  vm.runInNewContext(code, context);
  return context.tree;
}
function nodes(tree, type, path = []) {
  if (!tree || typeof tree !== 'object') return [];
  return [...(tree.type === type ? [{ tree, path }] : []), ...(tree.children ?? []).flat(Infinity).flatMap((child, index) => nodes(child, type, [...path, index]))];
}
function draw(timelineVisible, descriptionVisible) {
  return render(`<>${dock}</>`, { timelineVisible, descriptionVisible, RunTimeline, timelineProps: { run: { id: 'R' }, atMs: 392636 },
    propertyTextScale: 1, timelineSelection: null, selection: null });
}

test('hiding the timeline beside the event description retains its playback owner', () => {
  const before = nodes(draw(true, true), RunTimeline), after = nodes(draw(false, true), RunTimeline);
  assert.equal(before.length, 1);
  assert.equal(after.length, 1, 'hiding a dock must not unmount the playback timer');
  assert.equal(after[0].tree.props.hidden, true);
  assert.deepEqual(after[0].path, before[0].path);
  assert.equal(after[0].tree.props.run.id, 'R');
  assert.equal(after[0].tree.props.atMs, 392636);
});

test('hiding both bottom panels releases layout space without removing the playback owner', () => {
  const tree = draw(false, false);
  assert.equal(nodes(tree, RunTimeline).length, 1);
  const dock = nodes(tree, 'div').find(({ tree }) => tree.props.className === 'bottom-dock');
  assert.equal(dock.tree.props.style.display, 'none');
  assert.equal(nodes(tree, 'div').filter(({ tree }) => tree.props.className === 'dock-resizer').length, 0);
});

test('restoring either panel keeps the same timeline position and run identity', () => {
  const states = [[true, false], [false, false], [false, true], [true, true], [true, false]].map(([a, b]) => nodes(draw(a, b), RunTimeline));
  for (const state of states) {
    assert.equal(state.length, 1);
    assert.deepEqual(state[0].path, states[0][0].path);
    assert.equal(state[0].tree.props.run.id, 'R');
  }
  assert.equal(states.at(-1)[0].tree.props.hidden, false);
});

test('hidden timeline has no focusable displayed surface despite its flex stylesheet', () => {
  const start = timeline.indexOf('<section className="timeline sf-timeline"');
  const opening = timeline.slice(start, timeline.indexOf('    }}>', start) + 7);
  assert.ok(start >= 0 && opening.endsWith('>'));
  for (const hidden of [true, false]) {
    const tree = render(opening.replace(/>$/, '/>'), { props: { hidden }, run: { id: 'R' }, atMs: 392636, window: { start_ms: 0, end_ms: 1200000 } });
    assert.equal(tree.props.hidden, hidden);
    assert.equal(tree.props.style?.display, hidden ? 'none' : undefined);
  }
});
