import * as T from 'three';import fs from 'node:fs/promises';import assert from 'node:assert/strict';
import {build} from '../../../../modules/media/native-equipment-500/reviews/B02-Q04/source/finish-reviewed.mjs';import {specs,surface,filmHeight} from '../../../../modules/media/native-equipment-500/reviews/B02-Q04/source/batch-spec.mjs';
const home='modules/media/native-equipment-500/reviews/B02-Q04/',catalog=JSON.parse(await fs.readFile(home+'catalog.json','utf8')).equipment,checks=[];const near=(a,b,n)=>assert.ok(Math.abs(a-b)<1e-6,`${n}: ${a} != ${b}`);
for(const spec of catalog)for(const simple of [false,true]){
 const s=specs[spec.id],b=build(spec,simple),node=n=>{const o=b.root.getObjectByName(n);assert.ok(o,n);return o;},bounds=o=>new T.Box3().setFromObject(o);b.root.updateMatrixWorld(true);let legs=0;
 b.root.traverse(o=>{if(/^(InputTable|OutputTable|AuxSupplyTable|SpentBackingTable)Leg/.test(o.name)){const prefix=o.name.split('Leg')[0];near(bounds(o).min.y,bounds(node(o.name.replace('Leg','Foot'))).max.y,'leg/foot');near(bounds(o).max.y,bounds(node(prefix+'Top')).min.y,'leg/table');near(bounds(node(o.name.replace('Leg','Foot'))).min.y,0,'foot/floor');legs++;}});
 const mixer=new T.AnimationMixer(b.root);let action;const pose=(phase,u)=>{action?.stop();action=mixer.clipAction(b.animations.find(c=>c.name===phase));action.play();action.time=u;mixer.update(0);b.root.updateMatrixWorld(true);};
 for(const u of [0,.12,.20,.25,.40,.50,.65,.70,.75,.78,.82,.90,1]){
  pose('auto',u);
  if(['foil','emboss'].includes(s.mode)){
   const pipe=node('PressHydraulicFeed').geometry.parameters.path;near(pipe.getPoint(0).y,bounds(node('PressPowerPack')).max.y,'hydraulic feed / power pack port');near(pipe.getPoint(1).z,bounds(node('PressHydraulicPort')).min.z,'hydraulic feed / cylinder port');near(pipe.getPoint(1).y,1.35,'hydraulic cylinder port height');
   const rod=bounds(node('PressPiston')),barrel=bounds(node('PressHydraulicBarrel'));assert.ok(rod.max.y>=barrel.min.y&&rod.max.y<=barrel.max.y,'press piston insertion');
   for(const x of [-.20,.20])for(const z of [-.14,.14]){const stem=bounds(node('SpringStem'+x+'_'+z)),housing=bounds(node('SpringHousing'+x+'_'+z));assert.ok(stem.max.y>=housing.min.y&&stem.max.y<=housing.max.y,'spring stem insertion');if(u>=.20&&u<=.65)near(bounds(node('SpringFace'+x+'_'+z)).min.y,surface+s.size[1],'spring / paper top');}
   if(s.mode==='foil'){
    assert.ok(bounds(node('HeaterMovingCollector')).intersectsBox(bounds(node('HeaterVerticalConductor'))),'heater sliding power contact');
    const vrod=bounds(node('PeelVerticalPiston')),vbarrel=bounds(node('PeelVerticalCylinder')),hrod=bounds(node('FoilHorizontalPiston')),hbarrel=bounds(node('FoilHorizontalCylinder'));assert.ok(vrod.max.y>=vbarrel.min.y&&vrod.max.y<=vbarrel.max.y,'foil lift piston insertion');assert.ok(hrod.min.x>=hbarrel.min.x&&hrod.min.x<=hbarrel.max.x,'foil peel piston insertion');
    for(const sign of [-1,1])for(const z of [-.04,.04])near(bounds(node('CarrierVacuumPad'+sign+'_'+z)).max.y,bounds(node('CarrierVacuumBracket'+sign+'_'+z)).min.y,'foil vacuum pad / bracket');
    if(u>=.20&&u<=.65)near(bounds(node('HeatedDie')).min.y,surface+s.size[1]+s.carrier[1]+s.metal_thickness,'heat die / actual carrier top');
   }else if(u>=.40&&u<=.65){near(bounds(node('PositiveEmbossTool')).max.y,surface+s.raise,'lower male / embossed underside');near(bounds(node('FemaleEmbossRoof')).min.y,surface+s.size[1]+s.raise,'upper female / embossed top');}
  }else if(s.mode==='screen'||s.mode==='uv'){
   for(const sign of [-1,1]){const rod=bounds(node('ScreenLiftPiston'+sign)),barrel=bounds(node('ScreenLiftCylinder'+(sign*.45)));assert.ok(rod.max.y>=barrel.min.y&&rod.max.y<=barrel.max.y,'screen lift piston insertion');}
   const rod=bounds(node('SqueegeePiston')),barrel=bounds(node('SqueegeeCylinder'));assert.ok(rod.min.x>=barrel.min.x&&rod.min.x<=barrel.max.x,'squeegee piston insertion');
   near(bounds(node('SqueegeeRubber')).min.y,bounds(node('ScreenStencilMask')).max.y,'squeegee / stencil top');near(bounds(node('WorkingDoseWellFloor')).min.y,bounds(node('DoseWellBracket')).max.y,'working well / bracket');
   if(u>=.12&&u<=.50)near(bounds(node('ScreenMeshZ0')).min.y,surface+s.size[1],'screen contact / paper');
  }else{
   const rod=bounds(node('PadPiston')),barrel=bounds(node('PadVerticalCylinder'));assert.ok(rod.max.y>=barrel.min.y&&rod.max.y<=barrel.max.y,'pad piston insertion');near(bounds(node('PadVerticalCylinder')).max.y,bounds(node('PadCylinderMount')).min.y,'pad cylinder / mount');
   near(bounds(node('DoctorRubber')).min.y,bounds(node('ClichePlate')).max.y,'doctor / cliche top');
   if(u>=.20&&u<=.25)near(bounds(node('SiliconeTransferPad')).min.y,bounds(node('ClichePlate')).max.y,'pad pickup / cliche ink top');
   if(u>=.70&&u<=.78)near(bounds(node('SiliconeTransferPad')).min.y,surface+s.size[1]+filmHeight(s),'pad print / material stack top');
   const doctor=bounds(node('DoctorPiston')),cylinder=bounds(node('DoctorBladeCylinder'));assert.ok(doctor.min.x>=cylinder.min.x&&doctor.min.x<=cylinder.max.x,'doctor piston insertion');
   near(bounds(node('PadSlidingCollector')).max.y,bounds(node('PadConductorRail')).min.y,'moving collector / conductor rail');
  }
 }
 checks.push({id:spec.id,lod:simple?'simple':'detailed',table_legs:legs,source_supports_and_strokes:true});
}
await fs.writeFile(home+'contact-checks.json',JSON.stringify({passed:true,profile:'q04-source-finishing-fixtures',models:checks.length,checks,actual_Item_contact:'pending',Native:'pending'},null,2)+'\n');console.log('Q04 ten source models: supports, tool contact datums, all actuator insertions and moving service collectors PASS.');
