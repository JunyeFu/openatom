import * as T from 'three';import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {specs,seatX,thickness,cellPose,panelPoints,foldAngle,gluePoint,glueFraction,auxSeat,doseRadius,doseHeight,doseSegments,glueHeight,glueWidth,samples,finalHeight} from './batch-spec.mjs';import {definition,pack,version} from './contract.mjs';
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}};
const here=path.dirname(fileURLToPath(import.meta.url)),paper=new T.MeshStandardMaterial({name:'Original precreased paperboard',color:'#d2b882',roughness:.86}),adhesive=new T.MeshStandardMaterial({name:'Metered water based glue',color:'#d6ba6b',roughness:.48});
export function buildItem(cfg){
 const s=specs[cfg.device],root=new T.Group();root.name='ActualSleeveStock_'+cfg.id;const clips=[],cells=[];let liquid,band;
 const mesh=(n,g,pos,mat=paper)=>{const o=new T.Mesh(g,mat);o.name=n;o.position.fromArray(pos);root.add(o);return o;};
 if(cfg.role==='blocker')mesh('ExplicitOutputBlocker',new T.BoxGeometry(.1,.02,.1),[0,.01,0]);
 if(['input','output'].includes(cfg.role)){
  for(const side of[-1,1])mesh('CentralPanel'+side,new T.BoxGeometry(.40,thickness,.0999),[0,thickness/2,side*.05005]);mesh('CentralScoredBridge',new T.BoxGeometry(.40,.0004,.0002),[0,.0002,0]);
  for(const side of[-1,1]){const qs=panelPoints(side);for(let i=0;i<qs.length-1;i++){const a=qs[i],b=qs[i+1],p=cellPose(side,a,b,cfg.role==='output'?Math.PI:0),o=mesh('ActualFoldCell'+side+'_'+i,new T.BoxGeometry(.40,p.thickness,p.depth),p.position);o.rotation.x=p.angle;cells.push({side,a,b,o});}}
 }
 if(cfg.role==='auxiliary'){
  liquid=mesh('ActualGlueDose',new T.CylinderGeometry(doseRadius,doseRadius,doseHeight,doseSegments),[0,doseHeight/2,0],adhesive);band=mesh('ActualSeamGlue',new T.BoxGeometry(1,glueHeight,glueWidth),[0,0,0],adhesive);band.scale.x=0;
 }
 if(cfg.role==='output'){const p=gluePoint('unload',1);band=mesh('ActualSeamGlue',new T.BoxGeometry(.40,glueHeight,glueWidth),[0,p.y,p.z],adhesive);band.rotation.x=p.angle;}
 const state=cfg.role==='input'?s.initial:cfg.role==='output'?s.final:cfg.role==='auxiliary'?'adhesive_dose_ready':null,states=state?[state]:[],names=state?[state,...['load','auto','unload'].map(ph=>state+':'+ph)]:['idle'];
 for(const name of names){const phase=name.split(':')[1]??'load',ts=name.includes(':')?samples:[0,1],uAt=u=>name.includes(':')?u:0,list=[];
  const vec=(o,key,f)=>list.push(new T.VectorKeyframeTrack(o.name+'.'+key,ts,ts.flatMap(u=>f(uAt(u)))));
  if(cfg.role==='input')for(const{side,a,b,o}of cells){vec(o,'position',u=>cellPose(side,a,b,foldAngle(side,phase,u)).position);list.push(new T.QuaternionKeyframeTrack(o.name+'.quaternion',ts,ts.flatMap(u=>new T.Quaternion().setFromAxisAngle(new T.Vector3(1,0,0),cellPose(side,a,b,foldAngle(side,phase,uAt(u))).angle).toArray())));}
  if(liquid){vec(liquid,'scale',u=>[1,1-glueFraction(phase,u),1]);vec(liquid,'position',u=>[0,doseHeight*(1-glueFraction(phase,u))/2,0]);vec(band,'scale',u=>[.40*glueFraction(phase,u),1,1]);vec(band,'position',u=>{const a=auxSeat(s),p=gluePoint(phase,u);return[seatX(s,phase,u)-.20+.20*glueFraction(phase,u)-a[0],s.surface+p.y-a[1],p.z-a[2]];});list.push(new T.QuaternionKeyframeTrack(band.name+'.quaternion',ts,ts.flatMap(u=>new T.Quaternion().setFromAxisAngle(new T.Vector3(1,0,0),gluePoint(phase,uAt(u)).angle).toArray())));}
  if(!list.length){const o=root.children[0];list.push(new T.VectorKeyframeTrack(o.name+'.position',[0,1],[...o.position.toArray(),...o.position.toArray()]));}clips.push(new T.AnimationClip(name,1,list));
 }
 return{root,clips,states};
}
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.scale??[1,1,1])[i]])),unit:'meter',coordinate_frame:'right_handed_y_up'});
export async function exportItem(cfg,dir,copySource=true){
 const {root,clips,states}=buildItem(cfg),id='qa.paper.b01q13.media.'+cfg.id,itemId='qa.paper.b01q13.items.'+cfg.id;
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(root,{binary:true,animations:clips}));await fs.mkdir(dir,{recursive:true});await fs.writeFile(path.join(dir,'paper.glb'),bytes);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12))),asset={id:id+'.glb',path:'paper.glb',kind:'glb'};
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.85,texture:null}})),poses:[],actions:[],timeline:(j.animations??[]).map((a,i)=>({id:cfg.id+'-'+a.name,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n'),files=['paper.glb','LICENSE.txt','source/item-build.mjs','source/batch-spec.mjs','source/contract.mjs','source/rebuild.json'];
 if(copySource){await fs.mkdir(path.join(dir,'source'),{recursive:true});for(const file of ['item-build.mjs','batch-spec.mjs','contract.mjs'])await fs.copyFile(path.join(here,file),path.join(dir,'source',file));await fs.copyFile(path.join(here,'../LICENSE.txt'),path.join(dir,'LICENSE.txt'));await json(path.join(dir,'source/rebuild.json'),cfg);}
 await json(path.join(dir,'module.json'),pack(definition(id,'真实持有纸材 '+cfg.id,'media',media,[asset]),files,[],[asset]));
 const wrapper={...media,source:{kind:'media_ref',data:{id,version}},nodes:[],timeline:[],lod:{detailed:null,simplified:{kind:'media_ref',data:{id,version}},region:'device_icons'}};
 const aux=cfg.role.startsWith('auxiliary'),tracking=aux?'batch':'individual',s=specs[cfg.device];
 const attributes=states.length?{presentation_state:{kind:'enum',value:states[0]}}:{},attribute_schema=states.length?[{name:'presentation_state',value_type:'enum',unit:null,shape:null,options:states,basis:'module_declared'}]:[];
 const values=cfg.role==='blocker'?{}:aux?{declared_dose_volume_m3:s.volume}:cfg.role==='input'?{length_m:.40,developed_width_m:.42,area_m2:.168,thickness_m:thickness}:{length_m:.40,flat_width_m:.20,developed_area_m2:.168,finished_total_height_m:finalHeight,adhesive_volume_m3:s.volume};
 for(const [name,value]of Object.entries(values)){attributes[name]={kind:'number',value};attribute_schema.push({name,value_type:'number',unit:null,shape:null,options:[],basis:'module_declared'});}
 const itemDir=path.join(dir,'item-definition');await fs.mkdir(itemDir,{recursive:true});const d=definition(itemId,'明确纸材类型 '+cfg.id,'items',{unit:'piece',tracking,spec_ref:null,attributes,attribute_schema,presentation_or_media_ref:wrapper});d.attribute_schema=attribute_schema;await json(path.join(itemDir,'module.json'),pack(d,[],[{id,version}]));
 root.traverse(o=>{if(o.isMesh)o.geometry.dispose();});return {id,itemId,version,tracking,states,attributes,attribute_schema,bytes:bytes.length};
}
if(await fs.access(path.join(here,'rebuild.json')).then(()=>true,()=>false))await exportItem(JSON.parse(await fs.readFile(path.join(here,'rebuild.json'),'utf8')),path.resolve(here,'..'),false);
