import {readFile,writeFile,mkdir,readdir} from 'node:fs/promises';
export const root=new URL('../../../',import.meta.url),out=new URL('.work/delivery/consumer/',root);
await mkdir(out,{recursive:true});
for(const file of await readdir(new URL('packages/presentation/src/',root))) {
 if(!file.endsWith('.ts'))continue;
 const {stripTypeScriptTypes}=await import('node:module');
 const text=await readFile(new URL('packages/presentation/src/'+file,root),'utf8');
 const code=stripTypeScriptTypes(text,{mode:'transform'}).replace(/from '\.\/(\w+)'/g,"from './$1.mjs'");
 await writeFile(new URL(file.replace(/\.ts$/,'.mjs'),out),code);
}
export const json=async path=>JSON.parse(await readFile(new URL(path,root),'utf8'));
export const bytes=async path=>{const b=await readFile(new URL(path,root));return b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength);};
