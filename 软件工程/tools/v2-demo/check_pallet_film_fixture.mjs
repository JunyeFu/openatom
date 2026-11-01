import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import * as T from 'three';
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js';
const [fixture]=process.argv.slice(2),read=async p=>JSON.parse(await fs.readFile(p,'utf8'));
const plan=await read(path.join(fixture,'media-plan.json')),loader=new GLTFLoader(),samples=[];
// Numeric geometry only; actual texture/render evidence is collected in Native.
loader.register(()=>({name:'geometry-audit-only',loadTexture:async()=>new T.Texture()}));
async function load(id){const ref=plan.modules.find(m=>m.id===id),m=(await read(path.join(ref.directory,'module.json'))).data;const b=await fs.readFile(path.join(ref.directory,m.definition.payload.data.source.data.path));return loader.parseAsync(b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength),'');}
for(const lod of ['detailed','simplified']){
 const machine=await load('sf.paperfactory.pallet.media.pallet-stretch-wrapper.'+lod),film=await load(plan.film_media_ref.id);
 const seat=machine.scene.getObjectByName('WorkingAuxiliary');assert(seat);seat.add(film.scene);
 for(const fraction of [.13,.38,.9]){
  const a=new T.AnimationMixer(machine.scene),b=new T.AnimationMixer(film.scene);
  for(const [m,g,clip] of [[a,machine,machine.animations.find(c=>c.name==='auto')],[b,film,film.animations.find(c=>c.name.endsWith(':auto'))]]){const action=m.clipAction(clip);action.setLoop(T.LoopOnce,1);action.clampWhenFinished=true;action.play();m.setTime(clip.duration*fraction);}
  machine.scene.updateMatrixWorld(true);
  const roll=new T.Box3().setFromObject(film.scene.getObjectByName('MeteredFilmRoll'));
  const center=roll.getCenter(new T.Vector3()),spindle=machine.scene.getObjectByName('FilmSpindle').getWorldPosition(new T.Vector3());
  assert(center.distanceTo(spindle)<.015,`Film roll not on spindle ${center.toArray()} vs ${spindle.toArray()}`);
  assert(Math.abs(center.y-.46)<.001);const bounds=new T.Box3().setFromObject(film.scene);
  assert(bounds.max.y<=.72&&bounds.min.y>=.2,`Film exceeds one-layer setting ${bounds.min.y}..${bounds.max.y}`);
  samples.push({lod,fraction,roll_center:center.toArray(),spindle:spindle.toArray(),film_bounds:{min:bounds.min.toArray(),max:bounds.max.toArray()}});a.stopAllAction();b.stopAllAction();
 }
}
await fs.writeFile(path.join(fixture,'film-geometry-receipt.json'),JSON.stringify({status:'both_LODs_actual_roll_on_spindle_and_low_film_verified',samples,Native_verified:false},null,2));
console.log('Actual coupled wrapper and film geometry passed:',samples.length,'samples');
