import * as T from 'three';import fs from 'node:fs/promises';import path from 'node:path';import{fileURLToPath}from'node:url';import{GLTFExporter}from'three/addons/exporters/GLTFExporter.js';
import{clip}from'./geometry.mjs';import{stock}from'./stock.mjs';import{specs,samples,auxPort,auxSeat,componentState}from'./batch-spec.mjs';import{definition,pack,version}from'./contract.mjs';
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}};
const here=path.dirname(fileURLToPath(import.meta.url));
export function buildItem(cfg){
 const spec=specs[cfg.device],root=new T.Group();root.name='ItemRoot';const built=stock(cfg.device,cfg.role);root.add(built.root);
 const states=cfg.role==='blocker'?[]:[cfg.role==='auxiliary'?spec.auxiliary:cfg.role==='component'?componentState:cfg.role==='output'?spec.final:spec.initial],clips=[];
 const initial=p=>p.pose(cfg.role==='output'?'unload':'load',cfg.role==='output'?1:0,cfg.role==='output');
 for(const p of built.parts){const a=initial(p);if(a.p)p.o.position.fromArray(a.p);if(a.r)p.o.quaternion.setFromAxisAngle(new T.Vector3(...['x','y','z'].map(k=>Number(k===a.r[0]))),a.r[1]);if(a.s)p.o.scale.fromArray(a.s);}
 for(const state of states.length?states:['idle'])for(const phase of[null,'load','auto','unload']){
  const tracks=[],ph=cfg.role==='output'?'unload':phase??'load';
  for(const p of built.parts){
   const at=u=>p.pose(ph,cfg.role==='output'?1:phase?u:0,cfg.role==='output'||!!phase),times=p.times??samples,a=at(0);
   if(a.p)tracks.push(new T.VectorKeyframeTrack(p.o.name+'.position',times,times.flatMap(u=>at(u).p)));
   if(a.r)tracks.push(new T.QuaternionKeyframeTrack(p.o.name+'.quaternion',times,times.flatMap(u=>{const r=at(u).r;return new T.Quaternion().setFromAxisAngle(new T.Vector3(...['x','y','z'].map(k=>Number(k===r[0]))),r[1]).toArray();})));
   if(a.s)tracks.push(new T.VectorKeyframeTrack(p.o.name+'.scale',times,times.flatMap(u=>at(u).s)));
  }
  if(cfg.role==='auxiliary')tracks.push(new T.VectorKeyframeTrack(built.root.name+'.position',[0,1],[0,1].flatMap(u=>{if(!phase)return[0,0,0];const k=phase==='load'?u:1;return auxPort().map((v,i)=>v+(auxSeat()[i]-v)*k-auxSeat()[i]);})));
  if(!tracks.length)tracks.push(new T.VectorKeyframeTrack(built.root.name+'.position',[0,1],[0,0,0,0,0,0]));
  clips.push(clip(phase?state+':'+phase:state,tracks));
 }
 return{root,clips,states};
}
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.scale??[1,1,1])[i]])),unit:'meter',coordinate_frame:'right_handed_y_up'});
export async function exportItem(cfg,dir,copySource=true){
 const{root,clips,states}=buildItem(cfg),spec=specs[cfg.device],id='qa.paper.b02q10.media.'+cfg.id,itemId='qa.paper.b02q10.items.'+cfg.id;
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(root,{binary:true,animations:clips}));await fs.mkdir(dir,{recursive:true});await fs.writeFile(path.join(dir,'paper.glb'),bytes);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12))),asset={id:id+'.glb',path:'paper.glb',kind:'glb'};
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.8,texture:null}})),poses:[],actions:[],timeline:j.animations.map((a,i)=>({id:cfg.id+'-'+a.name,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n'),sources=['item-build.mjs','batch-spec.mjs','contract.mjs','geometry.mjs','stock.mjs'];
 if(copySource){await fs.mkdir(path.join(dir,'source'),{recursive:true});for(const f of sources)await fs.copyFile(path.join(here,f),path.join(dir,'source',f));await fs.copyFile(path.join(here,'../LICENSE.txt'),path.join(dir,'LICENSE.txt'));await json(path.join(dir,'source/rebuild.json'),cfg);}
 await json(path.join(dir,'module.json'),pack(definition(id,'实际物料 '+cfg.id,'media',media,[asset]),['paper.glb','LICENSE.txt',...sources.map(f=>'source/'+f),'source/rebuild.json'],[],[asset]));
 const tracking=['auxiliary','blocker','waste'].includes(cfg.role)?'batch':'individual',attributes=states.length?{presentation_state:{kind:'enum',value:states[0]}}:{},attribute_schema=states.length?[{name:'presentation_state',value_type:'enum',unit:null,shape:null,options:states,basis:'module_declared'}]:[];
 let values={};
 if(cfg.role!=='blocker'){
  const area=spec.kind==='plate'?Math.PI*spec.rt**2:Math.PI*(spec.rt+spec.rb)*Math.hypot(spec.H,spec.rt-spec.rb);
  values=cfg.role==='component'?{bottom_radius_m:spec.rb,paper_thickness_m:spec.t,heat_seal_coating_ready:1}:cfg.role==='input'?{paper_area_m2:area,paper_thickness_m:spec.t,heat_seal_coating_ready:spec.kind==='plate'?0:1}:{height_m:spec.H,top_diameter_m:2*spec.rt,bottom_diameter_m:2*spec.rb,paper_thickness_m:spec.t,sidewall_or_blank_area_m2:area,open_top:1,...(spec.kind==='plate'?{pressed_plate:1}:{sealed_bottom:1,source_sidewalls:1,source_bottom_discs:1,nominal_capacity_ml:Math.PI*spec.H/3*(spec.rb**2+spec.rb*spec.rt+spec.rt**2)*1e6})};
 }
 for(const[name,value]of Object.entries(values)){attributes[name]={kind:'number',value};attribute_schema.push({name,value_type:'number',unit:null,shape:null,options:[],basis:'module_declared'});}
 const itemDir=path.join(dir,'item-definition');await fs.mkdir(itemDir,{recursive:true});const wrapper={...media,source:{kind:'media_ref',data:{id,version}},nodes:[],timeline:[],lod:{detailed:null,simplified:{kind:'media_ref',data:{id,version}},region:'device_icons'}},d=definition(itemId,'明确物料类型 '+cfg.id,'items',{unit:'piece',tracking,spec_ref:null,attributes,attribute_schema,presentation_or_media_ref:wrapper});d.attribute_schema=attribute_schema;await json(path.join(itemDir,'module.json'),pack(d,[],[{id,version}]));
 return{id,itemId,version,tracking,states,attributes,attribute_schema,bytes:bytes.length};
}
if(await fs.access(path.join(here,'rebuild.json')).then(()=>true,()=>false))await exportItem(JSON.parse(await fs.readFile(path.join(here,'rebuild.json'),'utf8')),path.resolve(here,'..'),false);
