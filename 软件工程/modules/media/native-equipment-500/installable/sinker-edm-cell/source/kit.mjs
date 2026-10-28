// Original modular machine geometry, derived from the project's CC0 construction style.
// Modules are mechanical assemblies; none create stock, quantities or a simulation clock.
import * as T from 'three';
import {RoundedBoxGeometry} from 'three/addons/geometries/RoundedBoxGeometry.js';
import {variants} from './mechanisms.mjs';
import {extraAssembly} from './assemblies.mjs';
const PI=Math.PI,V=a=>new T.Vector3(...a);
const mat=(name,color,metalness,roughness,extra={})=>new T.MeshStandardMaterial({name,color,metalness,roughness,...extra});
const M={steel:mat('Brushed stainless','#afbcc5',.78,.32),al:mat('Anodised aluminium','#c6cdd2',.72,.35),blue:mat('Powder coat blue','#306d8c',.32,.42),dark:mat('Cast iron graphite','#243542',.5,.44),white:mat('Ceramic white shell','#dce4e6',.25,.36),rubber:mat('Rubber','#17272e',.02,.83),belt:mat('Polyurethane belt','#328976',.08,.64),amber:mat('Safety ochre','#e6ad45',.22,.42),red:mat('EStop red','#ce514b',.05,.4),green:mat('Signal green','#4eae90',.08,.4),lens:mat('Optical glass','#2c536c',.55,.18),glass:mat('Guard window','#809eab',.15,.22,{transparent:true,opacity:.26}),label:mat('LABEL_ATLAS','#ffffff',0,.75)};

