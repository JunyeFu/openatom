import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import * as T from 'three';
import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {build} from './stripper-reviewed.mjs';
import {embedSurfaces} from './surface-pack.mjs';
import {definition,pack,presentation,embedLabel,version} from './contract.mjs';
const here=path.dirname(fileURLToPath(import.meta.url));
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(v=>{this.result=v;this.onloadend?.();});}readAsDataURL(b){b.arrayBuffer().then(v=>{this.result=`data:${b.type};base64,${Buffer.from(v).toString('base64')}`;this.onloadend?.();});}};
export async function exportModel(spec,simple,dir,label,copySource=true){
 const built=build(spec,simple),names=new Set();built.root.traverse(o=>{if(names.has(o.name))throw Error('Duplicate node '+o.name);names.add(o.name);});
 const id=spec.media_id+(simple?'.simple':'');
 let bytes=Buffer.from(await new GLTFExporter().parseAsync(built.root,{binary:true,animations:built.animations,onlyVisible:true}));
 bytes=await embedSurfaces(await embedLabel(bytes,label),simple);
 const j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12)).toString());
 const asset={id:id+'.glb',path:spec.id+'.glb',kind:'glb'},media=presentation(spec,j,asset,built,simple);
 await fs.mkdir(dir,{recursive:true});await fs.writeFile(path.join(dir,asset.path),bytes);await fs.writeFile(path.join(dir,'nameplate.png'),label);
 const sources=['export.mjs','stripper-reviewed.mjs','batch-spec.mjs','kit.mjs','mechanisms.mjs','assemblies.mjs','contract.mjs','surface-pack.mjs'],surfaces=await fs.readdir(path.join(here,'surfaces'));
 if(copySource){
  await fs.mkdir(path.join(dir,'source/surfaces'),{recursive:true});for(const f of sources)await fs.copyFile(path.join(here,f),path.join(dir,'source',f));
  for(const f of surfaces)await fs.copyFile(path.join(here,'surfaces',f),path.join(dir,'source/surfaces',f));
  await fs.writeFile(path.join(dir,'source/rebuild.json'),JSON.stringify({spec,simple},null,2)+'\n');await fs.copyFile(path.join(here,'../LICENSE.txt'),path.join(dir,'LICENSE.txt'));
 }
 const files=[asset.path,'nameplate.png','LICENSE.txt',...sources.map(f=>'source/'+f),'source/rebuild.json',...surfaces.map(f=>'source/surfaces/'+f)];
 const assets=[asset,{id:id+'.nameplate',path:'nameplate.png',kind:'png'}];
 const pkg=pack(definition(id,spec.title+(simple?' · 简化':' · 详细'),'media',media,assets),files,simple?[]:[{id:spec.media_id+'.simple',version}],assets);
 await fs.writeFile(path.join(dir,'module.json'),JSON.stringify(pkg,null,2)+'\n');
 const bounds=new T.Box3().setFromObject(built.root),triangles=j.meshes.reduce((a,m)=>a+m.primitives.reduce((a,p)=>a+j.accessors[p.indices??p.attributes.POSITION].count/(p.indices===undefined?3:3),0),0);
 const result={...spec,suffix:simple?'simple':'detailed',module_id:id,version,directory:dir,asset,bytes:bytes.length,nodes:j.nodes.length,triangles,phase_ids:j.animations.map(a=>a.name),primary_phase:'auto',focus_node:built.focus,bounds:{min:bounds.min.toArray(),max:bounds.max.toArray()},moving_names:built.moving_names};
 built.root.traverse(o=>{if(o.isMesh)o.geometry.dispose();});return result;
}
// A package rebuild reads only its own spec, source, labels and surfaces.
if(await fs.access(path.join(here,'rebuild.json')).then(()=>true,()=>false)){
 const {spec,simple}=JSON.parse(await fs.readFile(path.join(here,'rebuild.json'),'utf8'));
 await exportModel(spec,simple,path.resolve(here,'..'),await fs.readFile(path.resolve(here,'../nameplate.png')),false);
}
