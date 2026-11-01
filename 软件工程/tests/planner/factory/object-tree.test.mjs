import test from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { build } from 'vite';
import { fileURLToPath } from 'node:url';

const root = new URL('../../../', import.meta.url);
const output = new URL('.work/single-window-20261018/native-nfr-preparation/summary-repair/compiled/', root);
await build({ configFile: false, root: fileURLToPath(root), logLevel: 'warn', build: {
  ssr: true, outDir: fileURLToPath(output), emptyOutDir: false, minify: false,
  lib: { entry: fileURLToPath(new URL('apps/planner/src/features/factory/ObjectTreeBranch.tsx', root)), formats: ['es'] },
  rolldownOptions: { output: { entryFileNames: 'object-tree.mjs' } },
} });
const { ObjectTreeBranch } = await import(new URL('object-tree.mjs', output));

for (const [kind, noun] of [['hall', '厂房'], ['line', '线组']]) {
  test(`${noun} keeps object selection outside its native keyboard disclosure`, () => {
    const markup = renderToStaticMarkup(React.createElement(ObjectTreeBranch, {
      kind, title: 'H1 <长名称>', selected: true,
      onSelect() { throw new Error('Rendering must not select an object or send a Host command'); },
    }, React.createElement('button', null, '▣ D01')));
    const summaries = [...markup.matchAll(/<summary\b[^>]*>(.*?)<\/summary>/gs)];
    assert.equal(summaries.length, 1);
    assert.doesNotMatch(summaries[0][1], /<(button|a|input|select|textarea)\b|tabindex=/i);
    assert.match(summaries[0][0], new RegExp(`aria-label="展开或收起${noun} H1 &lt;长名称&gt;"`));
    assert.match(markup, /<details open="">/);
    assert.ok(markup.indexOf('tree-branch-title selected') < markup.indexOf('<details'));
    assert.ok(markup.indexOf('▣ D01') > markup.indexOf('</summary>'));
    assert.doesNotMatch(markup, /role="button"|tabindex=/i);
  });
}
