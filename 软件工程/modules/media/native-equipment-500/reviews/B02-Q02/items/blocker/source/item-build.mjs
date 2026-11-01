import * as T from 'three';import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';
import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {paper,specs,radius,unwindAngle,rewindAngle,foldCount,foldRadius,foldStraight,foldEntryY,foldTimes,foldHeadX,foldFraction,foldLayer} from './batch-spec.mjs';import {definition,pack,version} from './contract.mjs';
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}};
const here=path.dirname(fileURLToPath(import.meta.url));
const mat=new T.MeshStandardMaterial({name:'Actual uncoated paper',color:'#dccda6',roughness:.85,side:T.DoubleSide});
const edge=new T.MeshStandardMaterial({name:'Actual paper layer edges',color:'#b8a57d',roughness:.9,side:T.DoubleSide});
// Centers/radii in the X/Y machine plane. The common upper external tangent touches both circles.
export function tangent(a,ra,b,rb=0){const dx=b[0]-a[0],dy=b[1]-a[1],d=Math.hypot(dx,dy),k=(ra-rb)/d,h=Math.sqrt(1-k*k),ux=dx/d,uy=dy/d;let nx=k*ux-h*uy,ny=k*uy+h*ux;if(ny<0){nx=k*ux+h*uy;ny=k*uy-h*ux;}return [[a[0]+nx*ra,a[1]+ny*ra],[b[0]+nx*rb,b[1]+ny*rb]];}
export function buildItem(cfg){
 const root=new T.Group();root.name='ActualPaper_'+cfg.id;const objects=[],rolls=[],sheets=[],folds=[],webs=[];
 const mesh=(name,geometry,position,material=mat)=>{const o=new T.Mesh(geometry,material);o.name=name;o.position.fromArray(position);root.add(o);objects.push(o);return o;};
 const annulus=(name,r0,r1,width,point)=>{const shape=new T.Shape();shape.absarc(0,0,r1,0,Math.PI*2);const hole=new T.Path();hole.absarc(0,0,r0,0,Math.PI*2,true);shape.holes.push(hole);const g=new T.ExtrudeGeometry(shape,{depth:width,bevelEnabled:false,curveSegments:32});g.translate(0,0,-width/2);return mesh(name,g,point,rolls.length%2?mat:edge);};
 const s=specs[cfg.device],datum=cfg.role==='input'&&s.mode!=='sheet'?s.datum:[0,0,0],local=p=>p.map((v,i)=>v-datum[i]);
 if(cfg.id==='blocker')mesh('ExplicitOutputBlocker',new T.BoxGeometry(.1,.02,.1),[0,.01,0]);
 else if(cfg.role==='output'){
  if(s.outputShape==='roll')annulus('FinishedNarrowRoll',paper.core,radius(),s.outputWidth,[0,0,0]);
  else mesh('OneActualFinishedSheet',new T.BoxGeometry(...s.sheet),[0,s.sheet[1]/2,0]);
 }else if(s.mode==='sheet')mesh('OneActualPaperboard',new T.BoxGeometry(...s.sheet),[0,-s.sheet[1]/2,0]);
 else {
  const addRoll=(label,point,width,growing)=>{
   const spin=new T.Group();spin.name=label+'Spin';spin.position.fromArray(local(point));root.add(spin);const parts=[];
   for(let i=0;i<paper.layers;i++){const o=annulus(label+'Layer'+i,radius(i/paper.layers),radius((i+1)/paper.layers),width,[0,0,0]);o.removeFromParent();spin.add(o);parts.push(o);}
   rolls.push({spin,parts,growing});
  };
  if(s.inputAxis!=null)addRoll('InputPaper',[s.inputAxis,1.27,0],paper.width,false);
  if(s.outputAxis!=null)for(const z of s.mode==='slit'?[-.135,.135]:[0])addRoll('OutputPaper'+z,[s.outputAxis,1.27,z],s.mode==='slit'?.25:.50,true);
  if(s.fold)for(let i=0;i<60;i++){
   const o=mesh('HeldBatchLayer'+i,new T.BoxGeometry(s.mode==='crosscut'?.50:foldStraight,paper.thickness,.50),local([s.fold[0],s.fold[1]+(i+.5)*paper.thickness,0]),i%2?mat:edge);sheets.push(o);
   if(s.mode!=='crosscut'&&i<59){const shape=new T.Shape();shape.moveTo(0,-paper.thickness);shape.absarc(0,0,paper.thickness,-Math.PI/2,Math.PI/2,false);shape.closePath();const geo=new T.ExtrudeGeometry(shape,{depth:.5,bevelEnabled:false,curveSegments:8});geo.translate(0,0,-.25);const f=mesh('ContinuousFold'+i,geo,local([s.fold[0]+(i%2?-1:1)*foldStraight/2,s.fold[1]+(i+1)*paper.thickness,0]),edge);if(i%2)f.rotation.y=Math.PI;folds.push(f);}
  }
  // Distinct segments follow the tangent to the changing actual roll, then the real guide rollers.
  for(let i=0;i<(s.mode==='slit'?6:['unwind','rewind'].includes(s.mode)?4:3);i++)webs.push(mesh('HeldPaperWeb'+i,new T.BoxGeometry(1,paper.thickness,s.mode==='slit'&&i>=2?.25:.50),[0,0,0]));
 }
 const states=cfg.id==='blocker'?[]:cfg.role==='output'?[s.outputShape==='roll'?'rolled':'sheet']:s.mode==='sheet'?['adhered','separated']:[s.initialState,...(s.finalState?[s.finalState]:[])];
 const names=cfg.role==='input'&&s.mode!=='sheet'?[...states,...['load','auto','unload'].map(p=>s.initialState+':'+p)]:states;
 const clips=[];
 for(const name of names){
  const tracks=[],samples=['unwind','rewind'].includes(s.mode)?foldTimes:Array.from({length:513},(_,i)=>i/512),phase=name.split(':')[1],uAt=t=>phase==='auto'?t:phase==='unload'||name===s.finalState?1:0;
  const vectors=(o,property,f)=>tracks.push(new T.VectorKeyframeTrack(o.name+'.'+property,samples,samples.flatMap(t=>f(uAt(t)))));
  for(const r of rolls){for(const [i,o]of r.parts.entries())vectors(o,'scale',u=>{const on=r.growing?u>=(i+1)/paper.layers:1-u>i/paper.layers;return on?[1,1,1]:[0,0,0];});tracks.push(new T.QuaternionKeyframeTrack(r.spin.name+'.quaternion',samples,samples.flatMap(t=>new T.Quaternion().setFromAxisAngle(new T.Vector3(0,0,1),(r.growing?rewindAngle:unwindAngle)(uAt(t))).toArray())));}
  for(const [i,o]of sheets.entries()){
   if(s.mode==='crosscut')vectors(o,'scale',u=>i<Math.floor(u*60)?[1,1,1]:[0,0,0]);
   else {const f=u=>Math.min(1,Math.max(0,foldFraction(s,u)*60-i));vectors(o,'scale',u=>[f(u),f(u)>0?1:0,1]);vectors(o,'position',u=>local([s.fold[0]+(i%2?-1:1)*(f(u)-1)*foldStraight/2,s.fold[1]+(i+.5)*paper.thickness,0]));}
  }
  for(const [i,o]of folds.entries())vectors(o,'scale',u=>foldFraction(s,u)*60>i+1?[1,1,1]:[0,0,0]);
  function paths(u){
   const cIn=[s.inputAxis,1.27],cOut=[s.outputAxis,1.27],gL=[s.mode==='rewind'?0:-.25,.95],gR=[s.mode==='unwind'?0:.25,.95],rIn=radius(1-u),rOut=radius(u);
   if(s.mode==='unwind'){const x=foldHeadX(s,u),top=s.fold[1]+(foldLayer(s,u)+.5)*paper.thickness;return [tangent(cIn,rIn,gL,.05),[[gL[0],1],[gR[0],1]],tangent(gR,.05,[x,foldEntryY]),[[x,foldEntryY],[x,top]]];}
   if(s.mode==='rewind'){const x=foldHeadX(s,u),top=s.fold[1]+(foldLayer(s,u)+.5)*paper.thickness;return [[[x,top],[x,foldEntryY]],tangent(gL,.05,[x,foldEntryY]).reverse(),[[gL[0],1],[gR[0],1]],tangent(gR,.05,cOut,rOut)];}
   if(s.mode==='slit')return [tangent(cIn,rIn,gL,.05),[[gL[0],1],[0,1]],...[-.125,.125].map(z=>[[0,1,z],[gR[0],1,z]]),...[-.135,.135].map(z=>tangent(gR,.05,cOut,rOut).map(p=>[...p,z]))];
   return [tangent(cIn,rIn,gL,.05),[[gL[0],1],[gR[0],1]],tangent(gR,.05,[s.fold[0],s.fold[1]+Math.floor(u*60)*paper.thickness])];
  }
  for(const [i,o]of webs.entries()){
   const segment=u=>paths(u)[i],center=u=>{const [a,b]=segment(u);return local([(a[0]+b[0])/2,(a[1]+b[1])/2,((a[2]??0)+(b[2]??0))/2]);};vectors(o,'position',center);
   vectors(o,'scale',u=>{const [a,b]=segment(u);return phase==='auto'&&u>0&&u<1?[Math.hypot(b[0]-a[0],b[1]-a[1]),1,1]:[0,0,0];});
   tracks.push(new T.QuaternionKeyframeTrack(o.name+'.quaternion',samples,samples.flatMap(t=>{const [a,b]=segment(uAt(t));return new T.Quaternion().setFromAxisAngle(new T.Vector3(0,0,1),Math.atan2(b[1]-a[1],b[0]-a[0])).toArray();})));
  }
  if(s.mode==='sheet'){const o=objects[0];tracks.push(new T.VectorKeyframeTrack(o.name+'.position',[0,1],name==='separated'?[0,s.sheet[1]/2,0,0,s.sheet[1]/2,0]:[0,-s.sheet[1]/2,0,0,-s.sheet[1]/2,0]));}
  if(!tracks.length){const o=objects[0];tracks.push(new T.VectorKeyframeTrack(o.name+'.position',[0,1],[...o.position.toArray(),...o.position.toArray()]));}
  clips.push(new T.AnimationClip(name,1,tracks));
 }
 return {root,clips,states};
}
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.scale??[1,1,1])[i]])),unit:'meter',coordinate_frame:'right_handed_y_up'});
export async function exportItem(cfg,dir,copySource=true){
 const {root,clips,states}=buildItem(cfg),id='qa.paper.q02.media.'+cfg.id,itemId='qa.paper.q02.items.'+cfg.id;
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(root,{binary:true,animations:clips}));await fs.mkdir(dir,{recursive:true});await fs.writeFile(path.join(dir,'paper.glb'),bytes);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12))),asset={id:id+'.glb',path:'paper.glb',kind:'glb'};
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.85,texture:null}})),poses:[],actions:[],timeline:(j.animations??[]).map((a,i)=>({id:cfg.id+'-'+a.name,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n'),files=['paper.glb','LICENSE.txt','source/item-build.mjs','source/batch-spec.mjs','source/contract.mjs','source/rebuild.json'];
 if(copySource){await fs.mkdir(path.join(dir,'source'),{recursive:true});for(const file of ['item-build.mjs','batch-spec.mjs','contract.mjs'])await fs.copyFile(path.join(here,file),path.join(dir,'source',file));await fs.copyFile(path.join(here,'../LICENSE.txt'),path.join(dir,'LICENSE.txt'));await json(path.join(dir,'source/rebuild.json'),cfg);}
 await json(path.join(dir,'module.json'),pack(definition(id,'真实持有纸材 '+cfg.id,'media',media,[asset]),files,[],[asset]));
 const wrapper={...media,source:{kind:'media_ref',data:{id,version}},nodes:[],timeline:[],lod:{detailed:null,simplified:{kind:'media_ref',data:{id,version}},region:'device_icons'}};
 const attributes=states.length?{presentation_state:{kind:'enum',value:states[0]}}:{},attribute_schema=states.length?[{name:'presentation_state',value_type:'enum',unit:null,shape:null,options:states,basis:'module_declared'}]:[];
 if(cfg.id!=='blocker')for(const [name,value]of Object.entries(cfg.device==='suction-separator'?{length_m:.48,width_m:.40,thickness_m:.0015}:{length_m:cfg.role==='output'&&specs[cfg.device].mode==='crosscut'?.5:30,width_m:cfg.role==='output'&&specs[cfg.device].mode==='slit'?.25:.5,thickness_m:.0001})){attributes[name]={kind:'number',value};attribute_schema.push({name,value_type:'number',unit:'meter',shape:null,options:[],basis:'module_declared'});}
 const itemDir=path.join(dir,'item-definition');await fs.mkdir(itemDir,{recursive:true});const d=definition(itemId,'明确纸材类型 '+cfg.id,'items',{unit:'piece',tracking:'individual',spec_ref:null,attributes,attribute_schema,presentation_or_media_ref:wrapper});d.attribute_schema=attribute_schema;await json(path.join(itemDir,'module.json'),pack(d,[],[{id,version}]));
 root.traverse(o=>{if(o.isMesh)o.geometry.dispose();});return {id,itemId,version,states,attributes,attribute_schema,bytes:bytes.length};
}
if(await fs.access(path.join(here,'rebuild.json')).then(()=>true,()=>false))await exportItem(JSON.parse(await fs.readFile(path.join(here,'rebuild.json'),'utf8')),path.resolve(here,'..'),false);
