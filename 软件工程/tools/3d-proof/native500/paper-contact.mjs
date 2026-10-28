import * as T from 'three';import fs from 'node:fs/promises';import assert from 'node:assert/strict';import path from 'node:path';import {pathToFileURL} from 'node:url';
const home=(process.argv[2]??'modules/media/native-equipment-500/reviews/B02-Q01')+'/';
const {build,stock}=await import(pathToFileURL(path.resolve(home,'source/paper-reviewed.mjs')).href);
const catalog=JSON.parse(await fs.readFile(home+'catalog.json','utf8')).equipment,checks=[];
const near=(a,b,label)=>assert.ok(Math.abs(a-b)<1e-6,`${label}: ${a} != ${b}`);
for(const spec of catalog)for(const simple of [false,true]){
 const b=build(spec,simple),node=n=>b.root.getObjectByName(n),mesh=(parent,prefix)=>parent.children.find(o=>o.name.startsWith(prefix));
 const mixer=new T.AnimationMixer(b.root);let action;
 const pose=(phase,u)=>{action?.stop();action=mixer.clipAction(b.animations.find(a=>a.name===phase));action.play();action.time=u; mixer.update(0);b.root.updateMatrixWorld(true);};
 const bounds=o=>new T.Box3().setFromObject(o);
 for(const sign of [-1,1])for(const z of [-.28,.28]){
  const leg=bounds(node('TransferLeg_'+sign+'_'+z)),foot=bounds(node('TransferRubberFoot_'+sign+'_'+z)),table=bounds(node('PaperTransferTable_'+sign));
  near(leg.min.y,foot.max.y,'transfer leg contacts rubber foot');near(leg.max.y,table.min.y,'transfer leg contacts tabletop underside');
 }
 const cargo=node(b.cargo),paper=new T.Mesh(new T.BoxGeometry(...stock[spec.id].input));paper.position.y=stock[spec.id].input[1]/2;cargo.add(paper);
 if(spec.id.includes('cutter')||spec.id==='three-knife-trimmer'){
  pose('auto',.5);const clamp=node('PaperHoldDown_0');near(bounds(mesh(clamp,'HoldDownBar')).min.y,bounds(paper).max.y,'clamp contact');
  near(bounds(mesh(clamp.parent,'FixtureBase_')).max.y,bounds(paper).min.y,'cutting table supports paper');
  near(bounds(mesh(node('PressSlide_1'),'WorkingBlade')).min.y,bounds(paper).min.y,'blade bottom stroke');
  for(const u of [0,.2,.5,.8,1]){pose('auto',u);const rod=bounds(mesh(node('PressSlide_1'),'RamRod')),barrel=bounds(mesh(node('PressSlide_1').parent,'HydraulicBarrel'));assert.ok(rod.max.y>=barrel.min.y && rod.max.y<=barrel.max.y,'ram remains inside cylinder throughout the stroke');}
 }
 if(spec.id==='pile-turner'){
  for(const u of [0,.25,.5,.75,1]){pose('auto',u);near(cargo.position.y,-.2225,'stock held in rotating cradle');near(node('PileClamp').position.y,-.085,'closed clamp');}
  pose('unload',1);near(bounds(paper).min.y,1.0475,'flipped paper rests on lower plate and output table');
 }
 if(spec.id==='paper-jogger'){
  pose('auto',.53125);const spring=mesh(node('JogDeck_0').parent,'TraySpring');near(bounds(spring).max.y,bounds(mesh(node('JogDeck_0'),'JoggingTray')).min.y,'spring under tray');
  near(bounds(paper).min.y,.8775,'paper sits on vibrating deck');
 }
 if(spec.id==='air-table-loader'){
  for(const u of [0,.2,.5]){pose('auto',u);near(bounds(mesh(node('Pusher_1'),'PushFace')).max.x,bounds(paper).min.x,'pusher contacts rear paper edge');near(bounds(paper).min.y,.8425,'air deck support');}
  pose('auto',1);near(bounds(paper).max.x,.30,'registered front edge');near(cargo.position.x,.10,'paper remains when pusher retracts');
  const rod=bounds(mesh(node('Pusher_1'),'PistonRod'));assert.ok(rod.max.x-rod.min.x>.33 && rod.max.y-rod.min.y<.019);
 }
 checks.push({id:spec.id,lod:simple?'simple':'detailed',declared_stock_m:stock[spec.id].input,contact_checks:true});
}
await fs.writeFile(home+'contact-checks.json',JSON.stringify({passed:true,checks_profile:'paper-contact-v2-floor-support',transfer_support_contact_pairs:checks.length*8,scope:'declared rigid contact geometry, not a general collision solver',checks},null,2)+'\n');console.log('10 models: transfer-foot support, clamp/blade, rotating holder, tray springs and air-table contact PASS.');
