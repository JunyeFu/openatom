// Original CC0 repair details. All dimensions are illustrative L03 geometry.
// Production quantities and process rules are not supplied by these meshes.
import {mergeGeometries} from 'three/addons/utils/BufferGeometryUtils.js';
function annular(T,outer,inner,width,segments){
  const a=new T.CylinderGeometry(outer,outer,width,segments,1,true),b=new T.CylinderGeometry(inner,inner,width,segments,1,true);
  const normals=b.attributes.normal;for(let i=0;i<normals.array.length;i++)normals.array[i]*=-1;
  const index=b.index.array;for(let i=0;i<index.length;i+=3){const t=index[i];index[i]=index[i+2];index[i+2]=t;}
  const top=new T.RingGeometry(inner,outer,segments).rotateX(-Math.PI/2).translate(0,width/2,0);
  const bottom=new T.RingGeometry(inner,outer,segments).rotateX(Math.PI/2).translate(0,-width/2,0);
  return mergeGeometries([a,b,top,bottom]);
}
export function refine(root,family,h){
  const {T,group,box,cyl,rod,ring,bolt,cable,mesh,M,simple}=h;
  if(['assembly','rework'].includes(family)){
    const seam=root.getObjectByName('SpindleSeam');
    // A real hollow sleeve avoids solid end caps intersecting the spindle skin.
    seam.geometry=new T.LatheGeometry([[.0292,-.006],[.0315,-.006],[.0315,.006],[.0292,.006],[.0292,-.006]].map(v=>new T.Vector2(...v)),simple?24:64);
  }
  if(['assembly','rework','packing','inspection'].includes(family)){
    for(const x of [-.58,.58])for(const z of [-.35,.35]){
      box(root,`CornerBracket_${x}_${z}`,[.085,.07,.012],[x,.675,z+Math.sign(z)*.032],M.dark,.002);
      bolt(root,`BracketFix_${x}_${z}`,[x,.68,z+Math.sign(z)*.04],'z',.004,Math.sign(z));
    }
    // Cabinet hinges/latch and an earthed cable gland are present at the rear.
    for(const y of [.38,.62])cyl(root,'DoorHinge'+y,.008,.045,[.49,y,-.306],M.steel);
    box(root,'CabinetLatch',[.018,.08,.015],[.075,.49,-.309],M.dark,.002);
    cable(root,'RearServiceHarness',[[.27,.66,-.24],[.29,.68,-.33],[.54,.65,-.35],[.56,.35,-.35]],.007);
    // Terminate the harness in a leg-mounted connection box. Previously its
    // visible tail stopped a few millimetres short of the lower frame.
    box(root,'RearHarnessJunction',[.06,.08,.05],[.58,.35,-.35],M.dark,.004);
    cyl(root,'RearHarnessGland',.011,.014,[.558,.392,-.35],M.steel);
    for(const y of [.325,.375])bolt(root,'HarnessBoxFix'+y,[.58,y,-.378],'z',.003,-1);
  }
  if(family==='rework'){
    // A supported extraction hood distinguishes the service bench from assembly.
    cyl(root,'ExtractionMast',.014,.48,[-.48,1.015,.24],M.al);
    rod(root,'ExtractionArm',[-.48,1.24,.24],[-.24,1.15,.17],.012,M.al);
    const hood=cyl(root,'ExtractionHood',.045,.065,[-.22,1.115,.16],M.dark,'y',.025);
    ring(root,'HoodRim',.044,.003,[-.22,1.082,.16],M.al,'y');
    cable(root,'ExtractionHose',[[-.22,1.15,.16],[-.38,1.28,.22],[-.50,1.22,.24],[-.52,.84,.25],[-.52,.4,.28]],.017);
    box(root,'PartsTray',[.26,.025,.12],[.35,.793,.27],M.blue,.005);
    for(let i=0;i<4;i++)box(root,'TrayDivider'+i,[.004,.025,.10],[.25+i*.065,.813,.27],M.al,.001);
  }
  if(family==='robot'){
    // Larger links keep the required L03 reach without stretched thin housings.
    for(const n of ['UpperArmShell','ForearmShell']){
      const part=root.getObjectByName(n);if(part)part.material=M.blue;
    }
    const j1=root.getObjectByName('J1_BaseYaw');
    // Separate the pedestal/seal/turntable surfaces. The torus previously
    // intersected the pedestal skin, leaving a black serrated rim in close-up.
    const pedestal=root.getObjectByName('Pedestal');
    pedestal.geometry=new T.CylinderGeometry(.18,.18,.20,simple?12:32);
    pedestal.position.y=.163;
    const baseSeal=root.getObjectByName('BaseSeal');
    baseSeal.geometry=annular(T,.182,.167,.012,simple?16:48);
    baseSeal.rotation.set(0,0,0);baseSeal.position.y=.269;
    const yaw=root.getObjectByName('YawBearing');
    yaw.geometry=new T.CylinderGeometry(.17,.17,.019,simple?12:32);
    yaw.position.y=-.0055;
    // The fork plate ends below the shoulder bearing's lowest sweep (.310m).
    const oldYoke=root.getObjectByName('ShoulderYoke');oldYoke.geometry=new T.BoxGeometry(.34,.010,.44);oldYoke.position.y=.009;
    for(const z of [-.20,.20])box(j1,'YokeCheek'+z,[.22,.25,.025],[0,.13,z],M.al,.014);
    const j6=root.getObjectByName('J6_ToolRoll');
    cyl(j6,'PneumaticInlet',.007,.026,[.14,.048,-.06],M.al,'z');
    cable(j6,'GripperAirLine',[[.14,.048,-.073],[.14,.01,-.10],[.08,-.025,-.09],[.03,-.025,-.04]],.0035);
  }
  if(family==='conveyor'){
    const deck=root.getObjectByName('ConveyorDeck');
    cyl(deck,'JunctionCableGland',.008,.018,[.36,.51,.27],M.dark,'y');
    for(const x of [-.60,.60]){
      rod(root,'DeckDiagonal'+x,[x,.29,-.24],[x,.69,.24],.012,M.al);
      const d=root.getObjectByName('ConveyorDeck');
      for(const z of [-.26,.26])box(d,'TensionSlot_'+x+'_'+z,[.08,.012,.006],[Math.sign(x)*.65,.66,z+Math.sign(z)*.032],M.dark,.003);
    }
  }
  if(family==='packing'){
    // The gate now has an explanatory drive train instead of an unpowered shaft.
    // Fixed bodies stay on the stand; the keyed coupling belongs to the gate pivot.
    for(const x of [-.32,.32]){
      cyl(root,'GateBearingBlock'+x,.028,.034,[x,1.165,-.185],M.dark,'x');
      ring(root,'GateBearingRace'+x,.014,.004,[x+Math.sign(x)*.019,1.165,-.185],M.steel,'x');
      box(root,'GateBearingFoot'+x,[.05,.015,.07],[x,1.132,-.185],M.steel,.002);
    }
    box(root,'GateDriveBracket',[.16,.015,.13],[.40,1.107,-.185],M.steel,.003);
    box(root,'GateDriveBackplate',[.017,.10,.13],[.325,1.15,-.185],M.steel,.002);
    cyl(root,'GateReductionHousing',.052,.075,[.416,1.165,-.185],M.dark,'x');
    cyl(root,'GateServoMotor',.041,.11,[.505,1.165,-.185],M.blue,'x');
    cyl(root,'GateServoCap',.042,.012,[.566,1.165,-.185],M.dark,'x');
    for(const y of [1.13,1.20])for(const z of [-.220,-.150])bolt(root,'GearboxFix_'+y+'_'+z,[.455,y,z],'x',.003,1);
    const gate=root.getObjectByName('PackingGate');
    cyl(gate,'GateDriveCoupling',.023,.035,[.365,0,0],M.steel,'x');
    box(gate,'GateCouplingIndex',[.025,.004,.009],[.365,.023,0],M.orange,.001);
    box(root,'GateMotorConnector',[.024,.024,.035],[.510,1.205,-.185],M.dark,.002);
    cable(root,'GateMotorSupply',[[.510,1.215,-.185],[.55,1.23,-.25],[.59,1.05,-.26],[.56,.78,-.29]],.005);
    box(root,'GateSupplyJunction',[.035,.034,.045],[.56,.795,-.29],M.dark,.003);
  }
  if(family==='inspection'){
    const head=root.getObjectByName('OpticalHead');
    // Camera housing top is 1.415m and the old cable starts at 1.420m.
    // A real strain-relief connector bridges that visible 5mm gap at every pose.
    cyl(head,'CameraEthernetGland',.009,.018,[.03,1.420,.016],M.dark);
    ring(head,'CameraGlandLocknut',.010,.002,[.03,1.415,.016],M.steel,'y');
    // Visible lens barrel rings and a supported cover do not declare accuracy.
    for(const y of [1.216,1.231,1.247])ring(head,'FocusKnurl'+y,.0305,.0016,[0,y,.016],M.al,'y');
    for(const x of [-.025,.025])bolt(head,'CameraMountScrew'+x,[x,1.29,-.015],'y',.003);
  }
  if(family==='sealer'){
    for(const [name,outer,inner,width] of [['TapeRoll',.115,.041,.073],['TapeCore',.040,.032,.078]]){
      const part=root.getObjectByName(name);
      part.geometry=annular(T,outer,inner,width,simple?24:80);
    }
  }
}

