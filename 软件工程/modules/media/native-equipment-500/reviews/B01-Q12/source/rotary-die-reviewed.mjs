import * as T from 'three';import {RoundedBoxGeometry} from 'three/addons/geometries/RoundedBoxGeometry.js';
import {build as base} from './kit.mjs';import {specs,seatX,inputPort,outputPort,scrapPort,trayTop,trayZ,shapePoints,cornerPositions,dieX,ruleRadius,shellRadius,anvilRadius,rootWidth,tipWidth,dieAngle,travel,pusherX,pusherBottom,samples} from './batch-spec.mjs';
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
 const rotate=(o,angle)=>{for(const phase of ['load','auto','unload']){const ts=phase==='auto'?samples:[0,1];tracks[phase].push(new T.QuaternionKeyframeTrack(o.name+'.quaternion',ts,ts.flatMap(u=>new T.Quaternion().setFromAxisAngle(new T.Vector3(0,0,1),angle(phase==='auto'?u:phase==='load'?0:1)).toArray())));}moving.push(o.name);};
 const stand=(name,x,z,top,w=.065,base=.80)=>{box(root,name+'Foot',[.13,.012,.13],[x,base+.006,z],m.dark);box(root,name+'Column',[w,top-base,w],[x,(top+base)/2,z]);};
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
 const cargo=group(root,'PaperSeat',[s.inputX,s.surface,0]);b.cargo=cargo.name;b.anchors={product_staging:group(root,'BlankStaging',[s.stagingX,s.surface,0]).name,scrap_staging:group(root,'CornerWasteStaging',[s.clearX,s.surface-.06,0]).name};
 for(const phase of ['load','auto','unload'])pos(phase,cargo,samples,samples.map(u=>[seatX(s,phase,u),s.surface,0]));moving.push(cargo.name);
 // Beveled steel-rule ribs wrap the actual cutter pitch circle, not a decorative roll texture.
 function ribGeometry(a,b){
  const longitudinal=Math.abs(b[0]-a[0])>1e-9,n=longitudinal?Math.max(8,Math.ceil(Math.abs(b[0]-a[0])/(simple?.005:.0025))):1,ts=[...new Set([0,1,...Array.from({length:n+1},(_,i)=>i/n),...(longitudinal?[.173,.371,.5,.683,.827]:[])])].sort((x,y)=>x-y),vertices=[],indices=[];
  for(const u of ts){const x=a[0]+(b[0]-a[0])*u,z=a[1]+(b[1]-a[1])*u,angle=x/ruleRadius;for(const[r,w]of[[shellRadius,rootWidth],[ruleRadius,tipWidth]])for(const side of[-1,1])vertices.push(r*Math.sin(angle)+(longitudinal?0:side*w/2*Math.cos(angle)),-r*Math.cos(angle)+(longitudinal?0:side*w/2*Math.sin(angle)),z+(longitudinal?side*w/2:0));}
  const quad=(a,b,c,d)=>indices.push(a,b,c,a,c,d);
  for(let i=0;i<ts.length-1;i++){const k=i*4,j=k+4;quad(k,k+1,j+1,j);quad(k+2,j+2,j+3,k+3);quad(k,j,j+2,k+2);quad(k+1,k+3,j+3,j+1);}
  quad(0,2,3,1);const k=(ts.length-1)*4;quad(k,k+1,k+3,k+2);const g=new T.BufferGeometry();g.setAttribute('position',new T.Float32BufferAttribute(vertices,3));g.setIndex(indices);let v=0;for(let i=0;i<indices.length;i+=3){const p=[0,1,2].map(j=>new T.Vector3(...vertices.slice(indices[i+j]*3,indices[i+j]*3+3)));v+=p[0].dot(p[1].clone().cross(p[2]));}if(v<0)for(let i=0;i<indices.length;i+=3)[indices[i+1],indices[i+2]]=[indices[i+2],indices[i+1]];g.setIndex(indices);g.computeVertexNormals();return g;
 }
 const die=shaft('DieCylinder',dieX,s.surface+ruleRadius,shellRadius,.56,true);die.getObjectByName('DieCylinderRoll').material=m.dark;die.rotation.z=dieAngle(s,0);
 for(const z of[-.282,.282])cyl(die,'DieEndFlange'+z,.101,.004,[0,0,z],m.blue);
 for(let i=0;i<shapePoints.length;i++)mesh(die,'RotaryRule'+i,ribGeometry(shapePoints[i],shapePoints[(i+1)%shapePoints.length]),[0,0,0],m.steel);
 rotate(die,u=>dieAngle(s,u));rotate(shaft('AnvilCylinder',dieX,s.surface-anvilRadius,anvilRadius,.56,true),u=>-(seatX(s,'auto',u)-dieX)/anvilRadius);
 for(const[index,[a,z]]of[[-1.10,dieX-.09],[dieX+.09,.50]].entries()){
  const y=s.surface-.025,sh=new T.Shape(),hole=new T.Path(),ro=.025,ri=.018;sh.moveTo(a,y+ro);sh.lineTo(z,y+ro);sh.absarc(z,y,ro,Math.PI/2,-Math.PI/2,true);sh.lineTo(a,y-ro);sh.absarc(a,y,ro,-Math.PI/2,Math.PI/2,true);sh.closePath();hole.moveTo(a,y+ri);hole.absarc(a,y,ri,Math.PI/2,Math.PI*1.5,false);hole.lineTo(z,y-ri);hole.absarc(z,y,ri,-Math.PI/2,Math.PI/2,false);hole.closePath();sh.holes.push(hole);const g=new T.ExtrudeGeometry(sh,{depth:.46,bevelEnabled:false,curveSegments:simple?12:32});g.translate(0,0,-.23);mesh(root,'RotaryFeedBelt'+index,g,[0,0,0],m.belt);for(const[j,x]of[a,z].entries())rotate(shaft('RotaryFeedRoll'+index+'_'+j,x,y,ri,.46,j===0),u=>-travel(s,u)/ri);
 }
 for(const z of[-.37,.37]){stand('RotaryGuardPost'+z,dieX-.23,z,1.32);box(root,'RotaryGuardSide'+z,[.46,.30,.012],[dieX,1.165,z],m.glass);}box(root,'RotaryGuardRoof',[.46,.018,.76],[dieX,1.329,0],m.blue);
 table('ClearingBase',s.clearX,0,.78,.88,.88);
 const crossShape=new T.Shape();crossShape.moveTo(...shapePoints[0]);for(const p of shapePoints.slice(1))crossShape.lineTo(...p);crossShape.closePath();const bed=new T.ExtrudeGeometry(crossShape,{depth:.020,bevelEnabled:false});bed.rotateX(Math.PI/2);mesh(root,'CrossBlankAnvilBed',bed,[s.clearX,s.surface,0],m.steel);for(const x of[-.10,.10])for(const z of[-.06,.06])stand('AnvilPedestal'+x+'_'+z,s.clearX+x,z,s.surface-.020,.065,.78);
 for(const sign of[-1,1]){box(root,'ReceivingSideGuide'+sign,[1.25,.030,.006],[.40,s.surface+.015,sign*.205],m.dark);for(const x of[0,.90]){stand('GuideStand'+x+'_'+sign,x,sign*.29,s.surface,.045,x>.5?.78:.80);box(root,'GuideBracket'+x+'_'+sign,[.07,.012,.12],[x,s.surface+.006,sign*.26],m.steel);}}
 // The pusher engages only after the entire sheet has cleared the cutting nip.
 for(const x of[-.17,.85])for(const sign of[-1,1]){stand('PusherRailPost'+x+'_'+sign,x,sign*.30,1.38,.055,x>.5?.78:.80);box(root,'PusherRailSeat'+x+'_'+sign,[.08,.022,.08],[x,1.391,sign*.30],m.steel);}
 for(const sign of[-1,1])cyl(root,'PusherGuideRail'+sign,.008,1.05,[.34,1.410,sign*.30],m.steel,'x');
 const bridge=group(root,'ReceivingPusherCarriage',[pusherX(s,'load',0),0,0]);for(const sign of[-1,1])box(bridge,'PusherRailShoe'+sign,[.08,.030,.07],[0,1.410,sign*.30],m.dark);box(bridge,'PusherBridge',[.10,.040,.66],[0,1.445,0],m.blue);
 cyl(bridge,'PusherLiftBarrel',.027,.160,[0,1.325,0],m.dark,'y');box(bridge,'PusherCylinderMount',[.08,.020,.10],[0,1.415,0],m.steel);
 const head=group(bridge,'ReceivingPusherHead',[0,1.19,0]);box(head,'ReceivingPusherFace',[.012,.030,.398],[0,-.015,0],m.rubber);box(head,'PusherHeadCoupling',[.06,.020,.30],[0,.010,0],m.steel);const rod=cyl(head,'PusherLiftRod',.010,1,[0,0,0],m.steel,'y');rod.position.y=.0375;rod.scale.y=.035;
 for(const phase of['load','auto','unload']){pos(phase,bridge,samples,samples.map(u=>[pusherX(s,phase,u),0,0]));pos(phase,head,samples,samples.map(u=>[0,pusherBottom(phase,u)+.03,0]));pos(phase,rod,samples,samples.map(u=>[0,(1.245-pusherBottom(phase,u)-.03+.02)/2,0]));tracks[phase].push(new T.VectorKeyframeTrack(rod.name+'.scale',samples,samples.flatMap(u=>[1,1.245-pusherBottom(phase,u)-.03-.02,1])));}moving.push(bridge.name,head.name,rod.name);
 box(root,'PneumaticValveBlock',[.18,.12,.12],[.40,.86,-.35],m.blue);
 box(root,'PusherAirSideChannel',[1.10,.04,.04],[.34,1.37,-.39],m.blue);for(const x of[-.17,.85])box(root,'PusherAirChannelBracket'+x,[.08,.04,.14],[x,1.37,-.35],m.steel);
 box(root,'PusherAirManifold',[.06,.030,.04],[.34,1.335,-.39],m.dark);cable(root,'PusherAirFeed',[[.40,.92,-.35],[.80,1.20,-.50],[.34,1.335,-.41]],.004);
 const hosePoint=(phase,u,t)=>{const x=pusherX(s,phase,u);return new T.Vector3(.34+(x-.34)*t,1.335-.010*t+.040*Math.sin(Math.PI*t),-.37+.343*t-.100*Math.sin(Math.PI*t));};
 for(let i=0;i<32;i++){
  const part=cyl(root,'MovingPusherHose'+i,.003,1,[0,0,0],m.rubber,'y'),state=(phase,u)=>{const a=hosePoint(phase,u,i/32),b=hosePoint(phase,u,(i+1)/32),d=b.clone().sub(a);return{p:a.add(b).multiplyScalar(.5).toArray(),q:new T.Quaternion().setFromUnitVectors(new T.Vector3(0,1,0),d.clone().normalize()).toArray(),h:d.length()};};
  const first=state('load',0);part.position.fromArray(first.p);part.quaternion.fromArray(first.q);part.scale.y=first.h;
  for(const phase of['load','auto','unload']){pos(phase,part,samples,samples.map(u=>state(phase,u).p));tracks[phase].push(new T.QuaternionKeyframeTrack(part.name+'.quaternion',samples,samples.flatMap(u=>state(phase,u).q)));tracks[phase].push(new T.VectorKeyframeTrack(part.name+'.scale',samples,samples.flatMap(u=>[1,state(phase,u).h,1])));}moving.push(part.name);
 }

 for(const sign of[-1,1])table(sign<0?'RearWasteSupport':'FrontWasteSupport',s.clearX,sign*.60,.97,.64,.38);
 for(const x of[-.225,-.175,.175,.225])for(const z of[-.70,.70])box(root,'WasteOuterRailStand'+x+'_'+z,[.020,.029,.035],[s.clearX+x,.9845,z],m.steel);
 for(const[i,p]of cornerPositions.entries()){
  const sign=Math.sign(p[2]),carrier=group(root,'CornerCarrier'+i,[s.clearX+p[0],1.02,sign*.65]);box(carrier,'CornerSupportPad'+i,[.10,.008,.10],[0,-.004,0],m.steel);box(carrier,'CarrierRailBlock'+i,[.035,.014,.075],[0,-.015,0],m.dark);
  for(const dx of[-.025,.025]){const rail=cyl(root,'WasteSlideRail'+i+'_'+dx,.006,.63,[s.clearX+p[0]+dx,1.005,sign*.435],m.steel,'z');}
  box(root,'WasteLiftPedestal'+i,[.09,.078,.055],[s.clearX+p[0],.819,sign*.15],m.blue);for(const dx of[-.025,.025])box(root,'WasteRailInnerStand'+i+'_'+dx,[.015,.219,.025],[s.clearX+p[0]+dx,.8895,sign*.15],m.blue);cyl(root,'WasteLiftBarrel'+i,.014,.12,[s.clearX+p[0],.918,sign*.15],m.dark,'y');cable(root,'WasteActuatorLine'+i,[[.40,.86,-.35],[s.clearX+p[0],.86,sign*.15],[s.clearX+p[0],.94,sign*.15]],.003);
  const lowerRail=group(root,'VerticalTrayLift'+i,[s.clearX+p[0],.993,sign*.15]);box(lowerRail,'LiftCrosshead'+i,[.030,.010,.08],[0,0,0]);cyl(lowerRail,'LiftRod'+i,.007,.08,[0,-.045,0],m.steel,'y');
  for(const phase of ['load','auto','unload']){const ts=samples;pos(phase,carrier,ts,ts.map(u=>[s.clearX+p[0],trayTop(s,phase,u),trayZ(sign,phase,u)]));pos(phase,lowerRail,ts,ts.map(u=>[s.clearX+p[0],trayTop(s,phase,u)-.027,sign*.15]));}moving.push(carrier.name,lowerRail.name);
 }
 table('ScrapBinTable',s.clearX,-.98,.85,.50,.40);box(root,'ScrapBinFloor',[.46,.010,.36],[s.clearX,.855,-.98],m.dark);for(const sign of[-1,1]){box(root,'ScrapBinSide'+sign,[.010,.09,.36],[s.clearX+sign*.235,.90,-.98],m.blue);box(root,'ScrapBinEnd'+sign,[.48,.09,.010],[s.clearX,.90,-.98+sign*.185],m.blue);}
 for(const[name,p]of[['input',inputPort(s)],['output',outputPort(s)]])root.getObjectByName('Port_'+name).position.fromArray(p);
 b.anchors.scrap=group(root,'Port_scrap',scrapPort(s)).name;
 b.animations=Object.entries(tracks).map(([name,list])=>new T.AnimationClip(name,1,list));b.moving_names=moving;b.focus='DieCylinder';b.envelope=[4.4,2.5];root.updateMatrixWorld(true);return b;
}
