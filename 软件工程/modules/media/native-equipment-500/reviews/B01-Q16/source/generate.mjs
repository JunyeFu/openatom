import fs from 'node:fs/promises';import path from 'node:path';import{fileURLToPath}from 'node:url';
import{exportModel}from './export.mjs';import{device,pack,version}from './contract.mjs';import{specs,profile,batch,inputPort,outputPort,auxPort,outputY,stagingX}from './batch-spec.mjs';
const home=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),library=path.resolve(home,'../..'),json=async(p,v)=>{await fs.mkdir(path.dirname(p),{recursive:true});await fs.writeFile(p,JSON.stringify(v,null,2)+'\n');};
const original=JSON.parse(await fs.readFile(library+'/catalog.json','utf8')).equipment,equipment=Object.entries(specs).map(([id,s])=>({...original.find(e=>e.id===id),...s,extra_outputs:[],version,media_version:version,review_batch:batch,basis:'原创示例尺寸、参数与声明工时；机构路径参考常见纸箱工艺，不代表厂家额定性能或物理求解。'})),entries=[],register=[];
for(const spec of equipment){for(const simple of[true,false]){const dir=path.join(home,'installable',spec.id+(simple?'-simple':'')),entry=await exportModel(spec,simple,dir,await fs.readFile(path.join(home,'nameplates',spec.id+'.png')));entry.directory=path.relative(home,dir).replaceAll('\\','/');entries.push(entry);register.push({id:entry.module_id,version,directory:entry.directory});}
 const d=device(spec),op=d.payload.data.behavior.inline;
 for(const input of op.inputs)input.required_quality=['released'];for(const output of op.outputs)output.declared_output_quality='released';
 for(const[i,p]of op.phases.entries())p.phase_resources=[{role:i===1?'auto_fixture':'operator',quantity:{unit:i===1?'slot':'person',value:{coefficient:1,decimal_places:0}},qualifications:[]}];
 if(spec.id==='compression-stack'){op.material_flow.required_primary_attributes={bond_state:{kind:'enum',value:'fresh'}};op.material_flow.attributes_after_consume={bond_state:{kind:'enum',value:'pressed'},presentation_state:{kind:'enum',value:spec.final}};}
 await json(home+'/devices/'+spec.id+'/module.json',pack(d,[],[spec.media_id+'.simple',spec.media_id].map(id=>({id,version}))));register.push({id:spec.definition_id,version,directory:'devices/'+spec.id});
}
for(const entry of entries.filter(e=>e.suffix==='detailed'))Object.assign(equipment.find(s=>s.id===entry.id),entry);
register.sort((a,b)=>(a.id.includes('.device.')?2:a.id.endsWith('.simple')?0:1)-(b.id.includes('.device.')?2:b.id.endsWith('.simple')?0:1));
await json(home+'/catalog.json',{review_batch:batch,modeling_profile:profile,status:'in_progress_not_counted',version,equipment});await json(home+'/inventory.json',{devices:equipment.length,models:entries.length,batches:[{id:'B01',title:'纸箱开箱、施胶与侧缝压合'}],entries});await json(home+'/registration.json',{device_count:equipment.length,media_count:entries.length,status:'in_progress_not_counted',register_order:register});
await json(home+'/geometry.json',Object.fromEntries(equipment.map(s=>[s.id,{input:inputPort(),output:outputPort(s.id),auxiliary:auxPort(),product_staging:[stagingX,outputY(s.id),0],width:3.6,depth:2.5}])));
await json(home+'/quality-review.json',{review_batch:batch,modeling_profile:profile,status:'in_progress_not_counted',version,batch_accepted_types:0,pending:['actual GLB review','critical layout and support','Host conditions','Native holder/wait/recovery','UI save and new-process reopen','public startup registration']});
console.log(batch+': three separate device candidates, six equipment GLBs; not counted.');
