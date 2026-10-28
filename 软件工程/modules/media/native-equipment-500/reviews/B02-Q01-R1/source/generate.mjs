import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {exportModel} from './export.mjs';
import {device,pack,version} from './contract.mjs';
import {stock} from './paper-reviewed.mjs';
const here=path.dirname(fileURLToPath(import.meta.url)),home=path.resolve(here,'..'),library=path.resolve(home,'../..');
const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n');
const original=JSON.parse(await fs.readFile(path.join(library,'catalog.json'),'utf8')).equipment;
const equipment=Object.keys(stock).map(id=>({...original.find(e=>e.id===id),version,media_version:version,stock:stock[id],review_batch:'B02-Q01-R1',basis:'支腿接地修订1.1.1；具名工装与真实holder/等待/重开验收；声明尺寸与周期。'}));
const entries=[],register=[];
for(const spec of equipment){
 for(const simple of [true,false]){
  const dir=path.join(home,'installable',spec.id+(simple?'-simple':'')),entry=await exportModel(spec,simple,dir,await fs.readFile(path.join(library,'nameplates',spec.id+'.png')));
  entry.directory=path.relative(home,dir).replaceAll('\\','/');entries.push(entry);register.push({id:entry.module_id,version,directory:entry.directory});
 }
 const pkg=pack(device(spec),[],[spec.media_id+'.simple',spec.media_id].map(id=>({id,version}))),op=pkg.data.definition.payload.data.behavior.inline;
 const req=(role,unit)=>({role,quantity:{unit,value:{coefficient:1,decimal_places:0}},qualifications:[]});
 op.phases[0].phase_resources=[req('operator','person')];op.phases[1].phase_resources=[req('auto_fixture','slot')];op.phases[2].phase_resources=[req('operator','person')];
 for(const input of op.inputs)input.required_quality=['released'];
 for(const output of op.outputs)output.declared_output_quality='released';
 const dir=path.join(home,'devices',spec.id);await fs.mkdir(dir,{recursive:true});await json(path.join(dir,'module.json'),pkg);register.push({id:spec.definition_id,version,directory:path.relative(home,dir).replaceAll('\\','/')});
}
register.sort((a,b)=>(a.id.includes('.device.')?2:a.id.endsWith('.simple')?0:1)-(b.id.includes('.device.')?2:b.id.endsWith('.simple')?0:1));
await json(path.join(home,'catalog.json'),{review_batch:'B02-Q01-R1',version,equipment});
await json(path.join(home,'inventory.json'),{devices:5,models:10,entries});
await json(path.join(home,'registration.json'),{device_count:5,media_count:10,register_order:register});
console.log('B02-Q01-R1: five reviewed device definitions and ten GLBs created @'+version);
