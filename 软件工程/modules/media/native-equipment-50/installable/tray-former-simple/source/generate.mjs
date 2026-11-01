// Generates canonical ModulePackages and self-contained GLB 2.0, using existing project dependencies.
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import * as T from 'three';
import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {equipment,version,references} from './catalog.mjs';
import {buildEquipment} from './geometry.mjs';
import {embedSurfaces} from './surface-pack.mjs';
const here=path.dirname(fileURLToPath(import.meta.url)),home=path.resolve(here,'..');
const settings=await fs.readFile(path.join(here,'rebuild.json'),'utf8').then(JSON.parse,()=>null);
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}readAsDataURL(b){b.arrayBuffer().then(v=>{this.result=`data:${b.type};base64,${Buffer.from(v).toString('base64')}`;this.onloadend?.();});}};
const json=(name,data)=>fs.writeFile(name,JSON.stringify(data,null,2)+'\n');
const decode=b=>JSON.parse(b.subarray(20,20+b.readUInt32LE(12)).toString());
const ref=(id)=>({id,version});
const vector=a=>({x:a[0],y:a[1],z:a[2]});
const pose=n=>({translation:vector(n.translation??[0,0,0]),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:vector(n.scale??[1,1,1]),unit:'meter',coordinate_frame:'right_handed_y_up'});
const duration=value=>({value,unit:'millisecond',basis:'module_declared',source_ref:null});
const quantity={value:{coefficient:1,decimal_places:0},unit:'piece'};
const auxiliary=spec=>['flexo-printer','offset-printer','varnish-coater','film-laminator','flute-laminator','window-patcher','folder-gluer','crash-lock-gluer','carton-stitcher','adhesive-dispenser','strapper','shrink-wrapper','tape-case-sealer','label-applicator','pallet-stretch-wrapper'].includes(spec.id);
const rights=assets=>assets.map(a=>({asset_ref:a.id,source:'Original parametric SF Native50 geometry, illustrative dimensions and original micro-surfaces; editable source supplied. Not manufacturer CAD.',license:'CC0-1.0',redistribution_confirmed:true,missing_reason:null}));
const definition=(id,title,category,data,assets=[])=>({id,version,title,definition_state:'runtime_candidate',contribution:{author:'Super Factory Native50 original modeling',license:'CC0-1.0',assets:rights(assets)},parameters:[],attribute_schema:[],payload:{category,data}});
const pack=(definition,files=[],dependencies=[],assets=[])=>({schema_version:'1.0',data:{definition,files:files.map(path=>({path,kind:'asset'})),dependencies,assets,rights:rights(assets),issues:[]}});
async function embedLabel(binary,png){
 const b=Buffer.from(binary),j=decode(b),offset=20+b.readUInt32LE(12),original=b.subarray(offset+8,offset+8+b.readUInt32LE(offset));
 const bin=Buffer.concat([original,png,Buffer.alloc((4-png.length%4)%4)]);j.bufferViews??=[];const bv=j.bufferViews.push({buffer:0,byteOffset:original.length,byteLength:png.length})-1;
 j.images??=[];j.textures??=[];const im=j.images.push({bufferView:bv,mimeType:'image/png',name:'Original equipment identification'})-1;
 const tex=j.textures.push({source:im})-1;for(const m of j.materials??[])if(m.name==='LABEL_ATLAS')m.pbrMetallicRoughness.baseColorTexture={index:tex};
 j.buffers[0].byteLength=bin.length;const raw=Buffer.from(JSON.stringify(j)),chunk=Buffer.concat([raw,Buffer.alloc((4-raw.length%4)%4,32)]),out=Buffer.alloc(28+chunk.length+bin.length);
 out.writeUInt32LE(0x46546c67);out.writeUInt32LE(2,4);out.writeUInt32LE(out.length,8);out.writeUInt32LE(chunk.length,12);out.writeUInt32LE(0x4e4f534a,16);chunk.copy(out,20);out.writeUInt32LE(bin.length,20+chunk.length);out.writeUInt32LE(0x004e4942,24+chunk.length);bin.copy(out,28+chunk.length);return out;
}
function presentation(spec,j,asset,built,simple){
 const indices=new Map(j.nodes.map((n,i)=>[n.name,i])),anchor=name=>{if(!indices.has(name))throw Error(spec.id+' missing anchor '+name);return{kind:'scene_node',data:indices.get(name)};};
 const ports=Object.fromEntries(['input','output',...(spec.rule==='inspect'?['rejected']:[]),...(spec.rule==='pack'?['container']:[]),...(spec.rule==='assemble'?['component']:[]),...(auxiliary(spec)?['auxiliary']:[])].map(id=>[id,anchor('Port_'+id)]));
 return {kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},
  nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.5,texture:null}})),
  poses:[],actions:[],timeline:j.animations.map((a,i)=>({id:spec.id+'-'+a.name,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),
  tags:[{id:'identity',label:spec.title,anchor:{kind:'viewport',data:{x:.18,y:.1}},progress_range:null,payload:{kind:'annotation',data:spec.basis}}],
  anchor_roles:{...ports,operator:anchor('Port_operator'),cargo:anchor(built.cargo)},port_anchors:ports,
  lod:{detailed:null,simplified:simple?{kind:'asset',data:asset}:{kind:'media_ref',data:ref(spec.media_id+'.simple')},region:'device_icons'},production_effect:'none'};
}
function device(spec){
 const role=(name)=>({role:name,port:name,quantity,required_quality:[],declared_output_quality:null});
 const policy={transform:'transform_with_origin',preserve:'preserve',inspect:'preserve',assemble:'assembly_with_composition',rework:'preserve',pack:'container_with_contents'}[spec.rule];
 let inputs=[role('input')],outputs=[role('output')];
 if(spec.rule==='assemble')inputs.push(role('component'));
 if(spec.rule==='pack')inputs.push(role('container'));
 if(auxiliary(spec))inputs.push(role('auxiliary'));
 if(spec.rule==='inspect')outputs.push(role('rejected'));
 if(spec.existing_media){
  const f=spec.existing_media.family;
  if(f==='assembly'){inputs=[role('shell'),role('pcb'),{...role('fastener'),quantity:{value:{coefficient:4,decimal_places:0},unit:'piece'}}];outputs=[role('assembled')];}
  if(f==='packing'){inputs=[role('product'),role('carton')];outputs=[role('packed')];}
  if(f==='robot'){inputs=[role('pickup')];outputs=[role('place')];}
  if(f==='inspection'){inputs=[role('input')];outputs=[role('passed'),role('rework')];}
  if(f==='sealer'){inputs=[role('loaded'),role('tape')];outputs=[role('sealed')];}
 }
 const ports=[...inputs.map(v=>({id:v.port,direction:'input',payload:{kind:'material',data:{item_role:v.role,unit:'piece',quantity_rule:null}}})),...outputs.map(v=>({id:v.port,direction:'output',payload:{kind:'material',data:{item_role:v.role,unit:'piece',quantity_rule:null}}}))];
 const total=spec.cycle_seconds*1000, times=[Math.round(total*.2),Math.round(total*.6),Math.round(total*.2)];
 const operation={builtin:{preserve:'transform',inspect:'inspect_and_route',assemble:'assemble',rework:'rework',pack:'pack',transform:'transform'}[spec.rule],item_roles:[...inputs,...outputs].map(v=>v.role),inputs,outputs,resource_roles:[],duration:null,
  phases:['load','auto','unload'].map((id,i)=>({id,duration:duration(times[i]),phase_resources:[],interruption_policy:'pause_resume',work_kind:i===0?'loading':i===2?'unloading':spec.rule==='inspect'?'inspection':spec.rule==='pack'?'packing':'processing'})),
  hold_on_output_wait:[],quality_and_routes:spec.rule==='inspect'?{source_kind:'preset',result_ref:null,max_completed_reworks:1,pass_port:'output',fail_port:'rejected',pending_policy:'hold_station',pending_location_role:null,isolation_location_role:null,release_required:true}:null,identity_policy:policy,family:spec.id};
 if(spec.existing_media?.family==='robot')operation.phases=['approach','grip','loaded_move','release','return'].map((id,i)=>({id,duration:duration([2000,1000,5000,1000,3000][i]),phase_resources:[],interruption_policy:'pause_resume',work_kind:i<2?'loading':i===4?'unloading':'transport'}));
 if(spec.existing_media?.family==='sealer')operation.phases.forEach((p,i)=>p.id=['infeed','seal','outfeed'][i]);
 if(spec.existing_media?.family==='inspection')Object.assign(operation.quality_and_routes,{pass_port:'passed',fail_port:'rework'});
 if(spec.rule==='preserve')operation.material_flow={primary_role:'input',auxiliary_roles:auxiliary(spec)?['auxiliary']:[],acquire_after_phase:null,consume_after_phase:auxiliary(spec)?'auto':null,reserve_before_phase:'auto',deliver_after_phase:'unload',required_primary_attributes:{},attributes_after_consume:{}};
 if(spec.rule==='pack')operation.pack_identity={output_role:'output',container_material_roles:['container'],content_roles:['input']};
 if(spec.existing_media?.family==='sealer')Object.assign(operation.material_flow,{primary_role:'loaded',auxiliary_roles:['tape'],consume_after_phase:'seal',reserve_before_phase:'seal',deliver_after_phase:'outfeed'});
 if(spec.existing_media?.family==='robot')Object.assign(operation.material_flow,{primary_role:'pickup',acquire_after_phase:'grip',consume_after_phase:'grip',reserve_before_phase:'loaded_move',deliver_after_phase:'release'});
 if(spec.existing_media?.family==='packing')operation.pack_identity={output_role:'packed',container_material_roles:['carton'],content_roles:['product']};
 const media={kind:'model',source:{kind:'media_ref',data:{id:spec.media_id,version:spec.media_version??version}},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:[],poses:[],actions:[],timeline:[],tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'media_ref',data:{id:spec.media_id+'.simple',version:spec.media_version??version}},region:'device_icons'},production_effect:'none'};
 return definition(spec.definition_id,spec.title,'devices',{placement:'fixed_during_simulation',ports,behavior:{selected:'inline',inline:operation,operation_ref:null},configuration:{},presentation:media,operations:['configure','inspect','preview_cycle']});
}

