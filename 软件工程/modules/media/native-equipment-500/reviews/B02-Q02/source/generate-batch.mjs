import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {loadBatch,version} from './catalog.mjs';
import {exportModel} from './export.mjs';
import {device,pack} from './contract.mjs';
const here=path.dirname(fileURLToPath(import.meta.url)),home=path.resolve(here,'..'),batch=await loadBatch(process.argv[2]??'B02');
const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n');
await fs.mkdir(path.join(home,'batches'),{recursive:true});await json(path.join(home,'batches',batch.id+'.catalog.json'),batch);
const py=spawnSync(process.env.SF_NATIVE500_PYTHON??'python',[path.join(here,'labels.py'),path.join(home,'batches',batch.id+'.catalog.json')],{windowsHide:true,encoding:'utf8'});if(py.status!==0)throw Error(py.stderr);
const entries=[],register=[];
for(const spec of batch.equipment){
 for(const simple of [true,false]){
  const dir=path.join(home,'installable',spec.id+(simple?'-simple':'')),label=await fs.readFile(path.join(home,'nameplates',spec.id+'.png'));
  const entry=await exportModel(spec,simple,dir,label);entry.directory=path.relative(home,dir).replaceAll('\\','/');entries.push(entry);register.push({id:entry.module_id,version,directory:entry.directory});
 }
 const dir=path.join(home,'devices',spec.id);await fs.mkdir(dir,{recursive:true});await json(path.join(dir,'module.json'),pack(device(spec),[],[spec.media_id+'.simple',spec.media_id].map(id=>({id,version}))));
 register.push({id:spec.definition_id,version,directory:path.relative(home,dir).replaceAll('\\','/')});console.log(`${batch.id} ${spec.index}/500 ${spec.id} ${entries.at(-1).bytes} bytes`);
}
await json(path.join(home,'batches',batch.id+'.inventory.json'),{batch:batch.id,title:batch.title,devices:50,models:entries.length,entries,register});
