import * as T from 'three';
import{geometry,clip}from './geometry.mjs';
import{specs,feet,boxPose,toolPose,gripping,inputPort,outputPort,pathEnd,ramp}from './batch-spec.mjs';
const V=p=>new T.Vector3(...p),axis=k=>new T.Vector3(...['x','y','z'].map(a=>Number(a===k)));
function joints(spec,ph,u){
 const t=toolPose(spec.id,ph,u),x=t[0]-.15,z=t[2]+.15,r=Math.hypot(x,z),yaw=Math.atan2(-z,x);
 if(spec.kind==='six'){const y=t[1]+.055-.8,L=.62,F=.58,b=-Math.acos((r*r+y*y-L*L-F*F)/(2*L*F)),a=Math.atan2(y,r)-Math.atan2(F*Math.sin(b),L+F*Math.cos(b));return{yaw,a,b,pitch:-a-b,wrist:-yaw};}
 const L=.60,F=.55,b=-Math.acos((r*r-L*L-F*F)/(2*L*F)),a=yaw-Math.atan2(F*Math.sin(b),L+F*Math.cos(b));return{a,b,slide:t[1]-1.50,wrist:-a-b};
}
export function build(spec,simple=false){
 const api=geometry(simple),{group,mesh,box,cyl,profile,bolt,m}=api,root=new T.Group();root.name='HandlingCell';const changes=[],moving=[];
 const pos=(o,fn)=>{changes.push({o,key:'position',fn});moving.push(o.name);},rot=(o,k,fn)=>{changes.push({o,key:'quaternion',fn:(ph,u)=>new T.Quaternion().setFromAxisAngle(axis(k),fn(ph,u)).toArray()});moving.push(o.name);};
 const link=(p,name,L,R,color)=>{const g=new T.CapsuleGeometry(R,L-2*R,simple?4:8,simple?12:24);g.rotateZ(-Math.PI/2);g.translate(L/2,0,0);return mesh(p,name,g,[0,0,0],color);};
 const armLike=['six','scara'].includes(spec.kind);
 for(const[x,z]of feet(spec.id)){cyl(root,'Foot_'+x+'_'+z,.045,.024,[x,.012,z],m.rubber);cyl(root,'Stud_'+x+'_'+z,.012,.052,[x,.05,z]);}
 function table(name,p,L=.48,W=.40){
  box(root,name+'Top',[L,.018,W],[p[0],p[1]-.009,p[2]],m.al);
  for(const dx of[-L/2+.05,L/2-.05])for(const dz of[-W/2+.05,W/2-.05]){box(root,name+'Pad_'+dx+'_'+dz,[.07,.025,.07],[p[0]+dx,.0125,p[2]+dz],m.rubber);profile(root,name+'Leg_'+dx+'_'+dz,p[1]-.043,[p[0]+dx,(p[1]-.018+.025)/2,p[2]+dz],'y',.035);}
 }
 table('InputTable',inputPort(spec.id));
 if(spec.kind!=='vacuum'){
  table('ReceiverTable',[.95,.9,.25],.74,.46);box(root,'ReceiverBelt',[.74,.004,.42],[.95,.898,.25],m.belt,false);
 }else{
  for(const x of[.70,.95,1.20])box(root,'PalletSkid_'+x,[.10,.10,.60],[x,.05,.25],m.edge);
  for(let i=0;i<9;i++)box(root,'PalletDeck_'+i,[.070,.04,.60],[.59+.09*i,.12,.25],m.paper,false);
 }
 let tool;
 if(armLike){
  box(root,'RobotBasePlate',[.50,.040,.50],[.15,.096,-.15],m.dark);for(const[x,z]of feet(spec.id))bolt(root,'BaseFix_'+x+'_'+z,[x,.120,z]);
  if(spec.kind==='six'){
   cyl(root,'Pedestal',.18,.144,[.15,.188,-.15],m.blue);
   const j1=group(root,'J1_BaseYaw',[.15,.26,-.15]);cyl(j1,'SwivelHousing',.16,.20,[0,.10,0],m.blue);cyl(j1,'ShoulderNeck',.105,.34,[0,.37,0],m.al);rot(j1,'y',(ph,u)=>joints(spec,ph,u).yaw);
   const j2=group(j1,'J2_Shoulder',[0,.54,0]);cyl(j2,'ShoulderServo',.11,.22,[0,0,0],m.blue,'z');cyl(j2,'ShoulderCap',.085,.012,[0,0,.116],m.dark,'z');link(j2,'UpperArmShell',.62,.065,m.blue);rot(j2,'z',(ph,u)=>joints(spec,ph,u).a);
   api.cable(j2,'UpperDresspack',[[0,0,.10],[.2,.10,.11],[.5,.10,.10],[.62,0,.10]],.008);
   const j3=group(j2,'J3_Elbow',[.62,0,0]);cyl(j3,'ElbowServo',.10,.20,[0,0,0],m.al,'z');cyl(j3,'ElbowCap',.075,.012,[0,0,.106],m.dark,'z');link(j3,'ForearmShell',.46,.055,m.blue);rot(j3,'z',(ph,u)=>joints(spec,ph,u).b);
   api.cable(j3,'ForearmDresspack',[[0,0,.09],[.18,.09,.09],[.40,.08,.08],[.45,0,.075]],.007);
   const j4=group(j3,'J4_ForearmRoll',[.44,0,0]);cyl(j4,'WristRollHousing',.065,.14,[.07,0,0],m.al,'x');cyl(j4,'WristRollCollar',.071,.012,[.13,0,0],m.dark,'x');
   const j5=group(j4,'J5_WristPitch',[.14,0,0]);cyl(j5,'PitchHousing',.070,.15,[0,0,0],m.blue,'z');rot(j5,'z',(ph,u)=>joints(spec,ph,u).pitch);
   tool=group(j5,'J6_ToolYaw',[0,-.055,0]);cyl(tool,'ToolFlange',.045,.060,[0,.020,0],m.al);rot(tool,'y',(ph,u)=>joints(spec,ph,u).wrist);
  }else{
   cyl(root,'SCARAColumn',.095,1.334,[.15,.783,-.15],m.blue);cyl(root,'SCARAShoulderBase',.15,.15,[.15,1.525,-.15],m.dark);
   const j1=group(root,'J1_SCARAShoulder',[.15,1.60,-.15]);cyl(j1,'ShoulderMotor',.105,.10,[0,0,0],m.blue);link(j1,'HorizontalUpperArm',.60,.055,m.blue);rot(j1,'y',(ph,u)=>joints(spec,ph,u).a);
   const j2=group(j1,'J2_SCARAElbow',[.60,-.10,0]);cyl(j2,'ElbowMotor',.10,.24,[0,.05,0],m.al);link(j2,'HorizontalForearm',.55,.045,m.blue);rot(j2,'y',(ph,u)=>joints(spec,ph,u).b);
   const end=group(j2,'QuillHead',[.55,0,0]);cyl(end,'VerticalQuillHousing',.050,.55,[0,.20,0],m.al);cyl(end,'QuillLowerGuide',.060,.040,[0,-.055,0],m.dark);
   const lift=group(end,'J3_SCARAVertical');pos(lift,(ph,u)=>[0,joints(spec,ph,u).slide,0]);cyl(lift,'QuillRod',.020,.55,[0,.30,0]);
   tool=group(lift,'J4_SCARAToolYaw');rot(tool,'y',(ph,u)=>joints(spec,ph,u).wrist);cyl(tool,'QuillFlange',.045,.030,[0,.015,0],m.al);
   api.cable(j1,'UpperSCARAService',[[0,.07,0],[.20,.10,.06],[.48,.10,.06],[.60,.03,0]],.007);api.cable(j2,'LowerSCARAService',[[0,.10,.06],[.20,.11,.06],[.45,.10,.06],[.55,.08,0]],.006);
  }
 }else{
  for(const x of[-1.35,1.35])for(const z of[-.65,.65])profile(root,'GantryColumn_'+x+'_'+z,1.924,[x,1.038,z]);
  for(const z of[-.65,.65])profile(root,'GantryLongRail_'+z,2.8,[0,2.0,z],'x');for(const x of[-1.35,1.35])profile(root,'GantryEnd_'+x,1.4,[x,2.0,0],'z');
  const travel=group(root,'J1_XTravel');pos(travel,(ph,u)=>[toolPose(spec.id,ph,u)[0],0,0]);for(const z of[-.65,.65])box(travel,'XSlide_'+z,[.14,.05,.10],[0,2.03,z],m.blue);profile(travel,'ZTravelRail',1.40,[0,2.065,0],'z');
  const cross=group(travel,'J2_ZTravel');pos(cross,(ph,u)=>[0,0,toolPose(spec.id,ph,u)[2]]);box(cross,'ZSlide',[.18,.040,.18],[0,2.115,0],m.blue);cyl(cross,'ActuatorMount',.070,.095,[0,2.0475,0],m.dark);cyl(cross,'OuterLiftHousing',.045,.58,[0,1.71,0],m.blue);
  for(const[i,r]of[[1,.032],[2,.022]]){const rod=cyl(cross,'TelescopingStage_'+i,r,.58,[0,0,0],m.steel);pos(rod,(ph,u)=>[0,1.71-(1.39-toolPose(spec.id,ph,u)[1])*i/2,0]);}
  tool=group(cross,'J3_VerticalTool');pos(tool,(ph,u)=>[0,toolPose(spec.id,ph,u)[1],0]);cyl(tool,'LiftToolCoupler',.045,.030,[0,.015,0],m.al);
 }
 if(spec.kind==='vacuum'){
  box(tool,'VacuumManifold',[.22,.03,.16],[0,-.010,0],m.blue);box(tool,'VacuumHeadPlate',[.22,.020,.16],[0,-.025,0],m.al);
  for(const x of[-.07,.07])for(const z of[-.05,.05]){cyl(tool,'SuctionCup_'+x+'_'+z,.024,.030,[x,-.045,z],m.rubber);api.cable(tool,'CupTube_'+x+'_'+z,[[x,-.020,z],[x,.020,z],[0,.025,0]],.003);}
 }else{
  box(tool,'ParallelGripperBody',[.40,.040,.12],[0,-.010,0],m.blue);
  for(const sign of[-1,1]){const jaw=group(tool,'GripperJaw_'+sign);box(jaw,'JawSlide_'+sign,[.06,.035,.10],[0,-.020,0],m.al);box(jaw,'GripPad_'+sign,[.012,.12,.07],[0,-.080,0],m.rubber);pos(jaw,(ph,u)=>[sign*(.165-.039*gripping(ph,u)),0,0]);}
  api.cable(tool,'GripperAirService',[[0,.025,0],[0,.040,-.07],[.12,-.005,-.07]],.0035);
 }
 const cargo=group(tool,'PaperSeat');
 const state=(ph,u)=>{for(const c of changes)c.o[c.key].fromArray(c.fn(ph,u));root.updateMatrixWorld(true);};
 const localCargo=(ph,u)=>{if(ph==='auto'||ph==='load'&&u>=.75||ph==='unload'&&u<=.55)return[0,-.18,0];state(ph,u);return tool.worldToLocal(V(boxPose(spec.id,ph,u))).toArray();};
 const cabinet=group(root,'Controller',[1.40,0,-.95]);box(cabinet,'CabinetFoot',[.40,.040,.40],[0,.020,0],m.dark);box(cabinet,'ControllerHousing',[.32,.95,.32],[0,.515,0],m.blue);box(cabinet,'ControllerDoor',[.29,.90,.012],[0,.51,.166],m.al);box(cabinet,'HMIScreen',[.23,.14,.010],[0,.83,.178],m.lens);cyl(cabinet,'EmergencyStop',.020,.015,[-.09,.69,.183],m.red,'z');
 const label=mesh(cabinet,'OriginalNameplate',new T.PlaneGeometry(.28,.08),[0,.55,.180],m.label);for(let i=0;i<label.geometry.attributes.uv.count;i++)label.geometry.attributes.uv.setY(i,1-label.geometry.attributes.uv.getY(i));profile(cabinet,'SignalStem',.13,[0,1.055,0],'y',.025);for(let i=0;i<3;i++)cyl(cabinet,'Signal_'+i,.020,.030,[0,1.15+i*.035,0],[m.green,m.amber,m.red][i]);
 box(root,'FloorCableChannel',[1.25,.030,.06],[.775,.015,-.95],m.dark,false);box(root,'BaseCableChannel',[.06,.030,.62],[.15,.015,-.67],m.dark,false);if(!armLike)box(root,'ColumnCableChannel',[.06,.030,.32],[1.35,.015,-.80],m.dark,false);
 group(root,'ProductStaging',pathEnd(spec.id));group(root,'Port_input',inputPort(spec.id));group(root,'Port_output',outputPort(spec.id));group(root,'OperatorSeat',[1.2,0,-.55]);
 const times=Array.from(new Set([...Array.from({length:25},(_,i)=>i/24),.55,.75])).sort((a,b)=>a-b),animations=[];
 for(const ph of['load','auto','unload']){const tracks=[];for(const c of changes)tracks.push(new(c.key==='quaternion'?T.QuaternionKeyframeTrack:T.VectorKeyframeTrack)(c.o.name+'.'+c.key,times,times.flatMap(u=>c.fn(ph,u))));tracks.push(new T.VectorKeyframeTrack(cargo.name+'.position',times,times.flatMap(u=>localCargo(ph,u))));animations.push(clip(ph,tracks));}
 state('load',0);cargo.position.fromArray(localCargo('load',0));root.updateMatrixWorld(true);
 return{root,animations,moving_names:[...new Set([...moving,cargo.name])],service_circuits:[],focus:tool.name,cargo:'PaperSeat',anchors:{product_staging:'ProductStaging'},envelope:[3.6,3.6]};
}
