import * as T from 'three';
import {RoundedBoxGeometry} from 'three/addons/geometries/RoundedBoxGeometry.js';
import {build as base} from './kit.mjs';
import {specs,radius,unwindAngle,rewindAngle,paper,suctionTimes,suctionPath,foldHeadX,foldTimes,foldEntryY} from './batch-spec.mjs';
export function build(spec,simple=false){
 const b=base(spec,simple),root=b.root,s=specs[spec.id],oldMaterials=new Map();root.traverse(o=>{if(o.isMesh)oldMaterials.set(o.material.name,o.material);});
 const m={al:oldMaterials.get('Anodised aluminium'),steel:oldMaterials.get('Brushed stainless'),blue:oldMaterials.get('Powder coat blue'),dark:oldMaterials.get('Cast iron graphite'),rubber:oldMaterials.get('Rubber'),green:oldMaterials.get('Polyurethane belt')};
 root.children.filter(o=>o.name.startsWith('Station_')).forEach(o=>o.removeFromParent());
 const moving=[],tracks={load:[],auto:[],unload:[]};
 const group=(p,name,pos=[0,0,0])=>{const o=new T.Group();o.name=name;o.position.fromArray(pos);p.add(o);return o;};
 const mesh=(p,name,geo,pos,material)=>{const o=new T.Mesh(geo,material);o.name=name;o.position.fromArray(pos);o.castShadow=o.receiveShadow=true;p.add(o);return o;};
 const box=(p,name,size,pos,mat=m.al)=>mesh(p,name,simple?new T.BoxGeometry(...size):new RoundedBoxGeometry(...size,2,Math.min(.003,...size.map(v=>v*.15))),pos,mat);
 const cyl=(p,name,r,h,pos,mat=m.al,axis='y',r2=r)=>{const o=mesh(p,name,new T.CylinderGeometry(r2,r,h,simple?12:32),pos,mat);if(axis==='z')o.rotation.x=Math.PI/2;if(axis==='x')o.rotation.z=Math.PI/2;return o;};
 const cable=(p,name,points,r=.004)=>mesh(p,name,new T.TubeGeometry(new T.CatmullRomCurve3(points.map(v=>new T.Vector3(...v))),24,r,simple?5:8,false),[0,0,0],m.rubber);
 const bolts=(p,name,x,y,z)=>{if(!simple){cyl(p,name+'Washer',.010,.002,[x,y,z],m.al);cyl(p,name+'Head',.006,.007,[x,y+.004,z],m.dark);}};
 const pos=(phase,o,times,values)=>tracks[phase].push(new T.VectorKeyframeTrack(o.name+'.position',times,values.flat()));
 const scale=(phase,o,times,values)=>tracks[phase].push(new T.VectorKeyframeTrack(o.name+'.scale',times,values.flat()));
 const hold=(phase,o)=>pos(phase,o,[0,1],[o.position.toArray(),o.position.toArray()]);
 const spin=(o,angle)=>{const times=Array.from({length:513},(_,i)=>i/512);tracks.auto.push(new T.QuaternionKeyframeTrack(o.name+'.quaternion',times,times.flatMap(u=>new T.Quaternion().setFromAxisAngle(new T.Vector3(0,0,1),angle(u)).toArray())));for(const phase of ['load','unload']){const q=new T.Quaternion().setFromAxisAngle(new T.Vector3(0,0,1),angle(phase==='load'?0:1)).toArray();tracks[phase].push(new T.QuaternionKeyframeTrack(o.name+'.quaternion',[0,1],[...q,...q]));}moving.push(o.name);};
 const stand=(name,x,z,top,w=.065)=>{box(root,name+'Column',[w,top-.80,w],[x,(top+.80)/2,z]);box(root,name+'Foot',[.13,.012,.13],[x,.806,z],m.dark);bolts(root,name,x+.04,.814,z);};
 function motor(name,x,y,z){const g=group(root,name,[x,y,z]);cyl(g,name+'Body',.055,.15,[0,0,-.01],m.blue,'z');cyl(g,name+'Fan',.060,.018,[0,0,-.094],m.dark,'z');box(g,name+'Gearbox',[.13,.13,.065],[0,0,.0975]);cyl(g,name+'Coupling',.027,.08,[0,0,.16],m.dark,'z');box(g,name+'Mount',[.17,.018,.20],[0,-.064,0],m.dark);box(root,name+'Stand',[.17,y-.873,.20],[x,(.80+y-.073)/2,z]);cable(root,name+'Power',[[.54,.54,-.43],[.64,.78,-.50],[x,.82,z],[x,y-.055,z]]);return g;}
 function spool(name,x,parts=[{z:0,width:.50}]){
  for(const z of [-.34,.34]){stand(name+'BearingStand'+z,x,z,1.23);box(root,name+'Bearing'+z,[.12,.12,.08],[x,1.27,z],m.dark);}
  const g=group(root,name,[x,1.27,0]);cyl(g,name+'Shaft',.025,.80,[0,0,0],m.steel,'z');
  for(const [i,part]of parts.entries()){
   const shape=new T.Shape();shape.absarc(0,0,.04,0,2*Math.PI,false);const hole=new T.Path();hole.absarc(0,0,.026,0,2*Math.PI,true);shape.holes.push(hole);
   const geo=new T.ExtrudeGeometry(shape,{depth:part.width,bevelEnabled:false,curveSegments:simple?8:24});geo.translate(0,0,-part.width/2);
   mesh(g,name+'ReusableCore'+i,geo,[0,0,part.z],m.rubber);
   for(const sign of [-1,1])if(!(parts.length>1&&part.z*sign<0))cyl(g,name+'ClampCone'+i+'_'+sign,.058,.02,[0,0,part.z+sign*(part.width/2+.01)],m.dark,'z',.04);
  }
  if(parts.length>1)cyl(g,name+'SharedCenterCollar',.058,.02,[0,0,0],m.dark,'z');
  motor(name+'Drive',x,1.27,-.55);return g;
 }
 function roller(name,x,y,r=.05,width=.58){
  for(const z of [-.34,.34]){stand(name+'Stand'+z,x,z,y-.045);box(root,name+'Bearing'+z,[.09,.09,.07],[x,y,z],m.dark);}
  const g=group(root,name,[x,y,0]);cyl(g,name+'Body',r,width,[0,0,0],m.steel,'z');cyl(g,name+'Axis',.014,.76,[0,0,0],m.steel,'z');return g;
 }
 function table(name,x,surface,width=.80){
  box(root,name+'Top',[width,.025,.72],[x,surface-.0125,0],m.steel);
  for(const dx of [-.30,.30])for(const z of [-.28,.28]){cyl(root,name+'Foot'+dx+'_'+z,.038,.02,[x+dx,.01,z],m.rubber);box(root,name+'Leg'+dx+'_'+z,[.045,surface-.045,.045],[x+dx,(surface-.005)/2,z]);}
  for(const z of [-.28,.28])box(root,name+'CrossBrace'+z,[.64,.035,.035],[x,surface*.55,z]);
 }
 function saddle(name,point){const [x,y]=point,dx=.04,r=.025,dy=Math.sqrt((radius()+r)**2-dx**2),top=y-dy-r;table(name,x,top);for(const d of [-dx,dx])cyl(root,name+'SupportRoll'+d,r,.56,[x+d,y-dy,0],m.rubber,'z');}
 const cargo=group(root,'PaperBatchDatum',s.datum);b.cargo=cargo.name;
 b.anchors={};
 if(s.mode==='sheet'){
  table('InputDatumTable',-.525,.9035,.75);table('ExternalInputTable',s.input[0],s.inputSurface);
  const belt=box(root,'DeliveryBelt',[.72,.014,.58],[.525,.894,0],m.green);
  for(const x of [.20,.85])spin(roller('DeliveryRoll'+x,x,.861,.033,.60),u=>paper.sheetLength*u/.033);
  for(const x of [-.90,.90])for(const z of [-.40,.40])stand('VacuumGantry'+x+'_'+z,x,z,1.51,.065);
  for(const z of [-.40,.40]){box(root,'GantryBeam'+z,[1.94,.07,.08],[0,1.52,z]);cyl(root,'TraverseRail'+z,.012,1.80,[0,1.48,z],m.steel,'x');}
  const carriage=group(root,'VacuumTraverse',[-.525,1.42,0]);box(carriage,'TraverseCrosshead',[.16,.05,.90],[0,.06,0]);
  for(const z of [-.40,.40])box(carriage,'TraverseGuideBlock'+z,[.13,.05,.075],[0,.06,z],m.dark);
  cyl(root,'TraverseLeadScrew',.009,1.95,[0,1.48,-.48],m.steel,'x');
  box(carriage,'TraverseNutBracket',[.09,.05,.12],[0,.06,-.45]);cyl(carriage,'TraverseLeadNut',.020,.07,[0,.06,-.48],m.dark,'x');
  const traverseMotor=group(root,'TraverseMotor',[-1.02,1.48,-.48]);cyl(traverseMotor,'TraverseMotorBody',.045,.13,[0,0,0],m.blue,'x');box(root,'TraverseMotorMount',[.18,.04,.14],[-1.02,1.415,-.48]);
  cable(root,'TraverseMotorPower',[[.54,.54,-.43],[.64,.80,-.50],[-.90,1.43,-.50],[-1.02,1.44,-.48]]);
  const head=group(carriage,'SuctionHead',[0,-.3475,0]);box(head,'VacuumBeam',[.58,.05,.08],[0,0,0]);
  for(const x of [-.20,0,.20]){cyl(head,'CupStem'+x,.008,.13,[x,-.08,0]);cyl(head,'BellowsCup'+x,.035,.025,[x,-.155,0],m.rubber);}
  cyl(carriage,'HeadPneumaticCylinder',.035,.46,[0,.10,0]);cyl(head,'HeadPiston',.012,.34,[0,.195,0],m.steel);
  box(carriage,'CylinderMount',[.11,.02,.11],[0,.10,0],m.dark);
  cable(carriage,'HeadCylinderAir',[[.14,.10,-.14],[.08,.10,-.07],[.03,.10,0]],.004);
  box(head,'VacuumManifold',[.20,.035,.25],[.10,.04,-.085]);box(carriage,'VacuumValveBlock',[.09,.04,.09],[.14,.10,-.14],m.dark);
  const hose=cyl(carriage,'FlexibleVacuumService',.009,1,[.14,-.103,-.14],m.rubber);
  const seat=group(head,'SuctionPaperSeat',[0,-.1675,0]);b.cargo=seat.name;
  for(const phase of ['load','unload']){
   const back=phase==='load'?0:1;const times=[0,1];
   if(!back){hold(phase,carriage);hold(phase,head);}
   else{pos(phase,carriage,[0,.2,.8,1],[[.525,1.42,0],[.525,1.42,0],[-.525,1.42,0],[-.525,1.42,0]]);pos(phase,head,[0,.2,.8,1],[[0,-.35,0],[0,-.1975,0],[0,-.1975,0],[0,-.3475,0]]);}
  }
  pos('auto',carriage,suctionTimes,suctionPath.map(p=>[p[0],1.42,0]));pos('auto',head,suctionTimes,suctionPath.map(p=>[0,p[1]-1.42,0]));
  for(const phase of ['load','auto','unload']){
   const times=phase==='auto'?suctionTimes:phase==='unload'?[0,.2,.8,1]:[0,1];
   const ys=phase==='auto'?suctionPath.map(p=>p[1]):phase==='unload'?[1.0700,1.2225,1.2225,1.0725]:[1.0725,1.0725];
   pos(phase,hose,times,ys.map(y=>[.14,(1.5+y+.0575)/2-1.42,-.14]));scale(phase,hose,times,ys.map(y=>[1,1.5-y-.0575,1]));
  }
  cable(root,'VacuumFixedSupply',[[.54,.54,-.43],[.64,.80,-.50],[-.90,1.45,-.50],[-.90,1.52,-.14]]);
  // A supported telescoping pneumatic service follows the traverse; endpoints remain connected.
  const traverseService=cyl(root,'TraversePneumaticService',.009,1,[-.6425,1.52,-.14],m.rubber,'x');
  for(const phase of ['load','auto','unload']){
   const times=phase==='auto'?suctionTimes:phase==='unload'?[0,.2,.8,1]:[0,1];
   const xs=phase==='auto'?suctionPath.map(p=>p[0]):phase==='unload'?[.525,.525,-.525,-.525]:[-.525,-.525];
   pos(phase,traverseService,times,xs.map(x=>[(x+.14-.90)/2,1.52,-.14]));
   scale(phase,traverseService,times,xs.map(x=>[1,x+.14+.90,1]));
  }
  moving.push(traverseService.name);
  moving.push(carriage.name,head.name,hose.name);b.focus=head.name;
 }else{
  if(s.inputAxis!=null)spin(spool('InputReel',s.inputAxis),unwindAngle);
  if(s.outputAxis!=null)spin(spool('OutputReel',s.outputAxis,s.mode==='slit'?[{z:-.135,width:.25},{z:.135,width:.25}]:undefined),rewindAngle);
  for(const x of s.mode==='unwind'?[-.25,0]:s.mode==='rewind'?[0,.25]:[-.25,.25])spin(roller('WebGuide'+x,x,.95),u=>paper.length*u/.05);
  if(s.mode==='unwind'||s.mode==='rewind'){
   const x=s.mode==='unwind'?.525:-.525;box(root,'WebFoldCollectionBed',[.72,.025,.62],[x,.8885,0],m.steel);
   for(const dx of [-.28,.28])for(const z of [-.25,.25])stand('FoldBed'+dx+'_'+z,x+dx,z,.876);
   // Rack-driven traversing slot lays down (or picks up) the same continuous web.
   for(const dx of [-.34,.34])for(const z of [-.32,.32])stand('FoldGantry'+dx+'_'+z,x+dx,z,1.20);
   for(const z of [-.32,.32]){box(root,'FoldRailBeam'+z,[.78,.04,.06],[x,1.20,z]);cyl(root,'FoldLinearRail'+z,.010,.76,[x,1.17,z],m.steel,'x');}
   box(root,'FoldRackBase',[.76,.02,.035],[x,1.1345,-.35],m.dark);
   const pitch=2*Math.PI*.020/24;
   if(!simple)for(let i=0;i<Math.floor(.72/pitch);i++)box(root,'FoldRackTooth'+i,[pitch*.46,.009,.025],[x-.36+(i+.5)*pitch,1.149,-.35],m.steel);
   const carriage=group(root,'FoldTraverse',[foldHeadX(s,0),1.15,0]);box(carriage,'FoldCrosshead',[.10,.04,.72],[0,0,0]);
   for(const z of [-.32,.32])box(carriage,'FoldRailBlock'+z,[.08,.04,.05],[0,.02,z],m.dark);
   const head=group(carriage,'FoldGuideSlot',[0,foldEntryY-.003-1.15,0]);
   for(const z of [-.27,.27])box(carriage,'FoldHeadHanger'+z,[.018,1.13-(foldEntryY-.006),.018],[0,(1.13+foldEntryY-.006)/2-1.15,z]);
   for(const sign of [-1,1])box(head,'FoldGuideLip'+sign,[.008,.006,.56],[sign*(.004+paper.thickness/2),0,0],m.steel);
   const pinion=group(carriage,'FoldPinion',[0,.02,-.35]);cyl(pinion,'FoldPinionHub',simple?.0255:.0165,.020,[0,0,0],m.dark,'z');
   if(!simple)for(let i=0;i<24;i++){const a=i*Math.PI/12,o=box(pinion,'FoldPinionTooth'+i,[.004,.007,.018],[Math.sin(a)*.020,Math.cos(a)*.020,0],m.steel);o.rotation.z=-a;}
   cyl(carriage,'FoldTraverseMotor',.045,.10,[0,.02,-.48],m.blue,'z');cyl(carriage,'FoldDriveCoupling',.015,.08,[0,.02,-.395],m.dark,'z');box(carriage,'FoldMotorBracket',[.14,.03,.20],[0,-.035,-.45]);
   cable(root,'FoldFixedPower',[[.54,.54,-.43],[.64,.80,-.50],[x-.40,1.25,-.48]]);
   cable(carriage,'FoldMotorPower',[[0,.02,-.53],[0,.10,-.48]]);
   const service=box(root,'FoldMovingPower',[1,.018,.018],[0,1.25,-.48],m.rubber);
   for(const phase of ['load','auto','unload']){
    const times=phase==='auto'?foldTimes:[0,1],xs=times.map(u=>foldHeadX(s,phase==='auto'?u:phase==='load'?0:1));
    pos(phase,carriage,times,xs.map(x=>[x,1.15,0]));pos(phase,service,times,xs.map(xx=>[(xx+x-.40)/2,1.25,-.48]));scale(phase,service,times,xs.map(xx=>[xx-x+.40,1,1]));
    tracks[phase].push(new T.QuaternionKeyframeTrack(pinion.name+'.quaternion',times,xs.flatMap(xx=>new T.Quaternion().setFromAxisAngle(new T.Vector3(0,0,1),-(xx-(x-.36))/.020).toArray())));
   }
   moving.push(carriage.name,pinion.name,service.name);
  }
  if(s.mode==='slit'){
   const knife=roller('SlitKnifeRotor',0,1.07,.025,.62);cyl(knife,'CenterSlittingDisc',.07,.003,[0,0,0],m.dark,'z');spin(knife,u=>paper.length*u/.07);
   spin(roller('SlitCounterRoll',0,.95,.05,.62),u=>paper.length*u/.05);
   for(const z of [-.135,.135]){const anchor=group(root,'NarrowRollSeat'+z,[s.outputAxis,1.27,z]);b.anchors[z<0?'out_left':'out_right']=anchor.name;}
  }
  if(s.mode==='crosscut'){
   const r=paper.sheetLength/(2*Math.PI),knife=roller('CrossKnifeRotor',0,1+r+.006,r-.004,.56);box(knife,'FullWidthCrossKnife',[.008,.012,.54],[0,-r,0],m.dark);spin(knife,u=>2*Math.PI*60*u);motor('CrossKnifeDrive',0,1+r+.006,-.55);
   spin(roller('CrossKnifeAnvil',0,.95,.05,.62),u=>paper.length*u/.05);
   box(root,'SheetDeliveryBed',[.80,.025,.64],[1.05,.8885,0],m.green);
   for(const dx of [-.32,.32])for(const z of [-.26,.26])stand('SheetBed'+dx+'_'+z,1.05+dx,z,.876);
   for(const z of [-.27,.27])box(root,'SheetSideFence'+z,[.76,.05,.012],[1.05,.926,z],m.blue);
   box(root,'SheetEndStop',[.012,.05,.55],[1.306,.926,0],m.blue);
   const anchor=group(root,'FinishedSheetStack',[1.05,.901,0]);b.anchors.out_stack=anchor.name;
  }
  if(s.inputShape==='roll')saddle('InputRollStand',s.input);else table('InputWebStand',s.input[0],s.input[1]);
  if(s.outputShape==='roll')saddle('OutputRollStand',s.output);
  b.focus=s.mode==='crosscut'?'CrossKnifeRotor':s.mode==='slit'?'SlitKnifeRotor':s.mode==='unwind'?'InputReel':'OutputReel';
 }
 if(s.outputShape!=='roll')table('ExternalOutputTable',s.output[0],s.outputSurface??s.output[1]);
 for(const [id,point]of [['input',s.input],['output',s.output]])root.getObjectByName('Port_'+id).position.fromArray(point);
 b.animations=Object.entries(tracks).map(([phase,list])=>new T.AnimationClip(phase,1,list));b.moving_names=moving;b.envelope=[s.mode==='sheet'||s.mode==='unwind'||s.mode==='rewind'?3.9:4.96,1.40];
 root.updateMatrixWorld(true);return b;
}
