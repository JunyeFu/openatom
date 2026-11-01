// Configured bag materials/tooling, authored before the actual Host run.
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import * as T from 'three';
import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {buildItem as foldingItem} from '../../modules/media/native-equipment-500/reviews/B02-Q08/source/item-build.mjs';
import {specs as foldingSpecs} from '../../modules/media/native-equipment-500/reviews/B02-Q08/source/batch-spec.mjs';
import {exportModel} from '../../modules/media/native-equipment-500/reviews/B02-Q08/source/export.mjs';
import {buildItem as handleItem} from '../../modules/media/native-equipment-500/reviews/B02-Q11/source/item-build.mjs';

const root=fileURLToPath(new URL('../../',import.meta.url)),output=path.resolve(process.argv[2]);
const spec=JSON.parse(await fs.readFile(path.join(root,'modules/templates/paper-packaging-factory/design/bag-line-spec.json'),'utf8'));
const version=spec.version,modules=[],materials={};
await fs.mkdir(output,{recursive:false});
globalThis.FileReader=class {readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}readAsDataURL(b){b.arrayBuffer().then(v=>{this.result=`data:${b.type};base64,${Buffer.from(v).toString('base64')}`;this.onloadend?.();});}};
const json=async(file,value)=>{await fs.mkdir(path.dirname(file),{recursive:true});await fs.writeFile(file,JSON.stringify(value,null,2)+'\n');};
const ref=id=>({id,version});
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.scale??[1,1,1])[i]])),unit:'meter',coordinate_frame:'right_handed_y_up'});
const material=new T.MeshStandardMaterial({color:'#bba078',roughness:.88}),glueMaterial=new T.MeshStandardMaterial({color:'#d7ad56',roughness:.6});
const clip=(name,tracks)=>new T.AnimationClip(name,1,tracks);
function wrapper(media){return {...media,source:{kind:'media_ref',data:ref(media._id)},nodes:[],timeline:[],lod:{detailed:null,simplified:{kind:'media_ref',data:ref(media._id)},region:'device_icons'},_id:undefined};}
async function emit(name,title,built,state,operation){
 const id='sf.paperfactory.bag.media.'+name,directory=path.join(output,'modules',id),asset={id:id+'.glb',path:'material.glb',kind:'glb'};
 await fs.mkdir(directory,{recursive:true});
 const clips=built.clips.map(c=>{const next=c.clone(),phase=c.name.includes(':')?c.name.split(':').at(-1):null;next.name=phase?`${state}:${operation.id}:${operation.version}:${phase}`:state;return next;});
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(built.root,{binary:true,animations:clips}));
 await fs.writeFile(path.join(directory,asset.path),bytes);
 const g=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12)).toString());
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},
  nodes:g.nodes.map((n,i)=>({id:i,name:n.name,parent_index:g.nodes.findIndex(p=>p.children?.includes(i))<0?null:g.nodes.findIndex(p=>p.children?.includes(i)),transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.8,texture:null}})),
  poses:[],actions:[],timeline:g.animations.map((a,i)=>({id:name+'-'+i,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 // Keep the accepted package's exact rights shape.
 const canonical=JSON.parse(await fs.readFile(path.join(root,'modules/media/native-equipment-500/reviews/B02-Q08/items/paper-bag-tuber-input/module.json'),'utf8')).data;
 const actualRights=canonical.rights.map(r=>({...r,asset_ref:asset.id}));
 const definition={id,version,title,definition_state:'runtime_candidate',contribution:{author:'Super Factory configured bag design',license:'CC0-1.0',assets:actualRights},parameters:[],attribute_schema:[],payload:{category:'media',data:media}};
 await json(path.join(directory,'module.json'),{schema_version:'1.0',data:{definition,assets:[asset],files:[{path:asset.path,kind:'asset'}],dependencies:[],rights:actualRights,issues:[]}});
 modules.push({...ref(id),directory});materials[name]={media_ref:ref(id),presentation:wrapper({...media,_id:id}),bytes:bytes.length,state};
}
function animated(parts,state,transform){
 const rootGroup=new T.Group();rootGroup.name='BagMaterialRoot';for(const p of parts)rootGroup.add(p.object);
 const times=[0,.2,.4,.6,.8,1],clips=[];
 for(const phase of [null,'load','auto','unload']){
  const tracks=[];
  for(const p of parts){const at=u=>transform(p,phase,phase?u:0);const a=at(0);p.object.position.fromArray(a.p);p.object.quaternion.fromArray(a.q);p.object.scale.fromArray(a.s??[1,1,1]);
   tracks.push(new T.VectorKeyframeTrack(p.object.name+'.position',times,times.flatMap(u=>at(u).p)));
   tracks.push(new T.QuaternionKeyframeTrack(p.object.name+'.quaternion',times,times.flatMap(u=>at(u).q)));
   if(a.s)tracks.push(new T.VectorKeyframeTrack(p.object.name+'.scale',times,times.flatMap(u=>at(u).s)));
  }
  clips.push(clip(phase?state+':'+phase:state,tracks));
 }
 return {root:rootGroup,clips};
}
const phaseProgress=(phase,u)=>phase==='auto'?u:phase==='unload'?1:0;
function tube(){
 const b=spec.bag,lengths=[b.width_m,b.depth_m/2,b.depth_m/2,b.width_m,b.depth_m/2,b.depth_m/2,b.seam_overlap_m];
 const parts=lengths.map((length,i)=>({index:i,object:new T.Mesh(new T.BoxGeometry(b.tube_height_m,b.paper_thickness_m,length),material)}));
 parts.forEach((p,i)=>p.object.name='GussetPanel_'+i);
 return animated(parts,'rectangular_tube',(p,ph,u)=>{
  const f=phaseProgress(ph,u),d=b.depth_m*(1-f)+.002*f,inset=Math.sqrt((b.depth_m/2)**2-(d/2)**2),w=b.width_m/2;
  const points=[[-w,0],[w,0],[w-inset,d/2],[w,d],[-w,d],[-w+inset,d/2],[-w,0],[-w+b.seam_overlap_m,0]];
  const i=p.index,a=points[i],c=points[i+1],angle=Math.atan2(-(c[1]-a[1]),c[0]-a[0]);
  return {p:[0,(a[1]+c[1])/2+(i===6?b.paper_thickness_m*1.05:0),(a[0]+c[0])/2],q:new T.Quaternion().setFromAxisAngle(new T.Vector3(1,0,0),angle).toArray()};
 });
}
function opening(){
 const b=spec.bag,w=b.width_m/2,h=b.height_m/2,lengths=[b.width_m,b.depth_m/2,b.depth_m/2,b.width_m,b.depth_m/2,b.depth_m/2];
 const body=new T.Group();body.name='OpeningBag';const panels=lengths.map((length,i)=>{const m=new T.Mesh(new T.BoxGeometry(length,b.height_m,b.paper_thickness_m),material);m.name='OpeningWall_'+i;body.add(m);return m;});
 const floor=new T.Mesh(new T.BoxGeometry(b.width_m,b.paper_thickness_m,b.depth_m),material);floor.name='OpeningBottom';body.add(floor);
 const rootGroup=new T.Group();rootGroup.name='OpeningMaterialRoot';rootGroup.add(body);
 const qflat=new T.Quaternion().setFromRotationMatrix(new T.Matrix4().makeBasis(new T.Vector3(0,0,1),new T.Vector3(1,0,0),new T.Vector3(0,1,0)));
 const clips=[],times=[0,.2,.4,.6,.8,1];
 function at(f){const d=.002+(b.depth_m-.002)*f,inset=Math.sqrt((b.depth_m/2)**2-(d/2)**2),pts=[[-w,-d/2],[w,-d/2],[w-inset,0],[w,d/2],[-w,d/2],[-w+inset,0],[-w,-d/2]],q=qflat.clone().slerp(new T.Quaternion(),f),m=new T.Matrix4().makeRotationFromQuaternion(q).elements,support=Math.abs(m[1])*w+Math.abs(m[5])*h+Math.abs(m[9])*d/2+b.paper_thickness_m;return {d,pts,q:q.toArray(),p:[0,support,0]};}
 for(const phase of [null,'load','auto','unload']){
  const f=u=>phaseProgress(phase,u),tracks=[new T.VectorKeyframeTrack(body.name+'.position',times,times.flatMap(u=>at(f(u)).p)),new T.QuaternionKeyframeTrack(body.name+'.quaternion',times,times.flatMap(u=>at(f(u)).q))];
  panels.forEach((panel,i)=>{
   const value=u=>{const v=at(f(u)),a=v.pts[i],c=v.pts[i+1];return {p:[(a[0]+c[0])/2,0,(a[1]+c[1])/2],q:new T.Quaternion().setFromAxisAngle(new T.Vector3(0,1,0),Math.atan2(-(c[1]-a[1]),c[0]-a[0])).toArray()};};
   tracks.push(new T.VectorKeyframeTrack(panel.name+'.position',times,times.flatMap(u=>value(u).p)),new T.QuaternionKeyframeTrack(panel.name+'.quaternion',times,times.flatMap(u=>value(u).q)));
  });
  tracks.push(new T.VectorKeyframeTrack(floor.name+'.position',times,times.flatMap(()=>[0,-h,0])),new T.VectorKeyframeTrack(floor.name+'.scale',times,times.flatMap(u=>[1,1,at(f(u)).d/b.depth_m])));
  clips.push(clip(phase?'closed_flat_bag:'+phase:'closed_flat_bag',tracks));
 }
 body.position.fromArray(at(0).p);body.quaternion.fromArray(at(0).q);floor.position.y=-h;
 return {root:rootGroup,clips};
}
class HandleCurve extends T.Curve {
 getPoint(t,target=new T.Vector3()){
  const radius=.055,leg=(spec.handles.cord_length_m-Math.PI*radius)/2,s=t*spec.handles.cord_length_m;
  if(s<leg)return target.set(-radius,s,0);
  if(s<=leg+Math.PI*radius){const a=Math.PI-(s-leg)/radius;return target.set(radius*Math.cos(a),leg+radius*Math.sin(a),0);}
  return target.set(radius,leg-(s-leg-Math.PI*radius),0);
 }
}
function realHandles(built){built.root.traverse(o=>{if(o.isMesh&&o.name.startsWith('PaperRope_')){o.geometry.dispose();o.geometry=new T.TubeGeometry(new HandleCurve(),64,.0025,8,false);}});return built;}
Object.assign(foldingSpecs['paper-bag-tuber'],{L:spec.bag.tube_height_m,W:spec.bag.width_m,H:spec.bag.depth_m});
Object.assign(foldingSpecs['paper-bag-bottomer'],{L:spec.bag.tube_height_m,W:spec.bag.width_m,flap:spec.bag.bottom_fold_m});
const canonical=id=>({id:'sf.native500.device.'+id,version:'1.1.0'}),operation=name=>ref('sf.paperfactory.bag.operation.'+name);
await emit('web','530×240mm计量纸幅',foldingItem({device:'paper-bag-tuber',id:'web',role:'input'}),'metered_rolled_paper_web',canonical('paper-bag-tuber'));
await emit('tube','180×80×240mm袋筒及侧褶展平',tube(),'rectangular_tube',operation('flatten'));
await emit('flat-tube','240×180mm侧褶袋筒与折底',foldingItem({device:'paper-bag-bottomer',id:'flat-tube',role:'input'}),'gusset_flat_tube',canonical('paper-bag-bottomer'));
await emit('closed-bag','糊底袋开袋与底角展开',opening(),'closed_flat_bag',operation('open'));
await emit('open-bag','180×80×200mm袋身',handleItem({device:'bag-handle-attacher',id:'open-bag',role:'input'}),'open_paper_bag',operation('attach'));
await emit('handle-pair','300mm双纸绳与四片补强片',realHandles(handleItem({device:'bag-handle-attacher',id:'handle-pair',role:'component'})),'two_prefabricated_rope_handles',operation('attach'));
await emit('finished-bag','180×80×200mm双提手纸袋',realHandles(handleItem({device:'bag-handle-attacher',id:'finished-bag',role:'output'})),'two_handle_paper_bag',operation('attach'));
await emit('strip','三条预裁纸条',handleItem({device:'rope-handle-twister',id:'strip',role:'input'}),'three_paper_strips',canonical('rope-handle-twister'));
await emit('cord','300mm三股纸绳',handleItem({device:'rope-handle-twister',id:'cord',role:'output'}),'twisted_three_ply_paper_cord',operation('handles'));
const patchRoot=new T.Group();patchRoot.name='PrecoatedPatch';const patch=new T.Mesh(new T.BoxGeometry(.020,.001,.025),material);patch.name='PatchPaper';patch.position.y=.0005;patchRoot.add(patch);const adhesive=new T.Mesh(new T.BoxGeometry(.020,.0001,.025),glueMaterial);adhesive.name='PreappliedAdhesive';adhesive.position.y=.00105;patchRoot.add(adhesive);
await emit('patch','20×25×1mm预涂胶补强片',{root:patchRoot,clips:[clip('precoated_patch',[new T.VectorKeyframeTrack(patchRoot.name+'.position',[0,1],[0,0,0,0,0,0])])]},'precoated_patch',operation('handles'));
const tooling={};
for(const id of ['paper-bag-tuber','paper-bag-bottomer']){
 const mediaId='sf.paperfactory.bag.tooling.'+id,label=await fs.readFile(path.join(root,'modules/media/native-equipment-500/reviews/B02-Q08/installable',id,'nameplate.png'));
 for(const simple of [true,false]){
  const actualId=mediaId+(simple?'.simple':''),directory=path.join(output,'modules',actualId);
  await exportModel({...foldingSpecs[id],id,media_id:mediaId,basis:'原成熟模型的显式夹具尺寸配置，配套180×80×200mm演示纸袋；不表示实测设备额定能力。'},simple,directory,label,false);
  const file=path.join(directory,'module.json'),pkg=JSON.parse(await fs.readFile(file,'utf8'));
  function exact(v){if(v&&typeof v==='object'){if(typeof v.id==='string'&&Object.hasOwn(v,'version')&&v.id.startsWith('sf.paperfactory.bag.tooling.'))v.version=version;for(const n of Object.values(v))exact(n);}}
  exact(pkg);pkg.data.definition.version=version;pkg.data.files=pkg.data.files.filter(f=>f.path===id+'.glb'||f.path==='nameplate.png');await json(file,pkg);modules.push({...ref(actualId),directory});
  if(!simple)tooling[id]={source:{kind:'media_ref',data:ref(actualId)},kind:'model',unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},nodes:[],poses:[],actions:[],timeline:[],tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'media_ref',data:ref(mediaId+'.simple')},region:'device_icons'},production_effect:'none'};
 }
}
await json(path.join(output,'media-plan.json'),{version,modules,materials,tooling,specification:spec,visual_verified:false});
console.log('Authored configured bag materials and tooling:',output,modules.length,'exact media modules');
