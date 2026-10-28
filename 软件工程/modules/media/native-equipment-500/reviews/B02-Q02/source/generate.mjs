import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {exportModel} from './export.mjs';
import {device,pack,version} from './contract.mjs';
import {specs,paper} from './batch-spec.mjs';
const here=path.dirname(fileURLToPath(import.meta.url)),home=path.resolve(here,'..'),library=path.resolve(home,'../..');
const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n');
const original=JSON.parse(await fs.readFile(path.join(library,'catalog.json'),'utf8')).equipment;
const equipment=Object.keys(specs).map(id=>({...original.find(e=>e.id===id),rule:specs[id].quantity===1?'preserve':'transform',version,media_version:version,review_batch:'B02-Q02',basis:'在审：独立纸材Item；分张/卷放/收卷/双窄卷/60张横切；声明尺寸与周期。'}));
const entries=[],register=[];
for(const spec of equipment){
 const s=specs[spec.id];
 for(const simple of [true,false]){
  const dir=path.join(home,'installable',spec.id+(simple?'-simple':'')),entry=await exportModel(spec,simple,dir,await fs.readFile(path.join(library,'nameplates',spec.id+'.png')));
  entry.directory=path.relative(home,dir).replaceAll('\\','/');entries.push(entry);register.push({id:entry.module_id,version,directory:entry.directory});
 }
 const pkg=pack(device(spec),[],[spec.media_id+'.simple',spec.media_id].map(id=>({id,version}))),op=pkg.data.definition.payload.data.behavior.inline;
 const req=(role,unit)=>({role,quantity:{unit,value:{coefficient:1,decimal_places:0}},qualifications:[]});
 for(const [i,phase]of op.phases.entries())phase.phase_resources=[req(i===1?'auto_fixture':'operator',i===1?'slot':'person')];
 for(const input of op.inputs)input.required_quality=['released'];
 for(const output of op.outputs){output.declared_output_quality='released';output.quantity=structuredClone(output.quantity);output.quantity.value.coefficient=s.quantity;}
 if(s.quantity===1){
  op.material_flow={primary_role:'input',auxiliary_roles:[],acquire_after_phase:null,consume_after_phase:'auto',reserve_before_phase:'auto',deliver_after_phase:s.mode==='sheet'?'auto':'unload',required_primary_attributes:{presentation_state:{kind:'enum',value:s.initialState}},attributes_after_consume:{presentation_state:{kind:'enum',value:s.finalState}}};
 }
 const dir=path.join(home,'devices',spec.id);await fs.mkdir(dir,{recursive:true});await json(path.join(dir,'module.json'),pkg);register.push({id:spec.definition_id,version,directory:path.relative(home,dir).replaceAll('\\','/')});
}
register.sort((a,b)=>(a.id.includes('.device.')?2:a.id.endsWith('.simple')?0:1)-(b.id.includes('.device.')?2:b.id.endsWith('.simple')?0:1));
await json(path.join(home,'catalog.json'),{review_batch:'B02-Q02',status:'in_progress_not_counted',version,paper,equipment});
await json(path.join(home,'inventory.json'),{devices:5,models:10,entries});
await json(path.join(home,'registration.json'),{device_count:5,media_count:10,status:'in_progress_not_counted',register_order:register});
await json(path.join(home,'quality-review.json'),{review_batch:'B02-Q02',status:'in_progress_not_counted',formal_supported_device_count:5,batch_accepted_types:0,remaining_device_count:495,pending:['per-device visual and contact review','actual paper item media','real Host material identity and quantity checks','Native holder/wait/save-reopen acceptance']});
console.log('B02-Q02: five candidate definitions and ten candidate GLBs; not counted.');
