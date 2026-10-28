import * as T from 'three';
import {build as chassis} from './chassis.mjs';
import {geometry,clip} from './geometry.mjs';
import {panelPlan} from './stock.mjs';
import {samples,workY,mandrelX,ramp} from './batch-spec.mjs';

export function build(spec,simple=false){
 const out=chassis(spec,simple),api=geometry(simple),{root}=out,{group,box,cyl,profile,motor,cable,m}=api;
 const tracks=Object.fromEntries(out.animations.map(c=>[c.name,[...c.tracks]]));
 const dyn=(o,key,fn,times=samples)=>{o[key].fromArray(fn('load',0));for(const ph of ['load','auto','unload'])tracks[ph].push(new (key==='quaternion'?T.QuaternionKeyframeTrack:T.VectorKeyframeTrack)(o.name+'.'+key,times,times.flatMap(u=>fn(ph,u))));out.moving_names.push(o.name);};
 const rotation=(o,axis,fn,times=samples)=>dyn(o,'quaternion',(ph,u)=>new T.Quaternion().setFromAxisAngle(new T.Vector3(axis==='x'?1:0,0,axis==='z'?1:0),fn(ph,u)).toArray(),times);
 const machine=group(root,'FoldingAndSealingStation');
 const width=spec.kind==='wrap'?spec.W:.50;
 box(machine,'WorkDeck',[.56,.016,width],[0,workY-.008,0],m.al);
 for(const x of[-.24,.24])profile(machine,'DeckBaseCross_'+x,1.50,[x,.73,0],'z');
 for(const x of[-.24,.24])for(const z of[-width/2+.04,width/2-.04])profile(machine,'DeckPost_'+x+'_'+z,.134,[x,.827,z]);
 box(machine,'CamDriveEnclosure',[.46,.12,.40],[0,.66,0],m.blue);motor(machine,'FoldingCamMotor',[-.39,.67,0],'x');
 for(const x of[-.20,.20])box(machine,'GearHanger_'+x,[.04,.10,.20],[x,.73,0]);
 box(machine,'MotorShelf',[.31,.025,.25],[-.32,.59,0]);
 if(spec.id==='paper-bag-tuber'){
  for(const x of[-1.20,-.80])box(machine,'UnwindBearing_'+x,[.045,.045,.055],[x,workY+.019,.26],m.dark);
  cyl(machine,'ReusableUnwindSpindle',.014,.70,[-1.10,workY+.019,.26],m.steel,'x');
  const wheel=group(machine,'UnwindChuck',[-1.445,workY+.019,.26]);cyl(wheel,'UnwindBrakeWheel',.045,.012,[0,0,0],m.blue,'x');box(wheel,'UnwindRotationMark',[.014,.009,.009],[0,.027,0],m.amber,false);
  rotation(wheel,'x',(ph,u)=>ph==='load'?12*Math.PI*ramp(u,0,.35):12*Math.PI,[...Array.from({length:25},(_,i)=>.35*i/24),1]);
 }
 const plan=panelPlan(spec.id);
 for(const p of plan.filter(p=>p.pose('auto',.5).axis&&!p.name.startsWith('CornerEar'))){
  const shoe=group(machine,'FoldShoe_'+p.name);
  box(shoe,p.name+'SupportPlate',[p.L+.020,.012,Math.max(.006,p.W-.004)],[0,-.006,0],m.blue);
  for(const sign of[-1,1]){
   if(p.pose('auto',.5).axis==='x')cyl(shoe,p.name+'Hinge_'+sign,.010,.045,[sign*(p.L/2+.025),-.007,-p.W/2],m.steel,'x');
   else cyl(shoe,p.name+'Hinge_'+sign,.010,.045,[-p.L/2,-.007,sign*(p.W/2+.025)],m.steel,'z');
  }
  const phase=(ph,u)=>ph==='auto'?(u<.86?u:.70*(1-ramp(u,.86,1))):0;
  dyn(shoe,'position',(ph,u)=>{const pose=p.pose('auto',phase(ph,u));return[pose.p[0],workY+pose.p[1],pose.p[2]];});
  rotation(shoe,p.pose('auto',.5).axis,(ph,u)=>p.pose('auto',phase(ph,u)).a);
 }
 if(spec.kind==='wrap'){
  const die=group(machine,'RetractingTubeMandrel');box(die,'HollowFormingMandrel',[spec.L*.90,spec.H-.002,spec.W-.002],[0,workY+spec.H/2,0],m.al);
  dyn(die,'position',(ph,u)=>[mandrelX(ph,u),0,0]);
  box(machine,'MandrelActuatorBody',[.40,.09,.09],[-.90,workY+spec.H/2,0],m.blue);
  box(machine,'MandrelStand',[.15,.205,.25],[-.90,.8025,0]);
  profile(machine,'MandrelBaseCross',1.50,[-.90,.73,0],'z');
  const rod=box(machine,'MandrelPiston',[1,.023,.023],[0,0,0],m.steel);
  dyn(rod,'position',(ph,u)=>[(-.70+mandrelX(ph,u)-spec.L*.45)/2,workY+spec.H/2,0]);
  dyn(rod,'scale',(ph,u)=>[mandrelX(ph,u)-spec.L*.45+.70,1,1]);
  out.focus='RetractingTubeMandrel';
 }else if(spec.kind==='heatseal'){
  for(const x of[-.40,.40])profile(machine,'HeatPressBaseCross_'+x,1.50,[x,.73,0],'z');
  for(const x of[-.40,.40])for(const z of[-.32,.32])profile(machine,'HeatPressPost_'+x+'_'+z,.64,[x,1.05,z]);
  box(machine,'HeatPressCrosshead',[.88,.06,.72],[0,1.39,0]);cyl(machine,'HeatPressCylinder',.045,.23,[0,1.535,0],m.blue);
  const head=group(machine,'ThreeEdgeHeatSealHead');box(head,'HeaterCarrier',[spec.L+.08,.035,spec.W+.08],[0,.048,0],m.blue);
  for(const z of[-spec.W/2,spec.W/2])box(head,'LongHeatJaw_'+z,[spec.L,.025,.010],[0,.0125,z],m.steel);
  box(head,'EndHeatJaw',[.010,.025,spec.W],[spec.L/2,.0125,0],m.steel);
  cyl(head,'MovingPressRod',.015,.28,[0,.20,0]);
  dyn(head,'position',(ph,u)=>[0,workY+.13-.129*(ph==='auto'?(ramp(u,.22,.70)-ramp(u,.86,1)):0),0]);
  box(machine,'HeaterServiceDuct',[.045,.35,.045],[.15,1.35,-.35],m.dark);cable(head,'MovingHeaterLead',[[.12,.05,-.10],[.15,.22,-.35],[.15,.34,-.35]]);cable(machine,'HeaterFeed',[[.15,1.50,-.35],[.22,1.55,-.40],[.40,1.10,-.52]]);
  out.focus='ThreeEdgeHeatSealHead';
 }else{
  // Hinged forming shoes provide the pressing contact, then reopen before unloading.
  box(machine,'AdhesiveFixtureFence',[.45,.025,.025],[0,workY+.0125,-.32],m.dark);
  out.focus=plan.some(p=>p.name==='BottomFold')?'FoldShoe_BottomFold':spec.kind==='lid'?'FoldShoe_LidEnd_1':'FoldShoe_EnvelopeBack';
 }
 if(spec.auxiliary){
  const edge=spec.kind==='wrap'?spec.W*1.5+2*spec.H+spec.lap/2:spec.kind==='envelope'?spec.W:.02;
  const targetZ=Math.min(.60,edge);
  for(const x of[-.58,.58])profile(machine,'GlueBaseCross_'+x,1.50,[x,.73,0],'z');
  for(const x of[-.58,.58])for(const z of[-.67,.67])profile(machine,'GlueGantryPost_'+x+'_'+z,.48,[x,1.01,z]);
  for(const x of[-.58,.58])for(const z of[-.67,.67])box(machine,'GlueShoe_'+x+'_'+z,[.09,.015,.09],[x,.7675,z],m.dark);
  for(const z of[-.67,.67])profile(machine,'GlueXGuide_'+z,1.25,[0,1.28,z],'x');
  const bridge=group(machine,'GlueMovingBridge');profile(bridge,'GlueZGuide',1.40,[0,1.28,0],'z');
  const gx=(ph,u)=>ph==='auto'?(u<.10?-spec.L/2+spec.L*ramp(u,0,.10):spec.L/2+(-.42-spec.L/2)*ramp(u,.12,.20)):-.42;
  dyn(bridge,'position',(ph,u)=>[gx(ph,u),0,0]);
  const gun=group(bridge,'MeteringGlueHead');box(gun,'GlueCarriage',[.07,.05,.06],[0,1.28,0],m.blue);
  const needle=group(gun,'LiftableGlueNeedle');cyl(needle,'GlueNeedle',.003,.20,[0,workY+.102,0],m.steel);
  dyn(gun,'position',(ph,u)=>[0,0,ph==='auto'?targetZ+(-.62-targetZ)*ramp(u,.12,.20):-.62]);
  dyn(needle,'position',(ph,u)=>[0,.22*(ph==='auto'?ramp(u,.10,.18):1),0]);
  box(gun,'NeedleGuide',[.025,.18,.025],[0,1.19,0],m.al);cable(machine,'GlueServiceBundle',[[-1,workY+.03,-.52],[-.68,1.1,-.60],[-.58,1.28,-.67]]);
 }
 out.animations=Object.entries(tracks).map(([name,t])=>clip(name,t));root.updateMatrixWorld(true);return out;
}
