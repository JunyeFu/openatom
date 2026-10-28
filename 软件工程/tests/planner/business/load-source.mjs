import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
export const root = new URL('../../../', import.meta.url);
const output = new URL('scenarios/真实业务设计/runtime-ui/compiled/', root);
await mkdir(output, { recursive: true });
export async function source(name) {
  const raw = await readFile(new URL(`apps/planner/src/features/business/${name}.ts`, root), 'utf8');
  const code = stripTypeScriptTypes(raw).replace(/from '\.\/(\w+)'/g, "from './$1.mjs'");
  const url = new URL(`${name}.mjs`, output); await writeFile(url, code); return import(url.href);
}
