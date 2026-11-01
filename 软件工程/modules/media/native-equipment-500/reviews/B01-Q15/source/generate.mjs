import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';
import {exportModel} from './export.mjs';import {device,pack,version} from './contract.mjs';import {specs,inputPort,outputPort,auxPort} from './batch-spec.mjs';
const home=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),library=path.resolve(home,'../..'),json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n');
const original=JSON.parse(await fs.readFile(library+'/catalog.json','utf8')).equipment,equipment=Object.entries(specs).map(([id,s])=>({...original.find(e=>e.id===id),title:'纸托盘成型机（四角胶接纸板托盒）',focus_node:'FormingMandrel',cycle_seconds:s.cycle_seconds,mechanisms:['guided-bottom-mandrel','four-wall-electric-folder','four-corner-tuck-slot-glue','four-corner-compression'],rule:'transform',auxiliary:s.auxiliary,extra_outputs:[],version,media_version:version,review_batch:'B01-Q15',flow:{input:'模切压痕纸板托坯 + 0.300mL已可施用热熔胶',output:'两来源四角胶接开口托盒'},basis:'预模切压痕纸板托坯经底面压持、四面折边、四角胶耳折入及0.300mL热熔胶施胶压持，生成保留两来源的新开口托盒。'})),entries=[],register=[];
for(const spec of equipment){
 for(const simple of [true,false]){const dir=path.join(home,'installable',spec.id+(simple?'-simple':'')),entry=await exportModel(spec,simple,dir,await fs.readFile(path.join(home,'nameplates',spec.id+'.png')));entry.directory=path.relative(home,dir).replaceAll('\\','/');entries.push(entry);register.push({id:entry.module_id,version,directory:entry.directory});}
 const d=device(spec),op=d.payload.data.behavior.inline,s=specs[spec.id];
 for(const input of op.inputs)input.required_quality=['released'];for(const output of op.outputs)output.declared_output_quality='released';
 for(const[i,p]of op.phases.entries())p.phase_resources=[{role:i===1?'auto_fixture':'operator',quantity:{unit:i===1?'slot':'person',value:{coefficient:1,decimal_places:0}},qualifications:[]}];
 const dir=home+'/devices/'+spec.id;await fs.mkdir(dir,{recursive:true});await json(dir+'/module.json',pack(d,[],[spec.media_id+'.simple',spec.media_id].map(id=>({id,version}))));register.push({id:spec.definition_id,version,directory:'devices/'+spec.id});
}
for(const entry of entries.filter(e=>e.suffix==='detailed'))Object.assign(equipment.find(s=>s.id===entry.id),entry);
register.sort((a,b)=>(a.id.includes('.device.')?2:a.id.endsWith('.simple')?0:1)-(b.id.includes('.device.')?2:b.id.endsWith('.simple')?0:1));
await json(home+'/catalog.json',{review_batch:'B01-Q15',status:'in_progress_not_counted',version,equipment});await json(home+'/inventory.json',{devices:equipment.length,models:entries.length,batches:[{id:'B01',title:'纸板托盒成型机逐机复审'}],entries});await json(home+'/registration.json',{device_count:equipment.length,media_count:entries.length,status:'in_progress_not_counted',register_order:register});await json(home+'/geometry.json',Object.fromEntries(Object.entries(specs).map(([id,s])=>[id,{input:inputPort(s),output:outputPort(s),...(s.auxiliary?{auxiliary:auxPort(s),product_staging:[s.stagingX,inputPort(s)[1],0]}:{}),width:3.4,depth:2.4}])));
await json(home+'/service-connections.json',{scope:'visible external utility routes and named closed internal paths; source/exported continuity and Native admission are separate',circuits:entries.find(e=>e.suffix==='detailed').service_circuits,device_admitted:false});
const priorQuality=JSON.parse(await fs.readFile(home+'/quality-review.json','utf8'));
const accepted=JSON.parse(await fs.readFile(library+'/accepted-registration.json','utf8')).device_count;await json(home+'/quality-review.json',{...priorQuality,review_batch:'B01-Q15',status:'in_progress_not_counted',batch_accepted_types:0,formal_supported_device_count:accepted,pending:priorQuality.pending});console.log('B01-Q15: one candidate, two GLBs; not counted.');


