import * as T from 'three';import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {specs,film,spoolX,spoolY,fedLength,pathState,filmPoint,samples} from './batch-spec.mjs';import {definition,pack,version} from './contract.mjs';
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}};
const here=path.dirname(fileURLToPath(import.meta.url));
const board=new T.MeshStandardMaterial({name:'Actual paperboard substrate',color:'#d9cfb5',roughness:.80});
const bop=new T.MeshStandardMaterial({name:'Actual precoated clear BOPP25um',color:'#d5e4ed',roughness:.18,metalness:.05,transparent:true,opacity:.38,depthWrite:false});
export function buildItem(cfg){
 const s=specs[cfg.device],root=new T.Group();root.name='ActualLamination_'+cfg.id;const clips=[],cells=[];let cover;
 const mesh=(n,g,p,m=board)=>{const o=new T.Mesh(g,m);o.name=n;o.position.fromArray(p);root.add(o);return o;};
 if(cfg.role==='blocker')mesh('ExplicitOutputBlocker',new T.BoxGeometry(.1,.02,.1),[0,.01,0]);
 else if(cfg.role==='input'){
  mesh('ActualBoard',new T.BoxGeometry(...s.size),[0,s.size[1]/2,0]);
  cover=mesh('ActualBondedBOPP',new T.BoxGeometry(1,film.thickness,film.width),[.25,s.size[1]+film.thickness/2,0],bop);cover.scale.x=0;
 }else for(let i=0;i<film.cells;i++)cells.push(mesh('ActualFilmCell'+i,new T.BoxGeometry(film.length/film.cells,film.thickness,film.width),[0,0,0],bop));
 for(const[i,o]of cells.entries()){const st=pathState(0),p=filmPoint((i+.5)*film.length/film.cells,st);o.position.set(p.x-spoolX,p.y-spoolY,0);o.rotation.z=p.yaw;}
 const states=cfg.role==='blocker'?[]:cfg.role==='input'?[s.initial,s.final]:['threaded_BOPP_tail'];
 const names=states.length?[...states,...['load','auto','unload'].map(ph=>states[0]+':'+ph)]:['idle'];
 for(const name of names){
  const phase=name.split(':')[1],ts=phase==='auto'?samples:[0,1],uAt=t=>phase==='auto'?t:phase==='unload'||name===s.final?1:0,list=[];
  const vector=(o,property,f)=>list.push(new T.VectorKeyframeTrack(o.name+'.'+property,ts,ts.flatMap(t=>f(uAt(t)))));
  if(cover){vector(cover,'scale',u=>[fedLength(u),1,1]);vector(cover,'position',u=>[.25-fedLength(u)/2,s.size[1]+film.thickness/2,0]);}
  for(const[i,o]of cells.entries()){
   const len=film.length/film.cells,start=i*len,end=(i+1)*len;
   const at=u=>{const st=pathState(u),l=Math.max(0,end-Math.max(start,st.fed)),d=l>0?(Math.max(start,st.fed)+end)/2-st.fed:0,p=filmPoint(d,st);return{p,l};};
   vector(o,'position',u=>{const{p}=at(u);return[p.x-spoolX,p.y-spoolY,0];});
   vector(o,'scale',u=>[at(u).l/len,1,1]);
   list.push(new T.QuaternionKeyframeTrack(o.name+'.quaternion',ts,ts.flatMap(t=>new T.Quaternion().setFromAxisAngle(new T.Vector3(0,0,1),at(uAt(t)).p.yaw).toArray())));
  }
  if(!list.length){const o=root.children[0];list.push(new T.VectorKeyframeTrack(o.name+'.position',[0,1],[...o.position.toArray(),...o.position.toArray()]));}
  clips.push(new T.AnimationClip(name,1,list));
 }
 return{root,clips,states};
}
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.scale??[1,1,1])[i]])),unit:'meter',coordinate_frame:'right_handed_y_up'});
export async function exportItem(cfg,dir,copySource=true){
 const {root,clips,states}=buildItem(cfg),id='qa.paper.b01q10.media.'+cfg.id,itemId='qa.paper.b01q10.items.'+cfg.id;
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(root,{binary:true,animations:clips}));await fs.mkdir(dir,{recursive:true});await fs.writeFile(path.join(dir,'paper.glb'),bytes);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12))),asset={id:id+'.glb',path:'paper.glb',kind:'glb'};
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.85,texture:null}})),poses:[],actions:[],timeline:(j.animations??[]).map((a,i)=>({id:cfg.id+'-'+a.name,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n'),files=['paper.glb','LICENSE.txt','source/item-build.mjs','source/batch-spec.mjs','source/contract.mjs','source/rebuild.json'];
 if(copySource){await fs.mkdir(path.join(dir,'source'),{recursive:true});for(const file of ['item-build.mjs','batch-spec.mjs','contract.mjs'])await fs.copyFile(path.join(here,file),path.join(dir,'source',file));await fs.copyFile(path.join(here,'../LICENSE.txt'),path.join(dir,'LICENSE.txt'));await json(path.join(dir,'source/rebuild.json'),cfg);}
 await json(path.join(dir,'module.json'),pack(definition(id,'真实持有纸材 '+cfg.id,'media',media,[asset]),files,[],[asset]));
 const wrapper={...media,source:{kind:'media_ref',data:{id,version}},nodes:[],timeline:[],lod:{detailed:null,simplified:{kind:'media_ref',data:{id,version}},region:'device_icons'}};
 const aux=cfg.role.startsWith('auxiliary'),tracking=aux?'batch':'individual',s=specs[cfg.device];
 const attributes=states.length?{presentation_state:{kind:'enum',value:states[0]}}:{},attribute_schema=states.length?[{name:'presentation_state',value_type:'enum',unit:null,shape:null,options:states,basis:'module_declared'}]:[];
 const values=aux?{declared_length_m:film.length,film_width_m:film.width,film_thickness_m:film.thickness,declared_film_volume_m3:s.volume}:cfg.role==='blocker'?{}:{length_m:.50,width_m:.40,substrate_thickness_m:.0008,area_m2:.20,declared_film_volume_m3:s.volume,finished_total_thickness_m:.000825};
 for(const [name,value]of Object.entries(values)){attributes[name]={kind:'number',value};attribute_schema.push({name,value_type:'number',unit:null,shape:null,options:[],basis:'module_declared'});}
 const itemDir=path.join(dir,'item-definition');await fs.mkdir(itemDir,{recursive:true});const d=definition(itemId,'明确纸材类型 '+cfg.id,'items',{unit:'piece',tracking,spec_ref:null,attributes,attribute_schema,presentation_or_media_ref:wrapper});d.attribute_schema=attribute_schema;await json(path.join(itemDir,'module.json'),pack(d,[],[{id,version}]));
 root.traverse(o=>{if(o.isMesh)o.geometry.dispose();});return {id,itemId,version,tracking,states,attributes,attribute_schema,bytes:bytes.length};
}
if(await fs.access(path.join(here,'rebuild.json')).then(()=>true,()=>false))await exportItem(JSON.parse(await fs.readFile(path.join(here,'rebuild.json'),'utf8')),path.resolve(here,'..'),false);
