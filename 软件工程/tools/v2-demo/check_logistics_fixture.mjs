import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import * as T from 'three';
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js';
const [source,fixture]=process.argv.slice(2),read=async p=>JSON.parse(await fs.readFile(p,'utf8'));
const packages=await read(path.join(source,'actual-frozen-modules.json')),plan=await read(path.join(fixture,'media-plan.json'));
const loader=new GLTFLoader(),samples=[];
// Numeric geometry/animation audit only; image decoding is verified in Native.
loader.register(()=>({name:'geometry-audit-only',loadTexture:async()=>new T.Texture()}));
const load=async(file)=>{const b=await fs.readFile(file);return loader.parseAsync(b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength),'');};
const point=(g,name)=>g.scene.getObjectByName(T.PropertyBinding.sanitizeNodeName(name)).getWorldPosition(new T.Vector3());
for(const declared of plan.height_calibration){
 const old=packages.find(m=>m.definition.id===declared.source_ref.id&&m.definition.version===declared.source_ref.version);
 const nextRef=plan.modules.find(m=>m.id.endsWith('.'+declared.lod)),next=await read(path.join(nextRef.directory,'module.json'));
 const a=await load(path.join(source,'project',old.definition.payload.data.source.data.path));
 const b=await load(path.join(nextRef.directory,next.data.definition.payload.data.source.data.path));
 assert.equal(a.animations.length,b.animations.length);
 for(let i=0;i<a.animations.length;i++)for(const fraction of [0,.25,.5,.75,1]){
  const am=new T.AnimationMixer(a.scene),bm=new T.AnimationMixer(b.scene);
  for(const [m,g] of [[am,a],[bm,b]]){const action=m.clipAction(g.animations[i]);action.setLoop(T.LoopOnce,1);action.clampWhenFinished=true;action.play();m.setTime(g.animations[i].duration*fraction);g.scene.updateMatrixWorld(true);}
  const oldSeat=point(a,'PaperSeat'),newSeat=point(b,'PaperSeat');assert(oldSeat.distanceTo(newSeat)<1e-6,'Cargo seat world path changed');
  const oldCup=point(a,'SuctionCup_-0.07_-0.05'),newCup=point(b,'SuctionCup_-0.07_-0.05');assert(Math.abs(newCup.y-oldCup.y-.12)<1e-6);assert(Math.abs(newCup.x-oldCup.x)<1e-6&&Math.abs(newCup.z-oldCup.z)<1e-6);
  const cupBounds=new T.Box3().setFromObject(b.scene.getObjectByName(T.PropertyBinding.sanitizeNodeName('SuctionCup_-0.07_-0.05')));
  if(a.animations[i].name==='auto')assert(Math.abs(cupBounds.min.y-newSeat.y-.24)<1e-6,'Suction cups must touch the real case top during travel');
  samples.push({lod:declared.lod,phase:a.animations[i].name,fraction,seat:newSeat.toArray(),cup_delta_y:newCup.y-oldCup.y,cup_bottom_above_case_top:cupBounds.min.y-newSeat.y-.24});
  am.stopAllAction();bm.stopAllAction();
 }
}
await fs.writeFile(path.join(fixture,'geometry-calibration-receipt.json'),JSON.stringify({status:'both_LODs_actual_animation_sampled',samples,cargo_bottom_path_preserved:true,suction_tool_raised_m:.12,Native_verified:false},null,2));
console.log(JSON.stringify({samples:samples.length,max_cup_gap:Math.max(...samples.map(s=>s.cup_bottom_above_case_top)),min_cup_gap:Math.min(...samples.map(s=>s.cup_bottom_above_case_top))}));
