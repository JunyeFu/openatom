import * as T from 'three';import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {specs,surface,wellSize,paperX,auxPort,auxSeat,filmHeight,printedFraction,foilX,foilY,padX,padY,clamp,samples} from './batch-spec.mjs';import {definition,pack,version} from './contract.mjs';
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}};
const here=path.dirname(fileURLToPath(import.meta.url)),paper=new T.MeshStandardMaterial({name:'Actual coated paperboard',color:'#d9cfb5',roughness:.80,side:T.DoubleSide}),gold=new T.MeshStandardMaterial({name:'Actual transferred metal layer',color:'#d8ad46',metalness:.9,roughness:.23}),pet=new T.MeshStandardMaterial({name:'Actual PET carrier',color:'#bbc9d5',roughness:.22,transparent:true,opacity:.4,side:T.DoubleSide});
export function buildItem(cfg){
 const s=specs[cfg.device],root=new T.Group();root.name='ActualFinish_'+cfg.id;const carrier=new T.Group();carrier.name='ActualMaterialCarrier';root.add(carrier);const clips=[],variants=[];let mark,patch,liquid,clicheInk,padInk,foilMetal,foilCarrier,wetMark,curedMark,wetVisibility,curedVisibility;
 const mesh=(n,g,p,mat=paper)=>{const o=new T.Mesh(g,mat);o.name=n;o.position.fromArray(p);carrier.add(o);return o;},box=(n,size,p,mat=paper)=>mesh(n,new T.BoxGeometry(...size),p,mat);
 const ink=new T.MeshStandardMaterial({name:'Actual metered '+s.auxiliary,color:s.mode==='uv'?'#bac8d8':'#204f85',roughness:s.mode==='uv'?.12:.5,transparent:s.mode==='uv',opacity:s.mode==='uv'?.45:1});
 function embossGeometry(){
  const xs=[-.25,-.054,-.05,.05,.054,.25],zs=[-.20,-.054,-.05,.05,.054,.20],v=[],idx=[],height=(x,z)=>s.raise*Math.min(clamp((.054-Math.abs(x))/.004),clamp((.054-Math.abs(z))/.004));
  for(const bottom of [false,true])for(const z of zs)for(const x of xs)v.push(x,height(x,z)+(bottom?0:s.size[1]),z);
  const plane=xs.length*zs.length;for(let j=0;j<zs.length-1;j++)for(let i=0;i<xs.length-1;i++){const a=j*xs.length+i,b=a+1,c=a+xs.length,d=c+1;idx.push(a,c,b,b,c,d,a+plane,b+plane,c+plane,b+plane,d+plane,c+plane);}
  const loop=[...xs.map((_,i)=>i),...zs.slice(1).map((_,j)=>(j+1)*xs.length+xs.length-1),...xs.slice(0,-1).map((_,i)=>(zs.length-1)*xs.length+xs.length-2-i),...zs.slice(1,-1).map((_,j)=>(zs.length-2-j)*xs.length)];
  for(let i=0;i<loop.length;i++){const a=loop[i],b=loop[(i+1)%loop.length];idx.push(a,b,a+plane,b,b+plane,a+plane);}
  const g=new T.BufferGeometry();g.setAttribute('position',new T.Float32BufferAttribute(v,3));g.setIndex(idx);const faces=g.toNonIndexed();faces.computeVertexNormals();return faces;
 }
 function paperWithPatch(){
  const [x,h,z]=s.size,g=new T.BoxGeometry(x,h,z),pos=g.attributes.position,index=g.index,kept=[];for(let i=0;i<index.count;i+=3){const a=index.getX(i),b=index.getX(i+1),c=index.getX(i+2);if([a,b,c].every(n=>pos.getY(n)>0))continue;kept.push(a,b,c);}g.clearGroups();g.setIndex(kept);mesh('ActualPaperSubstrate',g,[0,h/2,0]);
  const sh=new T.Shape();sh.moveTo(-x/2,-z/2);sh.lineTo(x/2,-z/2);sh.lineTo(x/2,z/2);sh.lineTo(-x/2,z/2);sh.closePath();const hole=new T.Path(),w=s.mark[0],d=s.mark[1];hole.moveTo(-w/2,-d/2);hole.lineTo(-w/2,d/2);hole.lineTo(w/2,d/2);hole.lineTo(w/2,-d/2);hole.closePath();sh.holes.push(hole);const plane=new T.ShapeGeometry(sh);plane.rotateX(-Math.PI/2);mesh('ActualPaperTopOutsideMark',plane,[0,h,0]);const p=new T.PlaneGeometry(w,d);p.rotateX(-Math.PI/2);patch=mesh('ActualUncoatedPatchFace',p,[0,h,0]);
 }
 if(cfg.role==='blocker')box('ExplicitOutputBlocker',[.10,.02,.10],[0,.01,0]);
 else if(cfg.role==='spent_backing')box('OneActualSpentPETCarrier',s.carrier,[0,s.carrier[1]/2,0],pet);
 else if(cfg.role==='auxiliary'){
  if(s.mode==='foil'){foilCarrier=box('ActualPETCarrier',s.carrier,[0,s.metal_thickness+s.carrier[1]/2,0],pet);foilMetal=box('ActualTransferMetal',[s.mark[0],s.metal_thickness,s.mark[1]],[0,s.metal_thickness/2,0],gold);}
  else if(s.mode==='pad'){clicheInk=box('ActualClicheInk',[...s.mark.slice(0,1),filmHeight(s),s.mark[1]],[0,filmHeight(s)/2,0],ink);padInk=box('ActualInkOnTransferPad',[s.mark[0],filmHeight(s),s.mark[1]],[0,0,0],ink);}
  else liquid=box('ActualMeteredWellLiquid',[wellSize,s.volume_m3/(wellSize*wellSize),wellSize],[0,s.volume_m3/(wellSize*wellSize)/2,0],ink);
 }else{
  if(s.mode==='emboss'){variants.push(box('ActualFlatSheet',s.size,[0,s.size[1]/2,0]),mesh('ActualEmbossedSheet',embossGeometry(),[0,0,0]));}
  else{
   if(s.mode==='uv')box('ActualPaperSubstrate',s.size,[0,s.size[1]/2,0]);else paperWithPatch();
   const h=filmHeight(s);if(s.mode==='uv'){wetMark=box('ActualWetVarnish',[s.mark[0],h,s.mark[1]],[0,s.size[1]+h/2,0],new T.MeshStandardMaterial({name:'Actual wet UV varnish',color:'#b7c7da',roughness:.08,transparent:true,opacity:.45}));curedMark=box('ActualCuredVarnish',[s.mark[0],h,s.mark[1]],[0,s.size[1]+h/2,0],new T.MeshStandardMaterial({name:'Actual cured UV varnish',color:'#bccbd8',roughness:.16,transparent:true,opacity:.45}));}
   else mark=box('ActualTransferredLayer',[s.mark[0],h,s.mark[1]],[0,s.size[1]+h/2,0],s.mode==='foil'?gold:ink);
  }
 }
 if(wetMark){wetVisibility=new T.Group();wetVisibility.name='WetLayerVisibility';curedVisibility=new T.Group();curedVisibility.name='CuredLayerVisibility';carrier.add(wetVisibility,curedVisibility);wetMark.removeFromParent();curedMark.removeFromParent();wetVisibility.add(wetMark);curedVisibility.add(curedMark);}
 const states=cfg.role==='input'?[s.initial,...(s.preserve?[s.final]:[])]:cfg.role==='output'?[s.final]:cfg.role==='auxiliary'?[s.mode==='foil'?'metallized_patch':'dose_ready']:cfg.role==='spent_backing'?['spent_carrier']:[];
 const names=['input','auxiliary'].includes(cfg.role)?[...states,...['load','auto','unload'].map(p=>states[0]+':'+p)]:states;
 for(const name of names){
  const list=[],phase=name.split(':')[1],uAt=t=>phase==='auto'?t:phase==='unload'||name===s.final?1:0;
  const vector=(o,prop,f)=>list.push(new T.VectorKeyframeTrack(o.name+'.'+prop,samples,samples.flatMap(t=>f(uAt(t),t))));
  if(cfg.role==='auxiliary')vector(carrier,'position',(u,t)=>phase==='load'?auxPort(s).map((v,i)=>(v-auxSeat(s,phase,t)[i])*(1-t)):[0,0,0]);
  const fraction=u=>cfg.role==='output'?1:printedFraction(s,u);
  if(mark){
   vector(mark,'scale',u=>{const f=fraction(u);return f<=0?[0,0,0]:s.mode==='screen'?[f,1,1]:[1,f,1];});vector(mark,'position',u=>{const f=fraction(u),h=filmHeight(s);return[s.mode==='screen'?-s.mark[0]/2+s.mark[0]*f/2:0,s.size[1]+h*(s.mode==='screen'?1:f)/2,0];});
  }
  if(patch){if(s.mode==='screen'){vector(patch,'scale',u=>{const f=fraction(u);return f>=1?[0,0,0]:[1-f,1,1];});vector(patch,'position',u=>[s.mark[0]*fraction(u)/2,s.size[1],0]);}else list.push(new T.VectorKeyframeTrack(patch.name+'.scale',samples,samples.flatMap(t=>fraction(uAt(t))<=0?[1,1,1]:[0,0,0]),T.InterpolateDiscrete));}
  if(wetMark)for(const [cured,o,visibility]of [[false,wetMark,wetVisibility],[true,curedMark,curedVisibility]]){
   vector(o,'scale',u=>[fraction(u),1,1]);vector(o,'position',u=>[-s.mark[0]/2+s.mark[0]*fraction(u)/2,s.size[1]+filmHeight(s)/2,0]);
   const ts=[0,.90,1];list.push(new T.VectorKeyframeTrack(visibility.name+'.scale',ts,ts.flatMap(t=>{const on=cured?(cfg.role==='output'||name===s.final||uAt(t)>=.90):!(cfg.role==='output'||name===s.final||uAt(t)>=.90);return on?[1,1,1]:[0,0,0];}),T.InterpolateDiscrete));
  }
  if(liquid){const h=s.volume_m3/(wellSize*wellSize);vector(liquid,'scale',u=>{const left=1-printedFraction(s,u);return left<=0?[0,0,0]:[1,left,1];});vector(liquid,'position',u=>[0,h*(1-printedFraction(s,u))/2,0]);}
  if(foilMetal){vector(foilMetal,'scale',u=>{const left=1-printedFraction(s,u);return left<=0?[0,0,0]:[1,left,1];});vector(foilMetal,'position',u=>[0,s.metal_thickness*(printedFraction(s,u)+(1-printedFraction(s,u))/2),0]);}
  if(clicheInk){
   const picked=u=>clamp((u-.20)/.08),applied=u=>printedFraction(s,u),h=filmHeight(s);
   vector(clicheInk,'scale',u=>1-picked(u)<=0?[0,0,0]:[1,1-picked(u),1]);vector(clicheInk,'position',u=>[0,h*(1-picked(u))/2,0]);
   vector(padInk,'scale',u=>picked(u)-applied(u)<=0?[0,0,0]:[1,picked(u)-applied(u),1]);vector(padInk,'position',u=>[padX(u)+.40,padY(u)-.08-h*(picked(u)-applied(u))/2-.90035,0]);
  }
  if(variants.length)for(const [i,o]of variants.entries())list.push(new T.VectorKeyframeTrack(o.name+'.scale',[0,.40,1],[0,.40,1].flatMap(t=>Number(uAt(t)>=.40)===i?[1,1,1]:[0,0,0]),T.InterpolateDiscrete));
  if(!list.length){const o=carrier.children[0];list.push(new T.VectorKeyframeTrack(o.name+'.position',[0,1],[...o.position.toArray(),...o.position.toArray()]));}
  clips.push(new T.AnimationClip(name,1,list));
 }
 return {root,clips,states};
}
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.scale??[1,1,1])[i]])),unit:'meter',coordinate_frame:'right_handed_y_up'});
export async function exportItem(cfg,dir,copySource=true){
 const {root,clips,states}=buildItem(cfg),id='qa.paper.q04.media.'+cfg.id,itemId='qa.paper.q04.items.'+cfg.id;
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(root,{binary:true,animations:clips}));await fs.mkdir(dir,{recursive:true});await fs.writeFile(path.join(dir,'paper.glb'),bytes);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12))),asset={id:id+'.glb',path:'paper.glb',kind:'glb'};
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.85,texture:null}})),poses:[],actions:[],timeline:(j.animations??[]).map((a,i)=>({id:cfg.id+'-'+a.name,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n'),files=['paper.glb','LICENSE.txt','source/item-build.mjs','source/batch-spec.mjs','source/contract.mjs','source/rebuild.json'];
 if(copySource){await fs.mkdir(path.join(dir,'source'),{recursive:true});for(const file of ['item-build.mjs','batch-spec.mjs','contract.mjs'])await fs.copyFile(path.join(here,file),path.join(dir,'source',file));await fs.copyFile(path.join(here,'../LICENSE.txt'),path.join(dir,'LICENSE.txt'));await json(path.join(dir,'source/rebuild.json'),cfg);}
 await json(path.join(dir,'module.json'),pack(definition(id,'真实持有纸材 '+cfg.id,'media',media,[asset]),files,[],[asset]));
 const wrapper={...media,source:{kind:'media_ref',data:{id,version}},nodes:[],timeline:[],lod:{detailed:null,simplified:{kind:'media_ref',data:{id,version}},region:'device_icons'}};
 const tracking=cfg.role==='auxiliary'&&specs[cfg.device].mode!=='foil'?'batch':'individual';
 const attributes=states.length?{presentation_state:{kind:'enum',value:states[0]}}:{},attribute_schema=states.length?[{name:'presentation_state',value_type:'enum',unit:null,shape:null,options:states,basis:'module_declared'}]:[];
 const s=specs[cfg.device],primary=['input','output'].includes(cfg.role),values={};
 if(cfg.role==='auxiliary'){
  if(s.mode==='foil')Object.assign(values,{carrier_area_m2:s.carrier[0]*s.carrier[2],carrier_volume_m3:s.carrier[0]*s.carrier[1]*s.carrier[2],transfer_area_m2:s.mark[0]*s.mark[1],transfer_volume_m3:s.mark[0]*s.mark[1]*s.metal_thickness});
  else values.declared_dose_volume_m3=s.volume_m3;
 }else if(cfg.role==='spent_backing')Object.assign(values,{carrier_area_m2:s.carrier[0]*s.carrier[2],carrier_volume_m3:s.carrier[0]*s.carrier[1]*s.carrier[2],remaining_transfer_volume_m3:0});
 else if(primary){Object.assign(values,{length_m:s.size[0],width_m:s.size[2],thickness_m:s.size[1],area_m2:s.size[0]*s.size[2]});if(s.mode==='emboss')values.declared_emboss_height_m=s.raise;else if(s.preserve)values.declared_finish_layer_volume_m3=s.volume_m3;else values.applied_layer_volume_m3=cfg.role==='output'?s.mark[0]*s.mark[1]*filmHeight(s):0;}
 for(const [name,value]of Object.entries(values)){attributes[name]={kind:'number',value};attribute_schema.push({name,value_type:'number',unit:null,shape:null,options:[],basis:'module_declared'});}
 const itemDir=path.join(dir,'item-definition');await fs.mkdir(itemDir,{recursive:true});const d=definition(itemId,'明确纸材类型 '+cfg.id,'items',{unit:'piece',tracking,spec_ref:null,attributes,attribute_schema,presentation_or_media_ref:wrapper});d.attribute_schema=attribute_schema;await json(path.join(itemDir,'module.json'),pack(d,[],[{id,version}]));
 root.traverse(o=>{if(o.isMesh)o.geometry.dispose();});return {id,itemId,version,tracking,states,attributes,attribute_schema,bytes:bytes.length};
}
if(await fs.access(path.join(here,'rebuild.json')).then(()=>true,()=>false))await exportItem(JSON.parse(await fs.readFile(path.join(here,'rebuild.json'),'utf8')),path.resolve(here,'..'),false);
