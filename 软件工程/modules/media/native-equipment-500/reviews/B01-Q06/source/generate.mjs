import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';
import {exportModel} from './export.mjs';import {device,pack,version} from './contract.mjs';import {specs,inputPort,outputPort} from './batch-spec.mjs';
const home=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),library=path.resolve(home,'../..'),json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n');
const original=JSON.parse(await fs.readFile(library+'/catalog.json','utf8')).equipment,equipment=Object.entries(specs).map(([id,s])=>({...original.find(e=>e.id===id),cycle_seconds:18,mechanisms:['print'],rule:'transform',auxiliary:s.auxiliary,extra_outputs:[],version,media_version:version,review_batch:'B01-Q06',basis:'大张纸板纵向剪切：1张0.20平方米原纸转为2张各0.10平方米分切纸板，无虚构边料。'})),entries=[],register=[];
for(const spec of equipment){
 for(const simple of [true,false]){const dir=path.join(home,'installable',spec.id+(simple?'-simple':'')),entry=await exportModel(spec,simple,dir,await fs.readFile(path.join(library,'../native-equipment-50/nameplates',spec.id+'.png')));entry.directory=path.relative(home,dir).replaceAll('\\','/');entries.push(entry);register.push({id:entry.module_id,version,directory:entry.directory});}
 const d=device(spec),op=d.payload.data.behavior.inline,s=specs[spec.id];
 for(const input of op.inputs)input.required_quality=['released'];for(const output of op.outputs)output.declared_output_quality='released';
 for(const[i,p]of op.phases.entries())p.phase_resources=[{role:i===1?'auto_fixture':'operator',quantity:{unit:i===1?'slot':'person',value:{coefficient:1,decimal_places:0}},qualifications:[]}];
 if(s.mode==='cure')op.phases[1].phase_resources.push({role:'lamp_circuit',quantity:{unit:'slot',value:{coefficient:1,decimal_places:0}},qualifications:[]});
 op.outputs[0].quantity=structuredClone(op.outputs[0].quantity);op.outputs[0].quantity.value.coefficient=2;
 const dir=home+'/devices/'+spec.id;await fs.mkdir(dir,{recursive:true});await json(dir+'/module.json',pack(d,[],[spec.media_id+'.simple',spec.media_id].map(id=>({id,version}))));register.push({id:spec.definition_id,version,directory:'devices/'+spec.id});
}
register.sort((a,b)=>(a.id.includes('.device.')?2:a.id.endsWith('.simple')?0:1)-(b.id.includes('.device.')?2:b.id.endsWith('.simple')?0:1));
await json(home+'/catalog.json',{review_batch:'B01-Q06',status:'in_progress_not_counted',version,equipment});await json(home+'/inventory.json',{devices:equipment.length,models:entries.length,batches:[{id:'B01',title:'纸包装单张分切逐机复审'}],entries});await json(home+'/registration.json',{device_count:equipment.length,media_count:entries.length,status:'in_progress_not_counted',register_order:register});await json(home+'/geometry.json',Object.fromEntries(Object.entries(specs).map(([id,s])=>[id,{input:inputPort(s),output:outputPort(s),width:4.2,depth:2}])));
const accepted=JSON.parse(await fs.readFile(library+'/accepted-registration.json','utf8')).device_count;await json(home+'/quality-review.json',{review_batch:'B01-Q06',status:'in_progress_not_counted',batch_accepted_types:0,formal_supported_device_count:accepted,pending:['per-device visual/mechanism review','actual holder and waits','ordinary Native save/reopen']});console.log('B01-Q06: one candidate, two GLBs; not counted.');