export function authoredMaterial(root,type,h){
  const {T,group,box,cyl,rod,ring,bolt,mesh,M,simple}=h, pi=Math.PI;
  const copper=new T.MeshStandardMaterial({name:'Copper finish',color:'#b48b49',metalness:.82,roughness:.32});
  const mask=new T.MeshStandardMaterial({name:'Solder mask',color:'#185e4d',metalness:.08,roughness:.48});
  const solder=new T.MeshStandardMaterial({name:'Tin solder',color:'#b9bec1',metalness:.82,roughness:.3});
  function pcb(parent,y=0){
    const board=group(parent,'PCBAssembly',[0,y,0]);
    box(board,'FR4Board',[.216,.002,.154],[0,.001,0],mask,.0007);
    for(const x of [-.103,.103])for(const z of [-.073,.073]){
      ring(board,'PlatedHole_'+x+'_'+z,.004,.00065,[x,.00215,z],copper,'y');
      cyl(board,'ThroughHole_'+x+'_'+z,.0027,.0021,[x,.001,z],M.dark);
    }
    for(let row=0;row<2;row++)for(let i=0;i<4;i++){
      const x=-.067+i*.044,z=-.035+row*.07;
      box(board,'IC_'+row+'_'+i,[.025,.007,.024],[x,.0055,z],M.dark,.001);
      for(const s of [-1,1])for(let k=0;k<5;k++){
        const px=x-.010+k*.005,pz=z+s*.016;
        box(board,'Lead_'+row+'_'+i+'_'+s+'_'+k,[.0016,.0008,.006],[px,.0025,pz],solder,.0002);
        if(!simple){
          box(board,'CopperTrack_'+row+'_'+i+'_'+s+'_'+k,[.0006,.00012,.016],[px,.00215,pz+s*.008],copper,.0001);
        }
      }
      if(!simple)box(board,'ICMark_'+row+'_'+i,[.014,.00015,.0014],[x,.0091,z+.003],M.al,.0001);
    }
    const conn=group(board,'TerminalBlock',[.083,.008,0]);
    box(conn,'TerminalBody',[.032,.014,.038],[0,0,0],M.blue,.001);
    for(const z of [-.012,0,.012])cyl(conn,'TerminalScrew'+z,.003,.001,[0,.0076,z],solder);
    for(let k=0;k<6;k++){
      box(board,'SMDCap'+k,[.006,.004,.003],[-.08+k*.026,.004,0],M.card,.0004);
      for(const x of [-.0032,.0032])box(board,'CapTermination'+k+'_'+x,[.001,.0038,.003],[-.08+k*.026+x,.004,0],solder,.0001);
    }
    return board;
  }
  if(type==='shell'||type==='control-box'){
    box(root,'HousingBase',[.24,.008,.18],[0,.004,0],M.al,.002);
    for(const z of [-.086,.086])box(root,'HousingWallZ'+z,[.24,.057,.008],[0,.0365,z],M.white,.002);
    for(const x of [-.116,.116])box(root,'HousingWallX'+x,[.008,.057,.164],[x,.0365,0],M.white,.002);
    for(const x of [-.103,.103])for(const z of [-.073,.073]){
      cyl(root,'BoardBoss_'+x+'_'+z,.005,.018,[x,.017,z],M.al);
      cyl(root,'BossHole_'+x+'_'+z,.0018,.001,[x,.0265,z],M.dark);
      if(type==='control-box')bolt(root,'AssemblyScrew_'+x+'_'+z,[x,.028,z],'y',.004);
    }
    for(const x of [-.07,.07]){
      box(root,'MountLug'+x,[.022,.006,.032],[x,.004,.099],M.al,.003);
      cyl(root,'LugHole'+x,.003,.0006,[x,.0075,.104],M.dark);
    }
    for(const x of [-.05,.05]){
      const port=cyl(root,'CableGland'+x,.011,.016,[x,.030,.096],M.dark,'z');
      ring(root,'GlandCollar'+x,.011,.002,[x,.030,.104],M.al,'z');
      cyl(root,'ConnectorBore'+x,.006,.0008,[x,.030,.105],M.rubber,'z');
    }
    if(type==='control-box')pcb(root,.025);
  }else if(type==='pcb')pcb(root);
  else if(type==='fastener'){
    const shaft=new T.LatheGeometry([[0,0],[.0012,0],[.0015,.002],[.0015,.030],[0,.030]].map(v=>new T.Vector2(...v)),simple?16:48);
    mesh(root,'ScrewShaft',shaft,M.steel);
    const sh=new T.Shape();sh.absarc(0,0,.0038,0,pi*2,false);
    const hole=new T.Path();for(let i=0;i<6;i++){const a=-i*pi/3; i?hole.lineTo(.0015*Math.cos(a),.0015*Math.sin(a)):hole.moveTo(.0015*Math.cos(a),.0015*Math.sin(a));}hole.closePath();sh.holes.push(hole);
    const geo=new T.ExtrudeGeometry(sh,{depth:.004,bevelEnabled:true,bevelSize:.0002,bevelThickness:.0002,bevelSegments:2,steps:1,curveSegments:24});geo.rotateX(-pi/2);
    mesh(root,'SocketCapHead',geo,M.steel,[0,.030,0]);
    cyl(root,'SocketFloor',.00145,.0001,[0,.031,0],M.dark);
    if(!simple){const pts=Array.from({length:513},(_,i)=>{const u=i/512,a=u*pi*26;return new T.Vector3(.0015*Math.cos(a),.001+.027*u,.0015*Math.sin(a));});mesh(root,'HelicalThread',new T.TubeGeometry(new T.CatmullRomCurve3(pts),512,.00023,6,false),M.steel);}
  }else if(type==='sealing-tape'){
    const w=.048,outer=.080,inner=.041;
    const points=[[inner,-w/2],[outer,-w/2],[outer,w/2],[inner,w/2],[inner,-w/2]].map(v=>new T.Vector2(...v));
    const roll=mesh(root,'WoundTape',annular(T,outer,inner,w,simple?32:96),M.card,[0,.080,0]);roll.rotation.x=pi/2;
    const core=mesh(root,'HollowCardboardCore',annular(T,inner,.037,w,simple?24:64),M.wood2,[0,.080,0]);core.rotation.x=pi/2;
    if(!simple)for(let i=0;i<8;i++)ring(root,'TapeLayer'+i,.045+i*.0045,.00016,[0,.080,w/2+.00025],M.wood2,'z');
    box(root,'TapeEndTab',[.035,.0007,.046],[.027,.1602,0],M.card,.0001);
  }else if(type==='carton'){
    box(root,'CartonBottom',[.49,.003,.34],[0,.0015,0],M.card,.0006);
    for(const x of [-.2435,.2435])box(root,'CartonWallX'+x,[.003,.356,.34],[x,.181,0],M.card,.0006);
    for(const z of [-.1685,.1685])box(root,'CartonWallZ'+z,[.484,.356,.003],[0,.181,z],M.card,.0006);
    const flaps=[];
    for(const z of [-.1685,.1685]){
      const g=group(root,'MajorFlap'+(z>0?'Front':'Rear'),[0,.359,z]);
      box(g,'MajorFlapPanel'+z,[.476,.002,.166],[0,0,-Math.sign(z)*.083],M.card,.0004);
      g.rotation.x=Math.sign(z)*pi/2;flaps.push({name:g.name,axis:[1,0,0],angle:g.rotation.x});
    }
    for(const x of [-.2435,.2435]){
      const g=group(root,'MinorFlap'+(x>0?'Right':'Left'),[x,.354,0]);
      box(g,'MinorFlapPanel'+x,[.162,.002,.324],[-Math.sign(x)*.081,0,0],M.card,.0004);
      g.rotation.z=-Math.sign(x)*pi/2;flaps.push({name:g.name,axis:[0,0,1],angle:g.rotation.z});
    }
    const tape=box(root,'ClosureTape',[.49,.0006,.046],[0,.3597,0],M.wood2,.0001);tape.scale.setScalar(.000001);
    const label=mesh(root,'PackageLabel',new T.PlaneGeometry(.105,.045),M.label,[.03,.19,.1705]);
    if(!simple)for(const z of [-.168,.168])for(let i=0;i<100;i++)cyl(root,'FluteEdge_'+z+'_'+i,.0007,.002,[-.24+i*.0048,.358,z],M.wood2,'y');
    group(root,'GaugeSeat');
    const stageTimes=[0,.4,.45,.85,1];
    function tracks(closed,fold=false){return flaps.map(f=>{
      const values=[];for(const u of stageTimes){
        // Complete the minor flaps before lowering the major flaps. Include
        // both breakpoints in the exported clip, rather than interpolating
        // across the two stages and letting the panels intersect.
        const t=fold?Math.min(1,Math.max(0,(u-(f.name.startsWith('Minor')?0:.45))/.4)):closed?1:0;
        values.push(...new T.Quaternion().setFromAxisAngle(new T.Vector3(...f.axis),f.angle*(1-t)).toArray());
      }return new T.QuaternionKeyframeTrack(f.name+'.quaternion',stageTimes,values);
    });}
    const tiny=[.000001,.000001,.000001];
    return [new T.AnimationClip('open',1,[...tracks(false),new T.VectorKeyframeTrack('ClosureTape.scale',[0,1],[...tiny,...tiny])]),
      new T.AnimationClip('sealed',1,[...tracks(true),new T.VectorKeyframeTrack('ClosureTape.scale',[0,1],[1,1,1,1,1,1])]),
      new T.AnimationClip('fold_preview',1,[...tracks(false,true),new T.VectorKeyframeTrack('ClosureTape.scale',[0,.85,1],[...tiny,...tiny,1,1,1])])];
  }else throw Error('Unknown material family '+type);
  group(root,'GaugeSeat');return [];
}
