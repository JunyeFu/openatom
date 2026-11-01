import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
const root = new URL('../../../', import.meta.url);
const output = new URL('scenarios/自定义模块/runtime-authoring/compiled/', root);
await mkdir(output, { recursive: true });
for (const name of ['layout', 'modulePresentation']) {
  const raw = await readFile(new URL(`apps/planner/src/scene/${name}.ts`, root), 'utf8');
  await writeFile(new URL(`${name}.mjs`, output), stripTypeScriptTypes(raw).replace("from './layout'", "from './layout.mjs'"));
}
export async function source(group, name) {
  const raw = await readFile(new URL(`apps/planner/src/features/${group}/${name}.ts`, root), 'utf8');
  const code = stripTypeScriptTypes(raw).replace(/from '\.\/(\w+)'/g, "from './$1.mjs'").replace("from '../../scene/modulePresentation'", "from './modulePresentation.mjs'");
  const file = new URL(`${name}.mjs`, output); await writeFile(file, code); return import(file.href);
}
export const rootUrl = root;
