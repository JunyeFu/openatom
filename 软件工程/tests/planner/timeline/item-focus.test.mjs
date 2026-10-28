import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { transformWithOxc } from 'vite';
import { source } from './load-source.mjs';

const time = await source('time'), projection = await source('projection'), input = await source('input');
const compiled = await transformWithOxc(readFileSync(new URL('../../../apps/planner/src/features/timeline/RunTimeline.tsx', import.meta.url), 'utf8'), 'RunTimeline.tsx', {
  jsx: { runtime: 'classic', pragma: 'h', pragmaFrag: 'Fragment' },
});
const code = compiled.code.replace(/^import .*;\r?\n/gm, '').replace('export function RunTimeline', 'function RunTimeline') + '\nmodule.exports.RunTimeline = RunTimeline;';

function mountedTimeline() {
  const states = [], refs = [];
  let stateCursor = 0, refCursor = 0, active = null;
  class Element { focus() { active = this; } }
  class Button extends Element {}
  class Input extends Element {}
  class Select extends Element {}
  const jsx = (type, attributes, ...children) => {
    const props = { ...attributes, children: children.length === 1 ? children[0] : children };
    if (typeof type === 'function') return type(props);
    const node = Object.assign(new (type === 'button' ? Button : type === 'input' ? Input : type === 'select' ? Select : Element)(), { type, props });
    if (props.ref) props.ref.current = node;
    return node;
  };
  const preferences = { version: 1, group_overrides: [], independent_device_ids: [], row_order: [], collapsed_group_ids: [], pinned_resource_ids: [], axis_windows: {}, row_scroll: 0, snap_enabled: true };
  const react = {
    useState(initial) { const index = stateCursor++; if (!(index in states)) states[index] = typeof initial === 'function' ? initial() : initial; return [states[index], value => { states[index] = typeof value === 'function' ? value(states[index]) : value; }]; },
    useRef(initial) { const index = refCursor++; return refs[index] ??= { current: initial }; },
    useMemo(fn) { return fn(); }, useEffect() {}, useLayoutEffect() {},
  };
  const module = { exports: {} };
  vm.runInNewContext(code, {
    ...projection, ...time, ...input, ...react, h: jsx, Fragment: 'fragment',
    module, exports: module.exports, HTMLInputElement: Input, HTMLButtonElement: Button, HTMLSelectElement: Select,
    preferenceKey: () => 'focus-test', useViewPreferences: () => ({ value: preferences, update() {}, storageError: '' }),
  });
  const props = { project: null, run: { id: 'focus-run', window: { start_ms: 0, end_ms: 1000 }, computed_until_ms: 1000, events: [], jobs: [], resource_claims: [], acceptance_events: [], scope: [], limitations: [] }, atMs: 1000, onSelect() {}, onTime() {} };
  let tree;
  function render() { stateCursor = refCursor = 0; tree = module.exports.RunTimeline(props); }
  function nodes(node) { if (Array.isArray(node)) return node.flatMap(child => nodes(child)); if (!node?.props) return []; return [node, ...nodes(node.props.children)]; }
  function button(label) { const node = nodes(tree).find(node => node.type === 'button' && node.props.children === label); assert.ok(node, `button ${label}`); return node; }
  render();
  return {
    render, button,
    find: label => nodes(tree).find(node => node.props['aria-label'] === label),
    get active() { return active; },
    get root() { return tree; },
  };
}

test('Escape inside the item panel closes it and returns focus to its still-mounted trigger', () => {
  const view = mountedTimeline();
  view.button('物品跟随').props.onClick(); view.render();
  const input = view.find('查找物品'); input.focus();
  let prevented = false;
  view.root.props.onKeyDown({ key: 'Escape', target: input, preventDefault() { prevented = true; }, stopPropagation() {} });
  assert.equal(view.active, view.button('物品跟随'));
  assert.equal(prevented, true);
  view.render(); assert.equal(view.find('物品跟随面板'), undefined);
});

test('the item panel Close button also restores focus without changing the selected Run', () => {
  const view = mountedTimeline();
  view.button('物品跟随').props.onClick(); view.render();
  view.button('关闭').focus(); view.button('关闭').props.onClick();
  assert.equal(view.active, view.button('物品跟随'));
  view.render(); assert.equal(view.find('物品跟随面板'), undefined);
  assert.equal(view.root.props['data-run-id'], 'focus-run');
});
