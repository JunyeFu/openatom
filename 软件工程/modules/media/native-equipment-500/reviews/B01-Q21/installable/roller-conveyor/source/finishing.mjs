import * as T from 'three';
import{geometry,clip}from './geometry.mjs';
import{feet,seat,heading,pathEnd,inputPort,outputPort,ramp}from './batch-spec.mjs';
export function build(spec,simple=false){
 const a=geometry(simple),{group,box,cyl,profile,mesh,m}=a,root=new T.Group();root.name='Conveyor';const tracks={load:[],auto:[],unload:[]},moving=[];
 const dyn=(o,key,fn,times=[0,.15,.30,.45,.60,.80,1])=>{o[key].fromArray(fn('load',0));for(const ph of['load','auto','unload'])tracks[ph].push(new(key==='quaternion'?T.QuaternionKeyframeTrack:T.VectorKeyframeTrack)(o.name+'.'+key,times,times.flatMap(u=>fn(ph,u))));moving.push(o.name);};
 const pos=(o,fn,times)=>dyn(o,'position',fn,times),rot=(o,axis,fn,times)=>dyn(o,'quaternion',(ph,u)=>new T.Quaternion().setFromAxisAngle(new T.Vector3(...['x','y','z'].map(k=>Number(k===axis))),fn(ph,u)).toArray(),times);
 const progress=(ph,u)=>ph==='load'?.2*u:ph==='auto'?.2+.6*u:.8+.2*u;
 const rotor=(o,axis)=>rot(o,axis,(ph,u)=>progress(ph,u)*Math.PI*8,Array.from({length:13},(_,i)=>i/12));
 for(const[x,z,H]of feet(spec.id)){cyl(root,'Foot_'+x+'_'+z,.055,.024,[x,.012,z],m.rubber);cyl(root,'Stud_'+x+'_'+z,.012,.052,[x,.05,z]);profile(root,'Leg_'+x+'_'+z,H-.176,[x,(H-.10+.076)/2,z]);}
 function bed(p,name,L,W,H,x,z,axis='x'){
  const dims=axis==='x'?[L,.088,W]:[W,.088,L];box(p,name+'Pan',dims,[x,H-.056,z],m.al);
  box(p,name+'Belt',axis==='x'?[L,.012,W-.08]:[W-.08,.012,L],[x,H-.006,z],m.belt,false);
 }
 function cross(x,z,H,axis='z',len=.58){profile(root,'FrameCross_'+x+'_'+z,len,[x,H-.13,z],axis);}
 function rollerBed(x0,x1,H){
  const len=x1-x0,mid=(x1+x0)/2;
  for(const z of[-.27,.27])box(root,'RollerSide_'+x0+'_'+z,[len,.10,.04],[mid,H-.05,z],m.al);
  for(let i=0;i<=Math.floor((len-.12)/.10);i++){
   const x=x0+.06+i*.10,g=group(root,'RollerAxis_'+x0+'_'+i,[x,H-.035,0]);cyl(g,'DrivenRoller_'+x0+'_'+i,.035,.50,[0,0,0],m.steel,'z');box(g,'RollerMark_'+x0+'_'+i,[.010,.010,.008],[0,.035,.253],m.amber,false);rotor(g,'z');
   for(const z of[-.27,.27])box(root,'RollerBearing_'+x0+'_'+i+'_'+z,[.045,.06,.04],[x,H-.035,z],m.dark);
  }
 }
 if(spec.kind==='belt'){
  bed(root,'Main',2.6,.58,.9,.2,0);for(const x of[-.90,1.25])cross(x,0,.9);
 }else if(spec.kind==='roller'){
  rollerBed(-1.1,1.5,.9);for(const x of[-.90,1.25])cross(x,0,.9);
 }else if(spec.kind==='curve'){
  bed(root,'CurveInfeed',.50,.58,.9,-.85,0);bed(root,'CurveOutfeed',.90,.58,.9,0,1.05,'z');
  function arc(r0,r1,H,name,material,base){const shape=new T.Shape();shape.absarc(-.6,.6,r1,-Math.PI/2,0,false);shape.absarc(-.6,.6,r0,0,-Math.PI/2,true);shape.closePath();const geo=new T.ExtrudeGeometry(shape,{depth:H,bevelEnabled:false,curveSegments:simple?12:24});geo.rotateX(Math.PI/2);geo.translate(0,H,0);mesh(root,name,geo,[0,base,0],material);}
  arc(.31,.89,.088,'CurveSteelPan',m.al,.8);arc(.35,.85,.012,'CurvedBelt',m.belt,.888);
  for(const radius of[.32,.88]){const points=Array.from({length:17},(_,i)=>{const t=-Math.PI/2+Math.PI/2*i/16;return[-.6+radius*Math.cos(t),.97,.6+radius*Math.sin(t)];});a.cable(root,'CurvedGuide_'+radius,points,.008).material=m.steel;for(const i of[0,8,16]){const p=points[i];cyl(root,'GuidePost_'+radius+'_'+i,.009,.075,[p[0],.9325,p[2]]);}}
  for(const z of[.6,1.3])cross(0,z,.9,'x');cross(-.9,0,.9);
 }else if(spec.kind==='transfer'){
  rollerBed(-1.1,.3,.9);bed(root,'CrossOutfeed',1.15,.58,.94,0,.925,'z');
  const lift=group(root,'PopUpCrossBelts');pos(lift,(ph,u)=>[0,ph==='load'?.88:ph==='auto'?.88+.06*ramp(u,.10,.30):.94,0]);
  for(const x of[-.15,-.05,.05,.15]){
   box(lift,'CrossBelt_'+x,[.022,.028,.65],[x,-.014,.075],m.belt,false);
   for(const z of[-.15,.15])box(lift,'CrossBeltStem_'+x+'_'+z,[.018,.050,.025],[x,-.053,z],m.steel);
  }
  for(const z of[-.15,.15]){box(lift,'LiftCrosshead_'+z,[.44,.022,.06],[0,-.085,z],m.blue);cyl(lift,'PopUpRod_'+z,.014,.26,[0,-.15,z]);cyl(root,'PopUpCylinder_'+z,.032,.16,[0,.64,z],m.blue);box(root,'PopUpCylinderFoot_'+z,[.12,.04,.12],[0,.54,z],m.dark);}
  profile(root,'PopUpLowerCross',.65,[0,.50,0],'z');for(const z of[-.30,.30])profile(root,'LowerLong_'+z,1.4,[-.4,.50,z],'x');for(const x of[-.90,.30])cross(x,0,.9);cross(0,1.30,.94,'x');
 }else{
  bed(root,'LowInfeed',.75,.58,.45,-.725,0);bed(root,'HighOutfeed',1.15,.58,.9,.925,0);
  const lift=group(root,'LiftCarriage');pos(lift,(ph,u)=>[0,ph==='load'?.45:ph==='auto'?.45+.45*u:.9,0]);bed(lift,'LiftDeck',.70,.58,0,0,0);
  for(const x of[-.22,.22])for(const z of[-.20,.20]){box(root,'GuideBase_'+x+'_'+z,[.10,.08,.10],[x,.04,z],m.dark);cyl(root,'LiftGuide_'+x+'_'+z,.012,.95,[x,.555,z]);cyl(lift,'GuideBushing_'+x+'_'+z,.025,.05,[x,-.05,z],m.al);}
  box(root,'LiftRamFoot',[.24,.05,.24],[0,.025,0],m.dark);cyl(root,'LiftCylinder',.055,.27,[0,.185,0],m.blue);
  const ram=cyl(root,'TelescopingLiftRod',.025,1,[0,0,0]);pos(ram,(ph,u)=>{const H=ph==='load'?.45:ph==='auto'?.45+.45*u:.9;return[0,(.14+H-.1)/2,0];});dyn(ram,'scale',(ph,u)=>[1,(ph==='load'?.45:ph==='auto'?.45+.45*u:.9)-.24,1]);cross(-.9,0,.45);cross(1.25,0,.9);
 }
 const panelX=spec.kind==='curve'||spec.kind==='transfer'?-.85:.95,panelH=spec.kind==='lift'?.9:.9;
 box(root,'ElectricalCabinet',[.27,.25,.16],[panelX,panelH-.25,-.36],m.blue);box(root,'CabinetDoor',[.24,.22,.010],[panelX,panelH-.25,-.445],m.al);box(root,'CabinetBracket',[.08,.16,.08],[panelX,panelH-.10,-.32],m.dark);
 profile(root,'HMIStand',.34,[panelX,panelH+.08,-.32],'y',.025);box(root,'HMIHousing',[.25,.18,.055],[panelX,panelH+.30,-.32],m.dark);box(root,'HMIScreen',[.19,.10,.006],[panelX,panelH+.32,-.289],m.lens);cyl(root,'EmergencyStop',.018,.014,[panelX-.07,panelH+.25,-.283],m.red,'z');
 profile(root,'BeaconStem',.30,[panelX+.22,panelH+.1,.27],'y',.025);for(let i=0;i<3;i++)cyl(root,'Signal_'+i,.022,.03,[panelX+.22,panelH+.28+i*.035,.27],[m.green,m.amber,m.red][i]);
 a.motor(root,'MainDrive',[panelX,panelH-.04,-.455]);box(root,'DriveMount',[.18,.04,.25],[panelX,panelH-.11,-.40],m.dark);a.cable(root,'ControlServiceBundle',[[panelX,panelH+.28,-.35],[panelX,panelH-.1,-.35],[panelX+.1,panelH-.2,-.40]],.005);
 const indicator=group(root,'DriveIndicator',[panelX,panelH-.04,.315]);cyl(indicator,'DriveEndCap',.025,.012,[0,0,0],m.dark,'z');box(indicator,'DriveIndex',[.009,.009,.008],[0,.022,.010],m.amber);rotor(indicator,'z');
 const label=mesh(root,'OriginalNameplate',new T.PlaneGeometry(.30,.08),[panelX,panelH-.05,.302],m.label);for(let i=0;i<label.geometry.attributes.uv.count;i++)label.geometry.attributes.uv.setY(i,1-label.geometry.attributes.uv.getY(i));box(root,'NameplateBacking',[.32,.10,.014],[panelX,panelH-.05,.293],m.dark);
 const cargo=group(root,'PaperSeat');pos(cargo,(ph,u)=>seat(spec.id,ph,u),[0,.10,.20,.30,.40,.50,.60,.70,.80,.90,1]);rot(cargo,'y',(ph,u)=>heading(spec.id,ph,u),[0,.25,.50,.75,1]);
 const staging=group(root,'ProductStaging',pathEnd(spec.id));staging.rotation.y=heading(spec.id,'unload',1);group(root,'Port_input',inputPort(spec.id));group(root,'Port_output',outputPort(spec.id));group(root,'OperatorSeat',[.9,0,.8]);
 root.updateMatrixWorld(true);return{root,animations:Object.entries(tracks).map(([name,t])=>clip(name,t)),moving_names:moving,service_circuits:[],focus:'PaperSeat',cargo:'PaperSeat',anchors:{product_staging:'ProductStaging'},envelope:[3.6,['curve','transfer'].includes(spec.kind)?3.6:1.6]};
}
