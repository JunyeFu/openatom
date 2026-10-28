// Actual GLB animation sampling; geometry does not supply production quantities.
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import * as T from 'three';
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js';
const directory=path.resolve(process.argv[2]),plan=JSON.parse(await fs.readFile(path.join(directory,'media-plan.json'),'utf8')),observations=[];
for(const name of ['tube','closed-bag','open-bag','handle-pair','finished-bag']){
 const ref=plan.materials[name].media_ref,folder=path.join(directory,'modules',ref.id),pkg=JSON.parse(await fs.readFile(path.join(folder,'module.json'),'utf8')).data;
 const media=pkg.definition.payload.data,bytes=await fs.readFile(path.join(folder,media.source.data.path));
 const model=await new GLTFLoader().parseAsync(bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength),'');
 for(const phase of ['static','auto'])for(const progress of phase==='auto'?[0,.25,.5,.75,1]:[1]){
  const declared=media.timeline.find(c=>phase==='static'?!c.phase_ref.includes(':'):c.phase_ref.endsWith(':auto'));
  const clip=model.animations[declared.source.data.animation_index],mixer=new T.AnimationMixer(model.scene),action=mixer.clipAction(clip);
  action.setLoop(T.LoopOnce,1);action.clampWhenFinished=true;action.play();mixer.setTime(progress);model.scene.updateMatrixWorld(true);
  const box=new T.Box3().setFromObject(model.scene),size=box.getSize(new T.Vector3()).toArray();
  assert.ok([...box.min.toArray(),...box.max.toArray()].every(Number.isFinite));
  if(name==='tube'&&phase==='static'){assert.ok(Math.abs(size[0]-.24)<.001);assert.ok(Math.abs(size[2]-.18)<.002);assert.ok(Math.abs(size[1]-.08)<.002);}
  if(name==='tube'&&phase==='auto'&&progress===1)assert.ok(size[1]<.006,'Gussets must flatten visibly');
  if(name==='closed-bag'&&phase==='auto'&&progress===1){assert.ok(Math.abs(size[0]-.18)<.002);assert.ok(Math.abs(size[2]-.08)<.002);assert.ok(Math.abs(size[1]-.20)<.002);}
  if(name==='finished-bag'){const body=new T.Box3().setFromObject(model.scene.getObjectByName('PaperBagBody')).getSize(new T.Vector3());assert.ok(Math.abs(body.x-.18)<.002);assert.ok(Math.abs(body.z-.08)<.002);assert.ok(size[2]<.09,'Handle ropes project outside the bag walls');assert.ok(size[1]>.28&&size[1]<.31);}
  observations.push({name,phase,progress,minimum:box.min.toArray(),maximum:box.max.toArray(),size});
  mixer.stopAllAction();mixer.uncacheRoot(model.scene);
 }
}
await fs.writeFile(path.join(directory,'geometry-receipt.json'),JSON.stringify({status:'actual_glb_state_dimensions_and_flatten_open_clips_sampled',samples:observations.length,observations,Native_visual_verified:false},null,2)+'\n');
console.log('Actual GLB bag states sampled:',observations.length);
