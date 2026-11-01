import {readFile,writeFile,mkdir,copyFile} from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';
const root=new URL('../../../',import.meta.url),folder=new URL('tests/planner/media-adoption-source/',root),out=new URL('scenarios/媒体表现/runtime-media-adoption-consumer/',root);await mkdir(out,{recursive:true});
const before=[['presentationHost','apps/planner/src/services/presentationHost.ts','ts'],['factoryPresentation','apps/planner/src/services/factoryPresentation.ts','ts'],['DeviceProcessWorkspace','apps/planner/src/features/factory/DeviceProcessWorkspace.tsx','tsx']],mapping=new Map(before.map(([n,p,e])=>[fileURLToPath(new URL(p,root)),fileURLToPath(new URL(n+'.before.'+e,folder))]));
for(const [name,p,extension]of before){
 const file=fileURLToPath(new URL(p,root));let s=await readFile(new URL(name+'.before.'+extension+'.txt',out),'utf8');
 const rewrite=(all,start,quote,spec)=>{let target=path.resolve(path.dirname(file),spec);for(const [old,next]of mapping)if(target===old.replace(/\.tsx?$/,''))target=next.replace(/\.tsx?$/,'');let relative=path.relative(fileURLToPath(folder),target).replaceAll('\\','/');if(!relative.startsWith('.'))relative='./'+relative;return start+quote+relative+quote;};
 s=s.replace(/(from\s*|import\s*)(['"])(\.[^'"]+)\2/g,rewrite).replace(/(import\(\s*)(['"])(\.[^'"]+)\2/g,rewrite);
 await writeFile(new URL(name+'.before.'+extension,folder),s);
}
const records=[];
for(const [name,p,previous]of [['process','tests/planner/process-workspace/presentationHost.test.mjs','scenarios/媒体表现/runtime-root-consumer/'],['factory','tests/planner/factory-media/source.test.mjs','scenarios/媒体表现/runtime-factory-canvas/']]){
 const target='regression-'+name+'.test.mjs';await writeFile(new URL(target,folder),(await readFile(new URL(p,root),'utf8')).replaceAll(previous,'scenarios/媒体表现/runtime-media-adoption-consumer/old-'+name+'/'));records.push({original:p,target,change:'outputs only; two Root-approved Draft qualifier evolutions live in original source'});
}
for(const name of ['source.mjs','fields.test.mjs','host.test.mjs'])await writeFile(new URL('regression-port-'+name,folder),(await readFile(new URL('tests/planner/port-output-media/'+name,root),'utf8')).replaceAll('scenarios/图片时间轴/runtime-port-output/','scenarios/媒体表现/runtime-media-adoption-consumer/old-port/').replaceAll("'./source.mjs'","'./regression-port-source.mjs'"));
await mkdir(new URL('old-port/',out),{recursive:true});await copyFile(new URL('scenarios/图片时间轴/runtime-port-output/expected.json',root),new URL('old-port/expected.json',out));
await writeFile(new URL('regression-factory-browser.mjs',folder),(await readFile(new URL('tests/planner/factory-media/browser.mjs',root),'utf8')).replaceAll('scenarios/媒体表现/runtime-factory-canvas/','scenarios/媒体表现/runtime-media-adoption-consumer/old-factory/'));
await writeFile(new URL('source-relocation.json',out),JSON.stringify(records,null,2)+'\n');
