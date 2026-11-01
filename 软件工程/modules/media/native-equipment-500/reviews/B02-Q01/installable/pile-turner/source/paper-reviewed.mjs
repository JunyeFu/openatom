import * as T from 'three';
import {build as original} from './kit.mjs';
// Exact paper workpieces used by this review. No item is embedded in machine geometry.
export const stock={
 'guillotine-paper-cutter':{input:[.50,.06,.44],output:[.50,.06,.20],seat:[0,.845,0]},
 'three-knife-trimmer':{input:[.53,.06,.50],output:[.49,.06,.48],seat:[0,.845,0]},
 'pile-turner':{input:[.46,.12,.42],output:[.46,.12,.42]},
 'paper-jogger':{input:[.50,.08,.44],output:[.50,.08,.44]},
 'air-table-loader':{input:[.40,.08,.36],output:[.40,.08,.36]},
};
export function build(spec,simple){
 const b=original(spec,simple),root=b.root,t=[0,.2,.5,.8,1];
 const node=n=>{const o=root.getObjectByName(n);if(!o)throw Error('Missing reviewed node '+n);return o;};
 const group=(p,name,pos)=>{const o=new T.Group();o.name=name;o.position.fromArray(pos);p.add(o);return o;};
 const box=(p,name,size,pos,material)=>{const o=new T.Mesh(new T.BoxGeometry(...size),material);o.name=name;o.position.fromArray(pos);o.castShadow=o.receiveShadow=true;p.add(o);return o;};
 const cylinder=(p,name,r,h,pos,material)=>{const o=new T.Mesh(new T.CylinderGeometry(r,r,h,simple?12:32),material);o.name=name;o.position.fromArray(pos);p.add(o);return o;};
 const material=()=>{let m;root.traverse(o=>{if(o.isMesh&&o.material.name==='Anodised aluminium')m=o.material;});return m;};
 const rubber=()=>{let m;root.traverse(o=>{if(o.isMesh&&o.material.name==='Rubber')m=o.material;});return m;};
 const cable=(parent,name,points)=>{const curve=new T.CatmullRomCurve3(points.map(p=>new T.Vector3(...p)));const o=new T.Mesh(new T.TubeGeometry(curve,24,.004,simple?5:8,false),rubber());o.name=name;parent.add(o);return o;};
 cable(root,'CabinetServiceFeed',[[.54,.54,-.43],[.64,.56,-.50],[.64,.80,-.50],[.42,.82,-.50]]);
 for(const x of [.42,.64])box(root,'ServiceCableClamp_'+x,[.016,.012,.020],[x,.806,-.50],material());
 const put=(phase,track)=>{const c=b.animations.find(c=>c.name===phase);c.tracks=c.tracks.filter(t=>t.name!==track.name);c.tracks.push(track);};
 const position=(phase,o,rows,times=t)=>put(phase,new T.VectorKeyframeTrack(o.name+'.position',times,rows.flat()));
 const rotation=(phase,o,angles,times=t)=>put(phase,new T.QuaternionKeyframeTrack(o.name+'.quaternion',times,angles.flatMap(a=>new T.Quaternion().setFromAxisAngle(new T.Vector3(0,0,1),a).toArray())));
 const hold=(phase,o,pos)=>position(phase,o,[pos,pos],[0,1]);
 function seat(p,pos){const o=group(p,'ReviewedPaperSeat',pos);b.cargo=o.name;return o;}
 if(['guillotine-paper-cutter','three-knife-trimmer'].includes(spec.id)){
  const clamp=node('PaperHoldDown_0'),station=clamp.parent,head=node('PressSlide_1');
  // Rear actuators clear the moving platen. The hold-down closes before cutting.
  station.children.filter(o=>o.name.startsWith('HoldDownGuide_')||o.name.startsWith('GuideFoot_')).forEach(o=>o.removeFromParent());
  station.children.find(o=>o.name.startsWith('FixtureBase_')).position.y=.032;
  clamp.position.y=.155;
  for(const x of [-.18,.18]){
   box(clamp,'ClampRearArm_'+x,[.045,.05,.18],[x,0,-.075],material());
   cylinder(station,'ClampRearCylinder_'+x,.024,.20,[x,.26,-.37],material());
   box(station,'ClampRearFoot_'+x,[.07,.16,.075],[x,.08,-.37],material());
   cylinder(clamp,'ClampRearPiston_'+x,.009,.22,[x,.08,-.15],material());
  }
  const ram=head.children.find(o=>o.name.startsWith('RamRod_'));ram.geometry.dispose();ram.geometry=new T.CylinderGeometry(.034,.034,.46,simple?12:32);ram.position.y=.2625;
  const barrel=head.parent.children.find(o=>o.name.startsWith('HydraulicBarrel_'));barrel.geometry.dispose();barrel.geometry=new T.CylinderGeometry(.075,.075,.52,simple?12:32);barrel.position.y=.89;
  cylinder(station,'RamCrownMountCollar',.09,.01,[0,.965,0],material());
  if(spec.id==='three-knife-trimmer')head.children.find(o=>o.name.startsWith('WorkingBlade_')).position.z=.24;
  position('load',clamp,[0,.2,.8,1,1].map(u=>[0,.155-.025*u,-.22]));
  hold('load',head,[0,.55,0]);hold('auto',clamp,[0,.13,-.22]);
  position('auto',head,[0,.25,1,1,0].map(u=>[0,.55-.36*u,0]));
  position('unload',clamp,[1,1,.8,.2,0].map(u=>[0,.155-.025*u,-.22]));hold('unload',head,[0,.55,0]);
  seat(station,[0,.045,0]);
  const tank=box(root,'HydraulicReservoir',[.30,.18,.20],[-.28,.373,-.20],material());
  cylinder(root,'HydraulicPumpMotor',.045,.10,[-.28,.51,-.20],material()).rotation.x=Math.PI/2;
  box(root,'PumpMotorSaddle',[.12,.05,.10],[-.28,.486,-.20],material());
  cylinder(root,'ReservoirFillCap',.018,.02,[-.38,.473,-.25],rubber());
  cable(root,'PumpToValveHose',[[-.28,.52,-.27],[-.45,.54,-.32],[-.45,.78,-.50],[.42,.82,-.50]]);
  cable(root,'MainRamSupplyHose',[[.42,.82,-.50],[-.34,.84,-.50],[-.34,1.55,-.31],[-.12,1.55,-.12],[-.074,1.55,0]]);
  for(const x of [-.18,.18])cable(root,'ClampSupplyHose_'+x,[[.42,.82,-.50],[x,.86,-.50],[x,1.06,-.50],[x,1.06,-.394]]);
 }
 if(spec.id==='pile-turner'){
  const cradle=node('PileCradle_0'),fixed=cradle.children.find(o=>o.name.startsWith('CradleClamp_'));fixed.removeFromParent();
  const clamp=group(cradle,'PileClamp',[0,.24,0]);fixed.position.set(0,0,0);clamp.add(fixed);
  // Twin screws and travelling nuts constrain the clamp; the drive has a bed-mounted stand.
  for(const x of [-.30,.30]){
   cylinder(cradle,'PileClampLeadScrew_'+x,.008,.53,[x,.025,-.275],material());
   cylinder(clamp,'PileClampTravellingNut_'+x,.020,.055,[x,0,-.275],material());
   cylinder(cradle,'PileClampMotor_'+x,.032,.06,[x,.31,-.275],material());
  }
  box(cradle,'PileClampUpperCrossmember',[.68,.03,.07],[0,.275,-.275],material());
  box(cradle,'PileClampCrossmemberFoot',[.035,.06,.07],[-.35,.25,-.275],material());
  box(cradle.parent,'TurnerMotorStand',[.16,.386,.20],[0,.193,-.64],material());
  box(cradle.parent,'TurnerMotorStandFoot',[.22,.02,.28],[0,.01,-.59],material());
  cable(root,'TurnerMotorPower',[[.42,.82,-.50],[.18,.84,-.64],[0,1.195,-.64]]);
  cylinder(cradle.parent,'ClampPowerSlipRing',.035,.03,[0,.47,-.45],rubber()).rotation.x=Math.PI/2;
  cable(root,'ClampSlipRingFeed',[[.42,.82,-.50],[.18,.86,-.50],[0,1.27,-.465]]);
  for(const x of [-.30,.30])cable(cradle,'RotatingClampMotorFeed_'+x,[[0,0,-.45],[-.35,0,-.30],[-.35,.275,-.30],[x,.275,-.30],[x,.31,-.30]]);
  const paper=seat(cradle,[0,-.2225,0]);
  position('load',clamp,[0,.2,1,1,1].map(u=>[0,.24-.325*u,0]));rotation('load',cradle,[0,0],[0,1]);hold('load',paper,[0,-.2225,0]);
  hold('auto',clamp,[0,-.085,0]);rotation('auto',cradle,[0,.2,.5,.8,1].map(u=>u*Math.PI));hold('auto',paper,[0,-.2225,0]);
  position('unload',clamp,[0,.2,.5,.8,1].map(u=>[0,-.085+.325*u,0]));rotation('unload',cradle,[Math.PI,Math.PI],[0,1]);
  position('unload',paper,[0,.2,.5,.8,1].map(u=>[0,-.2225+.325*u,0]));
 }
 if(spec.id==='paper-jogger'){
  const deck=node('JogDeck_0');seat(deck,[0,.0175,0]);
  for(const spring of deck.parent.children.filter(o=>o.name.startsWith('TraySpring_'))){spring.position.y=.02125;spring.geometry.dispose();spring.geometry=new T.CylinderGeometry(.025,.025,.0425,simple?12:32);}
  box(deck,'JogEccentricBearingMount',[.08,.035,.06],[.28,0,-.37],material());
  const times=Array.from({length:33},(_,i)=>i/32),values=times.map((_,i)=>[i===32?0:(i%2?.006:-.006),.06,0]);position('auto',deck,values,times);hold('load',deck,[0,.06,0]);hold('unload',deck,[0,.06,0]);
  const station=deck.parent,m=material();box(station,'JogDriveFoot',[.14,.02,.13],[.28,.01,-.43],m);cylinder(station,'JogDriveStator',.035,.08,[.28,.055,-.43],m).rotation.x=Math.PI/2;
  const rotor=group(station,'JogEccentricRotor',[.28,.055,-.37]);cylinder(rotor,'JogOutputShaft',.009,.06,[0,0,0],m).rotation.x=Math.PI/2;
  cylinder(rotor,'JogEccentricWeight',.025,.012,[.008,0,.025],m).rotation.x=Math.PI/2;rotation('auto',rotor,times.map(u=>u*8*Math.PI),times);
  cable(root,'JogDrivePower',[[.42,.82,-.50],[.28,.82,-.48],[.28,.855,-.47]]);
 }
 if(spec.id==='air-table-loader'){
  const table=node('Station_00_airtable'),push=node('Pusher_1'),fence=node('AirTableFence_0');
  for(const nozzle of table.children.filter(o=>o.name.startsWith('AirNozzle_')))nozzle.position.y=.042;
  push.children.find(o=>o.name.startsWith('PistonRod_')).rotation.z=Math.PI/2;
  for(const phase of ['load','auto','unload'])hold(phase,fence,[.3125,.10,0]);
  const paper=seat(table,[-.0875,.0425,0]);hold('load',paper,[-.0875,.0425,0]);hold('load',push,[-.30,.14,0]);
  position('auto',push,[0,.2,1,1,0].map(u=>[-.30+.1875*u,.14,0]));
  position('auto',paper,[0,.2,1,1,1].map(u=>[-.0875+.1875*u,.0425,0]));
  hold('unload',push,[-.30,.14,0]);hold('unload',paper,[.10,.0425,0]);
  box(root,'AirTableValveBlock',[.12,.06,.10],[-.42,.84,-.45],material());
  cable(root,'AirSupplyService',[[.42,.82,-.50],[-.42,.82,-.52],[-.42,.84,-.50]]);
  cable(root,'AirTablePlenumFeed',[[-.42,.84,-.45],[-.42,.81,-.40],[0,.81,-.38]]);
  cable(root,'PusherCylinderFeed',[[-.42,.84,-.45],[-.35,.86,-.45],[-.35,.94,-.406]]);
 }
 const surface=spec.id==='pile-turner'?1.0475:spec.id==='paper-jogger'?.8775:spec.id==='air-table-loader'?.8425:.845;
 // Explicit in/out support tables: stock at a Location never floats beside the machine.
 for(const sign of [-1,1]){
  const x=sign*1.05,m=material();box(root,'PaperTransferTable_'+sign,[.80,.025,.72],[x,surface-.0125,0],m);
  for(const z of [-.28,.28]){
   let rubber;root.traverse(o=>{if(o.isMesh&&o.material.name==='Rubber')rubber=o.material;});
   cylinder(root,'TransferRubberFoot_'+sign+'_'+z,.04,.02,[sign*1.28,.01,z],rubber);
   box(root,'TransferLeg_'+sign+'_'+z,[.045,surface-.065,.045],[sign*1.28,(surface-.025+.04)/2,z],m);
   box(root,'TransferBrace_'+sign+'_'+z,[.64,.035,.035],[sign*.98,.70,z],m);
  }
 }
 node('Port_input').position.set(-1.05,surface,0);node('Port_output').position.set(1.05,surface,0);b.envelope=[3.1,1.15];
 root.updateMatrixWorld(true);return b;
}
