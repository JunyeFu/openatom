import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../../../apps/planner/src/features/factory/Workbench.tsx', import.meta.url), 'utf8');
const nav = source.indexOf('<nav className="menubar"');
const start = source.indexOf('onClick={event => {', nav) + 'onClick={event => {'.length;
const end = source.indexOf('\n    }}>', start);
assert.ok(nav >= 0 && start > nav && end > start, 'actual Workbench menu click handler');
const click = new Function('HTMLElement', 'event', source.slice(start, end));

// Controlled event fixtures execute the actual handler. Native details activation,
// focus and project adoption are checked separately in the desktop application.
class ElementFixture {
  constructor(tagName, parentElement = null) {
    this.tagName = tagName;
    this.parentElement = parentElement;
    this.children = [];
    this.open = false;
    this.document = parentElement?.document ?? { activeElement: null };
    parentElement?.children.push(this);
  }
  closest(selector) {
    for (let node = this; node; node = node.parentElement) {
      if (node.tagName === selector) return node;
    }
    return null;
  }
  querySelectorAll(selector) {
    assert.equal(selector, 'details[open]');
    return this.children.filter(node => node.tagName === 'details' && node.open);
  }
  querySelector(selector) { return this.children.find(node => node.tagName === selector) ?? null; }
  focus() { this.document.activeElement = this; }
  removeAttribute(name) { assert.equal(name, 'open'); this.open = false; }
}
function menuFixture() {
  const bar = new ElementFixture('nav');
  const file = new ElementFixture('details', bar), edit = new ElementFixture('details', bar);
  const fileSummary = new ElementFixture('summary', file), editSummary = new ElementFixture('summary', edit);
  const openCommand = new ElementFixture('button', file), toolbarCommand = new ElementFixture('button', bar);
  function activate(target) {
    let prevented = false;
    target.focus();
    click(ElementFixture, { target, currentTarget: bar, preventDefault: () => { prevented = true; } });
    const summary = target.closest('summary');
    if (summary && !prevented) summary.parentElement.open = !summary.parentElement.open;
    assert.equal(prevented, false, 'preserve native summary activation and focus behavior');
  }
  return { bar, file, edit, fileSummary, editSummary, openCommand, toolbarCommand, activate };
}

test('opening File closes Edit so the project-open command is not covered', () => {
  const f = menuFixture(); f.edit.open = true;
  f.activate(new ElementFixture('span', f.fileSummary));
  assert.equal(f.file.open, true);
  assert.equal(f.edit.open, false);
});

test('opening Edit closes File in the reverse direction', () => {
  const f = menuFixture(); f.file.open = true;
  f.activate(f.editSummary);
  assert.equal(f.edit.open, true);
  assert.equal(f.file.open, false);
});

test('the project-open command closes every old popup before asynchronous adoption', () => {
  const f = menuFixture(); f.file.open = f.edit.open = true;
  f.activate(f.openCommand);
  assert.equal(f.file.open, false);
  assert.equal(f.edit.open, false);
});

test('a direct toolbar command also dismisses an old popup', () => {
  const f = menuFixture(); f.edit.open = true;
  f.activate(f.toolbarCommand);
  assert.equal(f.edit.open, false);
});

test('activating an already-open summary still closes it normally', () => {
  const f = menuFixture(); f.file.open = true;
  f.activate(f.fileSummary);
  assert.equal(f.file.open, false);
  assert.equal(f.edit.open, false);
});

test('a menu command leaves focus on its reachable summary instead of the hidden command', () => {
  const f = menuFixture(); f.file.open = true;
  f.activate(f.openCommand);
  assert.equal(f.bar.document.activeElement, f.fileSummary);
  assert.equal(f.file.open, false);
  f.activate(f.bar.document.activeElement);
  assert.equal(f.file.open, true);
});
