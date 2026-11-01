import * as T from 'three';import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {specs,seatX,faceDatum,facePath,crestCenters,crestTop,bottomLiner,mediumThickness,faceThickness,faceCells,nipX,glueHeight,coveredBand,appliedVolume,auxSeat,doseHeight,doseRadius,doseSegments,waveAt,waveSamples,samples} from './batch-spec.mjs';import {definition,pack,version} from './contract.mjs';
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}};
const here=path.dirname(fileURLToPath(import.meta.url));
const kraft=new T.MeshStandardMaterial({name:'Actual kraft liner',color:'#c8ae7b',roughness:.86}),medium=new T.MeshStandardMaterial({name:'Actual corrugated medium wall',color:'#bba06b',roughness:.90}),white=new T.MeshStandardMaterial({name:'Actual white face liner',color:'#e8e5d7',roughness:.77}),adhesive=new T.MeshStandardMaterial({name:'Actual wet water based adhesive',color:'#dcc884',roughness:.48});
export function mediumGeometry(){
 const top=[],bottom=[];for(const x of waveSamples){const p=waveAt(x),h=mediumThickness/2/Math.hypot(1,p.dy);top.push([x-p.dy*h,p.y+h]);bottom.push([x+p.dy*h,p.y-h]);}
 const sh=new T.Shape();sh.moveTo(...top[0]);for(const v of top.slice(1))sh.lineTo(...v);for(const v of bottom.reverse())sh.lineTo(...v);sh.closePath();const g=new T.ExtrudeGeometry(sh,{depth:.40,bevelEnabled:false});g.translate(0,0,-.20);return g;
}
export function buildItem(cfg){
 const s=specs[cfg.device],root=new T.Group();root.name='ActualFluteStock_'+cfg.id;const clips=[],face=[],bands=[];let liquid;
 const mesh=(n,g,p,mat=kraft)=>{const o=new T.Mesh(g,mat);o.name=n;o.position.fromArray(p);root.add(o);return o;};
 if(cfg.role==='blocker')mesh('ExplicitOutputBlocker',new T.BoxGeometry(.10,.02,.10),[0,.01,0]);
 if(['input','output'].includes(cfg.role)){
  mesh('ActualBottomLiner',new T.BoxGeometry(.50,bottomLiner,.40),[0,bottomLiner/2,0]);mesh('ActualCorrugatedMedium',mediumGeometry(),[0,0,0],medium);
 }
 if(cfg.role==='output'){
  mesh('ActualFinishedFace',new T.BoxGeometry(.50,faceThickness,.40),[0,crestTop+glueHeight+faceThickness/2,0],white);
  for(const[i,c]of crestCenters.entries())mesh('FinishedGlueLine'+i,new T.BoxGeometry(.001,glueHeight,.40),[c,crestTop+glueHeight/2,0],adhesive);
 }
 if(cfg.role==='component')for(let i=0;i<faceCells;i++)face.push(mesh('ActualFaceCell'+i,new T.BoxGeometry(.50/faceCells,faceThickness,.40),[0,0,0],white));
 if(cfg.role==='auxiliary'){
  liquid=mesh('ActualGlueDose',new T.CylinderGeometry(doseRadius,doseRadius,doseHeight,doseSegments),[0,doseHeight/2,0],adhesive);
  for(let i=0;i<crestCenters.length;i++){const o=mesh('ActualAppliedGlue'+i,new T.BoxGeometry(1,glueHeight,.40),[0,0,0],adhesive);o.scale.x=0;bands.push(o);}
 }
 const states=cfg.role==='blocker'?[]:[cfg.role==='input'?s.initial:cfg.role==='output'?s.final:cfg.role==='component'?'loose_face_liner':'adhesive_dose_ready'],names=states.length?[...states,...['load','auto','unload'].map(p=>states[0]+':'+p)]:['idle'];
 const point=(phase,u,i)=>{const d=seatX(s,phase,u)+.25-nipX,q=d-(i+.5)*.5/faceCells,p=facePath(q),anchor=faceDatum(s,phase,u);return{position:[p.x-anchor[0],p.y-anchor[1],0],angle:p.angle};};
 for(const[i,o]of face.entries()){const p=point('load',0,i);o.position.fromArray(p.position);o.rotation.z=p.angle;}
 for(const name of names){const phase=name.split(':')[1]??'load',ts=name.includes(':')?samples:[0,1],uAt=u=>name.includes(':')?u:0,list=[];
  const vec=(o,property,f)=>list.push(new T.VectorKeyframeTrack(o.name+'.'+property,ts,ts.flatMap(v=>f(uAt(v)))));
  for(const[i,o]of face.entries()){
   vec(o,'position',u=>point(phase,u,i).position);list.push(new T.QuaternionKeyframeTrack(o.name+'.quaternion',ts,ts.flatMap(v=>new T.Quaternion().setFromAxisAngle(new T.Vector3(0,0,1),point(phase,uAt(v),i).angle).toArray())));
  }
  if(liquid){const f=u=>1-appliedVolume(s,phase,u)/s.volume;vec(liquid,'scale',u=>[1,Math.max(0,f(u)),1]);vec(liquid,'position',u=>[0,doseHeight*Math.max(0,f(u))/2,0]);}
  for(const[i,o]of bands.entries()){
   vec(o,'scale',u=>[coveredBand(i,s,phase,u),1,1]);vec(o,'position',u=>[seatX(s,phase,u)+crestCenters[i]+.0005-coveredBand(i,s,phase,u)/2-auxSeat(s)[0],s.surface+crestTop+glueHeight/2-auxSeat(s)[1],-auxSeat(s)[2]]);
  }
  if(!list.length){const o=root.children[0];list.push(new T.VectorKeyframeTrack(o.name+'.position',[0,1],[...o.position.toArray(),...o.position.toArray()]));}clips.push(new T.AnimationClip(name,1,list));
 }
 return{root,clips,states};
}
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.scale??[1,1,1])[i]])),unit:'meter',coordinate_frame:'right_handed_y_up'});
export async function exportItem(cfg,dir,copySource=true){
 const {root,clips,states}=buildItem(cfg),id='qa.paper.b01q11.media.'+cfg.id,itemId='qa.paper.b01q11.items.'+cfg.id;
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(root,{binary:true,animations:clips}));await fs.mkdir(dir,{recursive:true});await fs.writeFile(path.join(dir,'paper.glb'),bytes);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12))),asset={id:id+'.glb',path:'paper.glb',kind:'glb'};
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.85,texture:null}})),poses:[],actions:[],timeline:(j.animations??[]).map((a,i)=>({id:cfg.id+'-'+a.name,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n'),files=['paper.glb','LICENSE.txt','source/item-build.mjs','source/batch-spec.mjs','source/contract.mjs','source/rebuild.json'];
 if(copySource){await fs.mkdir(path.join(dir,'source'),{recursive:true});for(const file of ['item-build.mjs','batch-spec.mjs','contract.mjs'])await fs.copyFile(path.join(here,file),path.join(dir,'source',file));await fs.copyFile(path.join(here,'../LICENSE.txt'),path.join(dir,'LICENSE.txt'));await json(path.join(dir,'source/rebuild.json'),cfg);}
 await json(path.join(dir,'module.json'),pack(definition(id,'真实持有纸材 '+cfg.id,'media',media,[asset]),files,[],[asset]));
 const wrapper={...media,source:{kind:'media_ref',data:{id,version}},nodes:[],timeline:[],lod:{detailed:null,simplified:{kind:'media_ref',data:{id,version}},region:'device_icons'}};
 const aux=cfg.role.startsWith('auxiliary'),tracking=aux?'batch':'individual',s=specs[cfg.device];
 const attributes=states.length?{presentation_state:{kind:'enum',value:states[0]}}:{},attribute_schema=states.length?[{name:'presentation_state',value_type:'enum',unit:null,shape:null,options:states,basis:'module_declared'}]:[];
 const values=cfg.role==='blocker'?{}:aux?{declared_dose_volume_m3:s.volume}:cfg.role==='component'?{length_m:.50,width_m:.40,area_m2:.20,face_thickness_m:faceThickness}:cfg.role==='input'?{length_m:.50,width_m:.40,area_m2:.20,substrate_height_m:crestTop,medium_thickness_m:mediumThickness,flute_pitch_m:.010}:{length_m:.50,width_m:.40,area_m2:.20,finished_total_thickness_m:crestTop+glueHeight+faceThickness,adhesive_volume_m3:s.volume};
 for(const [name,value]of Object.entries(values)){attributes[name]={kind:'number',value};attribute_schema.push({name,value_type:'number',unit:null,shape:null,options:[],basis:'module_declared'});}
 const itemDir=path.join(dir,'item-definition');await fs.mkdir(itemDir,{recursive:true});const d=definition(itemId,'明确纸材类型 '+cfg.id,'items',{unit:'piece',tracking,spec_ref:null,attributes,attribute_schema,presentation_or_media_ref:wrapper});d.attribute_schema=attribute_schema;await json(path.join(itemDir,'module.json'),pack(d,[],[{id,version}]));
 root.traverse(o=>{if(o.isMesh)o.geometry.dispose();});return {id,itemId,version,tracking,states,attributes,attribute_schema,bytes:bytes.length};
}
if(await fs.access(path.join(here,'rebuild.json')).then(()=>true,()=>false))await exportItem(JSON.parse(await fs.readFile(path.join(here,'rebuild.json'),'utf8')),path.resolve(here,'..'),false);
