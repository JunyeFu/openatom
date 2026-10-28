import * as T from 'three';
import {RoundedBoxGeometry} from 'three/addons/geometries/RoundedBoxGeometry.js';
const pi=Math.PI, v=a=>new T.Vector3(...a);
const material=(name,color,metalness,roughness)=>new T.MeshStandardMaterial({name,color,metalness,roughness});
const M={steel:material('Brushed stainless','#b1bec7',.78,.32),al:material('Anodised aluminium','#c4cbd0',.7,.36),dark:material('Cast iron graphite','#253746',.45,.43),blue:material('Powder coat blue','#306d8c',.35,.4),white:material('Ceramic white shell','#dce4e7',.28,.35),belt:material('Green polyurethane','#328976',.08,.62),rubber:material('Rubber black','#17262e',.02,.85),amber:material('Safety yellow','#edb34f',.2,.4),red:material('Emergency red','#ce514e',.08,.4),green:material('Signal green','#53b695',.08,.4),lens:material('Optical glass','#325b71',.5,.18),label:material('LABEL_ATLAS','#ffffff',0,.7),screen:material('HMI screen','#123d54',.2,.3)};

export function buildEquipment(spec,simple=false){
 let serial=0; const root=new T.Group();root.name='Equipment'; const cache=new Map();
 const geo=(key,fn)=>{if(!cache.has(key))cache.set(key,fn());return cache.get(key);};
 const group=(p,name,pos=[0,0,0])=>{const o=new T.Group();o.name=name;o.position.copy(v(pos));p.add(o);return o;};
 const mesh=(p,name,g,pos,m)=>{const o=new T.Mesh(g,m);o.name=name+'_'+serial++;o.position.copy(v(pos));o.castShadow=true;o.receiveShadow=true;p.add(o);return o;};
 const box=(p,name,d,pos,m=M.steel,r=.006)=>mesh(p,name,geo('b'+d+simple+r,()=>simple||!r?new T.BoxGeometry(...d):new RoundedBoxGeometry(...d,2,Math.min(r,...d.map(x=>x*.2)))),pos,m);
 const cyl=(p,name,r,h,pos,m=M.steel,axis='y',r2=r)=>{const o=mesh(p,name,geo('c'+[r,r2,h,simple],()=>new T.CylinderGeometry(r2,r,h,simple?12:32)),pos,m);if(axis==='x')o.rotation.z=pi/2;if(axis==='z')o.rotation.x=pi/2;return o;};
 const rod=(p,name,a,b,r=.012,m=M.steel)=>{const d=v(b).sub(v(a)),o=cyl(p,name,r,d.length(),v(a).add(v(b)).multiplyScalar(.5).toArray(),m);o.quaternion.setFromUnitVectors(v([0,1,0]),d.normalize());return o;};
 const ring=(p,name,r,t,pos,m=M.dark,axis='z')=>{const o=mesh(p,name,geo('t'+[r,t,simple],()=>new T.TorusGeometry(r,t,8,simple?16:36)),pos,m);if(axis==='x')o.rotation.y=pi/2;if(axis==='y')o.rotation.x=pi/2;return o;};
 const bolt=(p,pos,axis='y')=>{if(simple)return;cyl(p,'Washer',.012,.002,pos,M.steel,axis);const xyz=pos.slice(),i={x:0,y:1,z:2}[axis];xyz[i]+=.005;cyl(p,'SocketHead',.007,.009,xyz,M.dark,axis);xyz[i]+=.005;cyl(p,'Socket',.003,.001,xyz,M.rubber,axis);};
 function profile(p,length,pos,axis='y',w=.065){
  const key='profile'+[length,w,simple];const g=geo(key,()=>{if(simple)return new T.BoxGeometry(w,w,length);const h=w/2,a=w*.09,b=w*.19,s=new T.Shape();const pts=[[-h,-h],[-a,-h],[-a,-h+b],[a,-h+b],[a,-h],[h,-h],[h,-a],[h-b,-a],[h-b,a],[h,a],[h,h],[a,h],[a,h-b],[-a,h-b],[-a,h],[-h,h],[-h,a],[-h+b,a],[-h+b,-a],[-h,-a]];s.moveTo(...pts[0]);pts.slice(1).forEach(p=>s.lineTo(...p));s.closePath();const hole=new T.Path();hole.absarc(0,0,w*.1,0,pi*2,true);s.holes.push(hole);const e=new T.ExtrudeGeometry(s,{depth:length,bevelEnabled:false,curveSegments:8});e.translate(0,0,-length/2);return e;});
  const o=mesh(p,'Extrusion',g,pos,M.al);if(axis==='y')o.rotation.x=-pi/2;if(axis==='x')o.rotation.y=pi/2;return o;
 }
 function cable(p,points,r=.007){if(simple)return;return mesh(p,'ServiceCable',new T.TubeGeometry(new T.CatmullRomCurve3(points.map(v)),32,r,8,false),[0,0,0],M.rubber);}
 function motor(p,pos,axis='z',r=.07){const g=group(p,'Motor'+serial++,pos);if(axis==='x')g.rotation.y=pi/2;cyl(g,'Motor',r,.22,[0,0,0],M.blue,'z');cyl(g,'FanShroud',r*1.07,.035,[0,0,-.12],M.dark,'z');cyl(g,'ReductionGear',r*1.3,.08,[0,0,.15],M.al,'z');cyl(g,'DriveShaft',.018,.09,[0,0,.225],M.steel,'z');box(g,'MotorFoot',[r*2,.022,.26],[0,-r-.016,0],M.dark);box(g,'TerminalBox',[.08,.045,.08],[0,r+.02,0],M.dark);if(!simple)for(let i=0;i<7;i++)box(g,'CoolingFin',[r*1.7,.009,.19],[0,(i-3)*r*.22,0],M.blue,.001);return g;}
 function support(p,x,z,top=.72){cyl(p,'RubberFoot',.055,.03,[x,.015,z],M.rubber);cyl(p,'LevellingScrew',.01,.075,[x,.064,z]);cyl(p,'Locknut',.021,.014,[x,.091,z],M.dark);profile(p,top-.1,[x,(top+.1)/2,z]);box(p,'BaseGusset',[.11,.075,.009],[x,.135,z+.039],M.steel);bolt(p,[x,.14,z+.045],'z');}
 function frame(length=2,width=.95,height=.75){
  const f=group(root,'Chassis');for(const x of [-length/2+.1,length/2-.1])for(const z of [-width/2+.05,width/2-.05])support(f,x,z,height-.035);
  for(const z of [-width/2+.05,width/2-.05])for(const y of [.24,height-.07])profile(f,length-.2,[0,y,z],'x');
  for(const x of [-length/2+.1,length/2-.1])for(const y of [.24,height-.07])profile(f,width-.1,[x,y,0],'z');
  box(f,'ElectricalCabinet',[.48,.33,.20],[length*.12,.48,-width/2+.13],M.blue);box(f,'BackAccessDoor',[.45,.30,.012],[length*.12,.48,-width/2+.022],M.white);cyl(f,'DoorLock',.012,.016,[length*.12+.17,.48,-width/2+.011],M.dark,'z');
  for(let i=0;i<5&&!simple;i++)box(f,'Vent',[.18,.008,.003],[length*.12,.39+i*.022,-width/2+.013],M.dark,.001);
  cable(f,[[length*.12,.43,-width/2+.02],[length*.12-.3,.4,-width/2+.02],[length*.12-.3,.24,-width/2+.04]]);
  return f;
 }
 function label(p,pos,width=.32){const g=new T.PlaneGeometry(width,width/4),uv=g.attributes.uv;for(let i=0;i<uv.count;i++)uv.setY(i,1-uv.getY(i));return mesh(p,'OriginalNameplate',g,pos,M.label);}
 function controls(length,width,top=.75){
  const g=group(root,'OperatorPanel',[length*.35,top,width/2+.1]);rod(g,'HMIStem',[0,-.08,-.1],[0,.23,-.1],.018);box(g,'PanelBack',[.28,.24,.06],[0,.27,0],M.dark);box(g,'Screen',[.22,.12,.008],[0,.30,.034],M.screen);for(let i=0;i<4&&!simple;i++)box(g,'ScreenReadout',[.15,.007,.001],[-.01,.34-i*.024,.039],M.green,.001);cyl(g,'EmergencyStop',.023,.021,[-.075,.19,.046],M.red,'z');cyl(g,'Start',.014,.017,[.015,.19,.044],M.green,'z');cyl(g,'Selector',.012,.017,[.075,.19,.044],M.dark,'z');
  cable(root,[[length*.35,top+.16,width/2],[length*.35,top-.1,width/2],[length*.25,.49,width/2-.03]]);
  box(root,'PanelJunctionBox',[.09,.10,.07],[length*.25,.49,width/2-.03],M.dark);box(root,'JunctionMount',[.04,.23,.012],[length*.25,.58,width/2-.065],M.steel);cyl(root,'PanelCableGland',.011,.026,[length*.25+.048,.49,width/2-.03],M.dark,'x');
  const x=-length*.37,z=-width*.34;box(root,'SignalBracket',[.09,.014,.08],[x,top,z],M.dark);cyl(root,'SignalPost',.009,.24,[x,top+.12,z]);for(let i=0;i<3;i++)cyl(root,'SignalLight',.026,.035,[x,top+.27+i*.04,z],[M.green,M.amber,M.red][i]);
  label(root,[0,top-.12,width/2+.015]);
 }
 function sensor(x,y,z){box(root,'PhotocellBracket',[.03,.07,.05],[x,y-.027,z],M.steel);box(root,'Photoeye',[.025,.04,.025],[x,y,z],M.dark);cyl(root,'PhotoeyeLens',.007,.004,[x,y,z+.014],M.red,'z');bolt(root,[x,y-.045,z+.028],'z');}
 function bed(length=2,width=.85,y=.79,rollers=false,withBase=true){
  if(withBase){frame(length,width,y-.03);controls(length,width,y-.03);}
  for(const z of [-width/2,width/2])box(root,'ConveyorSide',[length,.11,.045],[0,y-.04,z],M.white);
  for(const x of [-length/2+.08,length/2-.08]){cyl(root,'EndRoller',.055,width,[x,y-.025,0],M.steel,'z');for(const z of [-width/2-.045,width/2+.045])box(root,'PillowBlock',[.105,.10,.045],[x,y-.025,z],M.dark);}
  const drive=group(root,rollers?'DriveRoller':'BeltDrive',[0,y,0]);
  if(rollers){drive.name='RollerDriveFrame';for(let i=0;i<Math.ceil(length/.14);i++){const roller=group(drive,i?'DriveRoller'+i:'DriveRoller',[-length/2+.1+i*.14,-.02,0]);cyl(roller,'PoweredRoller',.042,width-.04,[0,0,0],M.steel,'z');box(roller,'RollerIndex',[.012,.004,width-.05],[0,.043,0],M.dark,.001);}}
  else {box(root,'BeltTop',[length-.15,.014,width-.06],[0,y+.018,0],M.belt);box(root,'BeltReturn',[length-.15,.014,width-.06],[0,y-.084,0],M.rubber);for(let i=0;i<9;i++)box(drive,'BeltCleat',[.015,.004,width-.08],[-length*.43+i*length*.1,.028,0],M.dark,.001);}
  motor(root,[-length/2+.1,y-.03,-width/2-.18]);sensor(length*.34,y+.09,width/2-.015);return drive;
 }
 function gantry(length=1.6,width=.9,top=1.75){for(const z of [-width/2,width/2])profile(root,top-.75,[0,(top+.75)/2,z]);profile(root,width+.1,[0,top,0],'z',.09);box(root,'AxisRail',[length,.03,.04],[0,top-.08,0],M.steel);for(const x of [-length/2,length/2])box(root,'RailSupport',[.10,.13,.13],[x,top-.08,0],M.dark);motor(root,[length/2+.12,top-.08,0],'x',.05);}
 const clips=[];
 function move(name,axis,values){const o=root.getObjectByName(name);const base=o.position.toArray();return new T.VectorKeyframeTrack(name+'.position',[0,.2,.5,.8,1],values.flatMap(n=>base.map((b,i)=>i===axis?b+n:b)));}
 function rotate(name,axis,angles){return new T.QuaternionKeyframeTrack(name+'.quaternion',[0,.2,.5,.8,1],angles.flatMap(a=>new T.Quaternion().setFromAxisAngle(v(axis),a).toArray()));}
 function cycle(tracks){
 for(const phase of Object.keys(tracks)){
  const groups=new Map();for(const track of tracks[phase]){if(!groups.has(track.name))groups.set(track.name,[]);groups.get(track.name).push(track);}
  tracks[phase]=[...groups].map(([name,rows])=>{
   if(rows.length===1)return rows[0];if(!name.endsWith('.position'))throw Error('Unmerged duplicate '+name);
   const rest=root.getObjectByName(name.slice(0,-9)).position.toArray();
   for(const tr of rows)if(tr.times.length!==rows[0].times.length||tr.times.some((v,i)=>v!==rows[0].times[i]))throw Error('Mismatched times '+name);
   return new T.VectorKeyframeTrack(name,rows[0].times,Array.from(rows[0].values,(_,i)=>rest[i%3]+rows.reduce((s,tr)=>s+tr.values[i]-rest[i%3],0)));
  });
 }
 clips.push(new T.AnimationClip('load',1,tracks.load??[]),new T.AnimationClip('auto',1,tracks.auto),new T.AnimationClip('unload',1,tracks.unload??[]));
}
 const repetitive=(name,axis=[0,0,1])=>rotate(name,axis,[0,pi/2,pi,pi*1.5,pi*2]);
 const press=(name,stroke=.13,axis=1)=>move(name,axis,[0,0,-stroke,-stroke,0]);
 let length=2,width=.95,workY=.82,cargo='WorkpieceSeat';
 const f=spec.family;
 if(['assembly','rework','packing'].includes(f)){
  length=1.55;width=.95;frame(length,width);box(root,'Worktop',[length,.035,width],[0,.7675,0],M.steel);controls(length,width);box(root,'Fixture',[.52,.026,.4],[0,.798,0],M.dark);for(const x of [-.22,.22])for(const z of [-.15,.15]){cyl(root,'Locator',.008,.045,[x,.83,z]);bolt(root,[x,.815,z]);}
  if(f==='packing'){for(const x of [-.33,.33])profile(root,.48,[x,1.02,-.25]);cyl(root,'FoldHinge',.012,.77,[0,1.25,-.25],M.steel,'x');const gate=group(root,'PackingGate',[0,1.25,-.25]);box(gate,'FoldingPlate',[.61,.015,.42],[0,0,.21],M.blue);for(const x of [-.36,.36])box(root,'HingeBearing',[.07,.08,.07],[x,1.25,-.25],M.dark);motor(root,[-.5,1.25,-.25],'x',.06);cycle({auto:[rotate('PackingGate',[1,0,0],[-pi/2,-pi/2,0,0,-pi/2])]});}
  else{gantry(1,.75,1.72);const xc=group(root,'XCarriage',[-.103,1.6,0]);box(xc,'XBlock',[.2,.18,.08],[0,0,0],M.dark);box(xc,'YLinearRail',[.05,.04,.42],[0,-.03,0],M.steel);cyl(xc,'YLeadScrew',.008,.40,[.055,0,0],M.steel,'z');const yc=group(xc,'YCarriage',[0,0,-.073]);box(yc,'YBlock',[.15,.05,.10],[0,-.04,0],M.al);const zc=group(yc,'ToolHead',[0,-.34,0]);cyl(zc,'ElectricSpindle',.04,.21,[0,0,0],M.blue);cyl(zc,'Collet',.018,.05,[0,-.13,0]);cyl(zc,'DriverBit',.003,.08,[0,-.195,0],M.dark);group(zc,'ToolTip',[0,-.235,0]);for(const z of [-.06,.06])cyl(yc,'GuideRod',.008,.52,[0,-.25,z]);box(root,'JigClamp',[.12,.065,.06],[.24,.84,.11],M.al);if(f==='rework')for(let i=0;i<4;i++)cyl(root,'RepairToolSocket',.012,.025,[-.6+i*.06,.803,.17],M.dark);
   const time=[],xs=[],ys=[],zs=[],corners=[[-.103,-.073],[.103,-.073],[.103,.073],[-.103,.073]];
   for(let i=0;i<=320;i++){const t=i/320,q=Math.min(3,Math.floor(t*4)),u=t*4-q,from=corners[q],to=corners[(q+1)%4],m=Math.min(1,u/.2);time.push(t);xs.push(from[0]+(to[0]-from[0])*m,1.6,0);ys.push(0,0,from[1]+(to[1]-from[1])*m);const down=u<.25||u>.8?0:.144*Math.min(1,(u-.25)/.12,(.8-u)/.12);zs.push(0,-.34-down,0);}
   cycle({auto:[new T.VectorKeyframeTrack('XCarriage.position',time,xs),new T.VectorKeyframeTrack('YCarriage.position',time,ys),new T.VectorKeyframeTrack('ToolHead.position',time,zs)]});}
 }
 else if(['belt','rollers','uv','shrink','compression','sealer','marker','labeler','glue','inspection','weigher','coater','laminator','flute','window','printer','offset','rotary','slitter','scorer','folder','crash','stacker','strapper'].includes(f)){
  length=['folder','crash'].includes(f)?3.6:['printer','offset','flute'].includes(f)?3.2:2.15;width=['printer','offset','flute'].includes(f)?1.25:.95;
  bed(length,width,.79,f==='rollers');const rolling=f==='rollers'?'DriveRoller':'BeltDrive';let action=[move(rolling,0,[0,.035,.075,.11,.14])];
  if(f==='rollers'){action=[];root.traverse(o=>{if(o.isGroup&&o.name.startsWith('DriveRoller'))action.push(repetitive(o.name));});}
  if(['uv','shrink'].includes(f)){
   for(const z of [-width/2,width/2])box(root,'TunnelWall',[length*.64,.64,.06],[0,1.12,z],M.blue);box(root,'TunnelRoof',[length*.64,.06,width+.06],[0,1.47,0],M.white);for(const x of [-.35,.35]){cyl(root,'ExhaustFan',.13,.07,[x,1.54,0],M.dark);for(let j=-3;j<=3&&!simple;j++)box(root,'FanGrille',[.24,.005,.007],[x,1.585,j*.03],M.al,.001);}
   for(const x of [-length*.30,length*.30])for(const z of [-width*.27,0,width*.27])box(root,'EntryCurtain',[.025,.38,.19],[x,1.08,z],M.rubber);
   if(f==='uv')for(const x of [-.4,0,.4])box(root,'UVLamp',[.10,.035,width-.12],[x,1.40,0],M.lens);
   else{const jaw=group(root,'SealJaw',[-length*.4,1.19,0]);box(jaw,'LSealBar',[.06,.05,width],[0,0,0],M.amber);for(const z of [-width/2,width/2])profile(root,.54,[-length*.4,1.06,z]);action.push(press('SealJaw',.24));}
  }
  if(['coater','laminator','flute','window','printer','offset','rotary','slitter','scorer'].includes(f)){
   const count=f==='printer'?2:f==='offset'?3:1;const node={coater:'CoatingRoller',laminator:'PressureRoller',flute:'LaminationNip',window:'FilmApplicator',printer:'PrintRoller',offset:'BlanketCylinder',rotary:'DieCylinder',slitter:'KnifeShaft',scorer:'CreaseShaft'}[f];
   for(let i=0;i<count;i++){const x=(i-(count-1)/2)*.9;for(const z of [-width/2-.02,width/2+.02]){box(root,'BearingTower',[.34,.66,.10],[x,1.10,z],M.blue);box(root,'BearingCap',[.18,.18,.035],[x,1.28,z+.07],M.dark);bolt(root,[x,1.28,z+.091],'z');}const roll=group(root,i?node+i:node,[x,1.1,0]);cyl(roll,'UpperCylinder',.22,width-.07,[0,0,0],f==='rotary'?M.amber:M.steel,'z');cyl(root,'CounterRoller',.12,width-.04,[x,.81,0],M.rubber,'z');motor(root,[x,1.1,-width/2-.28]);if(i===0)action.push(repetitive(node));
    if(['printer','offset','coater'].includes(f)){box(root,'InkPan',[.38,.07,width-.05],[x,1.43,0],M.blue);cyl(root,'InkMeteringRoller',.06,width-.06,[x-.19,1.30,0],M.dark,'z');}
    if(['slitter','scorer'].includes(f))for(let k=-2;k<=2;k++){cyl(roll,'ToolDisc',f==='slitter'?.245:.23,.018,[0,0,k*.14],M.dark,'z');ring(roll,'Hub',.05,.012,[0,0,k*.14],M.al);}
    if(f==='rotary')for(let k=0;k<8;k++){const b=box(roll,'DieKnife',[.012,.027,width*.65],[.225*Math.sin(k*pi/4),.225*Math.cos(k*pi/4),0],M.dark,.001);b.rotation.z=-k*pi/4;}
   }
   if(['laminator','flute','window'].includes(f)){for(const z of [-width/2,width/2])profile(root,1.15,[-.55,1.32,z]);cyl(root,'FilmReelCore',.07,width+.15,[-.55,1.77,0],M.steel,'z');cyl(root,'FilmRoll',.17,width-.15,[-.55,1.77,0],M.white,'z');box(root,'FilmWeb',[.012,.64,width-.19],[-.47,1.38,0],M.lens,.001);}
   if(f==='flute'){box(root,'UpperFeedDeck',[.9,.035,width-.05],[-.85,1.05,0],M.belt);for(const z of [-width*.4,width*.4])rod(root,'UpperFeedBrace',[-1.1,.73,z],[-.6,1.05,z],.018);}
  }
  if(['folder','crash'].includes(f)){
   const left=group(root,'FoldLeft',[0,.85,-.28]),right=group(root,'FoldRight',[0,.85,.28]);box(left,'FoldingBelt',[1.7,.06,.14],[0,0,-.08],M.belt);box(right,'FoldingBelt',[1.7,.06,.14],[0,0,.08],M.belt);for(const x of [-.68,.68])for(const z of [-.44,.44]){rod(root,'FoldingSupport',[x,.68,z],[x,.94,z],.019);cyl(root,'GuideWheel',.045,.045,[x,.94,z],M.rubber,'z');}
   for(const z of [-.39,.39])rod(root,'FoldRail',[-1.15,.9,z],[1.1,.94,z*.3],.015,M.steel);const glue=group(root,'GlueGun',[-.5,1.0,.36]);cyl(glue,'GlueBody',.025,.14,[0,0,0],M.amber);cyl(glue,'GlueTip',.005,.06,[0,-.1,0],M.steel);rod(root,'GlueSupport',[-.5,.7,.43],[-.5,1.12,.36],.017);cable(root,[[-.5,1.06,.36],[-.7,1.15,.4],[-.9,.65,.4]],.01);
   action.push(rotate('FoldLeft',[1,0,0],[0,0,pi*.48,pi*.48,0]),rotate('FoldRight',[1,0,0],[0,0,-pi*.48,-pi*.48,0]));
   if(f==='crash'){const hook=group(root,'BottomHook',[-.7,.90,0]);rod(hook,'HookArm',[0,0,0],[.18,0,.18],.012,M.amber);action.push(rotate('BottomHook',[0,1,0],[0,0,pi*.7,pi*.7,0]));}
   for(const z of [-.3,.3])box(root,'CompressionRail',[1,.05,.11],[1.15,.97,z],M.dark);
  }
  if(f==='compression'){const p=group(root,'PressBelt',[0,1.15,0]);box(p,'UpperCompressionBelt',[1.6,.12,.72],[0,0,0],M.belt);for(const x of [-.65,.65])for(const z of [-.41,.41]){cyl(root,'SpringPost',.012,.48,[x,1.14,z]);cyl(root,'SpringCollar',.035,.12,[x,1.32,z],M.dark);}action.push(press('PressBelt',.16));}
  if(['marker','labeler','glue','inspection','weigher'].includes(f)){
   const node={marker:'PrintHead',labeler:'TampPad',glue:'GlueNozzle',inspection:'OpticalHead',weigher:'WeighDeck'}[f];
   if(f==='weigher'){const deck=group(root,node,[0,.82,0]);box(deck,'WeighPlatform',[.62,.016,.73],[0,0,0],M.steel);for(const x of [-.22,.22])box(root,'LoadCell',[.12,.045,.05],[x,.79,0],M.dark);action.push(press(node,.006));}
   else{gantry(1.2,width,1.67);const head=group(root,node,[0,1.3,0]);box(head,'ProcessHead',[.16,.18,.14],[0,0,0],f==='glue'?M.amber:M.blue);cyl(head,f==='inspection'?'Lens':'Nozzle',.03,.07,[0,-.12,0],f==='inspection'?M.lens:M.steel);if(f==='inspection'){ring(head,'LightRing',.10,.016,[0,-.17,0],M.white,'y');for(const x of [-.07,.07])rod(head,'RingMount',[x,-.04,0],[x,-.17,0],.004);}if(f==='labeler'){cyl(root,'LabelRoll',.14,.13,[-.33,1.35,0],M.white,'z');box(head,'LabelShoe',[.23,.02,.20],[0,-.15,0],M.white);}action.push(press(node,f==='inspection'?.035:.23));}
  }
  if(f==='sealer'){for(const z of [-.4,.4])profile(root,.8,[0,1.15,z]);profile(root,width,[0,1.5,0],'z');const head=group(root,'TapeHead',[0,1.15,0]);box(head,'TapeHeadFrame',[.38,.18,.16],[0,0,0],M.blue);cyl(head,'TapeRoll',.15,.06,[0,.26,0],M.amber,'z');cyl(head,'TapeCore',.055,.07,[0,.26,0],M.dark,'z');for(const x of [-.18,.18])cyl(head,'PressureWheel',.045,.16,[x,-.09,0],M.rubber,'z');for(const z of [-.32,.32])box(root,'SideDriveBelt',[1.1,.16,.08],[0,.96,z],M.belt);action.push(press('TapeHead',.055));cargo='CartonSeat';}
  if(f==='stacker'){for(const z of [-.43,.43])profile(root,.5,[.65,1.04,z]);const stop=group(root,'StackStop',[.72,1.02,0]);box(stop,'StopPlate',[.035,.35,.75],[0,0,0],M.blue);action.push(press('StackStop',.14));}
  if(f==='strapper'){for(const z of [-.52,.52])profile(root,1.0,[0,1.25,z],'y',.09);profile(root,1.12,[0,1.77,0],'z',.09);for(const z of [-.52,.52])box(root,'StrapTrack',[.045,.85,.025],[0,1.29,z],M.amber);box(root,'StrapTrackTop',[.045,.025,1.04],[0,1.72,0],M.amber);const head=group(root,'StrapHead',[0,.95,.35]);box(head,'StrapWelder',[.25,.18,.17],[0,0,0],M.dark);cyl(root,'StrapReel',.22,.12,[.64,.35,.59],M.amber,'z');rod(root,'ReelStand',[.64,.12,.59],[.64,.36,.59],.025);action.push(move('StrapHead',2,[0,0,-.08,-.08,0]));}
  cycle({auto:action});
 }
 else if(['platen','stripper','press','tray','rigid','erector','stitcher','feeder','lift','cutter','mill','lathe','drill','gantry','palletizer','casepacker','robot','scara','transfer','liftconveyor','curve','wrapper'].includes(f)){
  // Category-specific mechanisms continue below.
  buildSpecial();
 }
 else throw Error('Missing equipment geometry: '+f);

 function buildSpecial(){
  if(['platen','stripper','press','tray'].includes(f)){
   length=f==='platen'?2.4:1.75;width=1.25;frame(length,width);box(root,'LowerToolTable',[length-.15,.10,width-.12],[0,.80,0],M.steel);for(const x of [-.65,.65])for(const z of [-.46,.46]){cyl(root,'GuideColumn',.035,1.4,[x,1.40,z]);box(root,'ColumnMount',[.15,.10,.15],[x,.88,z],M.dark);bolt(root,[x+.045,.935,z]);}box(root,'PressCrown',[1.55,.22,1.12],[0,2.04,0],M.blue);cyl(root,'HydraulicRam',.13,.38,[0,1.76,0],M.al);const node={platen:'MovingPlaten',stripper:'StrippingPlate',press:'MovingPlaten',tray:'FormingMandrel'}[f],head=group(root,node,[0,1.48,0]);box(head,'MovingToolPlate',[1.4,.1,1.0],[0,0,0],f==='stripper'?M.amber:M.steel);cyl(head,'RamPiston',.07,.66,[0,.32,0],M.steel);
   if(f==='stripper')for(const x of [-.5,-.25,0,.25,.5])for(const z of [-.3,0,.3])cyl(head,'StrippingPin',.006,.17,[x,-.13,z],M.dark);
   if(f==='tray'){box(head,'FormingPlug',[.54,.22,.40],[0,-.15,0],M.blue);for(const x of [-.34,.34])box(root,'TrayMoldWall',[.07,.18,.56],[x,.94,0],M.dark);for(const z of [-.27,.27])box(root,'TrayMoldWall',[.75,.18,.07],[0,.94,z],M.dark);}
   else box(root,'LowerDie',[1.25,.055,.86],[0,.88,0],M.dark);
   controls(length,width);cycle({auto:[press(node,f==='tray'?.33:f==='stripper'?.35:.50)]});
  }
  if(['feeder','lift','liftconveyor'].includes(f)){
   length=1.8;width=1.2;frame(length,width);for(const x of [-.72,.72])for(const z of [-.48,.48])profile(root,1.6,[x,.9,z]);const deck=group(root,'LiftDeck',[0,.42,0]);box(deck,'Platform',[1.45,.08,1.02],[0,0,0],M.blue);for(const x of [-.73,.73])cyl(root,'LiftScrew',.02,1.35,[x,.9,0]);
   for(const z of [-.5,.5]){rod(root,'LiftBrace',[-.7,.2,z],[.7,.4,z],.018);rod(root,'LiftBrace',[.7,.2,z],[-.7,.4,z],.018);}motor(root,[.73,.26,-.68]);
   if(f==='feeder'){for(const z of [-.49,.49]){profile(root,1.04,[0,1.25,z]);profile(root,1.6,[0,1.78,z],'x');}profile(root,1.05,[0,1.78,0],'z');const slide=group(root,'SuctionX');for(const z of [-.46,.46])box(slide,'XGuideBlock',[.16,.07,.09],[0,1.78,z],M.dark);const beam=group(slide,'SuctionBeam',[0,1.52,0]);box(beam,'SuctionBar',[.8,.045,.06],[0,0,0],M.al);for(const x of [-.28,0,.28])cyl(beam,'VacuumCup',.045,.035,[x,-.05,0],M.rubber);cyl(beam,'SuctionPiston',.012,.22,[0,.1,0]);cyl(slide,'SuctionActuator',.035,.24,[0,1.69,0]);controls(length,width);cycle({load:[move('LiftDeck',1,[0,.1,.15,.15,.15])],auto:[press('SuctionBeam',.15),move('SuctionX',0,[0,0,.4,.6,0])],unload:[move('LiftDeck',1,[.15,.15,.15,.08,0])]});}
   else{if(f==='liftconveyor')for(let i=0;i<9;i++)cyl(deck,'DeckRoller',.04,.96,[-.64+i*.16,.06,0],M.steel,'z');controls(length,width);cycle({load:[move('LiftDeck',1,[0,0,0,0,0])],auto:[move('LiftDeck',1,[0,.1,.7,.7,.7])],unload:[move('LiftDeck',1,[.7,.7,.7,.3,0])]});}cargo='LiftDeck';workY=.46;
  }
  if(f==='drill'){
   length=1.2;width=.9;workY=.915;frame(length,width);box(root,'DrillBase',[.9,.10,.70],[0,.80,0],M.dark);cyl(root,'DrillColumn',.08,1.1,[.27,1.34,-.13]);box(root,'DrillHeadCasting',[.48,.28,.40],[.06,1.82,-.13],M.blue);box(root,'BeltGuard',[.65,.14,.48],[.15,2.03,-.13],M.white);motor(root,[.43,1.8,-.41]);cyl(root,'DrillWorkTable',.32,.06,[-.13,.885,-.13],M.steel);box(root,'WorkVice',[.28,.065,.20],[-.13,.95,-.13],M.dark);const head=group(root,'ToolHead',[-.13,1.57,-.13]);cyl(head,'Quill',.03,.27,[0,0,0]);cyl(head,'DrillChuck',.045,.075,[0,-.17,0],M.dark);cyl(head,'DrillBit',.007,.11,[0,-.255,0],M.steel);for(let i=0;i<3;i++){const angle=i*pi*2/3;rod(root,'FeedHandle',[.25,1.76,.12],[.25+.19*Math.cos(angle),1.76+.19*Math.sin(angle),.12],.008,M.dark);}controls(length,width);cycle({auto:[press('ToolHead',.18)]});
  }
  if(['cutter','mill','gantry','palletizer','casepacker'].includes(f)){
   length=f==='cutter'?2.6:f==='palletizer'?2.8:1.9;width=f==='cutter'?1.65:f==='palletizer'?1.9:1.15;frame(length,width);box(root,'MachineBed',[length,.07,width],[0,.78,0],f==='cutter'?M.dark:M.steel);const top=f==='palletizer'?2.6:1.9;
   for(const x of [-length*.42,length*.42])for(const z of [-width*.43,width*.43])profile(root,top-.8,[x,(top+.8)/2,z],'y',.09);
   for(const z of [-width*.43,width*.43]){profile(root,length*.9,[0,top,z],'x',.09);box(root,'LinearRail',[length*.84,.025,.03],[0,top+.05,z],M.steel);}
   const xc=group(root,'XCarriage',[-length*.22,top,0]);profile(xc,width*.92,[0,0,0],'z',.09);const head=group(xc,['gantry','palletizer','casepacker'].includes(f)?'GraspCenter':'ToolHead',[0,-.36,-width*.2]);
   box(head,'ZCarriage',[.17,.25,.09],[0,0,0],M.blue);for(const z of [-.06,.06])cyl(head,'ZGuide',.012,.35,[0,-.12,z]);
   if(['cutter','mill','drill'].includes(f)){cyl(head,'Spindle',.055,.22,[0,-.15,0],M.blue);cyl(head,'Collet',.02,.055,[0,-.29,0]);cyl(head,'Tool',.006,.09,[0,-.36,0],M.dark);if(f==='cutter'){cyl(head,'CreaseWheel',.03,.018,[.09,-.37,0],M.steel,'z');box(root,'VacuumBed',[length-.16,.015,width-.16],[0,.825,0],M.rubber);for(let i=0;i<8&&!simple;i++)box(root,'VacuumZone',[length-.2,.001,.005],[0,.834,-width*.4+i*width*.11],M.dark,.001);}else{box(root,'MachineVice',[.55,.12,.34],[0,.875,0],M.dark);cyl(root,'ViceScrew',.018,.55,[0,.94,0],M.steel,'x');}}
   else{const node=f==='casepacker'?'PackingHead':'GraspCenter';if(node==='PackingHead')head.name=node;for(const x of [-.20,.20]){cyl(head,'VacuumPad',.05,.035,[x,-.30,0],M.rubber);cyl(head,'VacuumStem',.012,.22,[x,-.17,0]);}box(head,'VacuumManifold',[.58,.06,.08],[0,-.07,0],M.blue);cargo=node;}
   if(f==='mill'){for(const z of [-width/2,width/2])box(root,'MachineEnclosure',[length,.9,.04],[0,1.28,z],M.white);box(root,'BackEnclosure',[.05,1,width],[length/2,1.30,0],M.blue);for(const z of [-.2,.2])box(root,'FrontDoorWindow',[.03,.47,.35],[-length/2-.02,1.45,z],M.lens);}
   controls(length,width);const node=head.name;cycle({load:[move(node,1,[0,0,-.10,-.10,0])],auto:[move('XCarriage',0,[0,length*.25,length*.4,0,0]),move(node,2,[0,0,width*.32,width*.32,0]),press(node,f==='palletizer'?.5:.24)],unload:[move(node,1,[0,-.12,-.12,0,0])]});
  }
  if(f==='lathe'){
   length=2.5;width=1.2;frame(length,width);box(root,'LatheBed',[2.25,.20,.70],[0,.85,0],M.dark);box(root,'Headstock',[.55,.70,.65],[-.8,1.20,0],M.blue);const chuck=group(root,'LatheChuck',[-.46,1.36,0]);cyl(chuck,'ChuckBody',.20,.13,[0,0,0],M.steel,'x');for(let i=0;i<3;i++){const jaw=box(chuck,'ChuckJaw',[.11,.09,.08],[.085,.11*Math.cos(i*pi*2/3),.11*Math.sin(i*pi*2/3)],M.dark);jaw.rotation.x=i*pi*2/3;}const slide=group(root,'ToolSlide',[.1,1.03,.24]);box(slide,'CrossSlide',[.44,.13,.42],[0,0,0],M.al);box(slide,'ToolHolder',[.11,.15,.11],[-.1,.13,0],M.dark);box(root,'Tailstock',[.35,.32,.38],[.8,1.10,0],M.blue);cyl(root,'TailstockQuill',.038,.42,[.54,1.36,0],M.steel,'x');box(root,'BackSplash',[2.3,.9,.06],[0,1.28,-.49],M.white);box(root,'LowerCoolantTray',[2.3,.08,.85],[0,.7,0],M.al);controls(length,width);cycle({auto:[repetitive('LatheChuck',[1,0,0]),move('ToolSlide',0,[0,-.2,-.35,-.2,0])]});
  }
  if(f==='robot'||f==='scara'){
   length=1.5;width=1.15;box(root,'RobotPedestal',[.65,.18,.65],[0,.09,0],M.dark);for(const x of [-.25,.25])for(const z of [-.25,.25])bolt(root,[x,.188,z]);const j1=group(root,'J1',[0,.24,0]);cyl(j1,'BaseSwivel',.23,.30,[0,.15,0],M.blue);
   const j2=group(j1,'J2',[0,.40,0]);cyl(j2,'ShoulderBearing',.15,.32,[0,0,0],M.white,'z');
   if(f==='robot'){box(j2,'UpperArm',[.22,.67,.20],[0,.32,0],M.white);const j3=group(j2,'J3',[0,.67,0]);cyl(j3,'Elbow',.14,.28,[0,0,0],M.white,'z');box(j3,'Forearm',[.62,.18,.17],[.28,0,0],M.white);const j4=group(j3,'J4',[.61,0,0]);cyl(j4,'WristRoll',.09,.16,[.04,0,0],M.blue,'x');const j5=group(j4,'J5',[.13,0,0]);cyl(j5,'WristPitch',.08,.16,[0,0,0],M.white,'z');const j6=group(j5,'J6',[.10,0,0]);cyl(j6,'ToolFlange',.065,.06,[.02,0,0],M.dark,'x');const tool=group(j6,'GraspCenter',[.12,-.10,0]);box(tool,'GripperBlock',[.12,.08,.28],[0,.06,0],M.blue);for(const z of [-.12,.12])box(tool,'GripperFinger',[.06,.15,.025],[0,-.04,z],M.dark);cycle({load:[rotate('J2',[0,0,1],[0,-.25,-.4,-.4,-.4])],auto:[rotate('J1',[0,1,0],[0,.3,.8,1.1,1.1]),rotate('J2',[0,0,1],[-.4,-.15,0,-.3,-.4]),rotate('J3',[0,0,1],[0,.25,.45,.25,0]),rotate('J5',[0,0,1],[0,-.2,-.4,-.2,0])],unload:[rotate('J1',[0,1,0],[1.1,1.1,.8,.3,0]),rotate('J2',[0,0,1],[-.4,-.3,-.15,0,0])]});}
   else{box(j2,'ScaraUpperArm',[.66,.16,.20],[.28,0,0],M.white);const j3=group(j2,'J3',[.6,0,0]);cyl(j3,'ScaraElbow',.12,.22,[0,0,0],M.blue);box(j3,'ScaraForearm',[.57,.14,.17],[.24,.03,0],M.white);const tip=group(j3,'GraspCenter',[.52,-.2,0]);cyl(tip,'Quill',.025,.42,[0,.12,0],M.steel);box(tip,'VacuumTool',[.18,.04,.16],[0,-.1,0],M.blue);for(const x of [-.06,.06])cyl(tip,'VacuumCup',.025,.03,[x,-.135,0],M.rubber);cycle({load:[press('GraspCenter',.08)],auto:[rotate('J2',[0,1,0],[0,.3,.8,.8,.8]),rotate('J3',[0,1,0],[0,-.4,-.8,-.8,-.8]),press('GraspCenter',.1)],unload:[rotate('J2',[0,1,0],[.8,.8,.4,.2,0]),rotate('J3',[0,1,0],[-.8,-.8,-.4,-.2,0])]});}
   const cabinet=group(root,'RobotController',[-.63,.39,-.35]);box(cabinet,'ControlBox',[.40,.70,.38],[0,0,0],M.blue);label(cabinet,[0,.10,.195]);cable(root,[[-.43,.4,-.35],[-.25,.20,-.35],[0,.20,-.22]],.018);cargo='GraspCenter';workY=1.2;
  }
  if(f==='erector'||f==='rigid'||f==='stitcher'){
   length=2.2;width=1.2;frame(length,width);box(root,'Worktop',[length,.04,width],[0,.79,0],M.steel);for(const x of [-.8,.8])for(const z of [-.48,.48])profile(root,1.2,[x,1.38,z]);profile(root,1.8,[0,1.98,-.48],'x');
   if(f==='erector'){for(const z of [-.43,.43])box(root,'BlankMagazine',[.7,.45,.03],[-.65,1.15,z],M.blue);const arm=group(root,'VacuumArm',[-.1,1.3,-.1]);rod(arm,'OpeningArm',[0,0,0],[.45,0,0],.025,M.al);for(const z of [-.15,.15])cyl(arm,'OpeningCup',.05,.045,[.45,0,z],M.rubber,'x');cyl(root,'ArmDrive',.09,.12,[-.1,1.3,-.20],M.blue,'z');for(const z of [-.3,.3])box(root,'LowerFlapGuide',[.7,.08,.04],[.5,.86,z],M.amber);cycle({auto:[rotate('VacuumArm',[0,1,0],[0,0,pi/2,pi/2,0])]});}
   if(f==='rigid'){const corner=group(root,'CornerFolder',[0,.9,0]);for(const x of [-.35,.35])for(const z of [-.25,.25]){const a=box(corner,'CornerPad',[.13,.18,.13],[x,.09,z],M.amber);a.rotation.y=pi/4;}box(root,'BoxNest',[.5,.06,.35],[0,.85,0],M.dark);cycle({auto:[move('CornerFolder',1,[0,0,.10,.10,0])]});}
   if(f==='stitcher'){const head=group(root,'StitchHead',[.2,1.5,0]);box(head,'WireDriver',[.20,.32,.22],[0,0,0],M.blue);box(head,'StapleAnvil',[.04,.12,.05],[0,-.20,0],M.steel);cyl(root,'WireReel',.18,.06,[.2,1.85,-.25],M.steel,'z');rod(root,'StitcherThroat',[.2,.80,-.25],[.2,1.62,-.25],.05,M.dark);cycle({auto:[press('StitchHead',.18)]});}controls(length,width);
  }
  if(f==='transfer'){
   bed(1.65,1.2,.79,true);const deck=group(root,'PopUpDeck',[0,.80,0]);for(const x of [-.36,0,.36])box(deck,'CrossBelt',[.07,.035,1.08],[x,0,0],M.belt);for(const z of [-.35,.35])cyl(root,'PopCylinder',.03,.25,[0,.56,z]);const tracks=[move('PopUpDeck',1,[0,0,.055,.055,0])];root.traverse(o=>{if(o.isGroup&&o.name.startsWith('DriveRoller'))tracks.push(repetitive(o.name));});cycle({auto:tracks});
  }
  if(f==='curve'){
   length=2.4;width=2.4;for(const x of [-.8,.8])for(const z of [-.8,.8])support(root,x,z,.74);const drive=group(root,'TurnRoller'),tracks=[];for(let i=0;i<14;i++){const a=i*pi/26,g=group(drive,'RadialRoller'+i,[1.1*Math.sin(a)-.65,.79,1.1*Math.cos(a)-.65]);const rotating=group(g,'CurveRollSpin'+i);cyl(rotating,'TaperedRoller',.045,.62,[0,0,0],M.steel,'x',.025);box(rotating,'RollerIndex',[.60,.004,.01],[0,.04,0],M.dark,.001);g.rotation.y=-a;for(const sign of [-1,1])box(g,'BearingBlock',[.05,.10,.1],[sign*.34,-.025,0],M.dark);tracks.push(repetitive(rotating.name,[1,0,0]));}for(const radius of [.75,1.45])for(let i=0;i<16;i++){const a=i*pi/32,b=(i+1)*pi/32;rod(root,'CurveRail',[radius*Math.sin(a)-.65,.86,radius*Math.cos(a)-.65],[radius*Math.sin(b)-.65,.86,radius*Math.cos(b)-.65],.018);}motor(root,[-.65,.70,.12]);controls(1.8,1.5);cycle({auto:tracks});
  }
  if(f==='wrapper'){
   length=2.5;width=1.9;box(root,'WrapperBase',[length,.10,width],[0,.05,0],M.dark);const turn=group(root,'Turntable',[.4,.15,0]);cyl(turn,'TurntableDeck',.83,.10,[0,0,0],M.blue);cyl(turn,'TurntableHub',.12,.13,[0,-.02,0],M.dark);profile(root,2.65,[-.95,1.43,0],'y',.16);box(root,'MastBack',[.12,2.6,.25],[-1.05,1.43,0],M.blue);cyl(root,'CarriageScrew',.018,2.3,[-.91,1.4,0]);const carriage=group(root,'FilmCarriage',[-.72,.55,0]);box(carriage,'FilmFrame',[.30,.66,.36],[0,0,0],M.dark);cyl(carriage,'StretchFilmRoll',.14,.51,[.21,0,0],M.white);cyl(carriage,'TensionRoller',.045,.54,[.12,0,.21],M.steel);motor(root,[-1,.25,-.34]);controls(2,1.7,.75);cargo='Turntable';workY=.20;cycle({auto:[repetitive('Turntable',[0,1,0]),move('FilmCarriage',1,[0,.2,1.25,.6,0])]});
  }
 }
 if(!root.getObjectByName('WorkpieceSeat'))group(root,'WorkpieceSeat',[0,workY,0]);
 if(!root.getObjectByName(cargo))cargo='WorkpieceSeat';
 for(const [name,point]of Object.entries({input:[-length/2,workY,0],output:[length/2,workY,0],operator:[length*.3,0,width/2+.7]}))group(root,'Port_'+name,point);
 if(spec.rule==='inspect')group(root,'Port_rejected',[length*.25,workY,-width/2]);
 if(spec.rule==='pack')group(root,'Port_container',[-length*.2,workY,width/2]);
 if(spec.rule==='assemble')group(root,'Port_component',[-length*.2,workY,width/2]);
 if(['flexo-printer','offset-printer','varnish-coater','film-laminator','flute-laminator','window-patcher','folder-gluer','crash-lock-gluer','carton-stitcher','adhesive-dispenser','strapper','shrink-wrapper','tape-case-sealer','label-applicator','pallet-stretch-wrapper'].includes(spec.id))group(root,'Port_auxiliary',[0,workY,-width/2]);
 root.updateMatrixWorld(true);
 return {root,animations:clips,cargo,focus:spec.focus_node,length,width,workY};
}
