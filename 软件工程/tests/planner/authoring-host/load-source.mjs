import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
const root = new URL('../../../', import.meta.url);
const output = new URL('scenarios/自定义模块/runtime-definition-cycle/compiled-service/', root);
await mkdir(output, { recursive: true });
for (const [name, path] of [
  ['projectHost', 'services/projectHost'], ['moduleAuthoring', 'services/moduleAuthoring'],
  ['authoringModel', 'features/modules/model'], ['formSchema', 'features/modules/formSchema'],
]) {
  const raw = await readFile(new URL(`apps/planner/src/${path}.ts`, root), 'utf8');
  const code = stripTypeScriptTypes(raw, { mode: 'transform' })
    .replace("from './projectHost'", "from './projectHost.mjs'")
    .replace("from '../features/modules/model'", "from './authoringModel.mjs'")
    .replace("from './formSchema'", "from './formSchema.mjs'");
  await writeFile(new URL(`${name}.mjs`, output), code);
}
export const service = await import(new URL('moduleAuthoring.mjs', output));
export const host = await import(new URL('projectHost.mjs', output));

