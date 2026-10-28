import {readFile,writeFile,mkdir} from 'node:fs/promises';
for(const name of ['regression-factory.test.mjs','regression-process.test.mjs','regression-port-source.mjs','regression-port-fields.test.mjs','regression-port-host.test.mjs','regression-factory-browser.mjs']){
 let source=await readFile(new URL('../media-adoption-source/'+name,import.meta.url),'utf8');source=source.replaceAll('runtime-media-adoption-consumer','runtime-item-layout-consumer');
 if(name==='regression-factory.test.mjs')source=source.replace("/import \\{ readMediaAsset \\} from '.\\/mediaAssets';/","/import \\{ readMediaAsset[^\\n]*from '.\\/mediaAssets';/");
 await writeFile(new URL(name,import.meta.url),source);
}
await writeFile(new URL('tsconfig.json',import.meta.url),JSON.stringify({extends:'../../../apps/planner/tsconfig.json',compilerOptions:{types:['vite/client']},include:['harness.tsx'],exclude:[]}));
const out=new URL('../../../scenarios/媒体表现/runtime-item-layout-consumer/old-port/',import.meta.url);await mkdir(out,{recursive:true});await writeFile(new URL('expected.json',out),await readFile(new URL('../../../scenarios/媒体表现/runtime-media-adoption-consumer/old-port/expected.json',import.meta.url)));
