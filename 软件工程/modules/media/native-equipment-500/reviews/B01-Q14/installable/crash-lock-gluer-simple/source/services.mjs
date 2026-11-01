import * as T from 'three';
import{samples,upperGlueCenters,glueHeadX,glueHeadY,sideGlueZ,folderUnitOffset,leftRadius,rightRadius,neutral}from './batch-spec.mjs';import{upperPadY,bottomHookState,diagonalToolState,toolKeyTimes}from './lock-tool-motion.mjs';
const V=a=>new T.Vector3(...a),Y=V([0,1,0]);
// Bounded fillets stay inside the route's corner box, including at the low glue pad.
function route(points){const p=points.map(V),path=new T.CurvePath();let last=p[0];for(let i=1;i<p.length-1;i++){const a=p[i].clone().sub(p[i-1]),b=p[i+1].clone().sub(p[i]),r=Math.min(.012,a.length()/3,b.length()/3),enter=p[i].clone().addScaledVector(a.normalize(),-r),leave=p[i].clone().addScaledVector(b.normalize(),r);path.add(new T.LineCurve3(last,enter));path.add(new T.QuadraticBezierCurve3(enter,p[i],leave));last=leave;}path.add(new T.LineCurve3(last,p.at(-1)));return path;}
export function addGlueServices(ctx,s){
 const{root,box,cyl,mesh,m,tracks,moving,simple}=ctx;
 const fixed=(name,points,r=.002)=>{const path=route(points); // Preserve each straight port entry, even on a long route.
  const sample=(t,kind)=>{const v=t*path.curves.length,i=Math.min(Math.floor(v),path.curves.length-1);return path.curves[i][kind](v-i);};path.getPointAt=t=>sample(t,'getPoint');path.getTangentAt=t=>sample(t,'getTangent');return mesh(root,name,new T.TubeGeometry(path,path.curves.length*8,r,simple?6:10),[0,0,0],m.rubber);};

 const animated=(prefix,points,radius,steps,times=samples)=>{
  const phases=['load','auto','unload'],keys=Object.fromEntries(phases.map(ph=>[ph,ph==='auto'?times:[0,1]])),segments=route(points('load',0)).curves.length*steps,states=Object.fromEntries(phases.map(ph=>[ph,keys[ph].map(u=>{const curve=route(points(ph,u));return Array.from({length:segments},(_,i)=>{const c=curve.curves[Math.floor(i/steps)],j=i%steps,a=c.getPoint(j/steps),b=c.getPoint((j+1)/steps),d=b.clone().sub(a);return{p:a.add(b).multiplyScalar(.5).toArray(),q:new T.Quaternion().setFromUnitVectors(Y,d.clone().normalize()).toArray(),scale:[1,d.length(),1]};});})]));
  for(let i=0;i<segments;i++){const initial=states.load[0][i],o=cyl(root,prefix+i,radius,1,initial.p,m.rubber);o.quaternion.fromArray(initial.q);o.scale.fromArray(initial.scale);for(const ph of phases){const vs=states[ph].map(v=>v[i]);tracks[ph].push(new T.VectorKeyframeTrack(o.name+'.position',keys[ph],vs.flatMap(v=>v.p)),new T.QuaternionKeyframeTrack(o.name+'.quaternion',keys[ph],vs.flatMap(v=>v.q)),new T.VectorKeyframeTrack(o.name+'.scale',keys[ph],vs.flatMap(v=>v.scale)));}moving.push(o.name);}
 };

 // Explicit electrical and compressed-air terminals are separate utilities.
 const cabinet=root.getObjectByName('ElectricalCabinet'),jx=root.children.find(o=>o.name.startsWith('FrontJunctionBox')).position.x,jz=root.children.find(o=>o.name.startsWith('FrontJunctionBox')).position.z;
 function socket(parent,name,at,axis,outer,inner,length){const section=new T.Shape(),hole=new T.Path();section.absarc(0,0,outer,0,Math.PI*2);hole.absarc(0,0,inner,0,Math.PI*2,true);section.holes.push(hole);const g=new T.ExtrudeGeometry(section,{depth:length,bevelEnabled:false,curveSegments:simple?16:40});if(axis==='x')g.rotateY(Math.PI/2);if(axis==='-x')g.rotateY(-Math.PI/2);if(axis==='y')g.rotateX(-Math.PI/2);if(axis==='-y')g.rotateX(Math.PI/2);if(axis==='diag')g.rotateY(Math.PI/4);if(axis==='-z')g.rotateY(Math.PI);return mesh(parent,name,g,at,m.steel);}
 socket(root,'CabinetPowerSocket',[cabinet.position.x+.22,.54,cabinet.position.z],'x',.010,.006,.020);socket(root,'JunctionInputSocket',[jx,.49,jz-.035],'-z',.008,.0045,.010);socket(root,'JunctionBeltSocket',[jx+.050,.49,jz],'x',.008,.0045,.010);socket(root,'BeltMotorPowerSocket',[1.80,.875,-.47],'-z',.006,.0045,.010);
 root.getObjectByName('BeltDrivePower').removeFromParent();fixed('CabinetPowerLink',[[cabinet.position.x+.24,.54,cabinet.position.z],[.65,.54,cabinet.position.z],[.65,.54,.48],[jx,.54,.48],[jx,.49,.48],[jx,.49,jz-.045]],.004);
 fixed('BeltDrivePower',[[jx+.060,.49,jz],[.84,.49,jz],[1.10,.49,jz],[1.10,.49,-.51],[1.10,.875,-.51],[1.80,.875,-.51],[1.80,.875,-.48]],.004);
 const console=root.getObjectByName('OperatorConsole');socket(console,'ConsolePowerSocket',[0,0,-.025],'-z',.006,.0035,.010);fixed('ConsolePower',[[jx,.564,jz],[jx,.60,jz],[.84,.60,jz],[.84,.60,1.27],[1.05,.60,1.27],[1.05,.98,1.27],[1.05,.98,1.315]],.003);
 root.children.find(o=>o.name.startsWith('FrontJunctionMount')).position.y+=.020;
 socket(root,'AirUtilityInlet',[1.02,.10,-.55],'x',.008,.0045,.020);box(root,'AirInletBracket',[.16,.010,.030],[.96,.087,-.55],m.steel);root.getObjectByName('AirSupplyTrunk').removeFromParent();fixed('AirSupplyTrunk',[[1.02,.10,-.55],[.97,.10,-.55],[.97,.10,-.62],[.97,1.50,-.62],[.62,1.50,-.62],[.62,1.50,-.55],[.58,1.50,-.55]],.004);
 // Six fixed barrel ports feed the upper holding/gluing cylinders above the tool bridges.
 const upperCylinders=[...Array.from({length:4},(_,i)=>'BottomRetainer'+i),...upperGlueCenters(s).map(c=>'UpperGlue_'+c.role)];
 upperCylinders.forEach((prefix,i)=>{const barrel=root.getObjectByName(prefix+'_Barrel'),x=barrel.position.x,z=barrel.position.z,port=[x+.018,1.44,z],supplyZ=-.575+i*.010,supplyX=.535+i*.006,colX=x+.065,high=1.78+i*.008;
  socket(root,prefix+'_AirPort',port,'x',.003,.0014,.006);socket(root,prefix+'_ValvePort',[supplyX,1.52,supplyZ],'y',.0025,.0014,.006);
  fixed(prefix+'_AirFeed',[[supplyX,1.526,supplyZ],[supplyX,high,supplyZ],[colX,high,supplyZ],[colX,high,z],[colX,1.44,z],[x+.024,1.44,z]],.0012);
 });
 // Traverse-axis power and a flexible compressed-air branch follow the lower lift.
 socket(root,'GlueAxisPowerPort',[.345,.55,-.36],'x',.006,.0035,.010);
 fixed('GlueAxisPower',[[jx+.060,.49,jz],[.84,.49,jz],[.84,.49,-.44],[.365,.49,-.44],[.365,.55,-.44],[.365,.55,-.36],[.355,.55,-.36]],.003);
 socket(root,'LowerLiftValvePort',[.55,1.50,-.51],'z',.003,.0018,.006);
 fixed('LowerLiftAirSupply',[[.55,1.50,-.504],[.55,1.50,-.490],[.84,1.50,-.490],[.84,1.50,-.45],[.84,.68,-.45]],.0015);
 socket(root.getObjectByName('GlueLiftBarrel'),'LowerLiftAirPort',[0,0,-.010],'-z',.003,.0018,.006);
 animated('LowerLiftAirHose',(ph,u)=>[[.84,.68,-.45],[glueHeadX(ph,u),.68,-.45],[glueHeadX(ph,u),.708,-.43],[glueHeadX(ph,u),.708,-.376]],.0015,4);
 box(root,'FolderPowerDistributionBox',[.040,.040,.040],[jx+.080,.49,jz],m.dark);
 for(const dz of[-.0175,.0175])box(root,'FolderDistributionMount'+dz,[.010,.020,.005],[jx+.055,.49,jz+dz],m.steel);
 // Each side-fold unit has an external moving feed and a coaxial slip-ring takeoff.
 for(const side of[-1,1])for(const sign of[-1,1]){
  const prefix='Folder'+side+'_'+sign,axisX=sign*.40,r=side<0?leftRadius:rightRadius,hingeY=s.surface+neutral+r,unit=root.getObjectByName(prefix+'SideUnit'),swing=root.getObjectByName(prefix+'Swing'),axis=sign>0?'x':'-x';
  const fixedEnd=[sign*.52,1.21,side*1.43];
  const branch=(side>0?2:0)+(sign>0?1:0),px=jx+(sign>0?.090:.070),py=.49+(side>0?.009:-.009),pz=jz+.050+branch*.010,by=.090+branch*.008,outerX=sign>0?1.02:-2.18;socket(root,prefix+'_PowerBranchPort',[px,py,jz+.020],'z',.003,.0022,.006);fixed(prefix+'_FixedPower',[[px,py,jz+.026],[px,py,pz],[px,by,pz],[outerX,by,pz],[outerX,1.45,pz],[outerX,1.45,side*1.43],[sign*.52,1.45,side*1.43],fixedEnd],.002);
  socket(root,prefix+'_FeedMotorPowerPort',[axisX,1.17,side*1.385],side>0?'z':'-z',.004,.0025,.006);
  fixed(prefix+'_FeedMotorPower',[fixedEnd,[axisX,1.21,side*1.43],[axisX,1.17,side*1.43],[axisX,1.17,side*1.391]],.002);
  socket(unit,prefix+'_MovingMotorPowerPort',[sign*.110,hingeY,0],axis,.004,.0025,.006);
  animated(prefix+'_MovingPower',(ph,u)=>{const z=side*folderUnitOffset(side,ph,u);return[fixedEnd,[sign*.56,1.21,side*1.43],[sign*.56,1.21,z],[sign*.56,hingeY,z],[sign*.516,hingeY,z]];},.002,6);
  const ring=new T.Shape(),bore=new T.Path();ring.absarc(0,0,.033,0,Math.PI*2);bore.absarc(0,0,.030,0,Math.PI*2,true);ring.holes.push(bore);const g=new T.ExtrudeGeometry(ring,{depth:.012,bevelEnabled:false,curveSegments:simple?16:40});g.rotateY(Math.PI/2);mesh(swing,prefix+'_RotarySlipRing',g,[sign*.07-.006,0,0],m.dark);
  socket(swing,prefix+'_FingerMotorPowerPort',[sign*.420,-r-.020,side*.15],axis,.003,.0018,.006);
  mesh(swing,prefix+'_RotatingPower',new T.TubeGeometry(route([[sign*.07,-.033,0],[sign*.07,-.043,0],[sign*.07,-.043,side*.19],[sign*.44,-.043,side*.19],[sign*.44,-r-.020,side*.15],[sign*.426,-r-.020,side*.15]]),64,.0015,simple?6:10),[0,0,0],m.rubber);
 }
 // Six electric actuator chains: base supply, lift feed, traverse feed and rotary takeoff.
 const xyz=[...Array.from({length:4},(_,i)=>({name:'BottomHook'+i,index:i,kind:'bottom'})),...[0,2].map(i=>({name:'Diagonal'+i,index:i,kind:'diagonal'}))];
 xyz.forEach(({name,index,kind},k)=>{
  const state=kind==='bottom'?bottomHookState:diagonalToolState,keys=[...new Set([...toolKeyTimes(kind),...samples])].sort((a,b)=>a-b),first=state(index,'load',0),minor=kind==='bottom'&&index%2===0,baseX=kind==='bottom'?-2.00:-1.15,baseZ=kind==='bottom'?(minor?(index===0?-1.10:1.10):first.c.z):(index===0?-.39:.48),out=baseZ<0?-1:1,normal=out<0?'-z':'z',barrelTop=kind==='bottom'?.73:.50,barrelBottom=kind==='bottom'?.13:.18,L=kind==='bottom'?.98:.48,stageZ=v=>minor?baseZ:v.c.z,stageY=v=>minor?1.14:v.c.y+(kind==='bottom'?.10:-.20),vAt=(ph,u)=>state(index,ph,u);
  const followPort=(suffix,at,axis,outer=.004,inner=.0025)=>{const o=socket(root,name+suffix,at(first),axis,outer,inner,.006);for(const ph of['load','auto','unload']){const ts=ph==='auto'?keys:[0,1];tracks[ph].push(new T.VectorKeyframeTrack(o.name+'.position',ts,ts.flatMap(u=>at(vAt(ph,u)))));}moving.push(o.name);return o;};
  const px=cabinet.position.x-.150+k*.060,level=.025+k*.006,ex=-2.10-k*.008,ez=1.40+k*.015;
  socket(root,name+'_CabinetPort',[px,.315,.80],'-y',.004,.0025,.006);socket(root,name+'_BasePowerPort',[baseX,barrelBottom,baseZ],'-y',.004,.0025,.006);
  fixed(name+'_BasePower',[[px,.309,.80],[px,level,.80],[px,level,ez],[ex,level,ez],[ex,level,baseZ+out*.20],[baseX-.060,level,baseZ+out*.20],[baseX-.060,level,baseZ],[baseX,level,baseZ],[baseX,barrelBottom-.006,baseZ]],.002);
  socket(root,name+'_LiftPowerOutlet',[baseX,barrelTop-.040,baseZ+out*.013],normal,.004,.0025,.006);
  followPort('_XBodyPowerIn',v=>[baseX+.050,stageY(v),baseZ+out*.025],normal);
  animated(name+'_LiftPower',(ph,u)=>{const v=vAt(ph,u),y=stageY(v);return[[baseX,barrelTop-.040,baseZ+out*.019],[baseX,barrelTop-.040,baseZ+out*.050],[baseX,y+.040,baseZ+out*.050],[baseX+.050,y+.040,baseZ+out*.050],[baseX+.050,y,baseZ+out*.050],[baseX+.050,y,baseZ+out*.031]];},.002,3,keys);
  followPort('_XBodyPowerOut',v=>[baseX+L-.030,stageY(v),baseZ+out*.025],normal);
  followPort('_CarriagePowerPort',v=>[v.c.x,stageY(v),stageZ(v)+out*.020],normal);
  animated(name+'_TraversePower',(ph,u)=>{const v=vAt(ph,u),y=stageY(v),z=stageZ(v);return[[baseX+L-.030,y,baseZ+out*.031],[baseX+L-.030,y,baseZ+out*.065],[baseX+L-.030,y-.035,baseZ+out*.065],[v.c.x,y-.035,z+out*.065],[v.c.x,y,z+out*.065],[v.c.x,y,z+out*.026]];},.002,3,kind==='diagonal'?[...new Set([...keys,...[[.265,.280],[.420,.490]].flatMap(([a,b])=>Array.from({length:513},(_,i)=>a+(b-a)*i/512))])].sort((a,b)=>a-b):keys);
  const motorY=kind==='bottom'&&!minor?1:-1,motorZ=v=>v.c.z+(minor?out*.010:0);
  followPort('_RotaryPowerPort',v=>[v.c.x,v.c.y+motorY*.018,motorZ(v)],motorY<0?'-y':'y');
  if(minor){
   const slide=root.getObjectByName(name+'_ToolYSlide');
   box(slide,name+'_EnergyChannelOuter',[.002,.430,.032],[-.0465,.915,.026],m.dark);
   for(const dz of[.011,.041])box(slide,name+'_EnergyChannelCover'+dz,[.015,.430,.002],[-.040,.915,dz],m.dark);
   for(const y of[.701,1.129])box(slide,name+'_EnergyChannelEnd'+y,[.015,.002,.032],[-.040,y,.026],m.dark);
   box(slide,name+'_EnergyChannelHead',[.0175,.020,.016],[-.03875,1.14,.020],m.dark);
   const pickup=box(root,name+'_PowerPickup',[.0225,.008,.008],[first.c.x-.03125,first.c.y-.024,baseZ+.020],m.dark);
   for(const ph of['load','auto','unload']){const ts=ph==='auto'?keys:[0,1];tracks[ph].push(new T.VectorKeyframeTrack(pickup.name+'.position',ts,ts.flatMap(u=>{const v=vAt(ph,u);return[v.c.x-.03125,v.c.y-.024,baseZ+.020];})));}moving.push(pickup.name);
  }
  if(minor){followPort('_ToolYPowerPort',v=>[v.c.x,.640,baseZ+.015],'-y');animated(name+'_ToolYPower',(ph,u)=>{const v=vAt(ph,u);return[[v.c.x,1.14,baseZ+out*.026],[v.c.x,1.14,baseZ+out*.045],[v.c.x-.060,1.14,baseZ+out*.045],[v.c.x-.060,.625,baseZ+out*.045],[v.c.x-.060,.625,baseZ+.015],[v.c.x,.625,baseZ+.015],[v.c.x,.634,baseZ+.015]];},.002,3,keys);}

 });
 // The working cup and its support both have an actual drain passage.
 const floor=root.getObjectByName('WorkingGlueCupFloor'),ring=new T.Shape(),drain=new T.Path();ring.absarc(0,0,.012,0,Math.PI*2);drain.absarc(0,0,.0022,0,Math.PI*2,true);ring.holes.push(drain);const fg=new T.ExtrudeGeometry(ring,{depth:.006,bevelEnabled:false,curveSegments:simple?16:40});fg.rotateX(-Math.PI/2);floor.geometry=fg;floor.position.set(.50,1.094,-.75);
 const support=root.getObjectByName('WorkingGlueCupArm'),plate=new T.Shape(),bore=new T.Path();plate.moveTo(.475,-.78);plate.lineTo(.975,-.78);plate.lineTo(.975,-.72);plate.lineTo(.475,-.72);plate.closePath();bore.absarc(.50,-.75,.0022,0,Math.PI*2,true);plate.holes.push(bore);const pg=new T.ExtrudeGeometry(plate,{depth:.020,bevelEnabled:false,curveSegments:24});pg.rotateX(Math.PI/2);support.geometry=pg;support.position.set(0,1.094,0);
 // Replace the old Catmull route: its unconstrained corner dipped into moving hardware.
 root.getObjectByName('SupplyGlueLine').removeFromParent();fixed('SupplyGlueLine',[[.50,1.10,-.75],[.50,1.04,-.75],[.30,1.04,-.75],[.30,1.42,-.75],[.30,1.42,-1.48],[1.02,1.42,-1.48],[1.02,.70,-1.48],[.40,.70,-1.48],[.40,.70,-.25],[.40,.56,-.25]]);
 const hx=-.10666666666666666;box(root,'UpperAdhesiveHeader',[.040,.020,.40],[hx,1.65,0],m.dark);for(const z of[-.18,.18])box(root,'UpperHeaderStand'+z,[.030,.040,.030],[hx,1.62,z],m.steel);
 fixed('UpperHeaderSupply',[[.42,.55,-.25],[.46,.55,-.25],[1.04,.55,-.25],[1.04,1.70,-.25],[hx,1.70,-.25],[hx,1.65,-.24],[hx,1.65,-.20]]);
 for(const center of upperGlueCenters(s)){
  const prefix='UpperGlue_'+center.role,carrier=root.getObjectByName(prefix),section=new T.Shape(),hole=new T.Path();section.absarc(0,0,.002,0,Math.PI*2);hole.absarc(0,0,.0008,0,Math.PI*2,true);section.holes.push(hole);const g=new T.ExtrudeGeometry(section,{depth:.006,bevelEnabled:false,curveSegments:simple?12:32});g.rotateY(Math.PI/2);mesh(carrier,prefix+'_FeedConnector',g,[.008,.008,0],m.steel);
  fixed(prefix+'_HeaderStub',[[hx+.020,1.65,center.point[2]],[-.0623,1.65,center.point[2]]],.0012);mesh(root,prefix+'_HeaderElbow',new T.TorusGeometry(.004,.0012,simple?6:10,16,Math.PI/2),[-.0623,1.646,center.point[2]],m.rubber);
  animated(prefix+'_SupplyHose',(ph,u)=>[[-.0583,1.646,center.point[2]],[-.0583,upperPadY(ph,u)+.008,center.point[2]],[center.point[0]+.014,upperPadY(ph,u)+.008,center.point[2]]],.0012,8);
 }
 const lower=root.getObjectByName('BottomGlueHead'),fitting=new T.Shape(),fb=new T.Path();fitting.absarc(0,0,.003,0,Math.PI*2);fb.absarc(0,0,.0022,0,Math.PI*2,true);fitting.holes.push(fb);const fg2=new T.ExtrudeGeometry(fitting,{depth:.006,bevelEnabled:false,curveSegments:simple?16:40});fg2.rotateY(Math.PI/2);mesh(lower,'LowerGlueFeedConnector',fg2,[.0175,-.021,0],m.steel);
 fixed('LowerGlueRiser',[[.40,.55,-.23],[.40,.55,-.19],[.40,.83,-.19]]);const gs=new T.Shape(),gh=new T.Path();gs.absarc(0,0,.008,0,Math.PI*2);gh.absarc(0,0,.0025,0,Math.PI*2,true);gs.holes.push(gh);const gg=new T.ExtrudeGeometry(gs,{depth:.012,bevelEnabled:false,curveSegments:24});gg.rotateX(-Math.PI/2);mesh(root,'LowerGlueGrommet',gg,[.40,.800,-.19],m.rubber);
 animated('MovingGlueHose',(ph,u)=>[[.40,.83,-.19],[.40,.86,-.19],[glueHeadX(ph,u)+.050,glueHeadY(ph,u)-.021,sideGlueZ],[glueHeadX(ph,u)+.0235,glueHeadY(ph,u)-.021,sideGlueZ]],.002,4);

}
