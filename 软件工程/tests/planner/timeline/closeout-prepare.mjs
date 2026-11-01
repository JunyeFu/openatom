// Relocate unchanged legacy tests' generated outputs into this task's own delivery.
// Assertions and frozen inputs are retained; the real Host producers are compiled unchanged.
import {readFile,writeFile,mkdir,copyFile,readdir} from 'node:fs/promises';
export const root=new URL('../../../',import.meta.url);
export const out=new URL('.work/p204-closeout-20261022/delivery/',root);
export const producerRoot=new URL('producer-root/',out);
const staged=new URL('staged-tests/',out);
await mkdir(staged,{recursive:true});
await mkdir(new URL('packages/commands/',producerRoot),{recursive:true});
await mkdir(new URL('.work/delivery/host/',producerRoot),{recursive:true});
await mkdir(new URL('.work/delivery/nfr-release/',producerRoot),{recursive:true});
const mappings=[];
for(const file of await readdir(new URL('tests/planner/timeline/',root))){
 if(!file.endsWith('.mjs')||file.startsWith('closeout-')||file==='browser.mjs'||file==='recheck-w5.mjs')continue;
 const raw=await readFile(new URL('tests/planner/timeline/'+file,root),'utf8');
 const value=raw.replaceAll('../../../',root.href)
  .replaceAll('.work/delivery/', '.work/p204-closeout-20261022/delivery/producer-root/.work/delivery/');
 await writeFile(new URL(file,staged),value);
 mappings.push({source:'tests/planner/timeline/'+file,target:'staged-tests/'+file,path_relocation_only:true});
}
for(const variant of ['balanced','dense']){
 const rel=`tests/planner/p201-whole/fixtures/b60/cut-r2/${variant}.input.json`,dest=new URL(rel,producerRoot);
 await mkdir(new URL('./',dest),{recursive:true});await copyFile(new URL(rel,root),dest);
}
const validation=await readFile(new URL('tests/planner/timeline/noncu-validate-nfr.py',root),'utf8');
await writeFile(new URL('validate-nfr.py',out),validation.replace("ROOT=Path(__file__).resolve().parents[3];OUT=ROOT/'.work/delivery/nfr-release'",`ROOT=Path(${JSON.stringify(decodeURIComponent(root.pathname).replace(/^\/([A-Za-z]:)/,'$1'))});OUT=Path(${JSON.stringify(decodeURIComponent(new URL('.work/delivery/nfr-release/',producerRoot).pathname).replace(/^\/([A-Za-z]:)/,'$1'))})`));
await writeFile(new URL('path-relocations.json',out),JSON.stringify({assertions_unchanged:true,real_producer_sources_unchanged:true,mappings},null,2));
console.log(JSON.stringify({staged_tests:mappings.length,output:out.href}));