const categories={
 screen:'screen',padprint:'padprint',feed:'feed',airtable:'feed',jogger:'feed',stack:'feed',pusher:'feed',guide:'feed',peel:'feed',suction:'feed',pileturn:'turn',
 unwind:'reel',rewind:'reel',foil:'reel',twist:'reel',winder:'reel',coiler:'reel',
 shear:'press',trim3:'press',crease:'press',hotpress:'press',emboss:'press',die:'press',press:'press',platepress:'press',handlepress:'press',cornercut:'press',punch:'press',combcut:'press',baler:'press',mold:'press',stamp:'press',testpress:'press',pressfit:'press',wireclose:'press',
 roll:'roll',print:'roll',slit:'roll',perforate:'roll',rotaryknife:'roll',slot:'roll',angleform:'roll',spiral:'roll',sleeve:'roll',fold:'fold',bottomfold:'fold',bagform:'fold',pressbelt:'roll',textile:'roll',calender:'roll',rollbend:'roll',combjig:'fixture',boxmold:'fixture',saddle:'fixture',bookclamp:'fixture',clamp:'fixture',tubeclamp:'fixture',
 inkjet:'head',camera:'head',glue:'head',laser:'head',drill:'head',multidrill:'head',mill:'head',gantry:'head',caser:'head',staple:'head',welder:'head',probe:'head',solder:'head',dispense:'head',screw:'head',wirecut:'head',crimp:'head',wirebond:'head',ultrasonic:'head',
 endgrind:'wheel',edgesaw:'wheel',cutoff:'wheel',grind:'wheel',lathe:'lathe',hob:'wheel',broach:'press',
 carousel:'turn',cupform:'form',bowlform:'form',cupseam:'form',bottlefill:'head',index:'turn',rotarytable:'turn',
 tank:'tank',mixer:'tank',kettle:'tank',reactor:'tank',centrifuge:'tank',filter:'filter',sieve:'sieve',silo:'hopper',hopper:'hopper',auger:'screw',screwbarrel:'screw',extrude:'screw',dose:'head',pump:'pump',washer:'drum',drum:'drum',shredder:'shredder',
 oven:'chamber',uv:'chamber',freezer:'chamber',chamber:'chamber',spray:'chamber',vacuumchamber:'chamber',jawseal:'press',
 bend:'press',mandrel:'lathe',thread:'head',racklift:'rack',divert:'feed',weigh:'feed',vibrate:'sieve',rotaryvalve:'pump',needle:'head',carder:'roll',loom:'loom',knitter:'loom',separator:'drum',pelletizer:'wheel',blowmold:'form',stretch:'form',anneal:'chamber',vacuumtable:'fixture',pick:'head',
};
export const supportedMechanisms=[...new Set([...Object.keys(categories),...Object.keys(variants)])];
export function build(spec,simple=false){
 const root=new T.Group();root.name='Machine';let serial=0;const geometries=new Map(),axes=[],moving=[];
 const cache=(k,fn)=>{if(!geometries.has(k))geometries.set(k,fn());return geometries.get(k);};
 function group(p,n,pos=[0,0,0]){const o=new T.Group();o.name=n;o.position.fromArray(pos);p.add(o);return o;}
 function mesh(p,n,g,pos,m){const o=new T.Mesh(g,m);o.name=n+'_'+serial++;o.position.fromArray(pos);o.castShadow=true;o.receiveShadow=true;p.add(o);return o;}
 function box(p,n,d,pos,m=M.steel,r=.005){return mesh(p,n,cache('b'+d+simple+r,()=>simple||!r?new T.BoxGeometry(...d):new RoundedBoxGeometry(...d,2,Math.min(r,...d.map(v=>v*.18)))),pos,m);}
 function cyl(p,n,r,h,pos,m=M.steel,axis='y',r2=r){const o=mesh(p,n,cache('c'+[r,h,r2,simple],()=>new T.CylinderGeometry(r2,r,h,simple?12:32)),pos,m);if(axis==='z')o.rotation.x=PI/2;if(axis==='x')o.rotation.z=PI/2;return o;}
 function ring(p,n,r,t,pos,m=M.dark,axis='z'){const o=mesh(p,n,cache('r'+[r,t,simple],()=>new T.TorusGeometry(r,t,8,simple?16:40)),pos,m);if(axis==='x')o.rotation.y=PI/2;if(axis==='y')o.rotation.x=PI/2;return o;}
 function rod(p,n,a,b,r=.012,m=M.steel){const d=V(b).sub(V(a)),o=cyl(p,n,r,d.length(),V(a).add(V(b)).multiplyScalar(.5).toArray(),m);o.quaternion.setFromUnitVectors(V([0,1,0]),d.normalize());return o;}
 function bolt(p,pos,axis='y'){if(simple)return;cyl(p,'Washer',.011,.002,pos,M.al,axis);const b=pos.slice(),j={x:0,y:1,z:2}[axis];b[j]+=.005;cyl(p,'SocketBolt',.007,.008,b,M.dark,axis);b[j]+=.0045;cyl(p,'SocketRecess',.003,.001,b,M.rubber,axis);}
 function profile(p,l,pos,axis='y',w=.065){const g=cache('p'+[l,w,simple],()=>{if(simple)return new T.BoxGeometry(w,w,l);const h=w/2,a=w*.09,b=w*.18,s=new T.Shape(),pts=[[-h,-h],[-a,-h],[-a,-h+b],[a,-h+b],[a,-h],[h,-h],[h,-a],[h-b,-a],[h-b,a],[h,a],[h,h],[a,h],[a,h-b],[-a,h-b],[-a,h],[-h,h],[-h,a],[-h+b,a],[-h+b,-a],[-h,-a]];s.moveTo(...pts[0]);pts.slice(1).forEach(p=>s.lineTo(...p));s.closePath();const hole=new T.Path();hole.absarc(0,0,w*.10,0,2*PI,true);s.holes.push(hole);const e=new T.ExtrudeGeometry(s,{depth:l,bevelEnabled:false,curveSegments:8});e.translate(0,0,-l/2);return e;});const o=mesh(p,'SlottedExtrusion',g,pos,M.al);if(axis==='y')o.rotation.x=-PI/2;if(axis==='x')o.rotation.y=PI/2;return o;}
 function cable(p,points,r=.006){if(!simple)mesh(p,'AnchoredCable',new T.TubeGeometry(new T.CatmullRomCurve3(points.map(V)),24,r,8,false),[0,0,0],M.rubber);}
 function motor(p,pos,axis='z',scale=1){const m=group(p,'DriveMotor'+serial++,pos);if(axis==='x')m.rotation.y=PI/2;const r=.065*scale;cyl(m,'MotorBody',r,.17*scale,[0,0,0],M.blue,'z');cyl(m,'FanEnd',r*1.08,.025,[0,0,-.10*scale],M.dark,'z');cyl(m,'Gearbox',r*1.25,.065,[0,0,.12*scale],M.al,'z');cyl(m,'OutputShaft',.014*scale,.065,[0,0,.18*scale],M.steel,'z');box(m,'MountFoot',[r*2.4,.018,.20*scale],[0,-r-.01,0],M.dark);box(m,'TerminalBox',[.06,.04,.065],[0,r+.015,0],M.dark);for(let i=0;i<5&&!simple;i++)box(m,'CoolingFin',[r*1.7,.006,.15*scale],[0,(i-2)*r*.3,0],M.blue,.001);return m;}
 function foot(p,x,z,h){cyl(p,'RubberFoot',.05,.028,[x,.014,z],M.rubber);cyl(p,'LevellingStud',.009,.075,[x,.064,z]);cyl(p,'Locknut',.018,.012,[x,.09,z],M.dark);profile(p,h-.10,[x,(h+.10)/2,z]);box(p,'LegGusset',[.10,.09,.009],[x,.15,z+.035],M.steel);bolt(p,[x,.16,z+.042],'z');}
 const n=spec.mechanisms.length,slot=[],fixtures=new Set(['clamp','tubeclamp','bookclamp','vacuumtable','saddle','combjig','boxmold']);
 for(let i=0;i<n;i++)slot.push(i>0&&(fixtures.has(spec.mechanisms[i-1])||variants[spec.mechanisms[i-1]]?.detail==='fixture'||(spec.mechanisms[i-1]==='chamber'&&['shot','blast','spray'].includes(spec.mechanisms[i])))?slot[i-1]:i>0?slot[i-1]+1:0);
 const count=slot.at(-1)+1,L=Math.max(1.55,count*1.05+.35),W=spec.width??1.15,base=.80;
 const frame=group(root,'Chassis');
 for(const x of [-L/2+.09,L/2-.09])for(const z of [-W/2+.05,W/2-.05])foot(frame,x,z,.74);
 for(const z of [-W/2+.05,W/2-.05])for(const y of [.24,.70])profile(frame,L-.18,[0,y,z],'x');
 for(const x of [-L/2+.09,L/2-.09])for(const y of [.24,.70])profile(frame,W-.10,[x,y,0],'z');
 box(frame,'ClosedMachineBed',[L,.06,W],[0,.77,0],M.steel);box(frame,'LowerShelf',[L-.18,.016,W-.14],[0,.275,0],M.al);
 const cab=group(frame,'ElectricalCabinet',[L*.22,.49,-W/2+.14]);box(cab,'Cabinet',[.44,.35,.23],[0,0,0],M.blue);box(cab,'BackDoor',[.41,.32,.010],[0,0,-.12],M.white);cyl(cab,'DoorLatch',.012,.012,[.15,0,-.13],M.dark,'z');for(let i=0;i<6&&!simple;i++)box(cab,'Vent',[.18,.006,.003],[0,-.10+i*.025,-.127],M.dark,.001);
 const hmi=group(root,'OperatorConsole',[L*.30,.98,W/2+.08]);rod(hmi,'ConsoleBracket',[0,-.27,-.1],[0,.02,-.1],.018);box(hmi,'ConsoleBack',[.28,.23,.06],[0,.07,0],M.dark);box(hmi,'Screen',[.215,.12,.008],[0,.10,.035],M.lens);for(let i=0;i<4&&!simple;i++)box(hmi,'ScreenReadout',[.15,.006,.001],[-.01,.14-i*.025,.040],M.green,.001);cyl(hmi,'EmergencyStop',.021,.020,[-.075,-.01,.045],M.red,'z');cyl(hmi,'StartButton',.013,.016,[.015,-.01,.043],M.green,'z');cyl(hmi,'Selector',.011,.016,[.077,-.01,.043],M.dark,'z');
 box(root,'FrontJunctionMount',[.045,.26,.012],[L*.23,.59,W/2-.056],M.steel);box(root,'FrontJunctionBox',[.10,.11,.07],[L*.23,.49,W/2-.025],M.dark);cable(root,[[L*.30,1.0,W/2-.02],[L*.30,.66,W/2-.02],[L*.23,.49,W/2-.025]]);cyl(root,'CableEntryGland',.011,.028,[L*.23,.55,W/2-.025],M.dark);
 const labelGeo=new T.PlaneGeometry(.36,.09);for(let i=0;i<labelGeo.attributes.uv.count;i++)labelGeo.attributes.uv.setY(i,1-labelGeo.attributes.uv.getY(i));mesh(frame,'OriginalNameplate',labelGeo,[0,.68,W/2+.001],M.label);
 const tx=-L*.40,tz=-W*.36;box(root,'LampBracket',[.08,.012,.07],[tx,.806,tz],M.dark);cyl(root,'LampPost',.009,.25,[tx,.937,tz]);for(let i=0;i<3;i++)cyl(root,'SignalSegment',.025,.035,[tx,1.08+i*.039,tz],[M.green,M.amber,M.red][i]);

 const modulePoints=[];let cargo=null;const categoriesUsed=[];
 function translated(o,axis,delta,phase='auto'){axes.push({o,kind:'position',axis,delta,phase});moving.push(o.name);}
 function rotated(o,axis=[0,0,1],amount=2*PI,phase='auto'){axes.push({o,kind:'rotation',axis,amount,phase});moving.push(o.name);}
 function bearing(p,x,y,z){box(p,'BearingBlock',[.12,.13,.06],[x,y,z],M.dark);bolt(p,[x-.036,y+.07,z]);bolt(p,[x+.036,y+.07,z]);}
 function pedestal(p,x,z,h=.20){profile(p,h,[x,h/2,z]);box(p,'PedestalFoot',[.13,.012,.13],[x,.006,z],M.dark);bolt(p,[x+.042,.015,z]);}
 function rotaryShaft(p,name,y,r=.10,len=.74){const g=group(p,name,[0,y,0]);cyl(g,'WorkingCylinder',r,len,[0,0,0],M.steel,'z');for(const z of [-len/2-.035,len/2+.035]){bearing(p,0,y,z);pedestal(p,0,z,y-.065);}motor(p,[0,y,-len/2-.245]);box(p,'MotorMount',[.14,y,.02],[0,y/2,-len/2-.20],M.steel);return g;}
 function fixedGantry(p,height=.94){for(const z of [-.42,.42]){profile(p,height,[0,height/2,z],'y',.08);box(p,'ColumnShoe',[.15,.014,.15],[0,.007,z],M.dark);bolt(p,[.045,.018,z]);}profile(p,.95,[0,height,0],'z',.08);}
 function optical(p,tag){const b=group(p,'Optical_'+serial++,[0,.56,0]);box(b,'CameraHousing',[.14,.15,.13],[0,0,0],M.blue);cyl(b,'Lens',.026,.07,[0,-.11,0],M.lens);ring(b,'LightRing',.075,.012,[0,-.15,0],M.white,'y');for(const x of [-.05,.05])rod(b,'RingSupport',[x,-.02,0],[x,-.15,0],.004);return b;}
 for(let i=0;i<n;i++){
  const raw=spec.mechanisms[i],variant=variants[raw],tag=variant?.base??raw;
  const category=['rolltool','cutting-roll','electrode-wheels'].includes(variant?.detail)?'rollbank':categories[tag]??(variant?tag:null);if(!category)throw Error('Unauthored mechanism '+raw+' in '+spec.id);categoriesUsed.push(category);
  const x=(slot[i]-(count-1)/2)*1.05,p=group(root,'Station_'+String(i).padStart(2,'0')+'_'+raw,[x,base,0]);let inlet=[x-.43,base+.05,0],outlet=[x+.43,base+.05,0],seat=group(p,'WorkSeat_'+i,[0,.05,0]);
  const standard=['screen','padprint','feed','fixture','reel','press','roll','fold','head','wheel','lathe','turn','form','tank','hopper','drum','screw','sieve','filter','pump','shredder','chamber','rack','loom'];
  if(!standard.includes(category))extraAssembly({p,tag:raw,category,i,simple,mechanisms:spec.mechanisms,M,group,box,cyl,ring,rod,bolt,profile,motor,bearing,pedestal,translated,rotated,fixedGantry,rotaryShaft});
  if(category==='feed'){
   if(tag==='stack'){for(const z of [-.32,.32])box(p,'StackGuide',[.62,.35,.025],[0,.18,z],M.blue);const stop=group(p,'StackStop_'+i,[.30,.20,0]);box(stop,'StopPlate',[.028,.32,.65],[0,0,0],M.dark);translated(stop,1,.12);}
   else if(tag==='suction'){fixedGantry(p);const head=group(p,'SuctionBeam_'+i,[0,.52,0]);box(head,'VacuumBeam',[.58,.05,.08],[0,0,0],M.al);for(const ax of [-.20,0,.20]){cyl(head,'CupStem',.009,.13,[ax,-.08,0]);cyl(head,'VacuumCup',.035,.025,[ax,-.155,0],M.rubber);}cyl(p,'FixedCylinder',.035,.28,[0,.78,0],M.al);cyl(head,'PistonRod',.012,.28,[0,.15,0]);translated(head,1,-.22);}
   else if(tag==='airtable'){box(p,'AirDeck',[.83,.025,.76],[0,.03,0],M.dark);for(let a=-3;a<=3&&!simple;a++)for(let b=-3;b<=3;b++)cyl(p,'AirNozzle',.005,.001,[a*.1,.043,b*.09],M.steel);const fence=group(p,'AirTableFence_'+i,[.30,.10,0]);box(fence,'RegistrationFence',[.025,.10,.70],[0,0,0],M.blue);translated(fence,0,-.12);}
   else if(tag==='jogger'){const deck=group(p,'JogDeck_'+i,[0,.06,0]);box(deck,'JoggingTray',[.82,.035,.72],[0,0,0],M.al);for(const z of [-.34,.34])box(deck,'TrayWall',[.8,.12,.018],[0,.075,z],M.blue);for(const ax of [-.25,.25])for(const z of [-.24,.24])cyl(p,'TraySpring',.025,.045,[ax,.022,z],M.dark);translated(deck,0,.012);}
   else if(tag==='weigh'){const deck=group(p,'WeighDeck_'+i,[0,.05,0]);box(deck,'WeighPlate',[.72,.018,.66],[0,0,0],M.steel);for(const ax of [-.2,.2])box(p,'LoadCell',[.10,.025,.045],[ax,.024,0],M.dark);translated(deck,1,-.004);}
   else if(tag==='pusher'||tag==='divert'){const g=group(p,'Pusher_'+i,[-.30,.14,0]);box(g,'PushFace',[.025,.18,.65],[0,0,0],M.belt);box(g,'RodClevisCrossbar',[.07,.05,.15],[0,0,-.325],M.al);cyl(p,'PneumaticBarrel',.026,.23,[-.35,.14,-.38],M.al,'x');for(const ax of [-.43,-.27]){box(p,'CylinderMount',[.045,.12,.07],[ax,.06,-.38],M.steel);box(p,'CylinderFoot',[.09,.012,.12],[ax,.006,-.38],M.dark);bolt(p,[ax+.027,.015,-.35]);}cyl(g,'PistonRod',.009,.34,[-.17,0,-.38]);translated(g,0,.30);}
   else{box(p,'Belt',[.88,.014,.68],[0,.094,0],M.belt);seat.position.y=.11;inlet[1]=outlet[1]=base+.11;for(const ax of [-.39,.39]){cyl(p,'BeltRoller',.045,.73,[ax,.065,0],M.steel,'z');for(const z of [-.39,.39])bearing(p,ax,.065,z);}const drive=group(p,'BeltDrive_'+i,[0,.10,0]);for(let j=0;j<8;j++)box(drive,'BeltCleat',[.008,.004,.65],[-.35+j*.10,0,0],M.dark,.001);translated(drive,0,.10);motor(p,[-.39,.07,-.58]);box(p,'BeltMotorBracket',[.15,.07,.06],[-.39,.035,-.53],M.steel);if(tag==='guide'||tag==='peel')for(const z of [-.32,.32])box(p,'WebGuide',[.70,.06,.018],[0,.08,z],M.steel);}
  }
  if(category==='screen'){
   fixedGantry(p,.80);for(const z of [-.34,.34])box(p,'ScreenRail',[.86,.035,.035],[0,.26,z],M.al);for(const ax of [-.41,.41])box(p,'ScreenCross',[.035,.035,.66],[ax,.26,0],M.al);
   box(p,'FineScreenMesh',[.78,.001,.63],[0,.25,0],M.glass,.0001);const blade=group(p,'Squeegee_'+i,[-.29,.32,0]);box(blade,'BladeCarrier',[.05,.06,.62],[0,0,0],M.blue);box(blade,'SqueegeeRubber',[.012,.052,.58],[0,-.046,0],M.rubber);cyl(p,'SqueegeeCylinder',.023,.5,[0,.40,-.39],M.al,'x');translated(blade,0,.58);
  }
  if(category==='padprint'){
   fixedGantry(p,.86);box(p,'ClicheTable',[.60,.035,.60],[0,.09,0],M.al);cyl(p,'SealedInkCup',.08,.07,[-.20,.14,0],M.blue);const pad=group(p,'TransferPad_'+i,[.18,.42,0]);cyl(pad,'PadHolder',.08,.07,[0,0,0],M.al);cyl(pad,'SiliconePad',.075,.13,[0,-.095,0],M.rubber,'y',.045);cyl(p,'PadActuator',.045,.23,[.18,.69,0],M.al);cyl(pad,'ActuatorRod',.015,.32,[0,.17,0]);translated(pad,1,-.16);translated(pad,0,-.27);
  }
  if(category==='fixture'){
   box(p,'FixtureBase',[.80,.026,.67],[0,.035,0],M.dark);for(const ax of [-.29,.29])for(const z of [-.24,.24])bolt(p,[ax,.053,z]);
   if(raw==='clamp'&&spec.batch==='B02'){
    const bar=group(p,'PaperHoldDown_'+i,[0,.13,-.22]);box(bar,'HoldDownBar',[.40,.05,.07],[0,0,0],M.al);for(const ax of [-.15,.15]){cyl(p,'HoldDownGuide',.009,.22,[ax,.16,-.22]);box(p,'GuideFoot',[.05,.012,.05],[ax,.055,-.22],M.dark);}translated(bar,1,-.03);
   }
   else if(tag==='tubeclamp'){const a=group(p,'TubeChuck_'+i,[0,.20,0]);cyl(a,'ChuckBody',.17,.13,[0,0,0],M.steel,'x');for(let j=0;j<3;j++){const q=j*2*PI/3;box(a,'ChuckJaw',[.09,.06,.07],[.1,.10*Math.cos(q),.10*Math.sin(q)],M.dark);}rotated(a,[1,0,0],PI/5);}
   else if(tag==='saddle'){const a=box(p,'Saddle',[.73,.04,.45],[0,.14,0],M.steel);a.rotation.x=PI/4;const stop=group(p,'SaddleStop_'+i,[.25,.10,0]);box(stop,'Stop',[.02,.13,.4],[0,0,0],M.blue);translated(stop,0,-.06);}
   else{for(const z of [-.24,.24])box(p,'FixedJaw',[.58,.12,.035],[0,.10,z],M.steel);const jaw=group(p,'ClampJaw_'+i,[-.28,.13,0]);box(jaw,'MovingJaw',[.035,.12,.44],[0,0,0],M.al);cyl(p,'ClampScrew',.012,.38,[-.34,.13,0],M.steel,'x');translated(jaw,0,.08);if(tag==='combjig')for(let j=-3;j<=3;j++)box(p,'DividerRegistrationSlot',[.008,.07,.4],[j*.07,.065,0],M.blue,.001);if(tag==='boxmold'||tag==='vacuumtable')box(p,'MoldInsert',[.40,.045,.29],[0,.065,0],M.blue);}
   if(raw==='socketbank'||raw==='pintest'||raw==='syringejig')for(let j=-2;j<=2;j++)for(const z of [-.10,.10]){box(p,'TestSocket',[.065,.025,.065],[j*.10,.07,z],M.rubber);cyl(p,'ContactPin',.003,.022,[j*.10,.09,z],M.amber);}
   if(raw==='granitetable'||raw==='magnettable')box(p,raw==='granitetable'?'GraniteSlab':'MagneticChuck',[.66,.06,.50],[0,.07,0],M.dark);
   if(raw==='vblock'||raw==='rollercradle')for(const ax of [-.20,.20]){const g=box(p,'VeeBlock',[.09,.10,.22],[ax,.10,0],M.steel);g.rotation.x=PI/4;}
  }
  if(category==='reel'){
   for(const z of [-.39,.39]){profile(p,.56,[0,.28,z]);box(p,'ReelSupportFoot',[.16,.015,.17],[0,.008,z],M.dark);}const reel=group(p,'Reel_'+i,[0,.47,0]);cyl(reel,'BareMandrel',.055,.87,[0,0,0],M.steel,'z');for(const z of [-.31,.31])cyl(reel,'CoreFlange',.24,.022,[0,0,z],M.blue,'z');cyl(reel,'EmptyCore',.105,.55,[0,0,0],M.dark,'z');motor(p,[0,.47,-.63]);box(p,'ReelGearMount',[.18,.47,.025],[0,.235,-.50],M.steel);rotated(reel,[0,0,1]);if(tag==='twist'){for(const ax of [-.26,.26])cyl(p,'TwistGuide',.035,.42,[ax,.39,0],M.steel,'z');}inlet=[x,base+.47,0];outlet=[x+.40,base+.47,0];
  }
  if(category==='press'){
   const h=tag==='baler'?1.15:.90;for(const ax of [-.34,.34])for(const z of [-.31,.31]){cyl(p,'GuideColumn',.022,h,[ax,h/2,z]);box(p,'ColumnFoot',[.12,.014,.12],[ax,.007,z],M.dark);bolt(p,[ax+.035,.018,z]);}box(p,'PressCrown',[.83,.12,.78],[0,h,0],M.blue);cyl(p,'HydraulicBarrel',.075,.28,[0,h-.13,0],M.al);const head=group(p,'PressSlide_'+i,[0,.55,0]);box(head,'ToolPlate',[.70,.065,.61],[0,0,0],M.steel);cyl(head,'RamRod',.034,.57,[0,.28,0]);const stroke=.30;
   if(['shear','trim3','jawseal','wireclose','combcut','cornercut'].includes(tag)){
    if(tag==='trim3')for(const ax of [-.25,.25])box(head,'SideTrimmingBlade',[.012,.12,.48],[ax,-.085,0],M.dark,.001);
    if(tag==='combcut')for(let j=-3;j<=3;j++)box(head,'SlotBlade',[.009,.12,.32],[j*.08,-.085,0],M.dark,.001);
    else if(tag==='cornercut'){cyl(head,'CornerDie',.10,.13,[-.20,-.09,-.20],M.dark);}
    else box(head,'WorkingBlade',[.60,.12,.013],[0,-.085,0],tag==='jawseal'?M.amber:M.dark,.001);
   }else if(tag==='punch')for(let j=-3;j<=3;j++)cyl(head,'PunchPin',.009,.12,[j*.07,-.09,0],M.dark);
   else if(tag==='platepress'){cyl(head,'PlateFormDie',.25,.12,[0,-.095,0],M.steel,'y',.20);cyl(p,'PlateLowerDie',.27,.04,[0,.04,0],M.dark);}
   else{box(head,'ToolInsert',[.44,.12,.36],[0,-.095,0],tag==='hotpress'?M.amber:M.dark);if(tag==='emboss')for(let j=-2;j<=2;j++)box(head,'EmbossRib',[.025,.012,.2],[j*.06,-.16,0],M.steel,.001);if(tag==='crease')box(head,'CreaseRule',[.008,.012,.45],[0,-.16,0],M.steel,.001);}
   box(p,'LowerDie',[.68,.04,.60],[0,.025,0],M.dark);translated(head,1,-stroke);if(tag==='baler'){for(const z of [-.39,.39])box(p,'BalerChamberWall',[.83,.72,.025],[0,.38,z],M.blue);box(p,'BalerRear',[.025,.72,.78],[-.41,.38,0],M.blue);}
  }
  if(category==='roll'){
   const roller=rotaryShaft(p,'ProcessRoll_'+i,.39,.17,.74);rotated(roller);seat.position.y=.215;inlet[1]=outlet[1]=base+.215;
   cyl(p,'CounterRoll',.105,.72,[0,.105,0],M.rubber,'z');for(const z of [-.40,.40]){bearing(p,0,.105,z);pedestal(p,0,z,.04);}
   if(['slit','slot','perforate','rotaryknife','carder','hob'].includes(tag))for(let j=-2;j<=2;j++){cyl(roller,'WorkingDisc',tag==='slot'?.21:.187,.015,[0,0,j*.12],M.dark,'z');if(tag==='perforate')for(let k=0;k<12;k++){const q=k*PI/6;cyl(roller,'PerforatingPin',.004,.024,[.187*Math.sin(q),.187*Math.cos(q),j*.12],M.steel);}}
   if(['print','roll','calender'].includes(tag)){cyl(p,'MeteringRoll',.05,.70,[-.22,.43,0],M.dark,'z');box(p,'FluidTray',[.30,.04,.70],[-.13,.53,0],M.blue);}
   if(tag==='angleform'||tag==='rollbend')for(const ax of [-.27,.27])cyl(p,'FormingRoll',.07,.70,[ax,.17,0],M.steel,'z');
   if(tag==='spiral'){const mandrel=group(p,'SpiralMandrel_'+i,[0,.31,0]);cyl(mandrel,'Mandrel',.09,.85,[0,0,0],M.steel,'x');rotated(mandrel,[1,0,0]);for(const z of [-.2,.2])rod(p,'WindingGuide',[-.3,.10,z],[.3,.52,-z],.023,M.belt);}
   if(tag==='pressbelt')box(p,'CompressionTrack',[.78,.08,.62],[0,.39,0],M.belt);
  }
  if(category==='fold'){
   box(p,'FoldingBed',[.86,.02,.70],[0,.04,0],M.belt);for(const z of [-.24,.24]){const wing=group(p,'FoldWing_'+i+'_'+z,[0,.055,z]);box(wing,'FoldingBlade',[.68,.022,.22],[0,0,z>0?.11:-.11],M.blue);for(const ax of [-.28,.28])bearing(p,ax,.055,z);rotated(wing,[1,0,0],z>0?-PI/2:PI/2);}if(tag==='bagform'){box(p,'FormingCollar',[.28,.35,.31],[0,.23,0],M.steel);cyl(p,'FormingTube',.12,.44,[0,.27,0],M.steel);}if(tag==='bottomfold')box(p,'BottomFoldingRail',[.18,.07,.5],[.28,.09,0],M.amber);
  }
  if(category==='head'){
   fixedGantry(p);box(p,'HorizontalGuide',[.78,.026,.04],[0,.84,0],M.steel);cyl(p,'LeadScrew',.009,.78,[0,.79,0],M.steel,'x');motor(p,[.47,.79,-.03],'x',.7);box(p,'MotorBracket',[.16,.18,.12],[.43,.79,-.06],M.dark);
   const xc=group(p,'XCarriage_'+i,[-.12,0,0]);box(xc,'AxisSaddle',[.18,.16,.08],[0,.78,0],M.al);for(const ax of [-.045,.045])cyl(xc,'QuillGuide',.008,.48,[ax,.55,-.045]);
   const head=group(xc,'WorkingHead_'+i,[0,.55,0]);
   if(tag==='camera'){const cam=optical(head,tag);cam.position.y=0;translated(head,1,-.025);}
   else if(['gantry','pick','caser'].includes(tag)){box(head,'VacuumToolPlate',[.47,.045,.20],[0,-.19,0],M.blue);for(const ax of [-.17,.17]){cyl(head,'VacuumStem',.009,.17,[ax,-.09,0]);cyl(head,'VacuumPad',.04,.025,[ax,-.23,0],M.rubber);}const carried=group(head,'CarriedItemSeat_'+i,[0,-.28,0]);cargo??=carried;translated(head,1,-.22);translated(xc,0,.24);}
   else if(tag==='screen'){} // Screen is handled by its explicitly authored module below.
   else{const count=tag==='multidrill'?3:1;for(let j=0;j<count;j++){const z=(j-(count-1)/2)*.16;
     if(['laser','inkjet','probe','wirebond','dose','dispense','bottlefill','solder'].includes(tag)){box(head,'ProcessHousing',[.12,.16,.12],[0,0,z],M.blue);cyl(head,'ProcessTip',tag==='probe'?.005:.013,.12,[0,-.14,z],tag==='probe'?M.rubber:M.steel);}
     else if(['glue','welder','ultrasonic','staple','crimp','wirecut','needle'].includes(tag)){box(head,'Actuator',[.15,.19,.14],[0,0,z],M.blue);box(head,'ToolJaw',[.065,.12,.055],[0,-.15,z],tag==='glue'?M.amber:M.dark);}
     else{cyl(head,'SpindleBody',.04,.18,[0,0,z],M.blue);cyl(head,'Collet',.017,.06,[0,-.12,z]);cyl(head,'ToolBit',.005,.08,[0,-.19,z],M.dark);}
    }translated(head,1,['laser','inkjet','probe','wirebond','dose','bottlefill'].includes(tag)?-.12:-.24);if(['mill','screw','wirebond','laser','inkjet','glue'].includes(tag))translated(xc,0,.24);}
   if(['glue','dispense','dose'].includes(tag)){cyl(p,'MeteringReservoir',.085,.24,[.32,.35,-.23],M.al);rod(p,'ReservoirStand',[.32,0,-.23],[.32,.24,-.23],.025);}
   if(variant?.detail==='cutting'){
    if(raw==='countersink'||raw==='centerdrill')cyl(head,'TaperedCutter',.005,.045,[0,-.23,0],M.steel,'y',.032);
    if(raw==='tap')for(let j=0;j<7;j++)ring(head,'TapThread',.012,.002,[0,-.18-j*.008,0],M.steel,'y');
    if(raw==='ream')for(let j=0;j<6;j++){const a=j*PI/3;box(head,'ReamerLand',[.003,.08,.003],[.011*Math.sin(a),-.20,.011*Math.cos(a)],M.steel,.0001);}
    if(raw==='bore'||raw==='counterbore')box(head,'OffsetBoringInsert',[.045,.025,.014],[.018,-.21,0],M.dark,.001);
   }
   box(xc,'EnclosedAxisServiceDuct',[.045,.45,.045],[.09,.60,-.09],M.dark);box(p,'FixedServiceTrunk',[.045,.72,.055],[0,.36,-.42],M.dark);
  }
  if(category==='wheel'||category==='lathe'){
   if(category==='lathe'){box(p,'Headstock',[.25,.36,.34],[-.28,.22,0],M.blue);const chuck=group(p,'Chuck_'+i,[-.11,.36,0]);cyl(chuck,'ChuckBody',.14,.10,[0,0,0],M.steel,'x');for(let j=0;j<3;j++){const q=j*2*PI/3;box(chuck,'ChuckJaw',[.08,.04,.045],[.07,.08*Math.cos(q),.08*Math.sin(q)],M.dark);}rotated(chuck,[1,0,0]);box(p,'Tailstock',[.18,.22,.22],[.33,.19,0],M.blue);cyl(p,'TailstockQuill',.025,.25,[.20,.36,0],M.steel,'x');const tool=group(p,'ToolSlide_'+i,[.1,.10,.20]);box(tool,'ToolHolder',[.12,.22,.10],[0,.11,0],M.dark);translated(tool,0,-.18);}
   else{fixedGantry(p,.72);const slide=group(p,'SawSlide_'+i,[0,.42,0]);const wheel=group(slide,'Wheel_'+i);cyl(wheel,'Disc',tag==='edgesaw'||tag==='cutoff'?.24:.18,.026,[0,0,0],M.steel,'z');cyl(wheel,'Hub',.055,.07,[0,0,0],M.dark,'z');if(['edgesaw','cutoff'].includes(tag)&&!simple)for(let j=0;j<28;j++){const q=j*2*PI/28,b=box(wheel,'SawTooth',[.025,.016,.025],[.24*Math.sin(q),.24*Math.cos(q),0],M.dark,.001);b.rotation.z=-q;}motor(slide,[0,0,-.22]);box(slide,'BearingBracket',[.14,.18,.08],[0,-.08,-.08],M.dark);translated(slide,1,-.12);rotated(wheel);box(p,'WorkFence',[.70,.055,.03],[0,.07,.24],M.blue);}
  }
  if(category==='turn'||category==='form'){
   if(tag==='pileturn'){const cradle=group(p,'PileCradle_'+i,[0,.47,0]);for(const z of [-.40,.40]){cyl(p,'TrunnionBearing',.10,.07,[0,.47,z],M.dark,'z');pedestal(p,0,z,.40);}box(cradle,'CradleBottom',[.73,.035,.65],[0,-.24,0],M.blue);box(cradle,'CradleBack',[.035,.50,.65],[-.35,0,0],M.blue);box(cradle,'CradleClamp',[.73,.035,.65],[0,.24,0],M.al);motor(p,[0,.47,-.64]);rotated(cradle,[0,0,1],PI);}
   else{const turn=group(p,'IndexTable_'+i,[0,.12,0]);cyl(turn,'IndexDisc',.39,.055,[0,0,0],M.blue);cyl(p,'IndexGearbox',.15,.09,[0,.055,0],M.dark);rotated(turn,[0,1,0],PI/2);for(let j=0;j<6;j++){const q=j*PI/3,xx=.27*Math.sin(q),zz=.27*Math.cos(q);cyl(turn,'IndexNest',.065,.025,[xx,.04,zz],M.dark);if(['cupform','bowlform','blowmold','stretch'].includes(tag)){const r=tag==='bowlform'?.085:.055;cyl(turn,'MetalFormMandrel',r,.17,[xx,.13,zz],M.steel,'y',r*.65);}}if(category==='form'){fixedGantry(p,.8);const head=group(p,'FormingHead_'+i,[0,.53,0]);cyl(head,'FormingDie',tag==='bowlform'?.13:.09,.16,[0,0,0],M.steel);cyl(p,'FormCylinder',.06,.22,[0,.69,0],M.al);cyl(head,'FormPiston',.024,.23,[0,.14,0]);translated(head,1,-.16);}cargo??=group(turn,'IndexedItemSeat_'+i,[.27,.055,0]);}
  }
  if(category==='tank'||category==='hopper'||category==='drum'||category==='screw'||category==='sieve'||category==='filter'||category==='pump'||category==='shredder'||category==='chamber'||category==='rack'||category==='loom')buildProcess(p,tag,category,i);
  if(raw==='spiralfeed'){inlet=[x+.29,base+.14,0];outlet=[x+.29,base+1.07,0];seat.position.set(.29,.14,0);}
  modulePoints.push({inlet,outlet,seat});
 }
 function buildProcess(p,tag,c,i){
  if(c==='tank'){
   for(const ax of [-.26,.26])for(const z of [-.24,.24])pedestal(p,ax,z,.12);cyl(p,'VesselBody',.33,.62,[0,.44,0],M.steel);cyl(p,'LowerDishedEnd',.30,.11,[0,.10,0],M.steel,'y',.22);cyl(p,'VesselLid',.34,.035,[0,.77,0],M.al);for(let j=0;j<8;j++){const q=j*PI/4;bolt(p,[.29*Math.sin(q),.795,.29*Math.cos(q)]);}ring(p,'LidGasket',.32,.012,[0,.785,0],M.dark,'y');cyl(p,'TopPort',.035,.12,[.15,.84,0],M.steel);cyl(p,'DrainValve',.035,.17,[.28,.17,0],M.blue,'x');box(p,'ValveLever',[.13,.02,.025],[.36,.23,0],M.amber);const spindle=group(p,'Agitator_'+i,[0,.80,0]);cyl(spindle,'AgitatorShaft',.012,.60,[0,-.28,0],M.steel);for(const y of [-.20,-.45])for(const q of [0,PI/2]){const b=box(spindle,'AgitatorBlade',[.43,.018,.055],[0,y,0],M.steel);b.rotation.y=q;}motor(p,[0,.96,-.08]);box(p,'AgitatorBridge',[.20,.045,.30],[0,.84,0],M.dark);rotated(spindle,[0,1,0]);if(tag==='reactor'||tag==='kettle')for(const y of [.25,.38,.51])ring(p,'JacketBand',.337,.018,[0,y,0],M.al,'y');
  }else if(c==='hopper'||c==='screw'){
   for(const z of [-.30,.30])profile(p,.50,[0,.25,z]);cyl(p,'FeedHopper',.10,.31,[-.20,.67,0],M.steel,'y',.26);ring(p,'HopperLip',.26,.012,[-.20,.83,0],M.al,'y');const screw=group(p,'ScrewDrive_'+i,[0,.31,0]);cyl(screw,'AugerShaft',.025,.84,[0,0,0],M.steel,'x');for(let j=0;j<10;j++){const g=group(screw,'AugerFlight'+i+'_'+j,[-.37+j*.08,0,0]);cyl(g,'Flight',.075,.014,[0,0,0],M.steel,'x');g.rotation.y=.18;}rotated(screw,[1,0,0]);for(const z of [-.10,.10])box(p,'AugerTrough',[.89,.16,.014],[0,.31,z],M.blue);motor(p,[-.55,.31,0],'x');box(p,'AugerDriveMount',[.14,.23,.13],[-.48,.15,0],M.steel);if(tag==='screwbarrel'||tag==='extrude'){for(let j=0;j<4;j++){const g=group(p,'HeaterBand'+i+'_'+j,[-.15+j*.14,.31,0]);cyl(g,'BarrelBand',.095,.10,[0,0,0],M.dark,'x');box(g,'HeaterTerminal',[.04,.04,.05],[0,.11,0],M.amber);}cyl(p,'ShapingDie',.12,.12,[.43,.31,0],M.al,'x');}
  }else if(c==='drum'){
   for(const ax of [-.25,.25])for(const z of [-.27,.27])pedestal(p,ax,z,.18);const drum=group(p,'Drum_'+i,[0,.44,0]);cyl(drum,'DrumShell',.28,.70,[0,0,0],M.steel,'x');for(const ax of [-.27,.27])ring(drum,'DrumBand',.284,.018,[ax,0,0],M.dark,'x');for(let j=0;j<5;j++){const q=j*2*PI/5;box(drum,'LiftingRib',[.64,.02,.025],[0,.282*Math.cos(q),.282*Math.sin(q)],M.al);}motor(p,[-.57,.44,0],'x');box(p,'DrumDriveStand',[.15,.35,.14],[-.5,.22,0],M.steel);rotated(drum,[1,0,0]);
  }else if(c==='sieve'||c==='filter'){
   const deck=group(p,'FilterDeck_'+i,[0,.35,0]);for(const ax of [-.3,.3])for(const z of [-.25,.25]){cyl(p,'IsolationSpring',.025,.14,[ax,.13,z],M.dark);profile(p,.10,[ax,.05,z]);}box(deck,'ScreenFrame',[.80,.08,.68],[0,0,0],M.blue);for(let j=-4;j<=4;j++)box(deck,'ScreenWire',[.74,.004,.005],[0,.045,j*.065],M.steel,.001);for(let j=-4;j<=4;j++)box(deck,'CrossWire',[.005,.004,.61],[j*.08,.048,0],M.al,.001);motor(deck,[0,-.16,-.1]);translated(deck,1,.012);if(c==='filter')for(let j=0;j<8;j++)box(deck,'FilterPlate',[.018,.44,.50],[-.3+j*.085,.24,0],M.white);
  }else if(c==='pump'){
   box(p,'PumpFoot',[.70,.04,.55],[0,.03,0],M.steel);const rotor=group(p,'PumpRotor_'+i,[0,.25,0]);cyl(rotor,'Rotor',.11,.14,[0,0,0],M.steel,'z');for(let j=0;j<6;j++){const q=j*PI/3,b=box(rotor,'ImpellerVane',[.13,.025,.08],[.08*Math.sin(q),.08*Math.cos(q),0],M.blue);b.rotation.z=-q;}ring(p,'VoluteRim',.16,.035,[0,.25,.10],M.al);motor(p,[0,.25,-.26]);cyl(p,'InletPipe',.045,.20,[0,.25,.22],M.steel,'z');cyl(p,'OutletPipe',.035,.17,[.17,.32,0],M.steel,'x');rotated(rotor);
  }else if(c==='shredder'){
   box(p,'ShredderHousing',[.88,.20,.76],[0,.26,0],M.blue);for(const z of [-.17,.17]){const g=group(p,'ShredderRotor_'+i+'_'+z,[0,.37,z]);cyl(g,'RotorShaft',.035,.8,[0,0,0],M.steel,'x');for(let j=0;j<9;j++){const wheel=cyl(g,'ShredderDisc',.12,.035,[-.34+j*.08,0,0],M.dark,'x');for(let k=0;k<4;k++){const q=k*PI/2;box(g,'RotorTooth',[.03,.05,.05],[-.34+j*.08,.11*Math.sin(q),.11*Math.cos(q)],M.steel);}}rotated(g,[1,0,0],z<0?2*PI:-2*PI);}motor(p,[-.54,.37,-.17],'x');for(const z of [-.39,.39])box(p,'HopperWall',[.91,.36,.025],[0,.55,z],M.steel);
  }else if(c==='chamber'){
   box(p,'ChamberBack',[.86,.70,.025],[0,.39,-.39],M.blue);for(const ax of [-.43,.43])box(p,'ChamberSide',[.025,.70,.78],[ax,.39,0],M.white);box(p,'ChamberRoof',[.90,.035,.80],[0,.76,0],M.blue);const hinge=group(p,'ChamberDoor_'+i,[-.43,.41,.40]);box(hinge,'GuardDoor',[.84,.65,.015],[.42,0,0],M.glass);box(hinge,'DoorHandle',[.018,.14,.035],[.74,0,.02],M.dark);rotated(hinge,[0,1,0],-PI*.45,'load');for(let j=-2;j<=2;j++)box(p,tag==='uv'?'UVBar':'HeaterElement',[.72,.015,.026],[0,.69,j*.12],tag==='uv'?M.lens:M.amber);const fan=group(p,'CirculationFan_'+i,[0,.80,0]);for(let j=0;j<4;j++){const b=box(fan,'FanBlade',[.27,.018,.04],[0,0,0],M.dark);b.rotation.y=j*PI/4;}cyl(p,'FanHousing',.17,.07,[0,.8,0],M.al);rotated(fan,[0,1,0]);
  }else if(c==='rack'){
   for(const ax of [-.36,.36])for(const z of [-.30,.30])profile(p,1.45,[ax,.725,z]);for(const y of [.30,.65,1.0])box(p,'RackShelf',[.73,.025,.60],[0,y,0],M.blue);const lift=group(p,'RackLift_'+i,[0,.16,.39]);box(lift,'LiftForks',[.58,.035,.35],[0,0,0],M.steel);for(const z of [-.20,.20])cyl(p,'LiftLeadScrew',.012,1.45,[.39,.73,z]);translated(lift,1,.90);
  }else if(c==='loom'){
   for(const z of [-.34,.34])profile(p,.70,[0,.35,z]);const beam=rotaryShaft(p,'TextileBeam_'+i,.28,.10,.68);rotated(beam);const reed=group(p,'Reed_'+i,[.2,.33,0]);box(reed,'ReedBar',[.045,.23,.65],[0,0,0],M.blue);for(let j=-7;j<=7;j++)box(reed,'ReedTooth',[.008,.20,.008],[0,0,j*.04],M.steel,.001);translated(reed,0,-.08);for(let j=-7;j<=7&&!simple;j++)rod(p,'GuideThread',[-.36,.4,j*.04],[.34,.31,j*.04],.001,M.rubber);
  }
 }
 const first=modulePoints[0],last=modulePoints.at(-1);
 group(root,'Port_input',first.inlet);group(root,'Port_output',last.outlet);group(root,'Port_auxiliary',[0,base+.04,-W/2]);group(root,'Port_rejected',[L/2,base+.05,-W*.35]);group(root,'Port_container',[-L/2,base+.05,W*.35]);group(root,'Port_component',[-L*.15,base+.05,W*.4]);group(root,'OperatorSeat',[L*.3,0,W/2+.65]);cargo??=first.seat;
 const t=[0,.2,.5,.8,1];const tracks=phase=>axes.filter(a=>a.phase===phase).filter((a,i,all)=>a.kind!=='position'||all.findIndex(b=>b.o===a.o&&b.kind==='position')===i).map(a=>{
  if(a.kind==='position'){const base=a.o.position.toArray(),d=[0,0,0];for(const b of axes)if(b.o===a.o&&b.kind==='position'&&b.phase===phase)d[b.axis]+=b.delta;return new T.VectorKeyframeTrack(a.o.name+'.position',t,[0,.2,1,1,0].flatMap(u=>base.map((v,j)=>v+d[j]*u)));}
  return new T.QuaternionKeyframeTrack(a.o.name+'.quaternion',t,[0,.25,.5,.75,1].flatMap(u=>new T.Quaternion().setFromAxisAngle(V(a.axis),a.amount*u).toArray()));
 });
 const auto=tracks('auto');if(!auto.length)throw Error('No mechanical action in '+spec.id);
 const clips=['load','auto','unload'].map(phase=>{let a=tracks(phase);if(!a.length)a=auto.map(tr=>{const n=tr.getValueSize(),values=Array.from(tr.values.slice(phase==='load'?0:tr.values.length-n,phase==='load'?n:undefined));return new tr.constructor(tr.name,[0,1],[...values,...values]);});return new T.AnimationClip(phase,1,a);});
 root.updateMatrixWorld(true);return {root,animations:clips,cargo:cargo.name,focus:moving[0],mechanisms:spec.mechanisms,categories:categoriesUsed,envelope:[L,W],moving_names:[...new Set(moving)]};
}
