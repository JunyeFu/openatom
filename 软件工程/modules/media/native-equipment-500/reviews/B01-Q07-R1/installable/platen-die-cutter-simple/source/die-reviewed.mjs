import * as T from 'three';import {RoundedBoxGeometry} from 'three/addons/geometries/RoundedBoxGeometry.js';
import {build as base} from './kit.mjs';import {specs,seatX,inputPort,outputPort,pressY,pressTimes,trayTop,trayZ,shapePoints,cornerPositions} from './batch-spec.mjs';
export function build(spec,simple=false){
 const b=base({...spec,mechanisms:['print']},simple),s=specs[spec.id],root=b.root;
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
 table('InputTable',s.inputX,0,s.surface,.80,.68);table('OutputTable',s.outputX,0,s.surface,.80,.68);
 const cargo=group(root,'PaperSeat',[s.inputX,s.surface,0]);b.cargo=cargo.name;b.anchors={product_staging:group(root,'BlankStaging',[s.stagingX,s.surface,0]).name,scrap_staging:group(root,'CornerWasteStaging',[0,s.surface-.06,0]).name};
 for(const phase of ['load','auto','unload'])pos(phase,cargo,[0,1],[[seatX(s,phase,0),s.surface,0],[seatX(s,phase,1),s.surface,0]]);moving.push(cargo.name);
 const crossShape=new T.Shape();crossShape.moveTo(...shapePoints[0]);for(const p of shapePoints.slice(1))crossShape.lineTo(...p);crossShape.closePath();const bed=new T.ExtrudeGeometry(crossShape,{depth:.020,bevelEnabled:false});bed.rotateX(Math.PI/2);mesh(root,'CrossBlankAnvilBed',bed,[0,s.surface,0],m.steel);for(const x of[-.10,.10])for(const z of[-.06,.06])stand('AnvilPedestal'+x+'_'+z,x,z,s.surface-.020);
 for(const x of[-.34,.34])for(const z of[-.29,.29])stand('PressColumn'+x+'_'+z,x,z,1.65,.065);box(root,'PressCrown',[.78,.06,.68],[0,1.68,0],m.blue);
 const head=group(root,'DiePlaten',[0,1.35,0]);box(head,'PlatenBackplate',[.62,.028,.52],[0,.014,0],m.al);
 for(let i=0;i<shapePoints.length;i++){
  const a=shapePoints[i],z=shapePoints[(i+1)%shapePoints.length],length=Math.hypot(z[0]-a[0],z[1]-a[1]),blade=box(head,'SteelRuleSegment'+i,[length,.024,.0006],[(a[0]+z[0])/2,-.012,(a[1]+z[1])/2],m.dark);blade.rotation.y=-Math.atan2(z[1]-a[1],z[0]-a[0]);
 }
 cyl(root,'HydraulicBarrel',.052,.24,[0,1.52,0],m.dark,'y');box(root,'BarrelMount',[.15,.015,.15],[0,1.6425,0],m.steel);
 const rod=cyl(head,'ExposedPistonRod',.020,1,[0,0,0],m.steel,'y');
 for(const phase of ['load','auto','unload']){const ts=phase==='auto'?pressTimes:[0,1],ys=phase==='auto'?pressY:[1.35,1.35];pos(phase,head,ts,ys.map(y=>[0,y,0]));pos(phase,rod,ts,ys.map(y=>[0,(1.40-y+.028)/2,0]));tracks[phase].push(new T.VectorKeyframeTrack(rod.name+'.scale',ts,ys.flatMap(y=>[1,1.40-y-.028,1])));}moving.push(head.name,rod.name);
 for(const x of[-.08,.08])for(const z of[-.06,.06]){
  const pad=group(head,'EjectionPad'+x+'_'+z,[x,-.026,z]);box(pad,'RubberEjectionFace'+x+'_'+z,[.045,.012,.035],[0,0,0],m.rubber);cyl(pad,'EjectionStem'+x+'_'+z,.007,.060,[0,.036,0],m.steel,'y');cyl(head,'SpringHousing'+x+'_'+z,.014,.055,[x,.025,z],m.dark,'y');
  for(const phase of ['load','auto','unload']){const ts=phase==='auto'?pressTimes:[0,1],ys=phase==='auto'?pressY:[1.35,1.35];pos(phase,pad,ts,ys.map(y=>[x,Math.max(-.026,s.surface+s.size[1]-y+.006),z]));}moving.push(pad.name);
 }
 box(root,'HydraulicPowerPack',[.28,.18,.22],[.53,.89,-.38],m.blue);cable(root,'HydraulicLine',[[.53,.98,-.38],[.38,1.50,-.32],[0,1.52,-.052]],.006);cable(root,'PowerPackCable',[[.54,.54,-.43],[.56,.80,-.45],[.53,.89,-.49]],.005);
 for(const sign of[-1,1])table(sign<0?'RearWasteSupport':'FrontWasteSupport',0,sign*.60,.97,.64,.38);
 for(const x of[-.225,-.175,.175,.225])for(const z of[-.70,.70])box(root,'WasteOuterRailStand'+x+'_'+z,[.020,.029,.035],[x,.9845,z],m.steel);
 for(const[i,p]of cornerPositions.entries()){
  const sign=Math.sign(p[2]),carrier=group(root,'CornerCarrier'+i,[p[0],1.02,sign*.65]);box(carrier,'CornerSupportPad'+i,[.10,.008,.10],[0,-.004,0],m.steel);box(carrier,'CarrierRailBlock'+i,[.035,.025,.075],[0,-.020,0],m.dark);
  for(const dx of[-.025,.025]){const rail=cyl(root,'WasteSlideRail'+i+'_'+dx,.006,.63,[p[0]+dx,1.005,sign*.435],m.steel,'z');}
  box(root,'WasteRailInnerStand'+i,[.09,.20,.055],[p[0],.90,sign*.15],m.blue);cyl(root,'WasteLiftBarrel'+i,.014,.12,[p[0],.93,sign*.15],m.dark,'y');cable(root,'WasteActuatorLine'+i,[[.53,.98,-.38],[p[0],.86,sign*.15],[p[0],.94,sign*.15]],.003);
  const lowerRail=group(root,'VerticalTrayLift'+i,[p[0],1.005,sign*.15]);box(lowerRail,'LiftCrosshead'+i,[.09,.010,.08],[0,0,0]);cyl(lowerRail,'LiftRod'+i,.007,.08,[0,-.045,0],m.steel,'y');
  for(const phase of ['load','auto','unload']){const ts=[...new Set([0,.60,.75,.83,.98,1])].sort((a,b)=>a-b);pos(phase,carrier,ts,ts.map(u=>[p[0],trayTop(s,phase,u),trayZ(sign,phase,u)]));pos(phase,lowerRail,ts,ts.map(u=>[p[0],trayTop(s,phase,u)-.015,sign*.15]));}moving.push(carrier.name,lowerRail.name);
 }
 table('ScrapBinTable',0,-.98,.85,.50,.40);box(root,'ScrapBinFloor',[.46,.010,.36],[0,.855,-.98],m.dark);for(const sign of[-1,1]){box(root,'ScrapBinSide'+sign,[.010,.09,.36],[sign*.235,.90,-.98],m.blue);box(root,'ScrapBinEnd'+sign,[.48,.09,.010],[0,.90,-.98+sign*.185],m.blue);}
 for(const z of[-.31,.31])box(root,'PressSideGuard'+z,[.64,.46,.012],[0,1.39,z],m.glass);
 for(const[name,p]of[['input',inputPort(s)],['output',outputPort(s)]])root.getObjectByName('Port_'+name).position.fromArray(p);
 b.anchors.scrap=group(root,'Port_scrap',[0,.86,-.98]).name;
 b.animations=Object.entries(tracks).map(([name,list])=>new T.AnimationClip(name,1,list));b.moving_names=moving;b.focus='DiePlaten';b.envelope=[4.2,2.5];root.updateMatrixWorld(true);return b;
}
