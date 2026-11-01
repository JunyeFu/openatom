import { build } from 'vite';
import { fileURLToPath } from 'node:url';
import { rootUrl } from './load-source.mjs';
export const reactOutput = new URL('scenarios/图片时间轴/runtime/whole-20261017/form-react-compiled/', rootUrl);
export async function compileReactFeature() {
  await build({ configFile: false, root: fileURLToPath(rootUrl), logLevel: 'warn', build: {
    ssr: true, outDir: fileURLToPath(reactOutput), emptyOutDir: false, minify: false,
    lib: { entry: fileURLToPath(new URL('tests/planner/authoring/render-entry.ts', rootUrl)), formats: ['es'] },
    rolldownOptions: { output: { entryFileNames: 'render-source.mjs' } },
  } });
  return import(new URL('render-source.mjs', reactOutput));
}
