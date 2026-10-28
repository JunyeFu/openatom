import * as T from 'three';import {RoundedBoxGeometry} from 'three/addons/geometries/RoundedBoxGeometry.js';
import {build as base} from './kit.mjs';import {specs,seatX,feedTravel,inputPort,outputPort,samples} from './batch-spec.mjs';
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
 const spans=s.mode==='score'?[[-1.10,-.10],[.10,1.10]]:[[-1.10,1.10]];
 for(const[index,[a,z]]of spans.entries()){
  const y=s.surface-.025,shape=new T.Shape(),ro=.025,ri=.017;shape.moveTo(a,y+ro);shape.lineTo(z,y+ro);shape.absarc(z,y,ro,Math.PI/2,-Math.PI/2,true);shape.lineTo(a,y-ro);shape.absarc(a,y,ro,-Math.PI/2,Math.PI/2,true);shape.closePath();const hole=new T.Path();hole.moveTo(a,y+ri);hole.absarc(a,y,ri,Math.PI/2,Math.PI*1.5,false);hole.lineTo(z,y-ri);hole.absarc(z,y,ri,-Math.PI/2,Math.PI/2,false);hole.closePath();shape.holes.push(hole);const g=new T.ExtrudeGeometry(shape,{depth:.46,bevelEnabled:false,curveSegments:simple?12:32});g.translate(0,0,-.23);mesh(root,'ClosedFeedBelt'+index,g,[0,0,0],m.belt);for(const[j,x]of[a,z].entries())rotate(shaft('FeedRoll'+index+'_'+j,x,y,.017,.46,j===0),u=>-feedTravel*u/.017);
 }
 const upperY=s.surface+s.size[1]+.08-s.score_depth;
 const upper=shaft('ScoringArbor',0,upperY,0,.46,true);rotate(upper,u=>feedTravel*u/.08);
 const counter=shaft('GroovedCounterArbor',0,s.surface-.06,0,.46,true);rotate(counter,u=>-feedTravel*u/.06);
 const profile=(name,parent,points,centerZ,material)=>{const geo=new T.LatheGeometry(points.map(([r,z])=>new T.Vector2(r,z)),simple?32:96);geo.rotateX(Math.PI/2);return mesh(parent,name,geo,[0,0,centerZ],material);};
 for(const[i,z]of s.score_centers.entries()){
  profile('MaleCreaseWheel'+i,upper,[[.014,-.010],[.070,-.010],[.0794,-.003],[.08,0],[.0794,.003],[.070,.010],[.014,.010],[.014,-.010]],z,m.steel);
  for(const sign of[-1,1]){cyl(upper,'WheelLockCollar'+i+'_'+sign,.025,.014,[0,0,z+sign*.022],m.dark);if(!simple)box(upper,'CollarClamp'+i+'_'+sign,[.010,.008,.008],[0,.027,z+sign*.022],m.dark);}
 }
 const lowerProfile=[[.014,-.23],[.06,-.23],[.06,-.104],[.0594,-.10],[.06,-.096],[.06,.096],[.0594,.10],[.06,.104],[.06,.23],[.014,.23],[.014,-.23]];
 profile('ActualGroovedCounterRoll',counter,lowerProfile,0,m.rubber);
 guard('ScoringGuard',0,.50);for(const z of[-.22,.22])box(root,'PaperRegistrationGuide'+z,[2.25,.020,.010],[0,s.surface+.015,z],m.blue);
 for(const[name,p]of[['input',inputPort(s)],['output',outputPort(s)]])root.getObjectByName('Port_'+name).position.fromArray(p);
 b.animations=Object.entries(tracks).map(([name,list])=>new T.AnimationClip(name,1,list));b.moving_names=moving;b.focus='ScoringArbor';b.envelope=[4.2,2];root.updateMatrixWorld(true);return b;
}
