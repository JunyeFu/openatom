import * as T from 'three';import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {specs,samples,scoredFraction} from './batch-spec.mjs';import {definition,pack,version} from './contract.mjs';
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}};
const here=path.dirname(fileURLToPath(import.meta.url)),paper=new T.MeshStandardMaterial({name:'Corrugated board surface',color:'#c8ae7b',roughness:.86});
export function buildItem(cfg){
 const s=specs[cfg.device],root=new T.Group();root.name='ActualCorrugatedBoard_'+cfg.id;const clips=[];
 const mesh=(n,g,p)=>{const o=new T.Mesh(g,paper);o.name=n;o.position.fromArray(p);root.add(o);return o;};let raw,scored;
 if(cfg.role==='blocker')raw=mesh('ExplicitOutputBlocker',new T.BoxGeometry(.1,.02,.1),[0,.01,0]);
 else{
  raw=mesh('ActualFlatBoard',new T.BoxGeometry(...s.size),[0,s.size[1]/2,0]);const shape=new T.Shape(),points=[[-.20,s.size[1]],...s.score_centers.flatMap(z=>[[z-.003,s.size[1]],[z,s.size[1]-s.score_depth],[z+.003,s.size[1]]]),[.20,s.size[1]]];shape.moveTo(-.2,0);for(const p of points)shape.lineTo(...p);shape.lineTo(.2,0);shape.closePath();const geo=new T.ExtrudeGeometry(shape,{depth:.50,bevelEnabled:false});geo.rotateY(Math.PI/2);geo.translate(-.25,0,0);scored=mesh('ActualScoredBoard',geo,[0,0,0]);
 }
 const states=cfg.role==='blocker'?[]:[s.initial,s.final],names=states.length?[...states,...states.flatMap(st=>['load','auto','unload'].map(ph=>st+':'+ph))]:['idle'];
 for(const name of names){const list=[],[st,ph]=name.split(':'),f=t=>st===s.final||ph==='unload'?1:ph==='auto'?scoredFraction(t):0,track=(o,prop,fn)=>list.push(new T.VectorKeyframeTrack(o.name+'.'+prop,samples,samples.flatMap(fn)));
  if(scored){track(raw,'scale',t=>[1-f(t),1,1]);track(raw,'position',t=>[-.25*f(t),s.size[1]/2,0]);track(scored,'scale',t=>[f(t),1,1]);track(scored,'position',t=>[.25-.25*f(t),0,0]);}
  else track(raw,'position',()=>raw.position.toArray());clips.push(new T.AnimationClip(name,1,list));
 }
 return{root,clips,states};
}
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.scale??[1,1,1])[i]])),unit:'meter',coordinate_frame:'right_handed_y_up'});
export async function exportItem(cfg,dir,copySource=true){
 const {root,clips,states}=buildItem(cfg),id='qa.paper.b01q05.media.'+cfg.id,itemId='qa.paper.b01q05.items.'+cfg.id;
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(root,{binary:true,animations:clips}));await fs.mkdir(dir,{recursive:true});await fs.writeFile(path.join(dir,'paper.glb'),bytes);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12))),asset={id:id+'.glb',path:'paper.glb',kind:'glb'};
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.85,texture:null}})),poses:[],actions:[],timeline:(j.animations??[]).map((a,i)=>({id:cfg.id+'-'+a.name,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n'),files=['paper.glb','LICENSE.txt','source/item-build.mjs','source/batch-spec.mjs','source/contract.mjs','source/rebuild.json'];
 if(copySource){await fs.mkdir(path.join(dir,'source'),{recursive:true});for(const file of ['item-build.mjs','batch-spec.mjs','contract.mjs'])await fs.copyFile(path.join(here,file),path.join(dir,'source',file));await fs.copyFile(path.join(here,'../LICENSE.txt'),path.join(dir,'LICENSE.txt'));await json(path.join(dir,'source/rebuild.json'),cfg);}
 await json(path.join(dir,'module.json'),pack(definition(id,'真实持有纸材 '+cfg.id,'media',media,[asset]),files,[],[asset]));
 const wrapper={...media,source:{kind:'media_ref',data:{id,version}},nodes:[],timeline:[],lod:{detailed:null,simplified:{kind:'media_ref',data:{id,version}},region:'device_icons'}};
 const aux=cfg.role.startsWith('auxiliary'),tracking=aux?'batch':'individual',s=specs[cfg.device];
 const attributes=states.length?{presentation_state:{kind:'enum',value:states[0]}}:{},attribute_schema=states.length?[{name:'presentation_state',value_type:'enum',unit:null,shape:null,options:states,basis:'module_declared'}]:[];
 const values=cfg.role==='blocker'?{}:{length_m:.50,width_m:.40,thickness_m:.003,area_m2:.20,declared_score_depth_m:.0006,declared_score_width_m:.006,declared_score_spacing_m:.20};
 for(const [name,value]of Object.entries(values)){attributes[name]={kind:'number',value};attribute_schema.push({name,value_type:'number',unit:null,shape:null,options:[],basis:'module_declared'});}
 const itemDir=path.join(dir,'item-definition');await fs.mkdir(itemDir,{recursive:true});const d=definition(itemId,'明确纸材类型 '+cfg.id,'items',{unit:'piece',tracking,spec_ref:null,attributes,attribute_schema,presentation_or_media_ref:wrapper});d.attribute_schema=attribute_schema;await json(path.join(itemDir,'module.json'),pack(d,[],[{id,version}]));
 root.traverse(o=>{if(o.isMesh)o.geometry.dispose();});return {id,itemId,version,tracking,states,attributes,attribute_schema,bytes:bytes.length};
}
if(await fs.access(path.join(here,'rebuild.json')).then(()=>true,()=>false))await exportItem(JSON.parse(await fs.readFile(path.join(here,'rebuild.json'),'utf8')),path.resolve(here,'..'),false);
