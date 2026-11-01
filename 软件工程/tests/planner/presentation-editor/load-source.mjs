import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
export const root=new URL('../../../',import.meta.url),out=new URL(process.env.P303_TEST_MODELS??'scenarios/图片时间轴/runtime/compiled/',root);await mkdir(out,{recursive:true});
for(const name of ['formSchema','model']){const path=new URL(`apps/planner/src/features/modules/${name}.ts`,root),code=stripTypeScriptTypes(await readFile(path,'utf8')).replace("from './formSchema'","from './formSchema.mjs'");await writeFile(new URL(name==='model'?'module-model.mjs':'formSchema.mjs',out),code);}
export async function source(name){const code=stripTypeScriptTypes(await readFile(new URL(`apps/planner/src/features/presentation-editor/${name}.ts`,root),'utf8')).replace("from '../modules/model'","from './module-model.mjs'");const target=new URL(name+'.mjs',out);await writeFile(target,code);return import(target.href);}
