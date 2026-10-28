import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
export const root=new URL('../../../',import.meta.url),output=new URL('.work/business-input-closeout/delivery/compiled/',root);
await mkdir(output,{recursive:true});
export async function source(name){
 const text=await readFile(new URL(`apps/planner/src/features/business/${name}.ts`,root),'utf8');
 const code=stripTypeScriptTypes(text).replace(/from '\.\/(\w+)'/g,"from './$1.mjs'");
 const path=new URL(name+'.mjs',output);await writeFile(path,code);return import(path.href);
}
