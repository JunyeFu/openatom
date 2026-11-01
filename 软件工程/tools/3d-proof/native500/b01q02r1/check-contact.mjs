import fs from 'node:fs/promises';import * as T from 'three';import assert from 'node:assert/strict';
import {build} from '../../../../modules/media/native-equipment-500/reviews/B01-Q02-R1/source/print-reviewed.mjs';
import {specs,inkSeat} from '../../../../modules/media/native-equipment-500/reviews/B01-Q02-R1/source/batch-spec.mjs';
const home='modules/media/native-equipment-500/reviews/B01-Q02-R1',spec=JSON.parse(await fs.readFile(home+'/catalog.json','utf8')).equipment[0],s=specs[spec.id],checks=[];
for(const simple of [true,false]){
 const b=build(spec,simple),box=n=>new T.Box3().setFromObject(b.root.getObjectByName(n),true),near=(a,b)=>assert.ok(Math.abs(a-b)<1e-6,`${a} vs ${b}`),mixer=new T.AnimationMixer(b.root);
 b.root.updateMatrixWorld(true);
 for(const c of s.colors){const impression=box('Impression_'+c.name+'Roll');near(impression.max.y,s.surface);assert.ok(impression.min.y>=.8-1e-6,'counter roll intersects machine bed');near(box('WorkingInk_'+c.name+'Floor').max.y,inkSeat(c)[1]);near(box('InkBracket_'+c.name+'Column').max.y,box('WorkingInk_'+c.name+'Floor').min.y);near(Math.abs(b.root.getObjectByName('Anilox_'+c.name).position.x-b.root.getObjectByName('PlateCylinder_'+c.name).position.x),.12);}
 for(const clip of b.animations){const a=mixer.clipAction(clip);a.setLoop(T.LoopOnce,1);a.clampWhenFinished=true;a.play();for(const u of [0,.275,.30,.325,.5,.675,.70,.725,1,.5,0]){a.time=u;mixer.update(0);b.root.updateMatrixWorld(true);b.root.traverse(o=>assert.ok(o.matrixWorld.elements.every(Number.isFinite)));}a.stop();}
 const auto=mixer.clipAction(b.animations.find(c=>c.name==='auto'));auto.play();
 for(const c of s.colors)for(const[name,r,sign]of[['PlateCylinder_'+c.name,.08,-1],['Impression_'+c.name,.06,1]]){const u=(c.x+1)/2;auto.time=u;mixer.update(0);b.root.updateMatrixWorld(true);const o=b.root.getObjectByName(name),contact=o.getWorldPosition(new T.Vector3()).add(new T.Vector3(0,sign*r,0)),local=o.worldToLocal(contact.clone());auto.time=u+.00001;mixer.update(0);b.root.updateMatrixWorld(true);const velocity=(o.localToWorld(local).x-contact.x)/.00001;assert.ok(Math.abs(velocity-2)<.001,'actual print nip must co-move with paper: '+name+' '+velocity);}auto.stop();
 checks.push({id:spec.id,lod:simple?'simple':'detailed',source_bed_clearance_ink_supports_and_finite_strokes:true});
}
await fs.writeFile(home+'/contact-checks.json',JSON.stringify({passed:true,profile:'b01q02r1-source-two-color-flexo',models:2,checks},null,2)+'\n');console.log('Two source models: bed clearance, separate ink cup supports and finite forward/reverse strokes PASS.');
