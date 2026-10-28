import * as T from 'three';
import{build as chassis}from './chassis.mjs';
import{geometry,clip}from './geometry.mjs';
import{rectangle,slab}from './stock.mjs';
import{workY,stagingX,componentPort,componentSeat,wasteSeat,wastePort,seat,pileLift,ramp}from './batch-spec.mjs';
export function build(spec,simple=false){
 const out=chassis(spec,simple),{root}=out,api=geometry(simple),{group,box,cyl,profile,mesh,m}=api,station=group(root,'PackagingStation'),tracks=Object.fromEntries(out.animations.map(c=>[c.name,[...c.tracks]]));
 const dyn=(o,key,fn,times=[0,.10,.15,.25,.35,.45,.55,.65,.80,.90,1])=>{o[key].fromArray(fn('load',0));for(const ph of['load','auto','unload'])tracks[ph].push(new(key==='quaternion'?T.QuaternionKeyframeTrack:T.VectorKeyframeTrack)(o.name+'.'+key,times,times.flatMap(u=>fn(ph,u))));out.moving_names.push(o.name);};
 const pos=(o,fn,times)=>dyn(o,'position',fn,times),rot=(o,axis,fn,times)=>dyn(o,'quaternion',(ph,u)=>new T.Quaternion().setFromAxisAngle(new T.Vector3(...['x','y','z'].map(k=>Number(k===axis))),fn(ph,u)).toArray(),times);
 box(root,'ContinuousOutputBelt',[spec.kind==='pallet'?1.94:1.08,.004,.62],[spec.kind==='pallet'?.67:1.10,workY-.002,0],m.belt,false);
 const cargo=root.getObjectByName('PaperSeat');
 if(spec.kind!=='pallet'){
  const shape=rectangle(.40,.38);
  if(spec.kind==='slot')for(const x of[-.05,.05]){const p=new T.Path();p.moveTo(x-.0035,-.005);p.lineTo(x-.0035,.065);p.lineTo(x+.0035,.065);p.lineTo(x+.0035,-.005);p.closePath();shape.holes.push(p);}
  mesh(cargo,'TransferNest',slab(shape,.012),[0,-.012,0],m.dark);
  for(const x of[-1.15,0,.85]){profile(station,'TransferBase_'+x,1.5,[x,.73,0],'z');for(const z of[-.16,.16])box(station,'TransferRailFoot_'+x+'_'+z,[.08,.025,.06],[x,.7725,z],m.dark);}
  for(const z of[-.16,.16]){profile(station,'TransferRail_'+z,2.5,[-.10,.80,z],'x',.030);box(cargo,'TransferSlider_'+z,[.075,.030,.06],[0,-.070,z],m.blue);box(cargo,'NestPost_'+z,[.04,.043,.04],[0,-.0335,z],m.steel);}
  api.motor(station,'NestFeedServo',[-1.22,.82,-.35],'x');
 }else{
  box(station,'PileInputNest',[.40,.016,.34],[-.45,workY-.008,0],m.dark);
  for(const x of[-.62,-.28]){profile(station,'PileInputBase_'+x,1.5,[x,.73,0],'z');for(const z of[-.13,.13])profile(station,'PileInputPost_'+x+'_'+z,.124,[x,.822,z]);}
 }
 if(spec.rule==='assemble'){
  group(root,'Port_component',componentPort());group(root,'ComponentSeat',componentSeat());out.anchors.component_seat='ComponentSeat';
  box(station,'ComponentFeedDeck',[1.55,.012,.66],[-.525,workY-.006,.33],m.al);
  for(const x of[-1.22,.12]){profile(station,'ComponentDeckBase_'+x,1.5,[x,.73,0],'z');for(const z of[.15,.55])profile(station,'ComponentDeckLeg_'+x+'_'+z,.128,[x,.824,z]);}
 }
 function gantry(){for(const x of[-.28,.28]){profile(station,'PressBase_'+x,1.5,[x,.73,0],'z');for(const z of[-.26,.26])profile(station,'PressPost_'+x+'_'+z,.66,[x,1.09,z]);}box(station,'PressCrosshead',[.64,.06,.58],[0,1.45,0],m.blue);}
 function press(name,high,low,top,stroke=(u)=>ramp(u,.15,.55)-ramp(u,.65,.85)){
  gantry();const g=group(station,name),bottom=top+.02,len=1.80-(workY+low+bottom);cyl(g,name+'Coupler',.022,.025,[0,top+.0125,0],m.dark);cyl(g,name+'Rod',.012,len,[0,bottom+len/2,0]);cyl(station,name+'Cylinder',.040,.62,[0,1.76,0],m.blue);
  pos(g,(ph,u)=>[0,workY+high-(high-low)*(ph==='auto'?stroke(u):0),0]);return g;
 }
 if(spec.kind==='handle'){
  const h=press('HandleBacker',.24,.005,.16);box(h,'InternalBagBacker',[.16,.16,.06],[0,.08,0],m.al);
  for(const sign of[-1,1]){const jaw=group(station,'HandlePress_'+sign);box(jaw,'HandlePressureBar_'+sign,[.16,.035,.012],[0,0,0],m.dark);box(jaw,'HandleJawRiser_'+sign,[.04,.15,.04],[0,-.09,0]);profile(station,'HandleJawGuide_'+sign,.20,[0,workY+.01,sign*.10],'z',.030);pos(jaw,(ph,u)=>[0,workY+.1875,sign*(.11-.061*(ph==='auto'?(ramp(u,.35,.50)-ramp(u,.65,.80)):0))]);profile(station,'HandleGuidePost_'+sign,.135,[0,.8275,sign*.10]);}
 }else if(spec.kind==='rope'){
  for(const sign of[-1,1]){const g=group(station,'TwistBearing_'+sign);box(g,'BearingFoot_'+sign,[.10,.025,.12],[sign*.20,.9125,0],m.blue);profile(station,'BearingSupport_'+sign,.14,[sign*.20,.83,0]);profile(station,'BearingBase_'+sign,1.5,[sign*.20,.73,0],'z');
   const spindle=group(g,'RotatingChuck_'+sign,[sign*.19,workY+.0345,0]);cyl(spindle,'ChuckBody_'+sign,.026,.08,[0,0,0],m.steel,'x');box(spindle,'ChuckIndex_'+sign,[.025,.008,.008],[0,.029,0],m.amber);
   rot(spindle,'x',(ph,u)=>sign*(ph==='auto'?u:ph==='unload'?1:0)*8*Math.PI*2,Array.from({length:33},(_,i)=>i/32));
   pos(g,(ph,u)=>[sign*.05*(ph==='unload'?1:ph==='auto'?ramp(u,.85,1):0),0,0]);
  }
 }else if(spec.kind==='slot'){
  const h=press('TwinSlottingHead',.080,-.002,.040);box(h,'BladeCrossbar',[.18,.020,.07],[0,.045,.02],m.dark);for(const x of[-.05,.05]){box(h,'SlotKnife_'+x,[.003,.035,.04],[x,.0175,.02]);box(h,'KnifeClamp_'+x,[.025,.015,.05],[x,.0325,.02],m.blue);}
  group(root,'Port_waste',wastePort());group(root,'WasteSeat',wasteSeat());out.anchors.waste_staging='WasteSeat';
  for(const[name,p,base]of[['PrivateWastePan',wasteSeat(),.302],['PublicWastePan',wastePort(),.024]]){box(station,name,[.28,.006,.24],[p[0],p[1]-.003,p[2]],m.dark);for(const z of[-.12,.12])box(station,name+'Wall_'+z,[.28,.04,.008],[p[0],p[1]+.02,p[2]+z],m.blue);for(const x of[-.10,.10])for(const z of[-.08,.08]){profile(station,name+'Leg_'+x+'_'+z,p[1]-.006-base,[p[0]+x,(p[1]-.006+base)/2,p[2]+z],'y',.020);if(base===.024)cyl(station,name+'Foot_'+x+'_'+z,.022,.024,[p[0]+x,.012,p[2]+z],m.rubber);}}
 }else if(spec.kind==='divider'){
  const h=press('DividerInsertionHead',.20,.08,.025);box(h,'DividerCrossbar',[.18,.020,.34],[0,.035,0],m.blue);for(const x of[-.05,.05])box(h,'CrossDividerPusher_'+x,[.008,.030,.30],[x,.015,0],m.dark);
 }else if(spec.kind==='baler'){
  const h=press('BaleCompressionPlaten',.28,.12,.025);box(h,'CompressionPlate',[.29,.025,.21],[0,.0125,0],m.dark);
  for(const x of[-.16,.16])box(station,'BalerSide_'+x,[.020,.27,.25],[x,workY+.135,0],m.blue);box(station,'BalerBack',[.34,.27,.020],[0,workY+.135,-.135],m.blue);
  const door=group(station,'BalerFrontDoor',[-.17,workY,.135]);box(door,'DoorPanel',[.34,.25,.016],[.17,.125,0],m.blue);box(door,'DoorHandle',[.02,.08,.025],[.29,.14,.025],m.dark);rot(door,'y',(ph,u)=>-Math.PI/2*(ph==='auto'?1-ramp(u,.05,.15)+ramp(u,.85,1):1));
 }else{
  for(const x of[-1.15,.25]){profile(station,'PalletizerBase_'+x,1.5,[x,.73,0],'z');for(const z of[-.30,.30])profile(station,'PalletizerPost_'+x+'_'+z,.79,[x,1.155,z]);}
  for(const x of[-1.15,.25])profile(station,'PalletizerTopBridge_'+x,.66,[x,1.55,0],'z');
  for(const z of[-.20,.20])profile(station,'PalletizerRail_'+z,1.60,[-.45,1.55,z],'x');
  const carriage=group(station,'PalletizerCarriage');box(carriage,'TravelCarriage',[.13,.050,.46],[0,1.575,0],m.blue);cyl(carriage,'LiftCylinder',.035,.32,[0,1.65,0],m.blue);pos(carriage,(ph,u)=>[ph==='unload'?-.45*u:seat(spec.id,ph,u)[0],0,0]);
  const grip=group(station,'PileClamp');box(grip,'GripBridge',[.14,.025,.31],[0,0,0],m.blue);cyl(grip,'GripRodCoupler',.022,.020,[0,.010,0],m.dark);cyl(grip,'ContinuousLiftRod',.012,.55,[0,.295,0]);
  pos(grip,(ph,u)=>[ph==='unload'?-.45*u:seat(spec.id,ph,u)[0],workY+(ph==='unload'?.20:.06+pileLift(ph,u)),0]);
  for(const sign of[-1,1]){const jaw=group(grip,'PileSideClamp_'+sign);box(jaw,'PileGripPad_'+sign,[.24,.060,.012],[0,-.040,0],m.rubber);box(jaw,'GripSlide_'+sign,[.08,.040,.035],[0,-.015,0],m.steel);pos(jaw,(ph,u)=>[0,0,sign*(.18-.064*(ph==='auto'?(ramp(u,.02,.10)-ramp(u,.80,.90)):0))]);}
 }
 out.focus=spec.kind==='pallet'?'PileClamp':spec.kind==='rope'?'RotatingChuck_1':spec.kind==='handle'?'HandleBacker':spec.kind==='slot'?'TwinSlottingHead':spec.kind==='divider'?'DividerInsertionHead':'BaleCompressionPlaten';out.animations=Object.entries(tracks).map(([name,t])=>clip(name,t));root.updateMatrixWorld(true);return out;
}
