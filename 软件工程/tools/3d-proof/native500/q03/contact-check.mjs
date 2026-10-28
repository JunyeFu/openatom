import * as T from 'three';import fs from 'node:fs/promises';import assert from 'node:assert/strict';
import {build} from '../../../../modules/media/native-equipment-500/reviews/B02-Q03/source/sheet-reviewed.mjs';
import {specs,pressTimes} from '../../../../modules/media/native-equipment-500/reviews/B02-Q03/source/batch-spec.mjs';
const home='modules/media/native-equipment-500/reviews/B02-Q03/',catalog=JSON.parse(await fs.readFile(home+'catalog.json','utf8')).equipment,checks=[];
const near=(a,b,label)=>assert.ok(Math.abs(a-b)<1e-6,`${label}: ${a} != ${b}`);
for(const spec of catalog)for(const simple of [false,true]){
 const b=build(spec,simple),s=specs[spec.id],node=n=>{const o=b.root.getObjectByName(n);assert.ok(o,n);return o;},bounds=o=>new T.Box3().setFromObject(o);b.root.updateMatrixWorld(true);let table_legs=0,motors=0;
 b.root.traverse(o=>{
  if(/^(InputTable|OutputTable|InkSupplyTable)Leg/.test(o.name)){
   const name=o.name.split('Leg')[0],foot=node(o.name.replace('Leg','Foot'));near(bounds(foot).min.y,0,o.name+' foot / floor');near(bounds(o).min.y,bounds(foot).max.y,o.name+' leg / foot');near(bounds(o).max.y,bounds(node(name+'Top')).min.y,o.name+' leg / top');table_legs++;
  }
  if(o.name.endsWith('DriveBody')){const prefix=o.name.slice(0,-4);near(bounds(o).min.y,bounds(node(prefix+'Mount')).max.y,prefix+' motor mount');near(bounds(node(prefix+'Mount')).min.y,bounds(node(prefix+'Stand')).max.y,prefix+' mount / stand');motors++;}
  if(o.name.startsWith('WasteCartLeg')){near(bounds(o).min.y,.07,'waste leg / caster');near(bounds(o).max.y,bounds(node('WasteCartFloor')).min.y,'waste leg / floor');}
 });
 const mixer=new T.AnimationMixer(b.root);let action;const pose=(phase,u)=>{action?.stop();action=mixer.clipAction(b.animations.find(a=>a.name===phase));action.play();action.time=u;mixer.update(0);b.root.updateMatrixWorld(true);};
 if(s.printed){near(bounds(node('SupplyCupFloor')).min.y,.90,'supply cup / table');near(bounds(node('InkCupFloor')).min.y,bounds(node('InkFountainBracket')).max.y,'fountain / bracket');near(bounds(node('DoctorBlade')).min.y,bounds(node('AniloxCylinderRoll')).max.y,'doctor / anilox');}
 if(s.mode==='crease')for(const u of pressTimes){pose('auto',u);const rod=bounds(node('CreasePiston')),barrel=bounds(node('CreaseHydraulicBarrel'));assert.ok(rod.max.y>=barrel.min.y&&rod.max.y<=barrel.max.y,'crease piston insertion');for(const x of [-.22,.22])for(const z of [-.16,.16])if(u<=.6)near(bounds(node('SpringClampFace'+x+'_'+z)).min.y,.8205,'spring clamp / paper top');}
 if(s.mode==='trim')for(const u of [0,.15,.64,.72,.82,1]){pose('auto',u);for(const sign of [-1,1]){const rod=bounds(node('EndKnifePiston'+sign)),barrel=bounds(node('EndKnifeCylinder'+sign));assert.ok(rod.max.y>=barrel.min.y&&rod.max.y<=barrel.max.y,'end knife piston insertion');const rail=bounds(node('SawConductorRail'+sign)),shoe=bounds(node('SawCollectorShoe'+sign));assert.ok(rail.intersectsBox(shoe),'saw sliding power collector / conductor rail');const saw=bounds(node('EdgeSawDisc'+sign)),shaft=bounds(node('SawShaft'+sign));assert.ok(saw.min.z<=shaft.max.z&&saw.max.z>=shaft.min.z,'saw / shaft contact');}}
 checks.push({id:spec.id,lod:simple?'simple':'detailed',table_legs,motors,source_contacts:true});
}
await fs.writeFile(home+'contact-checks.json',JSON.stringify({passed:true,profile:'q03-source-supports-and-strokes',models:checks.length,checks,actual_product_decoder_contact:'pending',Native:'pending'},null,2)+'\n');console.log('Q03 ten models: table/feet, motor mounts, waste cart, cup, doctor blade, spring pad and piston source contact checks PASS.');
