// Transpile the current production modules; only unused Native transport
// defaults throw. Tests inject captured results from the actual fresh Host.
import {readFile,writeFile,mkdir,readdir} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
export const root=new URL('../../../',import.meta.url),out=new URL('.work/cu-delivery/consumer/',root);
await mkdir(out,{recursive:true});
for(const file of await readdir(new URL('packages/presentation/src/',root))){
 if(!file.endsWith('.ts'))continue;
 const text=stripTypeScriptTypes(await readFile(new URL('packages/presentation/src/'+file,root),'utf8'),{mode:'transform'});
 await writeFile(new URL(file.replace(/\.ts$/,'.mjs'),out),text.replace(/from '\.\/(\w+)'/g,"from './$1.mjs'"));
}
const files={load:'apps/planner/src/features/media/load.ts',processItems:'apps/planner/src/features/media/processItems.ts',
 imageSurface:'apps/planner/src/features/media/imageSurface.ts',visualCapture:'apps/planner/src/features/render-surface/visualCapture.ts',
 factoryItems:'apps/planner/src/scene/factoryItems.ts',factoryMedia:'apps/planner/src/scene/factoryMedia.ts',
 factoryPresentation:'apps/planner/src/services/factoryPresentation.ts',presentationHost:'apps/planner/src/services/presentationHost.ts',
 frozenProcess:'apps/planner/src/services/frozenProcess.ts'};
for(const [name,file]of Object.entries(files)){
 let code=stripTypeScriptTypes(await readFile(new URL(file,root),'utf8'),{mode:'transform'});
 for(const method of ['dispatch','readMediaAsset','readRunObservation'])code=code.replace(new RegExp(`import\\s*\\{\\s*${method}\\s*\\}\\s*from\\s*['"][^'"]+['"];?`,'g'),`const ${method}=async()=>{throw Error('Native transport is not used by this captured actual Host consumer');};`);
 code=code.replace(/from\s+['"]([^'"]+)['"]/g,(all,ref)=>ref.endsWith('/packages/presentation/src')?"from './index.mjs'":files[ref.split('/').at(-1)]?`from './${ref.split('/').at(-1)}.mjs'`:all);
 await writeFile(new URL(name+'.mjs',out),code);
}
export const json=async path=>JSON.parse(await readFile(new URL(path,root),'utf8'));
export const bytes=async path=>{const b=await readFile(new URL(path,root));return b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength);};
