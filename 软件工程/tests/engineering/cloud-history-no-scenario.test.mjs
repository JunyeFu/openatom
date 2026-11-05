import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, rmdir, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { rolldown } from 'rolldown';

// Render the real component with React. Effects, source loading and tab changes
// remain the responsibility of the Native interaction checks.
const entry = fileURLToPath(new URL('./__cloud-panel-render-entry.mjs', import.meta.url));
const component = fileURLToPath(new URL('../../apps/planner/src/features/cloud/CloudProductionPanel.tsx', import.meta.url));
const bundle = await rolldown({
  input: entry,
  platform: 'node',
  transform: { jsx: { runtime: 'automatic' } },
  plugins: [{
    name: 'component-render-entry',
    resolveId(source) {
      if (source === entry) return entry;
      if (source.endsWith('.css')) return '\0component-render-style';
    },
    load(id) {
      if (id === '\0component-render-style') return 'export {};';
      if (id === entry) return `import {createElement} from 'react';
        import {renderToStaticMarkup} from 'react-dom/server';
        import {CloudProductionPanel} from ${JSON.stringify(component.replaceAll('\\', '/'))};
        export const render = props => renderToStaticMarkup(createElement(CloudProductionPanel, props));`;
    },
  }],
});
const temporaryDirectory = await mkdtemp(path.join(tmpdir(), 'sf-cloud-panel-render-'));
const output = path.join(temporaryDirectory, 'component.mjs');
const previousWindow = Object.getOwnPropertyDescriptor(globalThis, 'window');
after(async () => {
  await bundle.close();
  await unlink(output);
  await rmdir(temporaryDirectory);
  if (previousWindow) Object.defineProperty(globalThis, 'window', previousWindow);
  else delete globalThis.window;
});
const generated = await bundle.generate({ format: 'esm' });
const code = generated.output.find(value => value.type === 'chunk')?.code;
assert.ok(code);
await writeFile(output, code);
Object.defineProperty(globalThis, 'window', { value: {}, configurable: true });
const { render } = await import(pathToFileURL(output).href);
const fixture = JSON.parse(await readFile(new URL(
  '../../packages/commands/tests/fixtures/production/p202-halls-B.json', import.meta.url), 'utf8'));
const props = {
  project: fixture.project,
  scenario: null,
  capabilities: { api_version: '0.1', implemented_operations: [], available_command_kinds: [],
    providers: [], category_ids: [], media_kinds: [], native_primitives: [],
    max_ipc_frame_bytes: 16777216, limitations: [] },
  hostContextId: 'component-render-host',
  onClose() {}, onLocateDevice() {}, onInspectExecution() {},
};

test('an open project keeps the production-history picker when no current scenario is selected', () => {
  const markup = render(props);
  assert.match(markup, /<select[^>]*aria-label="生产记录"/);
  assert.match(markup, /role="tabpanel"/);
});

test('a missing project still blocks the history workspace', () => {
  const markup = render({ ...props, project: null });
  assert.doesNotMatch(markup, /<select[^>]*aria-label="生产记录"/);
  assert.match(markup, /请先打开工程/);
});

test('the initial view without a current scenario exposes no production preparation form', () => {
  const markup = render(props);
  assert.doesNotMatch(markup, /<form[^>]*class="cloud-prepare"/);
  assert.doesNotMatch(markup, /class="cloud-controls"/);
});
