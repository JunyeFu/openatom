import * as T from 'three';
import{geometry,V,clip}from './geometry.mjs';
import{specs,samples,seat,inputPort,outputPort,outputY,auxPort,auxSeat,stagingX,workY,ramp,opening,minorFold,majorFold,tapeFraction,glueFraction,pressFraction,carton as c,blank,flatCase}from './batch-spec.mjs';
export function build(spec,simple=false){
 const api=geometry(simple),{m,group,mesh,box,cyl,rod,profile,cable,bolt,motor}=api,root=new T.Group();root.name='Machine';const tracks={load:[],auto:[],unload:[]},moving=[];
 const dynamic=(o,key,fn)=>{o[key].fromArray(fn('load',0));for(const phase of Object.keys(tracks))tracks[phase].push(key==='quaternion'?new T.QuaternionKeyframeTrack(o.name+'.'+key,samples,samples.flatMap(u=>fn(phase,u))):new T.VectorKeyframeTrack(o.name+'.'+key,samples,samples.flatMap(u=>fn(phase,u))));moving.push(o.name);return o;};
 const pos=(o,fn)=>dynamic(o,'position',fn),rot=(o,axis,fn)=>dynamic(o,'quaternion',(ph,u)=>new T.Quaternion().setFromAxisAngle(V(axis),fn(ph,u)).toArray());
 const active=(ph,u,fn)=>ph==='auto'?fn(u):ph==='unload'?fn(1):fn(0);
 const frame=group(root,'Chassis');
 for(const x of[-1.52,1.52])for(const z of[-.78,.78]){cyl(frame,'Foot_'+x+'_'+z,.055,.024,[x,.012,z],m.rubber);cyl(frame,'Stud_'+x+'_'+z,.012,.052,[x,.05,z]);profile(frame,'Leg_'+x+'_'+z,.65,[x,.401,z]);bolt(frame,'FootBolt_'+x+'_'+z,[x,.082,z]);}
 for(const z of[-.78,.78])for(const y of[.26,.73])profile(frame,'Long_'+z+'_'+y,3.10,[0,y,z],'x');
 for(const x of[-1.52,-.70,.70,1.52])for(const y of[.26,.73])profile(frame,'Cross_'+x+'_'+y,1.50,[x,y,0],'z');
 box(frame,'LowerShelf',[2.94,.012,1.42],[0,.296,0],m.al);box(frame,'ElectricalCabinet',[.46,.39,.27],[.94,.52,-.58],m.blue);box(frame,'CabinetDoor',[.43,.36,.008],[.94,.52,-.72],m.al);cyl(frame,'CabinetLatch',.014,.012,[1.10,.52,-.73],m.dark,'z');
 for(const x of[.77,1.11])box(frame,'CabinetHanger_'+x,[.04,.035,.23],[x,.7175,-.60]);
 if(!simple)for(let i=0;i<6;i++)box(frame,'CabinetVent_'+i,[.21,.006,.002],[.91,.42+i*.024,-.726],m.dark,false);
 function table(name,x,y,length=.72){box(root,name+'Top',[length,.012,.68],[x,y-.006,0]);for(const dx of[-length/2+.065,length/2-.065]){profile(frame,name+'SupportCross_'+dx,1.50,[x+dx,.73,0],'z');for(const z of[-.25,.25])profile(root,name+'Post_'+dx+'_'+z,y-.766,[x+dx,(y+.766)/2-.012,z]);}}
 table('InputDeck',-1,workY,.76);table('OutputDeck',1.10,outputY(spec.id),1.08);
 for(const x of[.67,.84,1.01,1.18,1.35,1.52]){const r=.018; // Roller crowns lie exactly on the item support plane.
  cyl(root,'DischargeRoller_'+x,r,.62,[x,outputY(spec.id)-r,0],m.steel,'z');for(const z of[-.335,.335])box(root,'RollerBearing_'+x+'_'+z,[.045,.055,.028],[x,outputY(spec.id)-r,z],m.dark);
 }
 // Roller deck surface is a support plate below the crowns, not a second coincident face.
 root.getObjectByName('OutputDeckTop').position.y=outputY(spec.id)-.045;
 for(const z of[-.35,.35])box(root,'OutputSideFrame_'+z,[1.08,.058,.030],[1.10,outputY(spec.id)-.029,z],m.blue);
 motor(root,'OutputDrive',[1.35,outputY(spec.id)-.018,-.515]);box(root,'OutputMotorMount',[.15,.08,.15],[1.35,outputY(spec.id)-.075,-.515],m.dark);
 if(spec.auxiliary)for(const[x,name]of[[-1,'ConsumableDeck'],[.90,'WorkingConsumableDeck']]){box(root,name+'Top',[.60,.012,.28],[x,workY-.006,-.52]);for(const dx of[-.22,.22]){profile(frame,name+'SupportCross_'+dx,1.50,[x+dx,.73,0],'z');for(const dz of[-.10,.10])profile(root,name+'Post_'+dx+'_'+dz,.134,[x+dx,.827,-.52+dz]);}}
 const label=mesh(root,'OriginalNameplate',new T.PlaneGeometry(.36,.09),[.78,.65,.818],m.label);for(let i=0;i<label.geometry.attributes.uv.count;i++)label.geometry.attributes.uv.setY(i,1-label.geometry.attributes.uv.getY(i));box(root,'NameplateBack',[.38,.11,.018],[.78,.65,.807],m.dark);box(root,'NameplateBracket',[.04,.05,.05],[.78,.715,.79]);
 cyl(root,'HMITowerFoot',.07,.02,[1.16,.01,.98],m.rubber);cyl(root,'HMITower',.025,.96,[1.16,.50,.98]);box(root,'HMIHousing',[.29,.25,.07],[1.16,1.03,.98],m.dark);box(root,'HMIScreen',[.23,.13,.006],[1.16,1.075,1.019],m.lens);for(let i=0;i<4&&!simple;i++)box(root,'ScreenLine_'+i,[.15,.005,.001],[1.15,1.11-i*.025,1.023],m.green,false);cyl(root,'EStop',.022,.020,[1.07,.96,1.025],m.red,'z');cyl(root,'Start',.014,.016,[1.16,.96,1.025],m.green,'z');
 profile(root,'SignalPost',.5225,[-1.30,1.02125,-.73]);for(let i=0;i<3;i++)cyl(root,'Signal_'+i,.026,.035,[-1.30,1.30+i*.040,-.73],[m.green,m.amber,m.red][i]);for(const y of[1.32,1.36])cyl(root,'SignalSeparator_'+y,.026,.005,[-1.30,y,-.73],m.dark);
 box(root,'AirPreparationMount',[.20,.25,.015],[.95,.61,.80]);box(root,'AirRegulator',[.18,.10,.06],[.95,.61,.84],m.dark);for(const x of[.90,1.00])cyl(root,'AirBowl_'+x,.023,.13,[x,.505,.84],m.lens);cable(root,'CabinetServiceBundle',[[.78,.70,-.61],[.45,.70,-.69],[-.55,.70,-.69],spec.id==='compression-stack'?[-.30,1.50,-.22]:[-.62,1.55,-.54]]);
 cable(root,'HMIServiceBundle',[[1.16,1.00,.94],[1.16,.35,.94],[.94,.35,-.58]]);
 const cargo=group(root,'PaperSeat');pos(cargo,(ph,u)=>seat(ph,u));group(root,'ProductStaging',[stagingX,outputY(spec.id),0]);group(root,'WorkingAuxiliary',auxSeat());for(const[role,p]of[['input',inputPort()],['output',outputPort(spec.id)],...(spec.auxiliary?[['auxiliary',auxPort()]]:[])])group(root,'Port_'+role,p);group(root,'OperatorSeat',[1.1,0,1.18]);
 function gantry(prefix,height=1.75){for(const x of[-.62,.62]){profile(frame,prefix+'SupportCross_'+x,1.50,[x,.73,0],'z');for(const z of[-.54,.54]){profile(root,prefix+'Upright_'+x+'_'+z,height-.77,[x,(height+.77)/2,z]);box(root,prefix+'Shoe_'+x+'_'+z,[.12,.015,.12],[x,.7675,z],m.dark);bolt(root,prefix+'Bolt_'+x+'_'+z,[x+.035,.778,z]);}}for(const z of[-.54,.54])profile(root,prefix+'Top_'+z,1.32,[0,height,z],'x');for(const x of[-.62,.62])profile(root,prefix+'Bridge_'+x,1.14,[x,height,0],'z');}
 let focus;
 if(spec.id==='carton-erector'){
  gantry('Erector',2.00);for(const z of[-.54,.54]){profile(root,'FrontHeadRail_'+z,2.80,[-.10,1.86,z],'x');profile(root,'RearHeadRail_'+z,2.80,[-.10,2.00,z],'x');for(const x of[-1.35,1.10]){profile(frame,'TravelCross_'+x+'_'+z,1.50,[x,.73,0],'z');profile(root,'TravelUpright_'+x+'_'+z,1.24,[x,1.38,z]);}}focus='FrontVacuumCrossbar';
  for(const side of['Front','Back']){
   const rear=side==='Back',sgn=rear?1:-1,beamY=rear?2:1.86;
   const xyz=(ph,u)=>{const s=seat(ph,u),a=active(ph,u,opening);return[s[0]+(rear?c.width*Math.cos(a):0),s[1]+c.base+.15,(rear?-c.width/2+c.width*Math.sin(a): -c.width/2)+sgn*.128];};
   const x=group(root,side+'XCarriage');pos(x,(ph,u)=>[xyz(ph,u)[0],beamY,0]);box(x,side+'XBlock',[.14,.06,1.02],[0,0,0],m.blue);for(const z of[-.48,.48])box(x,side+'LinearBearing_'+z,[.16,.08,.07],[0,-.015,z],m.dark);
   const z=group(x,side+'ZCarriage');pos(z,(ph,u)=>[0,0,xyz(ph,u)[2]]);box(z,side+'ZBlock',[.13,.05,.10],[0,-.03,0],m.dark);cyl(z,side+'ClosedVerticalActuator',.025,.30,[0,-.22,0],m.blue);
   const down=group(z,side+'DropRod');pos(down,(ph,u)=>[0,xyz(ph,u)[1]-beamY,0]);const stem=cyl(down,side+'TelescopingRod',.010,1,[0,0,0]);pos(stem,(ph,u)=>[0,(.02+beamY-.23-xyz(ph,u)[1])/2,0]);dynamic(stem,'scale',(ph,u)=>[1,beamY-.23-xyz(ph,u)[1]-.02,1]);box(down,side+'VacuumCrossbar',[.28,.04,.05],[0,0,0],m.al);
   for(const dx of[-.10,.10]){cyl(down,side+'VacuumStem_'+dx,.008,.090,[dx,0,-sgn*.065],m.steel,'z');cyl(down,side+'VacuumCup_'+dx,.027,.018,[dx,0,-sgn*.1175],m.rubber,'z',.015);}
   // Visible service loop stays parented to the moving head; a closed flexible entry is explicit.
   cable(down,side+'VacuumLoop',[[-.10,0,0],[-.10,.08,0],[0,.13,0],[.10,.08,0],[.10,0,0]],.004);cyl(z,side+'AirEntry',.010,.025,[0,-.06,.035],m.dark,'z');
  }
  // Flap paddles rotate below the cardboard about its real fold axes.
  const corners=[['Front',0,-c.width/2,c.length,false,0],['Back',0,c.width/2,c.length,false,Math.PI],['Right',c.length/2,0,c.width,true,-Math.PI/2],['Left',-c.length/2,0,c.width,true,Math.PI/2]];
  for(const[name,x,z,len,minor,yaw]of corners){const base=group(root,name+'FolderMount',[x,workY+c.base+(minor?.0032:0),z]);base.rotation.y=yaw;const h=group(base,name+'FolderHinge');rot(h,[1,0,0],(ph,u)=>active(ph,u,v=>-(minor?minorFold(v):majorFold(v))*(1-ramp(v,.74,.78))));box(h,name+'FoldPaddle',[len-.04,.10,.006],[0,-.060,-.0045],m.al,false);cyl(base,name+'FoldShaft',.012,len+.07,[0,0,-.012],m.steel,'x');cyl(base,name+'ClosedRotaryActuator',.032,.060,[len/2+.04,0,-.012],m.blue,'x');box(base,name+'ActuatorBracket',[.10,.055,.12],[len/2+.04,-.05,-.035],m.dark);rod(base,name+'FolderPedestal',[len/2+.04,-.0775,-.035],[len/2+.04,.76-base.position.y,-.035],.018);}
  // Original pre-cut strip applicator: a travelling lower roller and two end wipe rollers.
  const head=group(root,'BottomTapeHead');pos(head,(ph,u)=>{const f=active(ph,u,tapeFraction);return[-.24+.48*f,.9954,0];});cyl(head,'TapePressureRoller',.022,.064,[0,0,0],m.rubber,'z');box(head,'TapeCassette',[.10,.12,.10],[0,-.10,0],m.blue);cyl(head,'TapeVerticalRod',.012,.20,[0,-.20,0]);box(head,'TapeRailCarriage',[.14,.05,.14],[0,-.31,0],m.dark);profile(root,'TapeTravelRail',.80,[0,.685,0],'x');for(const x of[-.32,.32])profile(root,'TapeRailPost_'+x,.395,[x,.4875,0]);
  for(const sx of[-1,1]){const y=(ph,u)=>.80+.249*active(ph,u,v=>ramp(v,.76,.78)-ramp(v,.92,.98)),end=group(root,'EndWiper_'+sx);pos(end,(ph,u)=>[sx*(c.length/2+.0225),y(ph,u),0]);cyl(end,'EndWipeRoller_'+sx,.021,.064,[0,0,0],m.rubber,'z');box(end,'EndWipeBracket_'+sx,[.045,.10,.05],[sx*.025,-.07,0],m.blue);cyl(root,'EndWipeClosedCylinder_'+sx,.024,.30,[sx*(c.length/2+.0475),.70,0],m.blue);const ram=cyl(root,'EndWipeLiftRod_'+sx,.008,1,[0,0,0]);pos(ram,(ph,u)=>[sx*(c.length/2+.0475),(.65+y(ph,u)-.07)/2,0]);dynamic(ram,'scale',(ph,u)=>[1,y(ph,u)-.72,1]);box(root,'EndWipeBase_'+sx,[.10,.015,.10],[sx*(c.length/2+.0475),.5425,0],m.dark);profile(root,'EndWipePost_'+sx,.23,[sx*(c.length/2+.0475),.42,0]);}
  for(const x of[-1.30,-.72]){profile(root,'MagazineSide_'+x,.43,[x,1.115,-.22]);box(root,'MagazineFence_'+x,[.035,.40,.025],[x,1.12,-.19],m.blue);}box(root,'MagazineLowerGuide',[.70,.025,.025],[-1,.9125,-.17],m.dark);
 }else if(spec.id==='adhesive-dispenser'){
  gantry('Glue',1.65);focus='MeteredGlueNozzle';table('VacuumWorkDeck',0,workY,.64);
  for(const z of[-.13,0,.13])for(const x of[-.18,0,.18])cyl(root,'VacuumPort_'+x+'_'+z,.004,.001,[x,workY+.0005,z],m.dark);
  const head=group(root,'GlueXCarriage');pos(head,(ph,u)=>[-blank.beadLength/2+blank.beadLength*active(ph,u,glueFraction),1.65,0]);box(head,'GlueXBridge',[.12,.07,1.10],[0,0,0],m.blue);for(const z of[-.48,.48])box(head,'GlueLinearBearing_'+z,[.15,.08,.08],[0,-.02,z],m.dark);
  const slider=group(head,'GlueZCarriage',[0,0,blank.beadZ]);box(slider,'GlueZBlock',[.12,.05,.09],[0,-.05,0],m.dark);cyl(slider,'ClosedZActuator',.025,.30,[0,-.24,0],m.blue);
  const gun=group(slider,'GlueYSlide');pos(gun,(ph,u)=>[0,workY+blank.thickness+blank.beadHeight+.045+.18*(1-active(ph,u,v=>ramp(v,.08,.20)-ramp(v,.76,.90)))-1.65,0]);cyl(gun,'GlueRod',.009,.36,[0,.20,0]);box(gun,'HeatedGun',[.055,.075,.05],[0,.025,0],m.blue);cyl(gun,'MeteredGlueNozzle',.001,.045,[0,-.0225,0],m.steel,'y',.008);
  const feedPoint=(ph,u,t)=>{const bottom=gun.position.y;const y=workY+blank.thickness+blank.beadHeight+.045+.18*(1-active(ph,u,v=>ramp(v,.08,.20)-ramp(v,.76,.90)))-1.65;const a=V([.035,-.10,0]),b=V([0,y+.060,.025]),mid=V([.16,(a.y+b.y)/2,.10]);return a.multiplyScalar((1-t)*(1-t)).add(mid.multiplyScalar(2*t*(1-t))).add(b.multiplyScalar(t*t));};
  for(let i=0;i<8;i++){const o=cyl(slider,'InsulatedGunFeed_'+i,.008,1,[0,0,0],m.rubber),ends=(ph,u)=>[feedPoint(ph,u,i/8),feedPoint(ph,u,(i+1)/8)];pos(o,(ph,u)=>{const[a,b]=ends(ph,u);return a.add(b).multiplyScalar(.5).toArray();});dynamic(o,'scale',(ph,u)=>{const[a,b]=ends(ph,u);return[1,a.distanceTo(b),1];});dynamic(o,'quaternion',(ph,u)=>{const[a,b]=ends(ph,u);return new T.Quaternion().setFromUnitVectors(V([0,1,0]),b.sub(a).normalize()).toArray();});}
  cyl(slider,'FixedHeatedHoseEntry',.011,.025,[.035,-.10,0],m.dark);cyl(gun,'MovingHeatedHoseEntry',.011,.025,[0,.06,.025],m.dark);

  box(root,'ClosedHeatedMeteringUnit',[.28,.28,.25],[.88,1.04,-.52],m.blue);box(root,'MelterControl',[.12,.05,.008],[.88,1.08,-.389],m.dark);cable(root,'HeatedMainHose',[[.88,1.17,-.52],[.70,1.50,-.60],[.30,1.65,-.60],[.62,1.65,-.54]],.009);
  cyl(root,'MeteredDoseCup',.015,.008,auxPort(),m.al);cyl(root,'WorkingDoseWell',.015,.008,auxSeat(),m.al);
 }else{
  focus='GuidedCompressionPlaten';table('CompressionBed',0,workY,.64);
  for(const x of[-.30,.30])for(const z of[-.22,.22]){cyl(root,'CompressionGuide_'+x+'_'+z,.014,.60,[x,1.20,z]);box(root,'GuideFoot_'+x+'_'+z,[.09,.014,.08],[x,.907,z],m.dark);bolt(root,'GuideBolt_'+x+'_'+z,[x+.026,.917,z]);}
  box(root,'CompressionCrown',[.76,.08,.62],[0,1.54,0],m.blue);cyl(root,'ClosedPressCylinder',.065,.30,[0,1.365,0],m.al);box(root,'PressureSensor',[.06,.05,.04],[.075,1.47,0],m.dark);
  const head=group(root,'GuidedCompressionPlaten');pos(head,(ph,u)=>[0,1.20-(1.20-(workY+flatCase.thickness))*active(ph,u,pressFraction),0]);box(head,'CompressionTool',[.48,.045,.36],[0,.0265,0]);box(head,'PressurePad',[.46,.004,.34],[0,.002,0],m.rubber);cyl(head,'CompressionRam',.025,.37,[0,.230,0]);for(const x of[-.30,.30])for(const z of[-.22,.22]){box(head,'GuideEar_'+x+'_'+z,[.09,.04,.08],[x,.025,z],m.al);cyl(head,'GuideBushing_'+x+'_'+z,.023,.055,[x,.0275,z],m.dark);}
  cable(root,'PressureService',[[.95,.66,.84],[.60,.85,.55],[.35,1.48,.30],[.075,1.47,.02]],.005);
 }
 // Supported guards leave working parts readable from the front and side.
 for(const z of[-.69,.69]){for(const x of[-.64,.64]){profile(frame,'GuardCross_'+x+'_'+z,1.50,[x,.73,0],'z');box(root,'GuardShoe_'+x+'_'+z,[.08,.010,.08],[x,.765,z],m.dark);profile(root,'GuardPost_'+x+'_'+z,.88,[x,1.21,z]);}box(root,'GuardRail_'+z,[1.34,.03,.04],[0,1.66,z],m.blue);box(root,'GuardPanel_'+z,[1.22,.70,.006],[0,1.26,z],m.glass,false);}
 const service_circuits=[{name:'pneumatic regulation',entry:'AirRegulator',boundary:'named closed internal valve/actuator paths'},{name:'main electrical and HMI',entry:'ElectricalCabinet',boundary:'visible service bundle, closed cabinet internal harness'},...(spec.id==='carton-erector'?[{name:'moving vacuum heads',entry:'FrontAirEntry',boundary:'parented local head loops and closed telescopic energy feed'}]:[])];
 root.updateMatrixWorld(true);return{root,animations:Object.entries(tracks).map(([name,t])=>clip(name,t)),moving_names:[...new Set(moving)],service_circuits,focus,cargo:'PaperSeat',anchors:{product_staging:'ProductStaging',aux_seat:'WorkingAuxiliary'},envelope:[3.6,2.5],workY};
}
