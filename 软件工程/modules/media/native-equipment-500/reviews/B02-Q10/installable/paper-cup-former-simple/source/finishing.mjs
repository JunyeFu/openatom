import * as T from 'three';
import{build as chassis}from './chassis.mjs';
import{geometry,clip}from './geometry.mjs';
import{vessel,plate}from './stock.mjs';
import{workY,componentPort,componentSeat,ramp}from './batch-spec.mjs';
export function build(spec,simple=false){
 const out=chassis(spec,simple),{root}=out,api=geometry(simple),{group,box,cyl,profile,mesh,m}=api,station=group(root,'VesselFormingStation');
 const tracks=Object.fromEntries(out.animations.map(c=>[c.name,[...c.tracks]]));
 // Narrow cup/bowl bottoms need continuous support between the driven rollers.
 box(root,'ContinuousOutputBelt',[1.08,.004,.62],[1.10,workY-.002,0],m.belt,false);
 const position=(o,fn,times=[0,.15,.30,.45,.65,.80,1])=>{o.position.fromArray(fn('load',0));for(const ph of ['load','auto','unload'])tracks[ph].push(new T.VectorKeyframeTrack(o.name+'.position',times,times.flatMap(u=>fn(ph,u))));out.moving_names.push(o.name);};
 // The supported transfer nest carries the same input to the forming position.
 const cargo=root.getObjectByName('PaperSeat'),nest=group(cargo,'TransferNest');
 box(nest,'NestPlate',[.35,.012,.38],[0,-.006,0],m.dark);for(const z of[-.205,.205])box(nest,'NestGuide_'+z,[.36,.025,.012],[0,.006,z],m.blue);
 for(const x of[-1.15,0,.85]){profile(station,'TransferBase_'+x,1.5,[x,.73,0],'z');for(const z of[-.16,.16])box(station,'TransferRailFoot_'+x+'_'+z,[.08,.025,.06],[x,.7725,z],m.dark);}
 for(const z of[-.16,.16]){profile(station,'TransferLinearRail_'+z,2.5,[-.1,.80,z],'x',.030);box(cargo,'TransferSlider_'+z,[.075,.030,.06],[0,-.070,z],m.blue);box(nest,'TelescopingNestPost_'+z,[.040,.061,.040],[0,-.0425,z],m.steel);}
 api.motor(station,'NestFeedServo',[-1.22,.82,-.35],'x');
 if(spec.kind==='plate'){
  position(nest,(ph,u)=>[0,spec.H*(ph==='load'?1:ph==='auto'?1-ramp(u,.30,.45):0),0],[0,.30,.45,1]);
  box(root,'RaisedInputSupport',[.76,spec.H,.68],[-1,workY+spec.H/2,0],m.dark);
 }
 for(const x of[-.28,.28]){profile(station,'BaseCross_'+x,1.5,[x,.73,0],'z');for(const z of[-.25,.25]){profile(station,'FormingPost_'+x+'_'+z,.69,[x,1.105,z]);box(station,'FormingShoe_'+x+'_'+z,[.10,.012,.10],[x,.766,z],m.dark);}}
 box(station,'FormingCrosshead',[.64,.06,.58],[0,1.48,0],m.blue);
 const head=group(station,'HeatedFormingMandrel'),clear=workY+spec.H+.085,low=workY+.001;
 mesh(head,spec.kind==='plate'?'PlatePressDie':'TaperedFormingMandrel',spec.kind==='plate'?plate(spec,true):vessel(spec,true),[0,0,0],m.al);
 cyl(head,'MandrelMount',.045,.03,[0,spec.H+.04,0],m.dark);const rodLength=1.64-(low+spec.H+.055);cyl(head,'ContinuousPressRod',.012,rodLength,[0,spec.H+.055+rodLength/2,0]);
 cyl(station,'PressCylinder',.040,.38,[0,1.67,0],m.blue);
 position(head,(ph,u)=>[0,clear-(clear-low)*(ph==='auto'?(ramp(u,.10,.30)-ramp(u,.65,.80)):0)+(spec.kind==='plate'&&ph==='auto'?spec.H*(1-ramp(u,.30,.45))*ramp(u,.10,.30):0),0]);
 for(const sign of[-1,1]){
  const jaw=group(station,'HeatSealSlide_'+sign),height=spec.kind==='plate'?.025:spec.H*.4,contact=spec.kind==='plate'?spec.rt+.010:(spec.rb+spec.rt)/2+.012;
  box(jaw,'HeatedJaw_'+sign,[.018,height,.025],[0,height/2,0],m.dark);box(jaw,'JawCarriage_'+sign,[.06,.018,.06],[0,-.009,0],m.blue);
  const riser=spec.H*.30+.0245;box(jaw,'JawSlideRiser_'+sign,[.04,riser,.04],[0,-.018-riser/2,0],m.steel);
  profile(station,'JawGuide_'+sign,.28,[sign*.13,workY-.055,0],'x',.025);
  position(jaw,(ph,u)=>[sign*(.18-(.18-contact)*(ph==='auto'?(ramp(u,.45,.55)-ramp(u,.70,.80)):0)),workY+spec.H*.30,0]);
 }
 profile(station,'JawGuideSupportBridge',.56,[0,.79,0],'x');for(const sign of[-1,1])box(station,'JawGuideShoe_'+sign,[.08,.0125,.06],[sign*.13,.82625,0],m.dark);
 if(spec.rule==='assemble'){
  group(root,'Port_component',componentPort());group(root,'ComponentSeat',componentSeat());out.anchors.component_seat='ComponentSeat';
  box(station,'BottomFeedDeck',[.60,.012,.28],[-1,workY-.006,.52],m.al);
  for(const x of[-1.22,-.78])profile(station,'BottomFeedLeg_'+x,.134,[x,.827,.52]);
  profile(station,'BottomDeckCross',1.5,[-1,.73,0],'z');
 }
 out.focus='HeatedFormingMandrel';out.animations=Object.entries(tracks).map(([name,t])=>clip(name,t));root.updateMatrixWorld(true);return out;
}
