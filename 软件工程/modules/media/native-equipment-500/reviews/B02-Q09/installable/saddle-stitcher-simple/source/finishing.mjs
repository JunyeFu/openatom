import * as T from 'three';
import {build as chassis} from './chassis.mjs';
import {geometry,clip} from './geometry.mjs';
import {rectangle,bore,slab,casePanels} from './stock.mjs';
import {samples,workY,book,saddle,wire,stack,caseFold,clampAmount,ramp,componentPort,componentSeat,wasteSeat,wastePort} from './batch-spec.mjs';

export function build(spec,simple=false){
 const out=chassis(spec,simple),{root}=out,api=geometry(simple),{group,box,cyl,profile,motor,cable,mesh,m}=api,station=group(root,'BindingStation');
 const tracks=Object.fromEntries(out.animations.map(c=>[c.name,[...c.tracks]]));
 const dyn=(o,key,fn,times=samples)=>{o[key].fromArray(fn('load',0));for(const ph of ['load','auto','unload'])tracks[ph].push(new (key==='quaternion'?T.QuaternionKeyframeTrack:T.VectorKeyframeTrack)(o.name+'.'+key,times,times.flatMap(u=>fn(ph,u))));out.moving_names.push(o.name);};
 const rot=(o,axis,fn,times=samples)=>dyn(o,'quaternion',(ph,u)=>new T.Quaternion().setFromAxisAngle(new T.Vector3(...['x','y','z'].map(a=>Number(a===axis))),fn(ph,u)).toArray(),times);
 const bed=(name,L,W,z=0,top=workY,shape=rectangle(L,W))=>{
  mesh(station,name,slab(shape,.016),[0,top-.016,z],m.al);
  for(const x of[-L/2+.035,L/2-.035]){profile(station,name+'Base_'+x,1.5,[x,.73,0],'z');for(const dz of[-W/2+.025,W/2-.025])profile(station,name+'Leg_'+x+'_'+dz,top-.776,[x,(top-.016+.76)/2,z+dz]);}
 };
 function gantry(name,height=1.28){for(const x of[-.28,.28]){profile(station,name+'Base_'+x,1.5,[x,.73,0],'z');for(const z of[-.26,.26]){profile(station,name+'Post_'+x+'_'+z,height-.76,[x,(height+.76)/2,z]);box(station,name+'Foot_'+x+'_'+z,[.09,.012,.09],[x,.766,z],m.dark);}}box(station,name+'Crosshead',[.64,.06,.58],[0,height,0]);}
 function holdDown(z=0,L=.17,W=.108){
  const h=group(station,'PaperHoldDown');box(h,'HoldPlate',[L,.012,W],[0,.006,z],m.dark);box(h,'HoldSpacer',[.04,.010,.04],[0,.014,z]);box(h,'HoldArm',[.05,.018,.15],[0,.025,z+.055]);cyl(h,'HoldPiston',.010,.40,[0,.234,z+.11]);
  dyn(h,'position',(ph,u)=>[0,workY+.080-.060*clampAmount(ph,u),0]);cyl(station,'HoldCylinder',.025,.18,[0,1.36,z+.11],m.blue);return h;
 }
 function wasteTrays(){
  for(const[name,p,base]of[['PrivateWastePan',wasteSeat(),.302],['PublicWastePan',wastePort(),.024]]){
   box(station,name+'Floor',[.28,.006,.24],[p[0],p[1]-.003,p[2]],m.dark);
   for(const x of[-.14,.14])box(station,name+'SideX_'+x,[.008,.06,.24],[p[0]+x,p[1]+.03,p[2]],m.blue);
   for(const z of[-.12,.12])box(station,name+'SideZ_'+z,[.28,.06,.008],[p[0],p[1]+.03,p[2]+z],m.blue);
   for(const x of[-.10,.10])for(const z of[-.08,.08]){const y=p[1]-.006;profile(station,name+'Leg_'+x+'_'+z,y-base,[p[0]+x,(y+base)/2,p[2]+z],'y',.025);if(base<.03)cyl(station,name+'Foot_'+x+'_'+z,.028,.024,[p[0]+x,.012,p[2]+z],m.rubber);}
  }
  const floor=rectangle(.46,.74),hole=new T.Path();hole.moveTo(-.02,-.35);hole.lineTo(-.02,-.15);hole.lineTo(.22,-.15);hole.lineTo(.22,-.35);hole.closePath();floor.holes.push(hole);
  mesh(station,'WasteTransferTrayFloor',slab(floor,.006),[.10,.634,-.25],m.al);
  for(const[x,z]of[[-.10,.08],[.30,.08],[-.10,-.61]])profile(station,'TransferTrayLeg_'+x+'_'+z,.332,[x,.468,z],'y',.020);
  box(station,'WasteTransferBack',[.46,.05,.006],[.10,.665,-.62],m.blue);box(station,'WasteTransferLeft',[.006,.05,.74],[-.13,.665,-.25],m.blue);box(station,'WasteTransferRight',[.006,.05,.74],[.33,.665,-.25],m.blue);
  for(const x of[-.108,.108])box(station,'VerticalWasteGuide_'+x,[.004,.24,.15],[x,.76,0],m.glass,false);
  group(root,'WasteStaging',wasteSeat());group(root,'Port_waste',wastePort());out.anchors.waste_staging='WasteStaging';
 }
 if(spec.rule==='assemble'){
  group(root,'ComponentSeat',componentSeat());group(root,'Port_component',componentPort());out.anchors.component_seat='ComponentSeat';
  box(station,'ComponentDeck',[.60,.012,.40],[-1,workY-.006,.52]);
  for(const x of[-1.22,-.78]){profile(station,'ComponentBase_'+x,1.5,[x,.73,0],'z');for(const z of[.40,.64])profile(station,'ComponentLeg_'+x+'_'+z,.134,[x,.827,z]);}
 }
 if(spec.kind==='case'){
  bed('CasingBed',.55,.50);const jig=group(station,'CaseFoldingShoes');
  for(const p of casePanels(api,jig)){
   // The backing board is supported by the fixed bed; the other two faces form the press.
   p.o.children.forEach(c=>p.o.remove(c));const W=p.o.name==='CaseBack'?book.boardW:p.o.name==='CaseSpine'?book.spine:book.boardW;
   box(p.o,p.o.name+'FormingPlate',[book.caseL+.02,.012,W],[0,-.006,0],m.blue);
   const t=(ph,u)=>ph==='auto'?(u<.86?u:.68*(1-ramp(u,.86,1))):0;
   dyn(p.o,'position',(ph,u)=>{const a=p.pose('auto',t(ph,u));return[a.p[0],workY+a.p[1],a.p[2]];});
   if(p.o.name!=='CaseBack')rot(p.o,'x',(ph,u)=>p.pose('auto',t(ph,u)).r[1]);
   for(const sign of[-1,1])cyl(p.o,p.o.name+'Hinge_'+sign,.010,.05,[sign*(book.caseL/2+.03),-.006,W/2],m.steel,'x');
  }
  box(station,'CaseCamDrive',[.42,.10,.22],[0,.68,-.03],m.blue);box(station,'CamMount',[.42,.06,.25],[0,.75,-.03]);
  gantry('CasingGlueRail',1.30);for(const z of[-.23,.23])profile(station,'CasingLinearRail_'+z,1.05,[.05,1.28,z],'x');const roller=group(station,'EndpaperGlueRoller');box(roller,'GlueMovingCarriage',[.08,.030,.56],[0,1.265,0]);cyl(roller,'GlueRoll',.012,book.W,[0,workY+book.boardT+book.H+.012,book.caseBlockZ],m.amber,'z');
  for(const z of[book.caseBlockZ-book.W/2-.035,book.caseBlockZ+book.W/2+.035]){profile(roller,'GlueArm_'+z,.32,[0,1.095,z]);box(roller,'GlueBearing_'+z,[.045,.04,.025],[0,.929,z],m.dark);}
  const x=(ph,u)=>ph!=='auto'?.45:u<.08?.45+(-.105-.45)*ramp(u,0,.08):u<.20?-.105+.21*ramp(u,.08,.20):.105+(.45-.105)*ramp(u,.20,.26);
  dyn(roller,'position',(ph,u)=>[x(ph,u),0,0]);out.focus='CaseFoldingShoes';
 }else if(spec.kind==='glue'){
  bed('GlueSideBedLeft',.34,.26,-.15);bed('GlueSideBedRight',.34,.26,.15);
  for(const x of[-.105,.105])box(station,'BookSpineRest_'+x,[.015,.014,.025],[x,workY-.007,0],m.dark);
  for(const sign of[-1,1]){
   const jaw=box(station,'BookClamp_'+sign,[.23,.12,.020],[0,workY+.085,sign*.1175],m.blue),z=(ph,u)=>sign*(.1175-.10*clampAmount(ph,u));dyn(jaw,'position',(ph,u)=>[0,workY+.085,z(ph,u)]);
   box(station,'ClampServo_'+sign,[.08,.08,.12],[0,workY+.09,sign*.27],m.dark);box(station,'ClampMount_'+sign,[.10,.06,.13],[0,workY+.03,sign*.245]);
   const rod=box(station,'ClampRod_'+sign,[.018,.018,1],[0,0,0]);dyn(rod,'position',(ph,u)=>[0,workY+.085,sign*(.21+Math.abs(z(ph,u))+.01)/2]);dyn(rod,'scale',(ph,u)=>[1,1,.21-Math.abs(z(ph,u))-.01]);
  }
  box(station,'HeatedGlueBath',[.24,.065,.080],[0,workY-.065,0],m.dark);box(station,'BathMount',[.28,.04,.10],[0,.73,0]);
  const lift=group(station,'SpineRollLift'),roll=group(lift,'SpineGlueRoller');box(lift,'RollBearingCarrier',[.32,.012,.075],[0,-.030,0]);for(const x of[-.11,.11])box(lift,'RollBearing_'+x,[.025,.040,.045],[x,0,0],m.dark);cyl(roll,'GlueRollShaft',.005,.26,[0,0,0],m.steel,'x');box(lift,'CoatingRollDrive',[.06,.05,.055],[-.155,0,0],m.blue);cyl(roll,'SpineCoatingRoll',.012,.190,[0,0,0],m.amber,'x');cyl(roll,'RollMark',.004,.003,[.097,.009,0],m.dark,'x');
  dyn(lift,'position',(ph,u)=>[0,workY-.018+.006*(ph==='auto'?(ramp(u,.12,.22)-ramp(u,.76,.82)):0),0]);
  rot(roll,'x',(ph,u)=>ph==='auto'?4*Math.PI*ramp(u,.22,.72):ph==='unload'?4*Math.PI:0,[0,.22,.2825,.345,.4075,.47,.5325,.595,.6575,.72,1]);out.focus='SpineGlueRoller';
 }else if(spec.kind==='stitch'){
  bed('SaddleBed',.36,.32);const stand=group(station,'OpenSaddleSupport');
  for(const x of[-.12,.12])box(stand,'SaddlePedestal_'+x,[.025,.10,.055],[x,workY+.05,0]);
  for(const sign of[-1,1]){const W=saddle.W-.017,g=group(stand,'SaddleWing_'+sign,[0,workY+saddle.apex-.012,sign*.012]);box(g,'SaddleRoof_'+sign,[.23,.010,W],[0,-.005,sign*W/2],m.al);g.rotation.x=sign*Math.PI/4;}
  for(const x of[-.09,0,.09])box(stand,'SpineRest_'+x,[.025,.010,.008],[x,workY+saddle.apex-.005,0],m.dark);
  gantry('StitchBridge',1.40);
  for(const x of saddle.staples){
   box(station,'WireMagazine_'+x,[.045,.13,.07],[x,1.26,0],m.blue);
   const head=group(station,'StaplePunch_'+x);box(head,'StapleRam_'+x,[.012,.10,.004],[x,.05,0],m.steel);box(head,'PunchSlide_'+x,[.034,.06,.045],[x,.13,0],m.dark);cyl(head,'ContinuousPunchRod_'+x,.008,.32,[x,.28,0]);cyl(station,'StitchActuator_'+x,.030,.16,[x,1.51,0],m.blue);
   dyn(head,'position',(ph,u)=>[0,workY+saddle.apex+.09-.088*(ph==='auto'?(ramp(u,.22,.35)-ramp(u,.65,.80)):0),0]);
   const anvil=group(station,'ClincherAnvil_'+x,[x,workY+saddle.apex-.002,0]);for(const sign of[-1,1]){const wing=group(anvil,'ClincherWing_'+x+'_'+sign);box(wing,'ClincherFace_'+x+'_'+sign,[.025,.008,.012],[0,-.004,sign*.006],m.dark);wing.rotation.x=sign*Math.PI/4;}box(station,'ClincherPost_'+x,[.025,saddle.apex-.009,.010],[x,workY+(saddle.apex-.009)/2,0]);
  }
  out.focus='OpenSaddleSupport';
 }else if(spec.kind==='wire'){
  bed('WireBookBed',.30,.14,.035,workY+wire.pageLift);gantry('WireCloseBridge',1.30);
  box(station,'LowerWireAnvil',[.215,.012,.026],[0,workY-.006,wire.cz],m.dark);
  const close=group(station,'TwinWireClosingComb');box(close,'WireClosingJaw',[.215,.012,.025],[0,.006,wire.cz],m.blue);cyl(close,'ClosingPiston',.015,.35,[0,.20,wire.cz]);
  for(let i=0;i<13;i++)box(close,'CombGuide_'+i,[.004,.008,.029],[wire.holes[i],.010,wire.cz],m.al);
  dyn(close,'position',(ph,u)=>[0,workY+.10-.079*(ph==='auto'?(ramp(u,.22,.70)-ramp(u,.86,1)):0),0]);cyl(station,'WireCloseCylinder',.04,.20,[0,1.40,wire.cz],m.blue);
  // The open edge overhangs the carrier; a flat support avoids rollers/bearings through the binding.
  for(const o of [...root.children])if(o.name.startsWith('DischargeRoller_')||o.name.startsWith('RollerBearing_')||['OutputDrive','OutputMotorMount'].includes(o.name))root.remove(o);
  box(root,'BoundBookCarrier',[1.08,.012,.34],[1.10,workY+wire.pageLift-.006,.14],m.al);
  for(const x of[.67,1.52]){if(x===.67)profile(root,'CarrierBaseCross',1.5,[x,.73,0],'z');for(const z of[0,.27])profile(root,'CarrierPost_'+x+'_'+z,.136,[x,.828,z]);}
  out.focus='TwinWireClosingComb';
 }else{
  const shape=rectangle(.35,.26);
  if(spec.kind==='drill')for(const x of stack.holes)bore(shape,x,stack.holeZ,.004);
  else for(const sx of[-1,1])for(const sz of[-1,1]){const x=sx*stack.L/2,z=sz*stack.W/2,p=new T.Path();p.moveTo(x-.013,z-.013);p.lineTo(x-.013,z+.013);p.lineTo(x+.013,z+.013);p.lineTo(x+.013,z-.013);p.closePath();shape.holes.push(p);}
  bed('CutThroughAnvil',.35,.26,0,workY,shape);gantry('CuttingBridge',1.27);holdDown(spec.kind==='drill'?.035:0,spec.kind==='drill'?.22:.17,spec.kind==='drill'?.035:.108);
  if(spec.kind==='drill'){
   for(const x of stack.holes){
    const h=group(station,'HollowDrill_'+x),ring=new T.Shape();ring.absarc(0,0,.003,0,Math.PI*2);const hole=new T.Path();hole.absarc(0,0,.0024,0,Math.PI*2,true);ring.holes.push(hole);
    mesh(h,'HollowPaperBit_'+x,slab(ring,.080),[x,0,stack.holeZ],m.steel);cyl(h,'DrillCollet_'+x,.014,.04,[x,.10,stack.holeZ],m.dark);cyl(h,'ColletMark_'+x,.003,.003,[x+.013,.10,stack.holeZ],m.amber,'x');cyl(h,'MovingQuill_'+x,.012,.40,[x,.32,stack.holeZ]);
    dyn(h,'position',(ph,u)=>[0,workY+.14-.142*(ph==='auto'?(ramp(u,.25,.50)-ramp(u,.65,.80)):0),0]);
    // Rotate about each quill's own axis, not the common group origin.
    const rotor=group(h,'RotatingSpindle_'+x,[x,0,stack.holeZ]);for(const child of [...h.children])if(child!==rotor){child.position.x-=x;child.position.z-=stack.holeZ;rotor.add(child);}rot(rotor,'y',(ph,u)=>ph==='auto'?4*Math.PI*ramp(u,.20,.80):ph==='unload'?4*Math.PI:0,[0,.20,.275,.35,.425,.50,.575,.65,.725,.80,1]);
    cyl(station,'FixedQuillGuide_'+x,.026,.18,[x,1.36,stack.holeZ],m.blue);cyl(station,'MotorAdapter_'+x,.04,.02,[x,1.46,stack.holeZ]);cyl(station,'DrillMotor_'+x,.05,.12,[x,1.53,stack.holeZ],m.blue);
   }
   out.focus='HollowDrill_0';
  }else{
   const h=group(station,'FourCornerCuttingHead');box(h,'CornerCarrier',[.30,.025,.22],[0,.060,0],m.blue);
   for(const sx of[-1,1])for(const sz of[-1,1]){
    const R=stack.cornerR,shape=new T.Shape(),pts=[];for(let i=0;i<=12;i++){const a=i*Math.PI/24;pts.push([sx*(stack.L/2-R+(R+.001)*Math.cos(a)),sz*(stack.W/2-R+(R+.001)*Math.sin(a))]);}for(let i=12;i>=0;i--){const a=i*Math.PI/24;pts.push([sx*(stack.L/2-R+(R-.001)*Math.cos(a)),sz*(stack.W/2-R+(R-.001)*Math.sin(a))]);}pts.forEach((p,i)=>i?shape.lineTo(...p):shape.moveTo(...p));shape.closePath();mesh(h,'RadiusKnife_'+sx+'_'+sz,slab(shape,.025),[0,0,0],m.steel);
    const x=sx*(stack.L/2-R/2),z=sz*(stack.W/2-R/2);box(h,'KnifeHolder_'+sx+'_'+sz,[.030,.020,.030],[x,.035,z],m.dark);box(h,'KnifeSpacer_'+sx+'_'+sz,[.025,.003,.025],[x,.0465,z]);cyl(h,'CornerPiston_'+sx+'_'+sz,.010,.35,[x,.2475,z]);cyl(station,'CornerCylinder_'+sx+'_'+sz,.027,.26,[x,1.29,z],m.blue);
   }
   dyn(h,'position',(ph,u)=>[0,workY+.06-.061*(ph==='auto'?(ramp(u,.30,.50)-ramp(u,.65,.80)):0),0]);out.focus='FourCornerCuttingHead';
  }
  wasteTrays();
 }
 out.animations=Object.entries(tracks).map(([name,t])=>clip(name,t));root.updateMatrixWorld(true);return out;
}
