import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, unlink, rmdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { rolldown } from 'rolldown';

const component = fileURLToPath(new URL('../../apps/planner/src/features/factory/RunReplayControl.tsx', import.meta.url));
const entry = fileURLToPath(new URL('./__run-replay-render.mjs', import.meta.url));
const bundle = await rolldown({ input: entry, platform: 'node', transform: { jsx: { runtime: 'automatic' } },
  plugins: [{ name: 'render-replay-control', resolveId: source => source === entry ? entry : null,
    load: id => id === entry ? `import {createElement} from 'react';
      import {renderToStaticMarkup} from 'react-dom/server';
      import {RunReplayControl} from ${JSON.stringify(component.replaceAll('\\', '/'))};
      export const render = props => renderToStaticMarkup(createElement(RunReplayControl, props));` : null }],
});
const directory = await mkdtemp(path.join(tmpdir(), 'sf-run-replay-'));
const output = path.join(directory, 'render.mjs');
after(async () => { await bundle.close(); await unlink(output); await rmdir(directory); });
const generated = await bundle.generate({ format: 'esm' });
await writeFile(output, generated.output.find(value => value.type === 'chunk').code);
const { render } = await import(pathToFileURL(output).href);
const ready = { hasProject: true, hasRecords: true, hasSelection: true, hasFrozenInput: true,
  busy: false, editing: false, active: false, onReplay() {} };

test('empty project explains absent records, next step and disabled replay together', () => {
  const markup = render({ ...ready, hasRecords: false, hasSelection: false, hasFrozenInput: false });
  assert.match(markup, /<button disabled="" aria-describedby="([^"]+)"/);
  assert.match(markup, /暂无运行记录。先配置生产方案并完成一次运行/);
  const id = markup.match(/aria-describedby="([^"]+)"/)[1];
  assert.ok(markup.includes(`id="${id}"`));
});

test('history without a selected record points at the available history picker', () => {
  const markup = render({ ...ready, hasSelection: false, hasFrozenInput: false });
  assert.match(markup, /请从“历史结果”选择一条运行记录/);
  assert.doesNotMatch(markup, /暂无运行记录/);
});

test('loaded frozen input enables replay independently of current scenario', () => {
  const markup = render(ready);
  assert.doesNotMatch(markup, /disabled|role="status"|aria-describedby/);
  assert.match(markup, />运行回看<\/button>/);
});

test('missing frozen input and active editing each give their own recovery path', () => {
  assert.match(render({ ...ready, hasFrozenInput: false }), /重新选择记录或刷新工程/);
  assert.match(render({ ...ready, editing: true }), /先返回画布并处理未应用输入/);
  assert.match(render({ ...ready, busy: true }), /正在处理，请完成后回看/);
});
