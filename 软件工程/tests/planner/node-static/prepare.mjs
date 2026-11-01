import {readFile,writeFile,mkdir} from 'node:fs/promises';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-node-static/',root),old=new URL('../node-inspector/',import.meta.url);
for(const name of ['inline-native.json','registry.json'])await writeFile(new URL(name,out),await readFile(new URL('scenarios/媒体表现/runtime-node-inspector/'+name,root)));
for(const name of ['product.mjs','product-boundaries.mjs','product-cycle.mjs','model.test.mjs','reader-actual.test.mjs','seed-actual.test.mjs','regression-factory.test.mjs','regression-process.test.mjs','regression-port-source.mjs','regression-port-fields.test.mjs','regression-port-host.test.mjs','regression-factory-browser.mjs','regression-media-browser.mjs','regression-port-browser.mjs','regression-media.test.mjs','regression-shared-preview.test.mjs','regression-item-source.test.mjs','regression-item-browser.mjs','regression-item-wait-handover.mjs']){
 let source=await readFile(new URL(name,old),'utf8');
 source=source.replaceAll('runtime-node-inspector/','runtime-node-static/').replaceAll('tests/planner/node-inspector/harness.html','tests/planner/node-static/harness.html');
 source=source.replaceAll("from './regression-port-source.mjs'","from './old-regression-port-source.mjs'");
 if(name==='product.mjs')source=source.replace("new URL('product.json',out)","new URL('old-product.json',out)");
 // Test loaders follow the newly imported pure helper; no assertions or fixture eligibility change.
 if(source.includes('stripTypeScriptTypes')&&source.includes('presentationHost')){
  const varName=/compiled\s*=\s*new URL/.test(source)?'compiled':'out';
  const insertion=`await writeFile(new URL('frozenProcess.mjs',${varName}),stripTypeScriptTypes(await readFile(new URL('apps/planner/src/services/frozenProcess.ts',root),'utf8'),{mode:'transform'}));\n`;
  const at=source.indexOf("const {createPresentationHost");
  const importAt=source.indexOf('await import(');
  // Emit before first runtime import, after the output directory was created.
  const line=source.lastIndexOf('\n',importAt)+1;
  source=source.slice(0,line)+insertion+source.slice(line);
  source=source.replaceAll(".replace(/import \\{ dispatch", ".replaceAll(\"from './frozenProcess'\",\"from './frozenProcess.mjs'\").replace(/import \\{ dispatch");
 }
 await writeFile(new URL('old-'+name,import.meta.url),source);
}
for(const dir of ['old-port','old-port-ui','old-item-layout','old-media','old-media-pure'])await mkdir(new URL(dir+'/',out),{recursive:true});
for(const name of ['old-port/expected.json','old-port-ui/expected.json','old-item-layout/registry.json'])await writeFile(new URL(name,out),await readFile(new URL('scenarios/媒体表现/runtime-node-inspector/'+name,root)));
await writeFile(new URL('tsconfig.json',import.meta.url),await readFile(new URL('tsconfig.json',old)));
let source=await readFile(new URL('build.mjs',old),'utf8');source=source.replaceAll('runtime-node-inspector/','runtime-node-static/').replaceAll('tests/planner/node-inspector/harness.html','tests/planner/node-static/harness.html');await writeFile(new URL('build.mjs',import.meta.url),source);
