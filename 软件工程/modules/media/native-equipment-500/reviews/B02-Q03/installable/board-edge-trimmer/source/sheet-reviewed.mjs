import * as T from 'three';import {RoundedBoxGeometry} from 'three/addons/geometries/RoundedBoxGeometry.js';
import {build as base} from './kit.mjs';import {specs,seatX,feedEnd,feedTravel,slotCenters,slotRadius,perforationRadius,pressTimes,pressY,scrapStage,scrapPort,inputPort,outputPort,inkPort,inkSeat} from './batch-spec.mjs';
export function build(spec,simple=false){
 const b=base(spec,simple),s=specs[spec.id],root=b.root;
 const material=(name,color,metalness,roughness,extra={})=>new T.MeshStandardMaterial({name,color,metalness,roughness,...extra});
 const m={al:material('Anodised aluminium','#c6cdd2',.72,.35),steel:material('Brushed stainless','#afbcc5',.78,.32),blue:material('Powder coat blue','#306d8c',.32,.42),dark:material('Cast iron graphite','#243542',.5,.44),rubber:material('Rubber','#17272e',.02,.83),belt:material('Polyurethane belt','#328976',.08,.64),glass:material('Guard window','#809eab',.15,.22,{transparent:true,opacity:.26})};
 root.children.filter(o=>o.name.startsWith('Station_')).forEach(o=>o.removeFromParent());const tracks={load:[],auto:[],unload:[]},moving=[];
 const group=(p,n,pos=[0,0,0])=>{const o=new T.Group();o.name=n;o.position.fromArray(pos);p.add(o);return o;};
 const mesh=(p,n,g,pos,mat=m.al)=>{const o=new T.Mesh(g,mat);o.name=n;o.position.fromArray(pos);o.castShadow=o.receiveShadow=true;p.add(o);return o;};
 const box=(p,n,size,pos,mat=m.al)=>mesh(p,n,simple?new T.BoxGeometry(...size):new RoundedBoxGeometry(...size,2,Math.min(.003,...size.map(v=>v*.1))),pos,mat);
 const cyl=(p,n,r,h,pos,mat=m.steel,axis='z')=>{const o=mesh(p,n,new T.CylinderGeometry(r,r,h,simple?16:40),pos,mat);if(axis==='z')o.rotation.x=Math.PI/2;if(axis==='x')o.rotation.z=Math.PI/2;return o;};
 const cable=(p,n,points,r=.004)=>mesh(p,n,new T.TubeGeometry(new T.CatmullRomCurve3(points.map(v=>new T.Vector3(...v))),24,r,simple?5:8,false),[0,0,0],m.rubber);
 const pos=(phase,o,t,v)=>tracks[phase].push(new T.VectorKeyframeTrack(o.name+'.position',t,v.flat()));
 const rotate=(o,angle)=>{for(const phase of ['load','auto','unload']){const ts=phase==='auto'?Array.from({length:257},(_,i)=>i/256):[0,1];tracks[phase].push(new T.QuaternionKeyframeTrack(o.name+'.quaternion',ts,ts.flatMap(u=>new T.Quaternion().setFromAxisAngle(new T.Vector3(0,0,1),angle(phase==='auto'?u:phase==='load'?0:1)).toArray())));}moving.push(o.name);};
 const stand=(name,x,z,top,w=.065)=>{box(root,name+'Foot',[.13,.012,.13],[x,.806,z],m.dark);box(root,name+'Column',[w,top-.80,w],[x,(top+.80)/2,z]);};
 function table(name,x,z,surface,width=.80,depth=.78){box(root,name+'Top',[width,.025,depth],[x,surface-.0125,z],m.steel);for(const dx of [-width*.36,width*.36])for(const dz of [-depth*.36,depth*.36]){cyl(root,name+'Foot'+dx+'_'+dz,.038,.02,[x+dx,.01,z+dz],m.rubber,'y');box(root,name+'Leg'+dx+'_'+dz,[.045,surface-.045,.045],[x+dx,(surface-.005)/2,z+dz]);}for(const dz of [-depth*.36,depth*.36])box(root,name+'Brace'+dz,[width*.75,.03,.03],[x,surface*.50,z+dz]);}
 function drive(name,x,y,z,sign=1){
  const g=group(root,name,[x,y,z]);cyl(g,name+'Body',.045,.11,[0,0,0],m.blue);cyl(g,name+'Fan',.049,.018,[0,0,-sign*.064],m.dark);cyl(g,name+'Shaft',.014,.12,[0,0,sign*.10]);box(g,name+'Mount',[.15,.018,.18],[0,-.054,0],m.dark);box(root,name+'Stand',[.15,y-.863,.18],[x,(.80+y-.063)/2,z]);cable(root,name+'Power',[[.54,.54,-.43],[.64,.79,-.50],[x,y-.04,z]]);return g;
 }
 function shaft(name,x,y,r,width,driven=false){
  for(const z of [-width/2-.04,width/2+.04]){stand(name+'Stand'+z,x,z,y-.045);box(root,name+'Bearing'+z,[.09,.09,.07],[x,y,z],m.dark);}
  const g=group(root,name,[x,y,0]);cyl(g,name+'Axis',.014,width+.22,[0,0,0]);if(r)cyl(g,name+'Roll',r,width,[0,0,0]);if(driven)drive(name+'Drive',x,y,-width/2-.21);return g;
 }
 function sector(name,p,inner,outer,a0,a1,z,width,mat=m.steel){const shape=new T.Shape();shape.absarc(0,0,outer,a0,a1);shape.absarc(0,0,inner,a1,a0,true);shape.closePath();const geo=new T.ExtrudeGeometry(shape,{depth:width,bevelEnabled:false,curveSegments:simple?8:24});geo.translate(0,0,-width/2);return mesh(p,name,geo,[0,0,z],mat);}
 function guard(name,x,width=.35){for(const z of [-.37,.37]){stand(name+'Stand'+z,x-width/2,z,1.25);box(root,name+'Side'+z,[width,.27,.012],[x,1.115,z],m.glass);}box(root,name+'Roof',[width,.018,.76],[x,1.259,0],m.blue);}
 function waste(){const [x,y,z]=scrapPort(s);box(root,'WasteCartFloor',[.75,.02,.56],[x,y-.01,z],m.steel);for(const dx of [-.36,.36])box(root,'WasteCartEnd'+dx,[.02,.33,.56],[x+dx,y+.155,z],m.blue);for(const dz of [-.27,.27])box(root,'WasteCartSide'+dz,[.75,.33,.02],[x,y+.155,z+dz],m.blue);for(const dx of [-.30,.30])for(const dz of [-.21,.21]){cyl(root,'WasteCaster'+dx+'_'+dz,.035,.025,[x+dx,.035,z+dz],m.rubber);box(root,'WasteCartLeg'+dx+'_'+dz,[.03,y-.09,.03],[x+dx,(y+.05)/2,z+dz]);box(root,'WasteCasterFork'+dx+'_'+dz,[.045,.04,.04],[x+dx,.075,z+dz]);}
  if(s.mode!=='trim')box(root,'UnderGapScrapCollector',[.20,.010,s.size[2]],[s.toolX+.11,.805,0],m.steel);
  for(const e of s.extra_outputs){const p=group(root,'Port_'+e.role,scrapPort(s));b.anchors[e.role+'_staging']=group(root,'Stage_'+e.role,scrapStage(s)).name;}
 }
 table('InputTable',s.inputX,0,s.surface,s.mode==='crease'?.70:.80,.78);table('OutputTable',s.outputX,0,s.surface,s.mode==='crease'?.70:.80,.78);
 const cargo=group(root,'PaperSeat',[seatX(s,'load',0),s.surface,0]);b.cargo=cargo.name;b.anchors={product_staging:group(root,'ProductStaging',[s.mode==='slot'||s.mode==='perforate'?feedEnd:0,s.surface,0]).name};
 for(const phase of ['load','auto','unload']){pos(phase,cargo,[0,1],[[seatX(s,phase,0),s.surface,0],[seatX(s,phase,1),s.surface,0]]);}moving.push(cargo.name);
 if(s.mode==='slot'||s.mode==='perforate'){
  const width=s.size[2]+.06;
  for(const [index,a,b]of [[0,-.790,s.toolX-.090],[1,s.toolX+.20,1.04]]){
   const shape=new T.Shape(),y=.865,ro=.035,ri=.025;shape.moveTo(a,y+ro);shape.lineTo(b,y+ro);shape.absarc(b,y,ro,Math.PI/2,-Math.PI/2,true);shape.lineTo(a,y-ro);shape.absarc(a,y,ro,-Math.PI/2,Math.PI/2,true);shape.closePath();
   const hole=new T.Path();hole.moveTo(a,y+ri);hole.absarc(a,y,ri,Math.PI/2,Math.PI*1.5,false);hole.lineTo(b,y-ri);hole.absarc(b,y,ri,-Math.PI/2,Math.PI/2,false);hole.closePath();shape.holes.push(hole);const geo=new T.ExtrudeGeometry(shape,{depth:width,bevelEnabled:false,curveSegments:simple?8:24});geo.translate(0,0,-width/2);mesh(root,'ClosedFeedBelt'+index,geo,[0,0,0],m.belt);
   for(const [j,x]of [a,b].entries())rotate(shaft('FeedRoll'+index+'_'+j,x,.865,.025,width,j===0),u=>-feedTravel*u/.025);
  }
  for(const z of [-s.size[2]/2-.02,s.size[2]/2+.02])box(root,'SheetGuide'+z,[1.60,.025,.012],[0,.9125,z],m.blue);
  const x=s.toolX,r=s.mode==='slot'?slotRadius:perforationRadius+.0006,axis=s.surface+s.size[1]+(s.mode==='slot'?slotRadius-.002:perforationRadius);
  const rotor=shaft('WorkingRotor',x,axis,s.mode==='slot'?.095:.065,width,true);const counter=shaft('RelievedCounter',x,.86,0,width,false);
  const cuts=s.mode==='slot'?slotCenters:[0],kerf=s.mode==='slot'?.008:.001;let z0=-width/2;
  for(const [i,z]of [...cuts,width/2+kerf/2].entries()){const end=z-kerf/2;if(end>z0)cyl(counter,'CounterSegment'+i,.04,end-z0,[0,0,(z0+end)/2],m.rubber);z0=z+kerf/2;}
  if(s.mode==='slot')for(const z of cuts)for(const [i,distance]of [.75,1.15].entries())sector('SlotKnife'+z+'_'+i,rotor,.095,r,-Math.PI/2+distance/r,-Math.PI/2+(distance+.10)/r,z,.006);
  else for(let i=0;i<25;i++)sector('PerforatingTooth'+i,rotor,.065,r,-Math.PI/2+.655/perforationRadius+i*2*Math.PI/25,-Math.PI/2+.655/perforationRadius+i*2*Math.PI/25+.010/perforationRadius,0,.0006);
  rotate(rotor,u=>-feedTravel*u/(s.mode==='slot'?slotRadius:perforationRadius));rotate(counter,u=>feedTravel*u/.04);guard('ToolGuard',x);
  if(s.printed){
   rotate(shaft('PrintImpressionRoll',-.35,.85,.04,.66,false),u=>feedTravel*u/.04);
   const print=shaft('PrintCylinder',-.35,1.003,.098,.66,true);sector('RaisedPrintPlate',print,.098,.101,-Math.PI/2+.24/.101,-Math.PI/2+.36/.101,-.225,.10,new T.MeshStandardMaterial({name:'Flexo photopolymer',color:'#49636f',roughness:.55}));rotate(print,u=>-feedTravel*u/.101);
   rotate(shaft('AniloxCylinder',-.53,1.003,.079,.66,true),u=>feedTravel*u/.079);box(root,'DoctorBlade',[.025,.006,.64],[-.53,1.085,0],m.steel);
   box(root,'InkFountainBracket',[.10,.015,.12],[-.53,1.0895,-.42],m.steel);const cup=group(root,'InkFountain',inkSeat);cyl(cup,'InkCupFloor',.025,.008,[0,-.004,0],m.steel,'y');const shape=new T.Shape();shape.absarc(0,0,.026,0,Math.PI*2);const hole=new T.Path();hole.absarc(0,0,.023,0,Math.PI*2,true);shape.holes.push(hole);const geo=new T.ExtrudeGeometry(shape,{depth:.05,bevelEnabled:false,curveSegments:24});geo.rotateX(-Math.PI/2);mesh(cup,'EmptyInkCupWall',geo,[0,0,0],m.glass);b.anchors.ink_cup=group(root,'InkDoseSeat',inkSeat).name;
   cable(root,'InkSupplyPipe',[[inkSeat[0],inkSeat[1],inkSeat[2]],[inkSeat[0],1.08,-.36],[-.53,1.083,-.30]],.004);table('InkSupplyTable',inkPort[0],inkPort[2],.90,.30,.24);const supply=group(root,'InkSupplyCupSeat',inkPort);cyl(supply,'SupplyCupFloor',.025,.008,[0,-.004,0],m.steel,'y');mesh(supply,'SupplyEmptyCupWall',geo.clone(),[0,0,0],m.glass);guard('PrintingGuard',-.35,.40);
  }
 }else if(s.mode==='trim'){
  box(root,'TrimCentralSupport',[.498,.02,.398],[0,.89,0],m.steel);for(const sign of [-1,1]){box(root,'TrimEndSupport'+sign,[.020,.02,.398],[sign*.261,.89,0],m.steel);box(root,'TrimSideSupport'+sign,[.54,.02,.020],[0,.89,sign*.211],m.steel);}
  for(const x of [-.18,.18])for(const z of [-.16,.16])stand('TrimTableSupport'+x+'_'+z,x,z,.88);
  for(const sign of [-1,1]){
   for(const x of [-.45,.45])stand('SawGantry'+sign+'_'+x,x,sign*.30,1.12);cyl(root,'SawLinearRail'+sign,.012,.90,[0,1.12,sign*.28],m.steel,'x');
   box(root,'SawConductorRail'+sign,[.90,.020,.025],[0,1.18,sign*.39],m.blue);for(const x of [-.45,.45]){box(root,'SawBusBracket'+sign+'_'+x,[.06,.060,.12],[x,1.15,sign*.34]);}cable(root,'SawFixedPowerFeed'+sign,[[.54,.54,-.43],[.64,.90,-.50],[.45,1.18,sign*.39]]);
   const g=group(root,'SawCarriage'+sign,[-.34,1.12,sign*.28]);box(g,'SawCollectorShoe'+sign,[.05,.018,.035],[0,.06,sign*.11],m.dark);box(g,'SawRailBlock'+sign,[.10,.05,.08],[0,0,0],m.dark);box(g,'SawHanger'+sign,[.055,.18,.055],[0,-.09,0]);const rotor=group(g,'EdgeSaw'+sign,[0,-.185,-sign*.08]);cyl(rotor,'EdgeSawDisc'+sign,.050,.0015,[0,0,0],m.dark);cyl(g,'SawShaft'+sign,.012,.14,[0,-.185,-sign*.02]);cyl(g,'SawMotor'+sign,.035,.10,[0,-.185,sign*.065],m.blue);box(g,'SawMotorFoot'+sign,[.09,.02,.13],[0,-.23,sign*.06]);
   sector('FixedSawGuard'+sign,g,.051,.060,0,Math.PI,-sign*.08,.018,m.blue).position.y=-.185;
   pos('auto',g,[0,.15,.64,1],[[-.34,1.12,sign*.28],[-.34,1.12,sign*.28],[.34,1.12,sign*.28],[.34,1.12,sign*.28]]);for(const phase of ['load','unload'])pos(phase,g,[0,1],[[phase==='load'?-.34:.34,1.12,sign*.28],[phase==='load'?-.34:.34,1.12,sign*.28]]);rotate(rotor,u=>u*64*Math.PI);moving.push(g.name);cable(g,'SawPower'+sign,[[0,-.185,sign*.11],[0,.06,sign*.11]]);
   const blade=group(root,'EndKnife'+sign,[sign*.25,1.20,0]);box(blade,'EndKnifeEdge'+sign,[.0015,.035,.40],[0,0,0],m.dark);box(blade,'EndKnifeCrosshead'+sign,[.06,.015,.88],[0,.025,0]);for(const z of [-.42,.42])stand('EndKnifeGuide'+sign+'_'+z,sign*.25,z,1.62);box(root,'EndKnifeCrown'+sign,[.13,.04,.95],[sign*.25,1.64,0],m.blue);cyl(root,'EndKnifeCylinder'+sign,.034,.40,[sign*.25,1.44,0],m.dark,'y');cyl(blade,'EndKnifePiston'+sign,.014,.3575,[0,.21125,0],m.steel,'y');cable(root,'EndKnifeAir'+sign,[[.54,.54,-.43],[.64,.80,-.50],[sign*.25,1.44,-.034]],.005);pos('auto',blade,[0,.65,.72,.82,.90,1],[[sign*.25,1.20,0],[sign*.25,1.20,0],[sign*.25,.915,0],[sign*.25,.915,0],[sign*.25,1.20,0],[sign*.25,1.20,0]]);for(const phase of ['load','unload'])pos(phase,blade,[0,1],[[sign*.25,1.20,0],[sign*.25,1.20,0]]);moving.push(blade.name);
  }
  for(const x of [-.15,.15])for(const sign of [-1,1]){stand('TrimClampStand'+x+'_'+sign,x,sign*.35,1.07);box(root,'TrimClampArm'+x+'_'+sign,[.06,.04,.23],[x,1.09,sign*.25]);cyl(root,'TrimClampCylinder'+x+'_'+sign,.022,.10,[x,1.05,sign*.15],m.dark,'y');const pad=group(root,'TrimHoldDown'+x+'_'+sign,[x,.975,sign*.15]);box(pad,'TrimPad'+x+'_'+sign,[.04,.020,.03],[0,0,0],m.rubber);cyl(pad,'TrimClampRod'+x+'_'+sign,.009,.09,[0,.055,0],m.steel,'y');pos('load',pad,[0,1],[[x,.975,sign*.15],[x,.913,sign*.15]]);pos('auto',pad,[0,1],[[x,.913,sign*.15],[x,.913,sign*.15]]);pos('unload',pad,[0,1],[[x,.913,sign*.15],[x,.975,sign*.15]]);moving.push(pad.name);}
  b.focus='SawCarriage1';
 }else{
  box(root,'CreaseLowerBed',[.62,.017,.52],[0,.8085,0],m.steel);let edge=-.31;for(const [i,x]of [-.125,0,.125,.312].entries()){const end=x-.002;if(end>edge)box(root,'FemaleMatrixLand'+i,[end-edge,.003,.52],[(edge+end)/2,.8185,0],m.dark);edge=x+.002;}
  for(const x of [-.34,.34])for(const z of [-.28,.28])stand('CreasePressColumn'+x+'_'+z,x,z,1.36,.06);box(root,'CreasePressCrown',[.76,.06,.64],[0,1.39,0],m.blue);
  const head=group(root,'CreasePlaten',[0,pressY[0],0]);box(head,'PressPlaten',[.60,.025,.48],[0,.0125,0],m.al);for(const x of [-.125,0,.125])box(head,'CreaseMaleDie'+x,[.002,.018,.40],[x,-.009,0],m.steel);
  cyl(root,'CreaseHydraulicBarrel',.050,.30,[0,1.23,0],m.dark,'y');cyl(head,'CreasePiston',.022,.26,[0,.155,0],m.steel,'y');pos('auto',head,pressTimes,pressY.map(y=>[0,y,0]));pos('load',head,[0,1],[[0,1.05,0],[0,pressY[0],0]]);pos('unload',head,[0,1],[[0,1.05,0],[0,1.05,0]]);moving.push(head.name);
  box(root,'HydraulicPowerPack',[.26,.18,.20],[.54,.89,-.38],m.blue);cable(root,'CreaseHydraulicLine',[[.54,.98,-.38],[.25,1.23,-.30],[0,1.23,-.05]],.006);
  for(const x of [-.22,.22])for(const z of [-.16,.16]){cyl(head,'SpringClampHousing'+x+'_'+z,.018,.07,[x,.06,z],m.dark,'y');const pad=group(head,'SpringClampPad'+x+'_'+z,[x,-.010,z]);box(pad,'SpringClampFace'+x+'_'+z,[.04,.020,.03],[0,0,0],m.rubber);cyl(pad,'SpringClampStem'+x+'_'+z,.008,.085,[0,.0525,0],m.steel,'y');for(const phase of ['load','auto','unload']){const ts=phase==='auto'?pressTimes:[0,1],ys=phase==='auto'?pressY:phase==='load'?[1.05,pressY[0]]:[1.05,1.05];pos(phase,pad,ts,ys.map(y=>[x,Math.max(-.010,s.surface+s.size[1]-y+.010),z]));}moving.push(pad.name);}
  b.focus=head.name;
 }
 if(s.extra_outputs.length)waste();for(const [name,point]of [['input',inputPort(s)],['output',outputPort(s)],['auxiliary',inkPort]])root.getObjectByName('Port_'+name).position.fromArray(point);
 b.animations=Object.entries(tracks).map(([name,list])=>new T.AnimationClip(name,1,list));b.moving_names=moving;if(s.mode==='slot'||s.mode==='perforate')b.focus='WorkingRotor';b.envelope=[s.mode==='crease'?3.30:4.40,2.0];root.updateMatrixWorld(true);return b;
}
