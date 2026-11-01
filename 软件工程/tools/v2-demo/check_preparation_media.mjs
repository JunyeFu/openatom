// Sample the exported asset itself, including its operation-qualified clips.
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import * as T from 'three';
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js';
const output=path.resolve(process.argv[2]);
const plan=JSON.parse(await fs.readFile(path.join(output,'media-plan.json'),'utf8'));
const receipts=[];
for(const module of plan.modules) {
 const bytes=await fs.readFile(path.join(module.directory,'material.glb'));
 const gltf=await new GLTFLoader().parseAsync(bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength),'');
 const mixer=new T.AnimationMixer(gltf.scene), groups=[];
 gltf.scene.traverse(n=>{if(/^Stage\d+$/.test(n.name)) groups.push(n);});
 const clips=[];
 for(const clip of gltf.animations) {
  assert(!clip.name.includes(':') || clip.name.split(':').length===4,clip.name);
  mixer.stopAllAction(); const action=mixer.clipAction(clip);action.play();mixer.setTime(clip.duration*.5);gltf.scene.updateMatrixWorld(true);
  const active=groups.filter(n=>n.scale.x>.5);assert.equal(active.length,1,clip.name);
  const box=new T.Box3().setFromObject(active[0]);const size=box.getSize(new T.Vector3());
  assert(!box.isEmpty()&&[size.x,size.y,size.z].every(Number.isFinite),clip.name);
  clips.push({phase:clip.name,active_group:active[0].name,dimensions_m:size.toArray()});
 }
 receipts.push({module_ref:{id:module.id,version:module.version},source_groups:groups.length,clips});
 mixer.stopAllAction();mixer.uncacheRoot(gltf.scene);
}
await fs.writeFile(path.join(output,'exported-media-receipt.json'),JSON.stringify({status:'actual_exported_glb_clips_sampled',receipts,Native_visual_verified:false},null,2));
console.log('Verified exported material clips:',receipts.map(r=>`${r.source_groups} sources / ${r.clips.length} clips`).join('; '));
