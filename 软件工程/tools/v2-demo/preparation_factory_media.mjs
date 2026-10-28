// One real item can visit multiple operations without changing its identity.
// Retain the accepted geometry and scope each source animation to that operation.
import fs from 'node:fs/promises';
import path from 'node:path';
import * as T from 'three';
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js';
import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';

const [source, output] = process.argv.slice(2).map(p=>path.resolve(p));
const read=async p=>JSON.parse(await fs.readFile(p,'utf8'));
const project=await read(path.join(source,'actual-project.json'));
const packages=await read(path.join(source,'actual-frozen-modules.json'));
const exact=new Map(packages.map(m=>[`${m.definition.id}@${m.definition.version}`,m]));
const pkg=r=>exact.get(`${r.id}@${r.version}`);
const definitions=[
 {type:'paper-unwinder-input',stages:[['paper-unwinder-input','paper-unwinder',{}],['paper-rewinder-input','paper-rewinder',{}],['rotary-sheet-cutter-input','rotary-sheet-cutter',{}]],rest:{rolled:0,unwound:0}},
 {type:'board-edge-trimmer-output',stages:[['board-edge-trimmer-output','board-edge-trimmer',{}],['sheet-slitter-input','sheet-slitter',{wide_sheet:'trimmed'}]],rest:{trimmed:0}},
 {type:'varnish-coater-input',stages:[['varnish-coater-input','varnish-coater',{}],['uv-curer-input','uv-curer',{}]],rest:{flat:0,varnished_wet:0,varnished_cured:1}},
];
await fs.mkdir(output,{recursive:false});
globalThis.FileReader=class {readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}};
const loader=new GLTFLoader(), modules=[], materials={};
const pose=n=>({translation:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.translation??[0,0,0])[i]])),rotation:Object.fromEntries(['x','y','z','w'].map((k,i)=>[k,(n.rotation??[0,0,0,1])[i]])),scale:Object.fromEntries(['x','y','z'].map((k,i)=>[k,(n.scale??[1,1,1])[i]])),unit:'meter',coordinate_frame:'right_handed_y_up'});
for (const definition of definitions) {
 const root=new T.Group(); root.name='PreparationMaterial'; const stages=[];
 for (const [index,[typeId,deviceName,aliases]] of definition.stages.entries()) {
  const type=project.item_types.find(t=>t.id===typeId), wrapper=pkg(type.module_ref).definition.payload.data.presentation_or_media_ref;
  const original=pkg(wrapper.source.data), media=original.definition.payload.data, asset=media.source.data;
  const bytes=await fs.readFile(path.join(source,'project',asset.path));
  const gltf=await loader.parseAsync(bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength),'');
  const names=new Map(); gltf.scene.traverse(n=>{const renamed=`S${index}_${n.name||n.uuid}`; names.set(n.name,renamed); names.set(n.uuid,renamed); n.name=renamed;});
  const group=new T.Group(); group.name=`Stage${index}`; group.scale.setScalar(0); group.add(gltf.scene); root.add(group);
  const operation=project.routes.find(r=>r.steps.length===1&&r.steps[0].device_id==='PF-'+deviceName).steps[0].operation_ref;
  const clips=media.timeline.map(row=>{
   const clip=gltf.animations[row.source.data.animation_index].clone();
   for(const track of clip.tracks) { const parsed=T.PropertyBinding.parseTrackName(track.name); if(!names.has(parsed.nodeName)) throw Error(`Unresolved track ${track.name}`); track.name=track.name.replace(parsed.nodeName,names.get(parsed.nodeName)); }
   const parts=row.phase_ref.split(':'); parts[0]=aliases[parts[0]]??parts[0];
   return {clip,phase:parts.length===1?parts[0]:`${parts[0]}:${operation.id}:${operation.version}:${parts.at(-1)}`,plain:parts.length===1};
  });
  stages.push({group,clips,rights:original.rights,source:wrapper.source.data});
 }
 const animations=[];
 for(const [index,stage] of stages.entries()) for(const entry of stage.clips) {
  if(entry.plain&&definition.rest[entry.phase]!==index) continue;
  const clip=entry.clip; clip.name=entry.phase;
  for(const [j,{group}] of stages.entries()) clip.tracks.push(new T.VectorKeyframeTrack(group.name+'.scale',[0,clip.duration],[j===index?1:0,j===index?1:0,j===index?1:0,j===index?1:0,j===index?1:0,j===index?1:0]));
  animations.push(clip);
 }
 const id='sf.paperfactory.preparation.media.'+definition.type, version='1.0.0';
 const directory=path.join(output,'modules',id); await fs.mkdir(directory,{recursive:true});
 const asset={id:id+'.glb',path:'material.glb',kind:'glb'};
 const bytes=Buffer.from(await new GLTFExporter().parseAsync(root,{binary:true,animations,onlyVisible:false}));
 await fs.writeFile(path.join(directory,asset.path),bytes);
 const g=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12)).toString());
 const parents=new Map();g.nodes.forEach((n,i)=>n.children?.forEach(c=>parents.set(c,i)));
 const media={kind:'model',source:{kind:'asset',data:asset},unit:'meter',coordinate_frame:'right_handed_y_up',scale:{x:1,y:1,z:1},
  nodes:g.nodes.map((n,i)=>({id:i,name:n.name,parent_index:parents.get(i)??null,transform:pose(n),geometry:{kind:'imported_node',data:{asset_ref:asset,node_index:i,unsupported_extensions:[]}},material:{color_rgba:[1,1,1,1],metallic:0,roughness:.8,texture:null}})),
  poses:[],actions:[],timeline:g.animations.map((a,i)=>({id:'preparation-'+i,phase_ref:a.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})),tags:[],anchor_roles:{},port_anchors:{},lod:{detailed:null,simplified:{kind:'asset',data:asset},region:'device_icons'},production_effect:'none'};
 const sourceRights=stages.flatMap(s=>s.rights);
 if(!sourceRights.every(r=>r.license===sourceRights[0].license&&r.redistribution_confirmed===sourceRights[0].redistribution_confirmed&&r.missing_reason===sourceRights[0].missing_reason)) throw Error('Composition needs explicit rights for its differing source licenses');
 const rights=[{...sourceRights[0],asset_ref:asset.id,source:stages.map(s=>`${s.source.id}@${s.source.version}: ${s.rights.map(r=>r.source).join(' ')}`).join('\n')}];
 const module={definition:{id,version,title:'跨工序物料 · '+definition.type,definition_state:'runtime_candidate',contribution:{author:'Super Factory preparation composition',license:'CC0-1.0',assets:rights},parameters:[],attribute_schema:[],payload:{category:'media',data:media}},assets:[asset],files:[{path:asset.path,kind:'asset'}],dependencies:[],rights,issues:[]};
 await fs.writeFile(path.join(directory,'module.json'),JSON.stringify({schema_version:'1.0',data:module},null,2));
 const ref={id,version}; modules.push({...ref,directory});
 materials[definition.type]={media_ref:ref,presentation:{...media,source:{kind:'media_ref',data:ref},nodes:[],timeline:[],lod:{detailed:null,simplified:{kind:'media_ref',data:ref},region:'device_icons'}},source_media:stages.map(s=>s.source),phase_refs:media.timeline.map(t=>t.phase_ref),bytes:bytes.length};
}
await fs.writeFile(path.join(output,'media-plan.json'),JSON.stringify({modules,materials},null,2));
console.log('Composed three identity-preserving multi-operation material assets:',output);
