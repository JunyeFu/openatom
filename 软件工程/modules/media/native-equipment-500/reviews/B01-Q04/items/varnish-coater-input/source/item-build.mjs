import * as T from 'three';import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {specs,samples,coatingFraction,printedPixelCount,pixels,pixelSize,printThickness,auxPort,auxSeat} from './batch-spec.mjs';import {definition,pack,version} from './contract.mjs';
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}};
const here=path.dirname(fileURLToPath(import.meta.url)),paper=new T.MeshStandardMaterial({name:'Actual paperboard',color:'#d9cfb5',roughness:.80,side:T.DoubleSide}),wet=new T.MeshStandardMaterial({name:'Wet coated paper surface',color:'#dbd4b9',roughness:.12}),cured=new T.MeshStandardMaterial({name:'Cured coated paper surface',color:'#dbd4b9',roughness:.32});
export function buildItem(cfg){
 const s=specs[cfg.device],root=new T.Group();root.name='ActualPaper_'+cfg.id;const clips=[],marks=[];let liquid,top,film,wetFilm,curedFilm;
 const mesh=(n,g,pos,mat=paper)=>{const o=new T.Mesh(g,mat);o.name=n;o.position.fromArray(pos);root.add(o);return o;},box=(n,size,pos,mat=paper)=>mesh(n,new T.BoxGeometry(...size),pos,mat),aux=cfg.role==='auxiliary';
 if(cfg.role==='blocker')box('ExplicitOutputBlocker',[.10,.02,.10],[0,.01,0]);
 else if(aux){const height=s.volume/(Math.PI*.012*.012);liquid=mesh('ActualDose',new T.CylinderGeometry(.012,.012,height,40),[0,height/2,0],new T.MeshStandardMaterial({name:s.auxiliary+' batch dose',color:s.mode==='code'?'#172024':'#cdbb83',roughness:.22}));}
 else{
  const g=new T.BoxGeometry(...s.size),p=g.attributes.position,idx=g.index,kept=[];for(let i=0;i<idx.count;i+=3){const t=[idx.getX(i),idx.getX(i+1),idx.getX(i+2)];if(s.mode==='code'||!t.every(n=>p.getY(n)>0))kept.push(...t);}g.clearGroups();g.setIndex(kept);mesh('ActualPaperSubstrate',g,[0,s.size[1]/2,0]);
  if(s.mode==='code')for(const[i,p]of pixels.entries())marks.push(box('CodePixel'+i,[pixelSize,printThickness,pixelSize],[p.x,s.size[1]+printThickness/2,p.z],new T.MeshStandardMaterial({name:'Actual A01 black ink',color:'#172024',roughness:.55})));
  else{const plane=new T.PlaneGeometry(.50,.40);plane.rotateX(-Math.PI/2);top=mesh('ActualUncoatedPaperTop',plane,[0,s.size[1],0]);wetFilm=box('ActualWetFilm',[.50,.000020,.40],[0,s.size[1]+.000010,0],wet);curedFilm=box('ActualCuredFilm',[.50,.000020,.40],[0,s.size[1]+.000010,0],cured);film=wetFilm;}
 }
 const states=cfg.role==='blocker'?[]:aux?['dose_ready']:s.mode==='code'?['flat','coded_A01']:[s.initial,...['flat','varnished_wet','varnished_cured'].filter(st=>st!==s.initial)];
 const names=states.length?[...states,...states.flatMap(st=>['load','auto','unload'].map(p=>st+':'+p))]:['idle'];
 for(const name of names){
  const list=[],[state,phase]=name.split(':'),uAt=t=>aux?(phase==='auto'?t:phase==='unload'?1:0):state!==s.initial?1:phase==='auto'?t:phase==='unload'?1:0;
  const vector=(o,prop,f,discrete=false)=>list.push(new T.VectorKeyframeTrack(o.name+'.'+prop,samples,samples.flatMap(t=>f(uAt(t),t)),discrete?T.InterpolateDiscrete:T.InterpolateLinear));
  if(aux){const height=s.volume/(Math.PI*.012*.012),fraction=u=>s.mode==='code'?printedPixelCount(u)/pixels.length:coatingFraction(u);vector(liquid,'scale',u=>[1,1-fraction(u),1],s.mode==='code');vector(liquid,'position',(u,t)=>{const offset=phase==='load'?auxPort(s).map((v,i)=>(v-auxSeat(s)[i])*(1-t)):[0,0,0];return[offset[0],height*(1-fraction(u))/2+offset[1],offset[2]];});}
  else if(cfg.role==='blocker'){}
  else if(s.mode==='code')for(let i=0;i<marks.length;i++)vector(marks[i],'scale',u=>-1+2*u+pixels[i].x>=-1e-9?[1,1,1]:[0,0,0],true);
  else{
   const f=u=>state==='flat'?s.mode==='coat'&&phase?coatingFraction(u):0:1;
   vector(top,'scale',u=>[1-f(u),1,1]);vector(top,'position',u=>[-.25*f(u),s.size[1],0]);
   vector(wetFilm,'scale',u=>[state==='varnished_cured'?0:f(u),1,1]);vector(wetFilm,'position',u=>[.25-.25*f(u),s.size[1]+.000010,0]);
   vector(curedFilm,'scale',()=>[state==='varnished_cured'?1:0,1,1],true);
  }
  if(!list.length){const o=root.children[0];list.push(new T.VectorKeyframeTrack(o.name+'.position',[0,1],[...o.position.toArray(),...o.position.toArray()]));}clips.push(new T.AnimationClip(name,1,list));
 }
 return{root,clips,states};
}
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.scale??[1,1,1])[i]])),unit:'meter',coordinate_frame:'right_handed_y_up'});
export async function exportItem(cfg,dir,copySource=true){
 const {root,clips,states}=buildItem(cfg),id='qa.paper.b01q04.media.'+cfg.id,itemId='qa.paper.b01q04.items.'+cfg.id;
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(root,{binary:true,animations:clips}));await fs.mkdir(dir,{recursive:true});await fs.writeFile(path.join(dir,'paper.glb'),bytes);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12))),asset={id:id+'.glb',path:'paper.glb',kind:'glb'};
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.85,texture:null}})),poses:[],actions:[],timeline:(j.animations??[]).map((a,i)=>({id:cfg.id+'-'+a.name,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n'),files=['paper.glb','LICENSE.txt','source/item-build.mjs','source/batch-spec.mjs','source/contract.mjs','source/rebuild.json'];
 if(copySource){await fs.mkdir(path.join(dir,'source'),{recursive:true});for(const file of ['item-build.mjs','batch-spec.mjs','contract.mjs'])await fs.copyFile(path.join(here,file),path.join(dir,'source',file));await fs.copyFile(path.join(here,'../LICENSE.txt'),path.join(dir,'LICENSE.txt'));await json(path.join(dir,'source/rebuild.json'),cfg);}
 await json(path.join(dir,'module.json'),pack(definition(id,'真实持有纸材 '+cfg.id,'media',media,[asset]),files,[],[asset]));
 const wrapper={...media,source:{kind:'media_ref',data:{id,version}},nodes:[],timeline:[],lod:{detailed:null,simplified:{kind:'media_ref',data:{id,version}},region:'device_icons'}};
 const aux=cfg.role.startsWith('auxiliary'),tracking=aux?'batch':'individual',s=specs[cfg.device];
 const attributes=states.length?{presentation_state:{kind:'enum',value:states[0]}}:{},attribute_schema=states.length?[{name:'presentation_state',value_type:'enum',unit:null,shape:null,options:states,basis:'module_declared'}]:[];
 const values=aux?{declared_dose_volume_m3:s.volume}:cfg.role==='blocker'?{}:{length_m:.50,width_m:.40,thickness_m:.0008,area_m2:.20,...(s.mode==='code'?{declared_code_ink_volume_m3:s.volume}:{declared_coating_volume_m3:s.volume})};
 for(const [name,value]of Object.entries(values)){attributes[name]={kind:'number',value};attribute_schema.push({name,value_type:'number',unit:null,shape:null,options:[],basis:'module_declared'});}
 const itemDir=path.join(dir,'item-definition');await fs.mkdir(itemDir,{recursive:true});const d=definition(itemId,'明确纸材类型 '+cfg.id,'items',{unit:'piece',tracking,spec_ref:null,attributes,attribute_schema,presentation_or_media_ref:wrapper});d.attribute_schema=attribute_schema;await json(path.join(itemDir,'module.json'),pack(d,[],[{id,version}]));
 root.traverse(o=>{if(o.isMesh)o.geometry.dispose();});return {id,itemId,version,tracking,states,attributes,attribute_schema,bytes:bytes.length};
}
if(await fs.access(path.join(here,'rebuild.json')).then(()=>true,()=>false))await exportItem(JSON.parse(await fs.readFile(path.join(here,'rebuild.json'),'utf8')),path.resolve(here,'..'),false);
