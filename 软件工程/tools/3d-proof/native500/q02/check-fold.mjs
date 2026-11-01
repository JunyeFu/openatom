import * as T from 'three';import fs from 'node:fs/promises';import assert from 'node:assert/strict';
import {build} from '../../../../modules/media/native-equipment-500/reviews/B02-Q02/source/web-reviewed.mjs';
import {buildItem} from '../../../../modules/media/native-equipment-500/reviews/B02-Q02/source/item-build.mjs';
import {specs,paper,foldCount,foldStraight,foldRadius,foldEntryY} from '../../../../modules/media/native-equipment-500/reviews/B02-Q02/source/batch-spec.mjs';
const home='modules/media/native-equipment-500/reviews/B02-Q02',catalog=JSON.parse(await fs.readFile(home+'/catalog.json','utf8')).equipment,checks=[];
const near=(a,b,label)=>assert.ok(Math.abs(a-b)<2e-6,`${label}: ${a} != ${b}`);
near(foldCount*foldStraight+(foldCount-1)*Math.PI*foldRadius,paper.length,'continuous ribbon centerline length');
for(const id of ['paper-unwinder','paper-rewinder'])for(const simple of [false,true]){
 const b=build(catalog.find(e=>e.id===id),simple),s=specs[id],item=buildItem({id:id+'-input',device:id,role:'input'}),cargo=b.root.getObjectByName(b.cargo);cargo.add(item.root);
 const deviceMixer=new T.AnimationMixer(b.root),itemMixer=new T.AnimationMixer(item.root),da=deviceMixer.clipAction(b.animations.find(a=>a.name==='auto')),ia=itemMixer.clipAction(item.clips.find(a=>a.name===s.initialState+':auto'));da.play();ia.play();
 const node=n=>{const o=b.root.getObjectByName(n);assert.ok(o,n);return o;},bounds=o=>new T.Box3().setFromObject(o);let samples=0;
 for(const u of [.001,1/60,.1175,.25,.501,29/60,.75,.999, .25,.001]){
  da.time=ia.time=u;deviceMixer.update(0);itemMixer.update(0);b.root.updateMatrixWorld(true);
  const slot=node('FoldGuideSlot'),lipL=bounds(node('FoldGuideLip-1')),lipR=bounds(node('FoldGuideLip1')),carriage=node('FoldTraverse');
  const web=node('HeldPaperWeb'+(s.mode==='unwind'?3:0));const bb=bounds(web);near(bb.min.x,lipL.max.x,'paper left face / guide slot');near(bb.max.x,lipR.min.x,'paper right face / guide slot');near(bb.max.y,foldEntryY,'paper enters head upper face');
  const amount=(s.mode==='unwind'?u:1-u)*60,index=Math.max(0,Math.ceil(amount)-1),layer=bounds(node('HeldBatchLayer'+index));near(bb.min.y,(layer.min.y+layer.max.y)/2,'web tail / live fold centerline');
  const endX=index%2?layer.min.x:layer.max.x;near((bb.min.x+bb.max.x)/2,endX,'head follows actual folding/unfolding edge');
  for(const z of [-.27,.27]){const hanger=bounds(node('FoldHeadHanger'+z));near(hanger.max.y,bounds(node('FoldCrosshead')).min.y,'hanger / crosshead');near(hanger.min.y,lipL.min.y,'hanger / guide lip');}
  const service=bounds(node('FoldMovingPower'));near(service.min.x,s.fold[0]-.40,'fixed moving-power endpoint');near(service.max.x,carriage.position.x,'moving power / carriage');
  const motor=bounds(node('FoldTraverseMotor')),coupling=bounds(node('FoldDriveCoupling'));assert.ok(coupling.min.z<=motor.max.z&&coupling.max.z>=bounds(node('FoldPinionHub')).min.z,'motor-to-pinion coupling overlap');
  assert.equal(item.root.children.filter(o=>o.name.startsWith('ContinuousFold')).length,59);samples++;
 }
 checks.push({id,lod:simple?'simple':'detailed',continuous_folds:59,centerline_length_m:30,forward_reverse_contact_samples:samples,passed:true});
}
await fs.writeFile(home+'/fold-contact-checks.json',JSON.stringify({passed:true,checks,scope:'Source mechanical and actual-held item poses; product decoder and Native still required'},null,2)+'\n');console.log('Continuous30m ribbon,59 folds and40 forward/reverse actual-item/slot/support/power contacts PASS.');
