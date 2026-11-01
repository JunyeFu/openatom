// Reopen the delivered editable source in isolated copies, leaving candidate bytes intact.
import fs from 'node:fs/promises';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
const root=process.cwd(),base=path.join(root,'modules/media/equipment-quality-v04');
const inventory=JSON.parse(await fs.readFile(path.join(base,'inventory.json'),'utf8'));
const output=path.join(root,'scenarios/媒体表现/model-quality-20261019/batch/source-rebuild.json'),results=[];
for(const e of inventory.entries){
  const source=path.join(base,e.directory),copy=path.join(root,'.work/model-quality-20261019/rebuild',e.family+(e.suffix==='simple'?'-simple':''));
  await fs.cp(source,copy,{recursive:true});
  const log=execFileSync(process.execPath,[path.join(copy,'source/generate.mjs')],{cwd:root,encoding:'utf8',windowsHide:true});
  const expected=await fs.readFile(path.join(source,e.asset.path)),actual=await fs.readFile(path.join(copy,e.asset.path));
  results.push({family:e.family,suffix:e.suffix,source:path.relative(root,source).replaceAll('\\','/'),reopened:true,byte_identical:expected.equals(actual),bytes:actual.length,log:log.trim()});
}
await fs.mkdir(path.dirname(output),{recursive:true});await fs.writeFile(output,JSON.stringify({scope:'14 families, 28 detailed/simple packages; independently reopened source copies; not visual acceptance',results},null,2));
console.log(JSON.stringify({models:results.length,byte_identical:results.filter(r=>r.byte_identical).length}));
if(results.some(r=>!r.byte_identical))process.exitCode=1;
