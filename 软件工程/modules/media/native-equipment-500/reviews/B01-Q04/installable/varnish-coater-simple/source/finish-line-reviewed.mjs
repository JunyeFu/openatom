import * as T from 'three';import {RoundedBoxGeometry} from 'three/addons/geometries/RoundedBoxGeometry.js';
import {build as base} from './kit.mjs';import {specs,seatX,feedTravel,inputPort,outputPort,auxPort,auxSeat,samples,pixels,pixelPitch,pixelSize,printThickness,printedPixelCount} from './batch-spec.mjs';
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
 table('InputTable',s.inputX,0,s.surface,.80,.68);table('OutputTable',s.outputX,0,s.surface,.80,.68);
 const cargo=group(root,'PaperSeat',[seatX(s,'load',0),s.surface,0]);b.cargo=cargo.name;b.anchors={};for(const phase of ['load','auto','unload'])pos(phase,cargo,[0,1],[[seatX(s,phase,0),s.surface,0],[seatX(s,phase,1),s.surface,0]]);moving.push(cargo.name);
 const spans=s.mode==='coat'?[[-1.10,-.10],[.10,1.10]]:[[-1.10,1.10]];
 for(const[index,[a,z]]of spans.entries()){
  const y=s.surface-.025,shape=new T.Shape(),ro=.025,ri=.017;shape.moveTo(a,y+ro);shape.lineTo(z,y+ro);shape.absarc(z,y,ro,Math.PI/2,-Math.PI/2,true);shape.lineTo(a,y-ro);shape.absarc(a,y,ro,-Math.PI/2,Math.PI/2,true);shape.closePath();const hole=new T.Path();hole.moveTo(a,y+ri);hole.absarc(a,y,ri,Math.PI/2,Math.PI*1.5,false);hole.lineTo(z,y-ri);hole.absarc(z,y,ri,-Math.PI/2,Math.PI/2,false);hole.closePath();shape.holes.push(hole);const g=new T.ExtrudeGeometry(shape,{depth:.46,bevelEnabled:false,curveSegments:simple?12:32});g.translate(0,0,-.23);mesh(root,'ClosedFeedBelt'+index,g,[0,0,0],m.belt);for(const[j,x]of[a,z].entries())rotate(shaft('FeedRoll'+index+'_'+j,x,y,.017,.46,j===0),u=>-feedTravel*u/.017);
 }
 if(s.auxiliary){
  table('AuxSupplyTable',-1.60,-.75,.90,.35,.35);
  const cup=(name,p)=>{cyl(root,name+'Floor',.016,.008,[p[0],p[1]-.004,p[2]],m.steel,'y');const sh=new T.Shape();sh.absarc(0,0,.016,0,Math.PI*2);const h=new T.Path();h.absarc(0,0,.014,0,Math.PI*2,true);sh.holes.push(h);const geo=new T.ExtrudeGeometry(sh,{depth:.025,bevelEnabled:false,curveSegments:simple?16:40});geo.rotateX(-Math.PI/2);mesh(root,name+'Wall',geo,p,m.blue);};
  const p=auxSeat(s);cup('WorkingDose',p);stand('DoseCupStand',p[0],p[2],p[1]-.008);cup('InputDose',auxPort(s));b.anchors.aux_seat=group(root,'ActualDoseSeat',p).name;
  cable(root,'DoseFeedTube',[p,[p[0],p[1]+.10,p[2]],[s.mode==='code'?0:-.12,s.mode==='code'?1.14:1.12,-.20]],.002);
 }
 if(s.mode==='code'){
  for(const z of[-.31,.31])stand('InkjetGantry'+z,0,z,1.16);box(root,'InkjetCrossbeam',[.08,.04,.68],[0,1.18,0],m.blue);
  box(root,'DODHead',[.036,.12,.06],[0,s.surface+s.size[1]+.003+.06,0],m.dark);for(let row=0;row<7;row++)cyl(root,'Nozzle'+row,.0006,.002,[0,s.surface+s.size[1]+.004,(row-3)*pixelPitch],m.steel,'y');
  stand('PhotoeyeBracket',-.32,-.28,.97);box(root,'Photoeye',[.04,.035,.06],[-.32,.985,-.28],m.dark);cable(root,'HeadSignal',[[0,1.107,0],[0,1.15,-.30],[.54,.54,-.43]]);b.focus='DODHead';
 }else if(s.mode==='coat'){
  const y=s.surface+s.size[1]+.000020+.08;
  const upper=shaft('ApplicationRoll',0,y,.08,.46,true);upper.getObjectByName('ApplicationRollRoll').material=m.rubber;rotate(upper,u=>feedTravel*u/.08);
  rotate(shaft('ImpressionRoll',0,s.surface-.06,.06,.46,false),u=>-feedTravel*u/.06);
  rotate(shaft('MeteringRoll',-.12,y,.04,.46,false),u=>-feedTravel*u/.04);
  box(root,'MeteringBlade',[.008,.03,.46],[-.164,y,0],m.dark);for(const z of[-.27,.27]){stand('BladeSupport'+z,-.20,z,y-.015);box(root,'BladeClamp'+z,[.07,.02,.06],[-.19,y+.015,z]);}
  guard('CoatingGuard',0,.52);b.focus='ApplicationRoll';
 }else{
  for(const x of[-.42,.42])for(const z of[-.34,.34])stand('UVHoodPost'+x+'_'+z,x,z,1.23);
  for(const z of[-.32,.32])box(root,'UVHoodSide'+z,[.90,.26,.025],[0,1.10,z],m.blue);
  for(const x of[-.365,.365])box(root,'UVRoofEnd'+x,[.17,.025,.67],[x,1.2425,0],m.blue);
  for(const z of[-.255,.255])box(root,'UVRoofRail'+z,[.56,.025,.16],[0,1.2425,z],m.blue);
  box(root,'UVInspectionWindow',[.56,.012,.35],[0,1.2425,0],m.glass);
  const quartz=material('Quartz housing','#d6e3ee',.05,.2,{transparent:true,opacity:.45});
  for(const x of[-.22,.22]){box(root,'LampReflector'+x,[.11,.028,.52],[x,1.20,0],m.steel);cyl(root,'LampQuartz'+x,.012,.48,[x,1.175,0],quartz);for(const z of[-.27,.27])box(root,'LampBracket'+x+'_'+z,[.07,.06,.025],[x,1.205,z],m.dark);}
  cyl(root,'CoolingFanHousing',.065,.035,[.365,1.2725,0],m.dark,'y');const fan=group(root,'CoolingFan',[.365,1.292,0]);for(let i=0;i<4;i++){const vane=box(fan,'FanBlade'+i,[.07,.003,.018],[.023,0,0],m.al);vane.rotation.y=i*Math.PI/2;vane.position.set(.023*Math.cos(i*Math.PI/2),0,-.023*Math.sin(i*Math.PI/2));}
  // Fan axis is vertical; keep the authored blade geometry in the rotor group.
  for(const phase of ['load','auto','unload']){const ts=phase==='auto'?samples:[0,1];tracks[phase].push(new T.QuaternionKeyframeTrack(fan.name+'.quaternion',ts,ts.flatMap(u=>new T.Quaternion().setFromAxisAngle(new T.Vector3(0,1,0),phase==='auto'?u*16*Math.PI:0).toArray())));}moving.push(fan.name);
  const indicator=box(root,'CuringStageIndicator',[.09,.025,.008],[.36,1.11,.338],material('Blue stage indicator','#438dd9',.1,.35));
  for(const phase of ['load','auto','unload']){const ts=[0,.2,.8,1];tracks[phase].push(new T.VectorKeyframeTrack(indicator.name+'.scale',ts,ts.flatMap(u=>phase==='auto'&&u>=.2&&u<.8?[1,1,1]:[0,0,0]),T.InterpolateDiscrete));}moving.push(indicator.name);cable(root,'LampPower',[[.54,.54,-.43],[.50,1.20,-.40],[.365,1.24,-.32]]);b.focus='LampQuartz-0.22';
 }
 for(const z of[-.22,.22])box(root,'PaperRegistrationGuide'+z,[2.25,.020,.010],[0,s.surface+.015,z],m.blue);
 for(const[name,p]of[['input',inputPort(s)],['output',outputPort(s)],...(s.auxiliary?[['auxiliary',auxPort(s)]]:[])])root.getObjectByName('Port_'+name).position.fromArray(p);
 b.animations=Object.entries(tracks).map(([name,list])=>new T.AnimationClip(name,1,list));b.moving_names=moving;b.envelope=[4.2,2];root.updateMatrixWorld(true);return b;
}
