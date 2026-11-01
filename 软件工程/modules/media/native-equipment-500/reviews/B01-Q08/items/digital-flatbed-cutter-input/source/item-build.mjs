import * as T from 'three';import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {specs,samples,cornerPositions,trayTop,trayZ,seatX,toolPose} from './batch-spec.mjs';import {definition,pack,version} from './contract.mjs';
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}};
const here=path.dirname(fileURLToPath(import.meta.url)),paper=new T.MeshStandardMaterial({name:'Actual cut and creased corrugated board',color:'#c8ae7b',roughness:.86});
function blankGeometry(stage){
 const xs=[-.25,-.152,-.15,-.148,.148,.15,.152,.25],zs=[-.20,-.102,-.10,-.098,.098,.10,.102,.20],vertices=[],indices=[];
 const depth=(x,z)=>Math.max(stage>=1&&Math.abs(x)<=.152?.0003*Math.max(0,1-Math.abs(z+.10)/.002):0,stage>=2&&Math.abs(z)<=.102?.0003*Math.max(0,1-Math.abs(x-.15)/.002):0,stage>=3&&Math.abs(x)<=.152?.0003*Math.max(0,1-Math.abs(z-.10)/.002):0,stage>=4&&Math.abs(z)<=.102?.0003*Math.max(0,1-Math.abs(x+.15)/.002):0);
 for(let xi=0;xi<xs.length-1;xi++)for(let zi=0;zi<zs.length-1;zi++){
  const x0=xs[xi],x1=xs[xi+1],z0=zs[zi],z1=zs[zi+1];if(Math.abs((x0+x1)/2)>.15&&Math.abs((z0+z1)/2)>.10)continue;const n=vertices.length/3;
  for(const[x,z]of[[x0,z0],[x1,z0],[x1,z1],[x0,z1]])vertices.push(x,.003-depth(x,z),z);for(const[x,z]of[[x0,z0],[x1,z0],[x1,z1],[x0,z1]])vertices.push(x,0,z);
  for(const i of[0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7])indices.push(n+i);
 }
 const g=new T.BufferGeometry();g.setAttribute('position',new T.Float32BufferAttribute(vertices,3));g.setIndex(indices);const out=g.toNonIndexed();out.computeVertexNormals();return out;
}
export function buildItem(cfg){
 const s=specs[cfg.device],root=new T.Group();root.name='ActualDigitalStock_'+cfg.id;const clips=[],corners=[],variants=[];const mesh=(n,g,p)=>{const o=new T.Mesh(g,paper);o.name=n;o.position.fromArray(p);root.add(o);return o;};
 if(cfg.role==='blocker')mesh('ExplicitOutputBlocker',new T.BoxGeometry(.1,.02,.1),[0,.01,0]);
 else if(cfg.role==='scrap')mesh('OneActualCornerWaste',new T.BoxGeometry(.10,.003,.10),[0,.0015,0]);
 else{
  for(let stage=0;stage<=4;stage++){const o=mesh('ActualCrossBlankStage'+stage,blankGeometry(stage),[0,0,0]);o.scale.setScalar(stage===(cfg.role==='output'?4:0)?1:0);variants.push(o);}
  if(cfg.role==='input')for(const[i,p]of cornerPositions.entries())corners.push({p,o:mesh('ActualAttachedCorner'+i,new T.BoxGeometry(.10,.003,.10),[p[0],.0015,p[2]])});
 }
 const states=cfg.role==='blocker'?[]:[cfg.role==='output'?s.final:cfg.role==='scrap'?'corner_waste':s.initial],names=states.length?[...states,...['load','auto','unload'].map(ph=>states[0]+':'+ph)]:['idle'];
 for(const name of names){const list=[],phase=name.split(':')[1],stage=u=>cfg.role==='output'||phase==='unload'?4:phase==='auto'?toolPose(u).creaseDone:0;
  for(const[i,o]of variants.entries())list.push(new T.VectorKeyframeTrack(o.name+'.scale',samples,samples.flatMap(u=>stage(u)===i?[1,1,1]:[0,0,0]),T.InterpolateDiscrete));
  for(const{p,o}of corners)list.push(new T.VectorKeyframeTrack(o.name+'.position',samples,samples.flatMap(u=>phase==='load'||!phase?[p[0],.0015,p[2]]:[p[0]-seatX(s,phase,u),trayTop(s,phase,u)-s.surface+.0015,trayZ(Math.sign(p[2]),phase,u)])));
  if(!list.length){const o=root.children[0];list.push(new T.VectorKeyframeTrack(o.name+'.position',[0,1],[...o.position.toArray(),...o.position.toArray()]));}clips.push(new T.AnimationClip(name,1,list));
 }
 return{root,clips,states};
}
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.scale??[1,1,1])[i]])),unit:'meter',coordinate_frame:'right_handed_y_up'});
export async function exportItem(cfg,dir,copySource=true){
 const {root,clips,states}=buildItem(cfg),id='qa.paper.b01q08.media.'+cfg.id,itemId='qa.paper.b01q08.items.'+cfg.id;
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(root,{binary:true,animations:clips}));await fs.mkdir(dir,{recursive:true});await fs.writeFile(path.join(dir,'paper.glb'),bytes);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12))),asset={id:id+'.glb',path:'paper.glb',kind:'glb'};
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.85,texture:null}})),poses:[],actions:[],timeline:(j.animations??[]).map((a,i)=>({id:cfg.id+'-'+a.name,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n'),files=['paper.glb','LICENSE.txt','source/item-build.mjs','source/batch-spec.mjs','source/contract.mjs','source/rebuild.json'];
 if(copySource){await fs.mkdir(path.join(dir,'source'),{recursive:true});for(const file of ['item-build.mjs','batch-spec.mjs','contract.mjs'])await fs.copyFile(path.join(here,file),path.join(dir,'source',file));await fs.copyFile(path.join(here,'../LICENSE.txt'),path.join(dir,'LICENSE.txt'));await json(path.join(dir,'source/rebuild.json'),cfg);}
 await json(path.join(dir,'module.json'),pack(definition(id,'真实持有纸材 '+cfg.id,'media',media,[asset]),files,[],[asset]));
 const wrapper={...media,source:{kind:'media_ref',data:{id,version}},nodes:[],timeline:[],lod:{detailed:null,simplified:{kind:'media_ref',data:{id,version}},region:'device_icons'}};
 const aux=cfg.role.startsWith('auxiliary'),tracking=aux?'batch':'individual',s=specs[cfg.device];
 const attributes=states.length?{presentation_state:{kind:'enum',value:states[0]}}:{},attribute_schema=states.length?[{name:'presentation_state',value_type:'enum',unit:null,shape:null,options:states,basis:'module_declared'}]:[];
 const values=cfg.role==='blocker'?{}:{length_m:cfg.role==='scrap'?.10:.50,width_m:cfg.role==='scrap'?.10:.40,thickness_m:.003,area_m2:cfg.role==='scrap'?.01:cfg.role==='output'?.16:.20};
 for(const [name,value]of Object.entries(values)){attributes[name]={kind:'number',value};attribute_schema.push({name,value_type:'number',unit:null,shape:null,options:[],basis:'module_declared'});}
 const itemDir=path.join(dir,'item-definition');await fs.mkdir(itemDir,{recursive:true});const d=definition(itemId,'明确纸材类型 '+cfg.id,'items',{unit:'piece',tracking,spec_ref:null,attributes,attribute_schema,presentation_or_media_ref:wrapper});d.attribute_schema=attribute_schema;await json(path.join(itemDir,'module.json'),pack(d,[],[{id,version}]));
 root.traverse(o=>{if(o.isMesh)o.geometry.dispose();});return {id,itemId,version,tracking,states,attributes,attribute_schema,bytes:bytes.length};
}
if(await fs.access(path.join(here,'rebuild.json')).then(()=>true,()=>false))await exportItem(JSON.parse(await fs.readFile(path.join(here,'rebuild.json'),'utf8')),path.resolve(here,'..'),false);
