import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';
import {exportModel} from './export.mjs';import {device,pack,version} from './contract.mjs';import {specs,paperPort,outputPort,auxPort,backingPort,workX} from './batch-spec.mjs';
const home=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),library=path.resolve(home,'../..');
const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n');
const original=JSON.parse(await fs.readFile(path.join(library,'catalog.json'),'utf8')).equipment;
const equipment=Object.entries(specs).map(([id,s])=>({...original.find(e=>e.id===id),rule:s.preserve?'preserve':'transform',auxiliary:s.auxiliary??null,body_length:s.mode==='pad'?2.10:undefined,extra_outputs:s.extra_outputs,version,media_version:version,review_batch:'B02-Q04',basis:'在审：纸表面整饰五工艺；明确工件身份、实际耗材和烫金PET载体输出。'}));
const entries=[],register=[];
for(const spec of equipment){
 const s=specs[spec.id];
 for(const simple of [true,false]){const dir=path.join(home,'installable',spec.id+(simple?'-simple':'')),entry=await exportModel(spec,simple,dir,await fs.readFile(path.join(library,'nameplates',spec.id+'.png')));entry.directory=path.relative(home,dir).replaceAll('\\','/');entries.push(entry);register.push({id:entry.module_id,version,directory:entry.directory});}
 const d=device(spec),data=d.payload.data,op=data.behavior.inline;
 // Role quantities are independent values; changing scrap count cannot change the input quantity.
 op.inputs=structuredClone(op.inputs);op.outputs=structuredClone(op.outputs);
 for(const input of op.inputs)input.required_quality=['released'];
 for(const output of op.outputs)output.declared_output_quality='released';
 for(const e of s.extra_outputs){op.outputs.push({role:e.role,port:e.role,quantity:{value:{coefficient:e.count,decimal_places:0},unit:'piece'},required_quality:[],declared_output_quality:'released'});op.item_roles.push(e.role);data.ports.push({id:e.role,direction:'output',payload:{kind:'material',data:{item_role:e.role,unit:'piece',quantity_rule:null}}});}
 for(const [i,phase]of op.phases.entries())phase.phase_resources=[{role:i===1?'auto_fixture':'operator',quantity:{unit:i===1?'slot':'person',value:{coefficient:1,decimal_places:0}},qualifications:[]}];
 if(s.preserve)op.material_flow={primary_role:'input',auxiliary_roles:s.auxiliary?['auxiliary']:[],acquire_after_phase:null,consume_after_phase:'auto',reserve_before_phase:'auto',deliver_after_phase:'unload',required_primary_attributes:{presentation_state:{kind:'enum',value:s.initial}},attributes_after_consume:{presentation_state:{kind:'enum',value:s.final}}};
 const dir=path.join(home,'devices',spec.id);await fs.mkdir(dir,{recursive:true});await json(path.join(dir,'module.json'),pack(d,[],[spec.media_id+'.simple',spec.media_id].map(id=>({id,version}))));register.push({id:spec.definition_id,version,directory:path.relative(home,dir).replaceAll('\\','/')});
}
register.sort((a,b)=>(a.id.includes('.device.')?2:a.id.endsWith('.simple')?0:1)-(b.id.includes('.device.')?2:b.id.endsWith('.simple')?0:1));
await json(path.join(home,'catalog.json'),{review_batch:'B02-Q04',status:'in_progress_not_counted',version,equipment});await json(path.join(home,'inventory.json'),{devices:5,models:10,entries});await json(path.join(home,'registration.json'),{device_count:5,media_count:10,status:'in_progress_not_counted',register_order:register});
const accepted=JSON.parse(await fs.readFile(path.join(library,'accepted-registration.json'),'utf8')).device_count;
await json(path.join(home,'quality-review.json'),{review_batch:'B02-Q04',status:'in_progress_not_counted',formal_supported_device_count:accepted,batch_accepted_types:0,remaining_device_count:500-accepted,pending:['per-device visual and contact review','actual substrate, consumable and spent carrier item media','real Host identity, quantity and area checks','Native holder, missing consumable, spent carrier and phase/output waits, recovery, save and reopen']});
console.log('B02-Q04: five candidate definitions and ten GLBs; not counted.');

await json(path.join(home,'geometry.json'),Object.fromEntries(Object.entries(specs).map(([id,s])=>[id,{input:paperPort(s),output:outputPort(s),auxiliary:auxPort(s),backing:backingPort,work_x:workX(s),width:3.9,depth:2.1}])));
