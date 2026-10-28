import * as T from 'three';import fs from 'node:fs/promises';import assert from 'node:assert/strict';
import {build} from '../../../../modules/media/native-equipment-500/reviews/B02-Q02/source/web-reviewed.mjs';
const home='modules/media/native-equipment-500/reviews/B02-Q02/',catalog=JSON.parse(await fs.readFile(home+'catalog.json','utf8')).equipment,checks=[];
const near=(a,b,label)=>assert.ok(Math.abs(a-b)<1e-6,`${label}: ${a} != ${b}`);
for(const spec of catalog)for(const simple of [false,true]){
 const b=build(spec,simple),node=n=>{const o=b.root.getObjectByName(n);assert.ok(o,n);return o;},bounds=o=>new T.Box3().setFromObject(o);
 b.root.updateMatrixWorld(true);let tables=0,motors=0;
 b.root.traverse(o=>{
  if(o.name.endsWith('Top')&&b.root.getObjectByName(o.name.slice(0,-3)+'Leg-0.3_-0.28')){
   const name=o.name.slice(0,-3);
   for(const dx of [-.30,.30])for(const z of [-.28,.28]){const leg=node(name+'Leg'+dx+'_'+z),foot=node(name+'Foot'+dx+'_'+z);near(bounds(leg).min.y,bounds(foot).max.y,'table leg / foot');near(bounds(leg).max.y,bounds(o).min.y,'table leg / top');}tables++;
  }
  if(o.name.endsWith('DriveBody')){
   const prefix=o.name.slice(0,-4),mount=node(prefix+'Mount'),stand=node(prefix+'Stand');near(bounds(o).min.y,bounds(mount).max.y,'motor body / mount');near(bounds(mount).min.y,bounds(stand).max.y,'motor mount / stand');motors++;
  }
 });
 const mixer=new T.AnimationMixer(b.root);let action;
 const pose=(phase,u)=>{action?.stop();action=mixer.clipAction(b.animations.find(a=>a.name===phase));action.play();action.time=u;mixer.update(0);b.root.updateMatrixWorld(true);};
 if(spec.id==='suction-separator'){
  for(const [u,y]of [[0,.905],[1,.9025]]){pose('auto',u);near(node('SuctionPaperSeat').getWorldPosition(new T.Vector3()).y,y,'cup / paper top datum');}
  for(const u of [0,.15,.35,.70,.85,1]){pose('auto',u);const rod=bounds(node('HeadPiston')),barrel=bounds(node('HeadPneumaticCylinder'));assert.ok(rod.max.y>=barrel.min.y&&rod.max.y<=barrel.max.y,'piston insertion');const hose=bounds(node('TraversePneumaticService')),carriage=node('VacuumTraverse').getWorldPosition(new T.Vector3());near(hose.min.x,-.90,'fixed pneumatic supply');near(hose.max.x,carriage.x+.14,'moving pneumatic supply');near(bounds(node('FlexibleVacuumService')).min.y,bounds(node('VacuumManifold')).max.y,'vacuum hose / manifold');}
 }
 if(spec.id==='slitter-rewinder'){pose('auto',0);near(bounds(node('CenterSlittingDisc')).min.y,1,'slit blade / web datum');near(bounds(node('SlitCounterRollBody')).max.y,1,'slit anvil / web datum');near(bounds(node('OutputReelSharedCenterCollar')).min.z,-.01,'shared two-roll collar');}
 if(spec.id==='rotary-sheet-cutter'){pose('auto',0);near(bounds(node('FullWidthCrossKnife')).min.y,1,'cross knife / web datum');near(bounds(node('CrossKnifeAnvilBody')).max.y,1,'cross anvil / web datum');near(bounds(node('SheetEndStop')).min.x,1.30,'sheet endpoint / stop');assert.ok(node('CrossKnifeDriveCoupling'));}
 const op=JSON.parse(await fs.readFile(home+'devices/'+spec.id+'/module.json','utf8')).data.definition.payload.data.behavior.inline;
 assert.equal(op.inputs[0].quantity.value.coefficient,1);assert.equal(op.outputs[0].quantity.value.coefficient, spec.id==='slitter-rewinder'?2:spec.id==='rotary-sheet-cutter'?60:1);
 checks.push({id:spec.id,lod:simple?'simple':'detailed',tables,motors,source_contact_assertions:true,declared_input_quantity:1,declared_output_quantity:op.outputs[0].quantity.value.coefficient});
}
await fs.writeFile(home+'contact-checks.json',JSON.stringify({passed:true,profile:'q02-source-contact-only',models:checks.length,checks,Native_acceptance:'pending',actual_Item_contact:'pending'},null,2)+'\n');console.log('Q02 source contacts and 1→1 / 1→2 / 1→60 declarations PASS; actual Item and Native remain pending.');
