import * as T from 'three';import fs from 'node:fs/promises';import path from 'node:path';import {fileURLToPath} from 'node:url';import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {specs,seatX,feedTravel,scrapPieces,scrapOffset,scrapStage} from './batch-spec.mjs';import {definition,pack,version} from './contract.mjs';
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}};
const here=path.dirname(fileURLToPath(import.meta.url));
const paper=new T.MeshStandardMaterial({name:'Actual paperboard',color:'#c7ad78',roughness:.87,side:T.DoubleSide});
export function buildItem(cfg){
 const s=specs[cfg.device],root=new T.Group();root.name='ActualPaper_'+cfg.id;const scraps=[],variants=[],tracks=[];
 const mesh=(name,g,p,mat=paper)=>{const o=new T.Mesh(g,mat);o.name=name;o.position.fromArray(p);root.add(o);return o;};
 const box=(name,size,p)=>mesh(name,new T.BoxGeometry(...size),p);
 const sheet=(name,creased=false)=>{
  const size=s.product??s.size,shape=new T.Shape();
  if(s.mode==='slot'){
   const h=.25,w=.30,k=.003,depth=.10;shape.moveTo(-h,-w);shape.lineTo(h,-w);
   for(const z of [-.15,0,.15]){shape.lineTo(h,z-k);shape.lineTo(h-depth,z-k);shape.lineTo(h-depth,z+k);shape.lineTo(h,z+k);}shape.lineTo(h,w);shape.lineTo(-h,w);
   for(const z of [.15,0,-.15]){shape.lineTo(-h,z+k);shape.lineTo(-h+depth,z+k);shape.lineTo(-h+depth,z-k);shape.lineTo(-h,z-k);}shape.closePath();
  }else{shape.moveTo(-size[0]/2,-size[2]/2);shape.lineTo(size[0]/2,-size[2]/2);shape.lineTo(size[0]/2,size[2]/2);shape.lineTo(-size[0]/2,size[2]/2);shape.closePath();}
  if(s.mode==='perforate')for(let i=0;i<25;i++){const x=-.24+i*.02,h=new T.Path();h.moveTo(x-.005,-.0003);h.lineTo(x-.005,.0003);h.lineTo(x+.005,.0003);h.lineTo(x+.005,-.0003);h.closePath();shape.holes.push(h);}
  if(creased){
   const profile=new T.Shape(),top=[];for(const x of [-.25,-.126,-.125,-.124,-.001,0,.001,.124,.125,.126,.25])top.push([x,[ -.125,0,.125].includes(x)?s.size[1]-.00015:s.size[1]]);
   profile.moveTo(...top[0]);for(const p of top.slice(1))profile.lineTo(...p);for(const [x,y]of [...top].reverse())profile.lineTo(x,y-s.size[1]);profile.closePath();const g=new T.ExtrudeGeometry(profile,{depth:size[2],bevelEnabled:false});g.translate(0,0,-size[2]/2);return mesh(name,g,[0,0,0]);
  }
  const g=new T.ExtrudeGeometry(shape,{depth:size[1],bevelEnabled:false});g.rotateX(Math.PI/2);g.translate(0,size[1],0);return mesh(name,g,[0,0,0]);
 };
 let mark,inkDose,inkHeight;
 if(cfg.role==='blocker')box('ExplicitOutputBlocker',[.10,.02,.10],[0,.01,0]);
 else if(cfg.role==='auxiliary'){
  const r=.023,h=s.ink_volume_m3/(Math.PI*r*r);inkHeight=h;inkDose=mesh('ActualTwoMillilitreInkDose',new T.CylinderGeometry(r,r,h,40),[0,h/2,0],new T.MeshStandardMaterial({name:'Actual flexographic ink',color:'#214f83',roughness:.32}));
 }else if(cfg.role!=='input'&&cfg.role!=='output'){const e=s.extra_outputs.find(v=>v.role===cfg.role);box('OneActual_'+cfg.role,e.size,[0,e.size[1]/2,0]);}
 else{
  if(s.mode==='crease'){variants.push(sheet('FlatUncutSheet'),sheet('CreasedUncutSheet',true));}
  else sheet('ActualCutSheet');
  if(cfg.role==='input')for(const [i,p]of scrapPieces(s).entries())scraps.push({o:box('AttachedCutPiece'+i,p.size,p.position),...p,index:scrapPieces(s).slice(0,i).filter(v=>v.role===p.role).length});
  if(s.printed)mark=mesh('ActualPrintedPanelMark',new T.BoxGeometry(.12,s.ink_volume_m3/(.12*.10),.10),[0,s.size[1]+s.ink_volume_m3/(.12*.10)/2,-.225],new T.MeshStandardMaterial({name:'Transferred blue ink',color:'#214f83',roughness:.7}));
 }
 const states=cfg.role==='auxiliary'?['ink_dose']:cfg.role==='input'?[s.initial,...(s.mode==='crease'?[s.final]:[])]:cfg.role==='output'?[s.final]:[];
 const names=cfg.role==='input'||cfg.role==='auxiliary'?[...states,...['load','auto','unload'].map(p=>states[0]+':'+p)]:states;
 for(const name of names){const list=[],phase=name.split(':')[1],times=Array.from({length:513},(_,i)=>i/512),uAt=t=>phase==='auto'?t:phase==='unload'||name===s.final?1:0;
  const vectors=(o,prop,f)=>list.push(new T.VectorKeyframeTrack(o.name+'.'+prop,times,times.flatMap(t=>f(uAt(t)))));
  for(const p of scraps){
   vectors(p.o,'position',u=>{
    const off=scrapOffset(s,p.role,p.index),stage=scrapStage(s),target=[stage[0]+off[0]-seatX(s,phase??'load',u),stage[1]+off[1]-s.surface+p.size[1]/2,stage[2]+off[2]];
    if(s.mode==='trim'){const f=Math.min(1,Math.max(0,(u-p.cut_u)/.04));return p.position.map((v,i)=>v+(target[i]-v)*f);}
    const distanceAfterCut=(u-p.cut_u)*feedTravel,advance=s.mode==='slot'?.060:.105;
    if(distanceAfterCut<=advance)return p.position;
    const f=Math.min(1,(distanceAfterCut-advance)/.030);
    return [s.toolX+.11+off[0]*f-seatX(s,phase??'load',u),p.position[1]+(target[1]-p.position[1])*f,p.position[2]+(target[2]-p.position[2])*f];
   });
  }
  for(const [i,o]of variants.entries()){const ts=[0,.4,1],values=ts.flatMap(t=>Number(uAt(t)>=.4)===i?[1,1,1]:[0,0,0]);list.push(new T.VectorKeyframeTrack(o.name+'.scale',ts,values,T.InterpolateDiscrete));}
  if(inkDose){const remaining=u=>phase==='load'||!phase?1:phase==='unload'?0:1-Math.min(1,Math.max(0,(seatX(s,'auto',u)+.35+.06)/.12));vectors(inkDose,'scale',u=>[1,remaining(u),1]);vectors(inkDose,'position',u=>[0,inkHeight*remaining(u)/2,0]);}
  if(mark){vectors(mark,'scale',u=>{const fraction=cfg.role==='output'?1:Math.min(1,Math.max(0,(seatX(s,phase??'load',u)+.35+.06)/.12));return[fraction,1,1];});vectors(mark,'position',u=>{const f=cfg.role==='output'?1:Math.min(1,Math.max(0,(seatX(s,phase??'load',u)+.35+.06)/.12));return[.06-.06*f,s.size[1]+s.ink_volume_m3/(.12*.10)/2,-.225];});}
  if(!list.length){const o=root.children[0];list.push(new T.VectorKeyframeTrack(o.name+'.position',[0,1],[...o.position.toArray(),...o.position.toArray()]));}
  tracks.push(new T.AnimationClip(name,1,list));
 }
 return {root,clips:tracks,states};
}
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.scale??[1,1,1])[i]])),unit:'meter',coordinate_frame:'right_handed_y_up'});
export async function exportItem(cfg,dir,copySource=true){
 const {root,clips,states}=buildItem(cfg),id='qa.paper.q03.media.'+cfg.id,itemId='qa.paper.q03.items.'+cfg.id;
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(root,{binary:true,animations:clips}));await fs.mkdir(dir,{recursive:true});await fs.writeFile(path.join(dir,'paper.glb'),bytes);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12))),asset={id:id+'.glb',path:'paper.glb',kind:'glb'};
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:j.nodes.map((n,i)=>({id:i,name:n.name,parent_index:j.nodes.findIndex(p=>p.children?.includes(i))<0?null:j.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.85,texture:null}})),poses:[],actions:[],timeline:(j.animations??[]).map((a,i)=>({id:cfg.id+'-'+a.name,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 const json=(p,v)=>fs.writeFile(p,JSON.stringify(v,null,2)+'\n'),files=['paper.glb','LICENSE.txt','source/item-build.mjs','source/batch-spec.mjs','source/contract.mjs','source/rebuild.json'];
 if(copySource){await fs.mkdir(path.join(dir,'source'),{recursive:true});for(const file of ['item-build.mjs','batch-spec.mjs','contract.mjs'])await fs.copyFile(path.join(here,file),path.join(dir,'source',file));await fs.copyFile(path.join(here,'../LICENSE.txt'),path.join(dir,'LICENSE.txt'));await json(path.join(dir,'source/rebuild.json'),cfg);}
 await json(path.join(dir,'module.json'),pack(definition(id,'真实持有纸材 '+cfg.id,'media',media,[asset]),files,[],[asset]));
 const wrapper={...media,source:{kind:'media_ref',data:{id,version}},nodes:[],timeline:[],lod:{detailed:null,simplified:{kind:'media_ref',data:{id,version}},region:'device_icons'}};
 const attributes=states.length?{presentation_state:{kind:'enum',value:states[0]}}:{},attribute_schema=states.length?[{name:'presentation_state',value_type:'enum',unit:null,shape:null,options:states,basis:'module_declared'}]:[];
 const s=specs[cfg.device],extra=s.extra_outputs.find(v=>v.role===cfg.role);
 const dims=cfg.role==='output'?(s.product??s.size):extra?.size??s.size;
 const values=cfg.role==='blocker'?{}:cfg.role==='auxiliary'?{volume_m3:s.ink_volume_m3}:{length_m:dims[0],width_m:dims[2],thickness_m:dims[1],area_m2:extra?extra.area:cfg.role==='output'?s.product_area:s.area};
 if(s.printed&&cfg.role==='output')values.applied_ink_volume_m3=s.ink_volume_m3;
 for(const [name,value]of Object.entries(values)){attributes[name]={kind:'number',value};attribute_schema.push({name,value_type:'number',unit:null,shape:null,options:[],basis:'module_declared'});}
 const itemDir=path.join(dir,'item-definition');await fs.mkdir(itemDir,{recursive:true});const d=definition(itemId,'明确纸材类型 '+cfg.id,'items',{unit:'piece',tracking:'individual',spec_ref:null,attributes,attribute_schema,presentation_or_media_ref:wrapper});d.attribute_schema=attribute_schema;await json(path.join(itemDir,'module.json'),pack(d,[],[{id,version}]));
 root.traverse(o=>{if(o.isMesh)o.geometry.dispose();});return {id,itemId,version,states,attributes,attribute_schema,bytes:bytes.length};
}
if(await fs.access(path.join(here,'rebuild.json')).then(()=>true,()=>false))await exportItem(JSON.parse(await fs.readFile(path.join(here,'rebuild.json'),'utf8')),path.resolve(here,'..'),false);
