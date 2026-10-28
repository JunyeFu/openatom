// Executable geometry / declared-motion checks, separate from the visual review.
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {stripTypeScriptTypes} from 'node:module';
import {execFileSync} from 'node:child_process';
import assert from 'node:assert/strict';
import * as T from 'three';
import '../../../tests/planner/presentation/pixel-decoder.mjs';
const root=process.cwd(),base=path.join(root,'modules/media/equipment-quality-v042');
const scratch=path.join(root,'.work/model-quality-20261022');
const runtime=path.join(scratch,'runtime');await fs.mkdir(runtime,{recursive:true});
for(const file of await fs.readdir(path.join(root,'packages/presentation/src'))){
  if(!file.endsWith('.ts'))continue;
  const source=await fs.readFile(path.join(root,'packages/presentation/src',file),'utf8');
  const code=stripTypeScriptTypes(source,{mode:'transform'}).replace(/from '\.\/(\w+)'/g,"from './$1.mjs'");
  await fs.writeFile(path.join(runtime,file.replace(/\.ts$/,'.mjs')),code);
}
const {decodeGlb}=await import(pathToFileURL(path.join(runtime,'glb.mjs')));
const {bindScene}=await import(pathToFileURL(path.join(runtime,'scene.mjs')));
const {assetKey}=await import(pathToFileURL(path.join(runtime,'types.mjs')));
const inventory=JSON.parse(await fs.readFile(path.join(base,'inventory.json'),'utf8'));
const expected={assembly:['load','auto','unload'],rework:['load','auto','unload'],packing:['load','auto','unload'],robot:['approach','grip','loaded_move','release','return'],conveyor:['load','auto','unload'],inspection:['load','auto','unload'],sealer:['infeed','seal','outfeed'],carton:['open','sealed','fold_preview']};
function transforms(rig){rig.root.updateMatrixWorld(true);return [...rig.nodes].map(([id,o])=>[id,...o.matrixWorld.elements]);}
function near(a,b,label){assert.equal(a.length,b.length,label);for(let i=0;i<a.length;i++)assert.ok(Math.abs(a[i]-b[i])<1e-7,label+':'+i);}
const results=[];
for(const entry of inventory.entries){
 const dir=path.join(base,entry.directory),pkg=JSON.parse(await fs.readFile(path.join(dir,'module.json'),'utf8')).data;
 const p=pkg.definition.payload.data;
 const read=async a=>{const b=await fs.readFile(path.join(dir,a.path));return b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength);};
 const bytes=await read(entry.asset),decoded=await decodeGlb(bytes);
 const rig=await bindScene(p,read,{assetKey:assetKey(entry.asset),decoded});
 const actual=decoded.gltf.animations.map(a=>a.name);assert.deepEqual(actual,expected[entry.family]??[]);
 let seekChecks=0;
 for(const phase of expected[entry.family]??[]){
  const clip=p.timeline.find(c=>c.phase_ref===phase);
  const apply=u=>{rig.apply({known:true,display_phase_id:phase,progress:u},{clip,progress:u});rig.root.updateMatrixWorld(true);};
  apply(0);const start=transforms(rig).flat();apply(.5);const middle=transforms(rig).flat();
  apply(1);apply(.5);near(transforms(rig).flat(),middle,entry.family+'/'+phase+'/reverse-mid');
  apply(0);near(transforms(rig).flat(),start,entry.family+'/'+phase+'/reverse-start');seekChecks+=2;
  if(entry.family==='packing')for(const u of [0,.25,.5,.75,1]){
   apply(u);const r=rig.root.getObjectByName('PusherRodExtension'),push=rig.root.getObjectByName('CartonPusher');
   const end=r.localToWorld(new T.Vector3(-.1,0,0));const face=push.localToWorld(new T.Vector3(.006,0,0));
   assert.ok(end.distanceTo(face)<1e-6,'Pusher rod detached at '+phase+'/'+u);
  }
 }
 rig.apply(null,null);rig.root.updateMatrixWorld(true);
 const declared=name=>{const node=p.nodes.find(n=>n.name===name);assert.ok(node,'Missing declared node '+name);return rig.nodes.get(node.id);};
 const ray=new T.Raycaster();
 function hole(mesh,start,direction){ray.set(new T.Vector3(...start),new T.Vector3(...direction));assert.equal(ray.intersectObject(mesh,false).length,0,'Expected actual open bore in '+entry.family);}
 if(['pcb','control-box'].includes(entry.family)){
  const board=declared('FR4Board');
  for(const x of [-.103,.103])for(const z of [-.073,.073])hole(board,[x,.5,z],[0,-1,0]);
  ray.set(new T.Vector3(0,.5,0),new T.Vector3(0,-1,0));assert.ok(ray.intersectObject(board,false).length>0,'Board solid material missing');
 }
 if(['shell','control-box'].includes(entry.family)){
  const wall=declared('HousingWallZ0.086');
  for(const x of [-.05,.05])hole(wall,[x,.030,.4],[0,0,-1]);
  for(const x of [-.07,.07])hole(declared('MountLug'+x),[x,.1,.104],[0,-1,0]);
 }
 assert.ok([...rig.nodes.values()].every(o=>o.matrixWorld.elements.every(Number.isFinite)),entry.family+' invalid pose');
 rig.dispose();
 // Reopen a self-contained source copy for every selected model, preserving the delivered export.
 const copy=path.join(scratch,'rebuild',entry.family+(entry.suffix==='simple'?'-simple':''));
 await fs.cp(dir,copy,{recursive:true});
 execFileSync(process.execPath,[path.join(copy,'source/generate.mjs')],{cwd:root,windowsHide:true,stdio:'pipe'});
 const rebuilt=await fs.readFile(path.join(copy,entry.asset.path));
 assert.ok(Buffer.from(bytes).equals(rebuilt),entry.family+'/'+entry.suffix+' source rebuild mismatch');
 results.push({family:entry.family,suffix:entry.suffix,version:entry.version,product_decoder:true,product_bind_scene:true,reverse_seek_checks:seekChecks,actual_open_bores:['pcb','control-box','shell'].includes(entry.family),pusher_contact:entry.family==='packing'?'15 phase positions passed':'not applicable',source_reopened:true,rebuilt_export_identical:true});
 console.log(entry.family+'/'+entry.suffix+' source + motion + geometry passed');
}
const output=path.join(root,'scenarios/媒体表现/model-quality-20261022/batch');await fs.mkdir(output,{recursive:true});
await fs.writeFile(path.join(output,'source-and-motion-checks.json'),JSON.stringify({scope:'Exact exported GLBs and production presentation consumer; this does not grant visual, actual Run or Native acceptance.',results},null,2));
console.log('Complete:',results.length);
