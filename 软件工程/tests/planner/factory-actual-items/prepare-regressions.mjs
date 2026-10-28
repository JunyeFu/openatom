import {readFile,writeFile,mkdir,copyFile} from 'node:fs/promises';
const root=new URL('../../../',import.meta.url),folder=new URL('tests/planner/factory-actual-items/',root),records=[];
for(const name of ['source.test.mjs','browser.mjs']){
 const original=`tests/planner/factory-media/${name}`,target=`regression-factory-${name}`;
 const before=await readFile(new URL(original,root),'utf8'),after=before.replaceAll('scenarios/媒体表现/runtime-factory-canvas/','scenarios/媒体表现/runtime-factory-items/old-factory/');
 await writeFile(new URL(target,folder),after);records.push({original,target,change:'output directory only; original source/expectations read-only'});
}
for(const name of ['source.mjs','fields.test.mjs','host.test.mjs']){
 const original=`tests/planner/port-output-media/${name}`,target=`regression-port-${name}`;
 const before=await readFile(new URL(original,root),'utf8'),after=before.replaceAll('scenarios/图片时间轴/runtime-port-output/','scenarios/媒体表现/runtime-factory-items/old-port/').replaceAll("'./source.mjs'","'./regression-port-source.mjs'");
 await writeFile(new URL(target,folder),after);records.push({original,target,change:'output/helper path only; original source/expectations read-only'});
}
const processOriginal='tests/planner/process-workspace/presentationHost.test.mjs',processTarget='regression-process-source.test.mjs';
await writeFile(new URL(processTarget,folder),(await readFile(new URL(processOriginal,root),'utf8')).replaceAll('scenarios/媒体表现/runtime-root-consumer/','scenarios/媒体表现/runtime-factory-items/old-process/'));
records.push({original:processOriginal,target:processTarget,change:'output directory only; original process service 13 expectations read-only'});
const out=new URL('scenarios/媒体表现/runtime-factory-items/',root);await mkdir(out,{recursive:true});await writeFile(new URL('regression-source-relocation.json',out),JSON.stringify(records,null,2)+'\n');
await mkdir(new URL('old-port/',out),{recursive:true});await copyFile(new URL('scenarios/图片时间轴/runtime-port-output/expected.json',root),new URL('old-port/expected.json',out));
const uiBefore=await readFile(new URL('tests/planner/port-output-media/browser.mjs',root),'utf8');
const uiAfter=uiBefore.replace("out=new URL('scenarios/图片时间轴/runtime-port-output/',root)","out=new URL('scenarios/媒体表现/runtime-factory-items/old-port-ui/',root)");
await writeFile(new URL('regression-port-browser.mjs',folder),uiAfter);
await mkdir(new URL('old-port-ui/',out),{recursive:true});await copyFile(new URL('core_consumer.exe',out),new URL('old-port-ui/core_consumer.exe',out));await copyFile(new URL('scenarios/图片时间轴/runtime-port-output/expected.json',root),new URL('old-port-ui/expected.json',out));
