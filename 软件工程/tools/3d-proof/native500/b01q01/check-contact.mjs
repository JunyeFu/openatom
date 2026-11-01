import fs from 'node:fs/promises';import assert from 'node:assert/strict';import * as T from 'three';
import {build} from '../../../../modules/media/native-equipment-500/reviews/B01-Q01/source/feed-reviewed.mjs';
import {paperPosition,specs} from '../../../../modules/media/native-equipment-500/reviews/B01-Q01/source/batch-spec.mjs';
const home='modules/media/native-equipment-500/reviews/B01-Q01',catalog=JSON.parse(await fs.readFile(home+'/catalog.json','utf8')).equipment,checks=[];
const bounds=o=>new T.Box3().setFromObject(o,true),near=(a,b)=>assert.ok(Math.abs(a-b)<1e-6,`${a} vs ${b}`);
for(const spec of catalog)for(const simple of [true,false]){
 const b=build(spec,simple),mixer=new T.AnimationMixer(b.root),part=n=>{const o=b.root.getObjectByName(n);assert.ok(o,n);return o;},s=specs[spec.id];
 for(const clip of b.animations){const a=mixer.clipAction(clip);a.setLoop(T.LoopOnce,1);a.clampWhenFinished=true;a.play();for(const u of [0,.12,.30,.45,.50,.55,.60,.65,.75,.90,1,.50,0]){a.time=u;mixer.update(0);b.root.updateMatrixWorld(true);b.root.traverse(o=>assert.ok(o.matrixWorld.elements.every(Number.isFinite)));
  if(s.mode==='feed'){
   const rod=bounds(part('PickupPiston')),barrel=bounds(part('VerticalCylinder'));assert.ok(rod.max.y<=barrel.max.y+1e-6&&rod.max.y>barrel.min.y,'vertical piston engagement');
   const p=paperPosition(s,clip.name,u);if(clip.name==='auto'&&u<=.50)near(bounds(part('VacuumCup-0.075_-0.12')).min.y,p[1]+.0008);
   const drive=bounds(part('TraverseDrivePiston')),cylinder=bounds(part('TraverseAirCylinder'));assert.ok(drive.min.x<cylinder.max.x&&drive.max.x>=cylinder.min.x,'horizontal drive engagement');
  }else near(bounds(part('LiftPlatform')).max.y,part('LiftDeck').position.y);
 }a.stop();}
 checks.push({id:spec.id,lod:simple?'simple':'detailed',source_supports_drives_and_strokes:true});
}
await fs.writeFile(home+'/contact-checks.json',JSON.stringify({passed:true,profile:'b01q01-source-feed-and-lift',models:checks.length,checks},null,2)+'\n');console.log('Four source models: piston engagement, actual supports and finite forward/reverse strokes PASS.');
