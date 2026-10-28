import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
const root = new URL('../../../', import.meta.url);
const output = new URL('scenarios/Agent共建/runtime-public-render-consumer/regression/old-timeline/compiled/', root);
await mkdir(output, { recursive: true });
export async function source(name) {
  const raw = await readFile(new URL(`apps/planner/src/features/timeline/${name}.ts`, root), 'utf8');
  const js = stripTypeScriptTypes(raw).replace(/from '\.\/(\w+)'/g, "from './$1.mjs'");
  const path = new URL(`${name}.mjs`, output); await writeFile(path, js); return import(path.href);
}
export const rootUrl = root;
