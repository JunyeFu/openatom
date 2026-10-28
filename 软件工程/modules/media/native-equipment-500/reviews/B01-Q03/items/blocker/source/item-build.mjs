import * as T from 'three';import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {specs,samples,fraction,inkPort,inkSeat} from './batch-spec.mjs';import {definition,pack,version} from './contract.mjs';
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}};
const here=path.dirname(fileURLToPath(import.meta.url)),paper=new T.MeshStandardMaterial({name:'Actual coated paperboard',color:'#d9cfb5',roughness:.80,side:T.DoubleSide}),gold=new T.MeshStandardMaterial({name:'Actual transferred metal layer',color:'#d8ad46',metalness:.9,roughness:.23}),pet=new T.MeshStandardMaterial({name:'Actual PET carrier',color:'#bbc9d5',roughness:.22,transparent:true,opacity:.4,side:T.DoubleSide});
export function buildItem(cfg){
 const s=specs[cfg.device],root=new T.Group();root.name='ActualFlexo_'+cfg.id;const clips=[],marks=[],patches=[];let liquid;
 const mesh=(n,g,pos,mat=paper)=>{const o=new T.Mesh(g,mat);o.name=n;o.position.fromArray(pos);root.add(o);return o;};
 const box=(n,size,pos,mat=paper)=>mesh(n,new T.BoxGeometry(...size),pos,mat),aux=cfg.role.startsWith('auxiliary'),c=aux?s.colors.find(c=>c.role===cfg.role):null;
 if(cfg.role==='blocker')box('ExplicitOutputBlocker',[.10,.02,.10],[0,.01,0]);
 else if(aux){const height=c.volume/(Math.PI*.020*.020);liquid=mesh('ActualInkDose_'+c.name,new T.CylinderGeometry(.020,.020,height,40),[0,height/2,0],new T.MeshStandardMaterial({name:'Actual '+c.name+' ink',color:c.color,roughness:.35}));}
 else{
  const g=new T.BoxGeometry(...s.size),pos=g.attributes.position,idx=g.index,kept=[];for(let i=0;i<idx.count;i+=3){const t=[idx.getX(i),idx.getX(i+1),idx.getX(i+2)];if(!t.every(n=>pos.getY(n)>0))kept.push(...t);}g.clearGroups();g.setIndex(kept);mesh('ActualPaperSubstrate',g,[0,s.size[1]/2,0]);
  const sh=new T.Shape();sh.moveTo(-.25,-.20);sh.lineTo(.25,-.20);sh.lineTo(.25,.20);sh.lineTo(-.25,.20);sh.closePath();for(const c of s.colors){const hole=new T.Path();hole.moveTo(-.05,c.z-.05);hole.lineTo(-.05,c.z+.05);hole.lineTo(.05,c.z+.05);hole.lineTo(.05,c.z-.05);hole.closePath();sh.holes.push(hole);}const top=new T.ShapeGeometry(sh);top.rotateX(-Math.PI/2);mesh('ActualPaperTopOutsideMarks',top,[0,s.size[1],0]);
  for(const c of s.colors){const plane=new T.PlaneGeometry(.10,.10);plane.rotateX(-Math.PI/2);patches.push(mesh('ActualUnprinted_'+c.name,plane,[0,s.size[1],c.z]));marks.push(box('ActualPrint_'+c.name,[.10,.000020,.10],[0,s.size[1]+.000010,c.z],new T.MeshStandardMaterial({name:'Transferred '+c.name+' ink',color:c.color,roughness:.50})));}
 }
 const states=cfg.role==='blocker'?[]:aux?['dose_ready']:[s.initial,s.final],names=states.length?[...states,...['load','auto','unload'].map(p=>states[0]+':'+p)]:['idle'];
 for(const name of names){const list=[],phase=name.split(':')[1],uAt=t=>phase==='auto'?t:phase==='unload'||name===s.final?1:0,vector=(o,prop,f)=>list.push(new T.VectorKeyframeTrack(o.name+'.'+prop,samples,samples.flatMap(t=>f(uAt(t),t))));
  if(aux){const height=c.volume/(Math.PI*.020*.020);vector(liquid,'scale',u=>[1,1-fraction(c,u),1]);vector(liquid,'position',(u,t)=>{const offset=phase==='load'?inkPort(c).map((v,i)=>(v-inkSeat(c)[i])*(1-t)):[0,0,0];return[offset[0],height*(1-fraction(c,u))/2+offset[1],offset[2]];});}
  else for(let i=0;i<marks.length;i++){const c=s.colors[i];vector(marks[i],'scale',u=>[fraction(c,u),1,1]);vector(marks[i],'position',u=>[.05-.05*fraction(c,u),s.size[1]+.000010,c.z]);vector(patches[i],'scale',u=>[1-fraction(c,u),1,1]);vector(patches[i],'position',u=>[-.05*fraction(c,u),s.size[1],c.z]);}
  if(!list.length){const o=root.children[0];list.push(new T.VectorKeyframeTrack(o.name+'.position',[0,1],[...o.position.toArray(),...o.position.toArray()]));}clips.push(new T.AnimationClip(name,1,list));
 }
 return{root,clips,states};
}
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.scale??[1,1,1])[i]])),unit:'meter',coordinate_frame:'right_handed_y_up'});
export async function exportItem(cfg,dir,copySource=true){
 const {root,clips,states}=buildItem(cfg),id='qa.paper.b01q03.media.'+cfg.id,itemId='qa.paper.b01q03.items.'+cfg.id;
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(root,{binary:true,animations:clips}));await fs.mkdir(dir,{recursive:true});await fs.writeFile(path.join(dir,'paper.glb'),bytes);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12))),asset={id:id+'.glb',path:'paper.glb',kind:'glb'};
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.85,texture:null}})),poses:[],actions:[],timeline:(j.animations??[]).map((a,i)=>({id:cfg.id+'-'+a.name,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n'),files=['paper.glb','LICENSE.txt','source/item-build.mjs','source/batch-spec.mjs','source/contract.mjs','source/rebuild.json'];
 if(copySource){await fs.mkdir(path.join(dir,'source'),{recursive:true});for(const file of ['item-build.mjs','batch-spec.mjs','contract.mjs'])await fs.copyFile(path.join(here,file),path.join(dir,'source',file));await fs.copyFile(path.join(here,'../LICENSE.txt'),path.join(dir,'LICENSE.txt'));await json(path.join(dir,'source/rebuild.json'),cfg);}
 await json(path.join(dir,'module.json'),pack(definition(id,'真实持有纸材 '+cfg.id,'media',media,[asset]),files,[],[asset]));
 const wrapper={...media,source:{kind:'media_ref',data:{id,version}},nodes:[],timeline:[],lod:{detailed:null,simplified:{kind:'media_ref',data:{id,version}},region:'device_icons'}};
 const aux=cfg.role.startsWith('auxiliary'),tracking=aux?'batch':'individual',s=specs[cfg.device];
 const attributes=states.length?{presentation_state:{kind:'enum',value:states[0]}}:{},attribute_schema=states.length?[{name:'presentation_state',value_type:'enum',unit:null,shape:null,options:states,basis:'module_declared'}]:[];
 const values=aux?{declared_dose_volume_m3:s.colors.find(c=>c.role===cfg.role).volume}:cfg.role==='blocker'?{}:{length_m:.50,width_m:.40,thickness_m:.0008,area_m2:.20,declared_blue_layer_volume_m3:.0000002};
 for(const [name,value]of Object.entries(values)){attributes[name]={kind:'number',value};attribute_schema.push({name,value_type:'number',unit:null,shape:null,options:[],basis:'module_declared'});}
 const itemDir=path.join(dir,'item-definition');await fs.mkdir(itemDir,{recursive:true});const d=definition(itemId,'明确纸材类型 '+cfg.id,'items',{unit:'piece',tracking,spec_ref:null,attributes,attribute_schema,presentation_or_media_ref:wrapper});d.attribute_schema=attribute_schema;await json(path.join(itemDir,'module.json'),pack(d,[],[{id,version}]));
 root.traverse(o=>{if(o.isMesh)o.geometry.dispose();});return {id,itemId,version,tracking,states,attributes,attribute_schema,bytes:bytes.length};
}
if(await fs.access(path.join(here,'rebuild.json')).then(()=>true,()=>false))await exportItem(JSON.parse(await fs.readFile(path.join(here,'rebuild.json'),'utf8')),path.resolve(here,'..'),false);