if(!settings){
 await json(path.join(home,'catalog.json'),{schema_version:'1.0',date:'2026-10-21',device_count:equipment.length,paper_devices:equipment.filter(s=>s.group==='paper').length,common_devices:equipment.filter(s=>s.group==='common').length,source_refs:['docs/00_项目基线/首版交付基线.md','docs/02_交互设计/设备建模精度与表现验收规范.json','modules/templates/carton-factory/candidate/definitions.json'],references,equipment});
 const py=spawnSync(process.env.SF_NATIVE50_PYTHON??'python',[path.join(here,'create_surfaces.py')],{cwd:home,windowsHide:true,encoding:'utf8'});if(py.status!==0)throw Error(py.stderr);console.log(py.stdout.trim());
}
const inventory=[],order=[];
for(const spec of settings?equipment.filter(s=>s.id===settings.id):equipment){
 for(const simple of settings?[settings.simple]:[true,false]){
  if(spec.existing_media&&!settings){
   const {family,directory}=spec.existing_media,suffix=simple?'-simple':'',dir=path.join(home,'installable',spec.id+suffix),origin=path.resolve(home,'../equipment-quality-'+directory+'/installable/'+family+suffix);
   await fs.cp(origin,dir,{recursive:true});const pkg=JSON.parse(await fs.readFile(path.join(dir,'module.json'),'utf8')).data,media=pkg.definition.payload.data,asset=media.source.data;
   const bytes=await fs.readFile(path.join(dir,asset.path)),j=decode(bytes),phase_ids=j.animations.map(a=>a.name),primary_phase=family==='robot'?'loaded_move':family==='sealer'?'seal':'auto';
   const triangles=(j.meshes??[]).reduce((n,m)=>n+m.primitives.reduce((a,p)=>a+(p.indices!==undefined?j.accessors[p.indices].count:j.accessors[p.attributes.POSITION].count)/3,0),0);
   inventory.push({...spec,suffix:simple?'simple':'detailed',module_id:pkg.definition.id,directory:path.relative(home,dir).replaceAll('\\','/'),asset,bytes:bytes.length,nodes:j.nodes.length,triangles,animations:j.animations.map((a,index)=>({index,name:a.name})),phase_ids,primary_phase,presentation:media,reused_exact_version:true});
   order.push({id:pkg.definition.id,version:pkg.definition.version,directory:dir});console.log(`${spec.index}/50 ${spec.id}: reuse exact ${pkg.definition.id}@${pkg.definition.version}`);continue;
  }
  const built=buildEquipment(spec,simple),{root}=built;const used=new Set();root.traverse(o=>{if(used.has(o.name))throw Error('Duplicate node '+o.name);used.add(o.name);});
  // Empty phase clips explicitly hold the first/last active pose, never invent stock.
  const main=built.animations.find(a=>a.name==='auto');
  for(const clip of built.animations)if(!clip.tracks.length){
   const endpoint=clip.name==='load'?0:1;
   clip.tracks=main.tracks.map(track=>{const size=track.getValueSize(),row=Array.from(track.values.slice(endpoint?track.values.length-size:0,endpoint?undefined:size));return new track.constructor(track.name,[0,1],[...row,...row]);});
  }
  const suffix=simple?'-simple':'',id=spec.media_id+(simple?'.simple':''),dir=settings?home:path.join(home,'installable',spec.id+suffix);
  await fs.mkdir(path.join(dir,'source/surfaces'),{recursive:true});const labelPath=settings?path.join(home,'nameplate.png'):path.join(home,'nameplates',spec.id+'.png');const label=await fs.readFile(labelPath);
  let binary=await new GLTFExporter().parseAsync(root,{binary:true,animations:built.animations,onlyVisible:true});binary=await embedSurfaces(await embedLabel(binary,label),simple);
  const bytes=Buffer.from(binary),j=decode(bytes),asset={id:id+'.glb',path:spec.id+'.glb',kind:'glb'};await fs.writeFile(path.join(dir,asset.path),bytes);await fs.writeFile(path.join(dir,'nameplate.png'),label);
  const media=presentation(spec,j,asset,built,simple),assets=[asset,{id:id+'.nameplate',path:'nameplate.png',kind:'png'}];
  if(!settings){
   for(const source of ['generate.mjs','geometry.mjs','catalog.mjs','surface-pack.mjs','create_surfaces.py'])await fs.copyFile(path.join(here,source),path.join(dir,'source',source));
   await fs.cp(path.join(here,'surfaces'),path.join(dir,'source/surfaces'),{recursive:true});await json(path.join(dir,'source/rebuild.json'),{id:spec.id,simple});await fs.copyFile(path.join(home,'LICENSE.txt'),path.join(dir,'LICENSE.txt'));
  }
  const files=[asset.path,'nameplate.png','LICENSE.txt','source/generate.mjs','source/geometry.mjs','source/catalog.mjs','source/surface-pack.mjs','source/create_surfaces.py','source/rebuild.json',...(await fs.readdir(path.join(here,'surfaces'))).map(n=>'source/surfaces/'+n)];
  await json(path.join(dir,'module.json'),pack(definition(id,spec.title+(simple?' · 简化':' · 详细'),'media',media,assets),files,simple?[]:[ref(spec.media_id+'.simple')],assets));
  const bounds=new T.Box3().setFromObject(root);const triangles=(j.meshes??[]).reduce((n,m)=>n+m.primitives.reduce((v,p)=>v+(p.indices!==undefined?j.accessors[p.indices].count:j.accessors[p.attributes.POSITION].count)/3,0),0);
  inventory.push({...spec,suffix:simple?'simple':'detailed',module_id:id,directory:path.relative(home,dir).replaceAll('\\','/'),asset,bytes:bytes.length,nodes:j.nodes.length,triangles,animations:j.animations.map((a,index)=>({index,name:a.name})),phase_ids:j.animations.map(a=>a.name),primary_phase:'auto',bounds:{min:bounds.min.toArray(),max:bounds.max.toArray()},presentation:media});
  order.push({id,version,directory:dir});console.log(`${spec.index}/50 ${spec.id} ${simple?'simple':'detailed'}: ${bytes.length} bytes / ${j.nodes.length} nodes / ${triangles} tris`);
  root.traverse(o=>{if(o.isMesh)o.geometry.dispose();});
 }
}
if(!settings){
 const supporting=[];const register=order.sort((a,b)=>Number(!a.id.endsWith('.simple'))-Number(!b.id.endsWith('.simple')));
 for(const spec of equipment.slice(1)){const dir=path.join(home,'devices',spec.id);await fs.mkdir(dir,{recursive:true});await json(path.join(dir,'module.json'),pack(device(spec),[],[spec.media_id,spec.media_id+'.simple'].map(id=>({id,version:spec.media_version??version}))));register.push({id:spec.definition_id,version,directory:dir});}
 // One of the 50 real device definitions is also the bundle root; no fictitious 51st device.
 for(const entry of register){
  const relative=path.relative(home,entry.directory).replaceAll('\\','/');const pkg=JSON.parse(await fs.readFile(path.join(entry.directory,'module.json'),'utf8'));
  supporting.push(relative+'/module.json',...pkg.data.files.map(f=>relative+'/'+f.path));
 }
 const first=device(equipment[0]);await json(path.join(home,'module.json'),pack(first,[...new Set(supporting)],register.filter(e=>e.id!==first.id).map(e=>({id:e.id,version:e.version}))));
 register.push({id:first.id,version,directory:home});
 await json(path.join(home,'registration.json'),{bundle_directory:'modules/media/native-equipment-50',register_order:register.map(e=>({...e,directory:'modules/media/native-equipment-50/'+path.relative(home,e.directory).replaceAll('\\','/')})),device_count:50,media_count:100});
 await json(path.join(home,'inventory.json'),{version,devices:equipment.length,models:inventory.length,scope:'canonical_installable_devices_and_actual_GLB_not_full_MQ_acceptance',entries:inventory});
}
