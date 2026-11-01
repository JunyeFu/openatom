// Mechanical assemblies for the additional industrial families. Units: metres.
// Empty fixtures/tooling only; stock belongs to the simulation consumer.
export function extraAssembly(c){
 const {p,tag:tag,category:k,i,simple,M,group,box,cyl,ring,rod,bolt,profile,motor,bearing,pedestal,translated,rotated,fixedGantry,rotaryShaft}=c,PI=Math.PI;
 const move=(name,pos=[0,.30,0])=>group(p,name+'_'+i,pos);
 const wheel=(parent,r=.18,axis='z')=>{cyl(parent,'ToolWheel',r,.045,[0,0,0],M.steel,axis);cyl(parent,'WheelHub',.045,.09,[0,0,0],M.dark,axis);};
 const stand=()=>{for(const z of [-.34,.34])profile(p,.48,[0,.24,z]);};
 const port=(x,y,z)=>{cyl(p,'ProcessPort',.035,.10,[x,y,z],M.al,'z');ring(p,'PortFlange',.05,.008,[x,y,z+.04],M.steel);};
 if(k==='horizontaltool'||k==='sidetool'){
  box(p,'HorizontalBed',[.82,.045,.65],[0,.04,0],M.dark);for(const z of [-.22,.22])profile(p,.74,[0,.12,z],'x',.04);
  const slide=move('ToolSaddle',[-.22,.18,0]);box(slide,'CrossSlide',[.28,.09,.49],[0,0,0],M.al);box(slide,'SpindleStand',[.20,.30,.20],[0,.18,0],M.blue);cyl(slide,'HorizontalSpindle',.055,.24,[.05,.30,0],M.steel,'x');motor(slide,[-.13,.30,-.1],'x');
  const radius=tag==='flare'?.075:tag==='endform'?.060:.010;cyl(slide,'HorizontalTool',radius,.18,[.25,.30,0],M.dark,'x',tag==='flare'?.012:radius);
  if(['lathetool','spinroller','flanger','balance'].includes(tag)){box(slide,'TurningTool',[.22,.035,.06],[.24,.27,0],M.dark);cyl(slide,'ContactRoller',.048,.04,[.32,.30,0],M.steel,'z');}
  box(p,'EndStop',[.07,.18,.36],[.33,.16,0],M.blue);translated(slide,0,.12);return;
 }
 if(k==='processhead'){
  fixedGantry(p,.92);const h=move('ProcessSlide',[0,.54,0]);box(h,'HeadBackplate',[.23,.28,.07],[0,.02,-.08],M.al);cyl(p,'LinearActuator',.042,.25,[0,.80,0],M.blue);cyl(h,'GuideRod',.015,.40,[0,.18,0]);
  const weld=['spotweld','projectionweld','migweld','tigweld','laserweld','studweld','induction'].includes(tag);
  if(tag==='induction'||tag==='capinduction'){ring(h,'InductionCopperCoil',.095,.012,[0,-.23,0],M.amber,'y');for(const z of [-.08,.08])rod(h,'CoilLead',[.04,-.22,z],[.04,-.03,z],.008,M.amber);}
  else if(weld){cyl(h,'TorchBody',.037,.19,[0,-.06,0],M.dark);cyl(h,'CopperElectrode',.012,.15,[0,-.22,0],M.amber);if(tag==='spotweld'||tag==='projectionweld')cyl(p,'LowerElectrode',.018,.08,[0,.09,0],M.amber);if(tag==='migweld')cyl(h,'WireGuide',.005,.19,[.028,-.11,0],M.steel);}
  else if(['microdispense','sniffer','utprobe','spectrometer','sideprobe','stylus','leakhead','forceprobe','uvhead'].includes(tag)){box(h,'InstrumentBody',[.13,.16,.14],[0,0,0],M.blue);cyl(h,'MeasuringTip',tag==='utprobe'?.025:.006,.16,[0,-.16,0],M.steel);if(tag==='uvhead')ring(h,'UVOptic',.045,.012,[0,-.15,0],M.lens,'y');}
  else if(['hone','superfinish','burnish','gearshape','slothead','sinkedm'].includes(tag)){cyl(h,'Quill',.04,.19,[0,-.04,0],M.al);cyl(h,'ToolShank',.015,.13,[0,-.19,0],M.dark);for(let q=0;q<4;q++){const a=q*PI/2;box(h,'ProcessInsert',[.017,.075,.017],[.023*Math.sin(a),-.24,.023*Math.cos(a)],M.dark,.001);}}
  else if(['caphead','seamhead','capcrimp','gaskethead','bearingpress','stopper','heatinsert','snappress'].includes(tag)){cyl(h,'PressSocket',.055,.20,[0,-.10,0],M.steel);ring(h,'SocketTool',.047,.012,[0,-.21,0],M.dark,'y');if(tag==='seamhead')for(const x of [-.07,.07])cyl(h,'SeamingRoller',.025,.065,[x,-.21,0],M.dark);}
  else if(['needle','overlock','buttonhole','needlebanks','steamhead'].includes(tag)){box(h,'SewingArm',[.30,.09,.12],[.10,-.10,0],M.white);cyl(h,'NeedleBar',.007,.15,[.20,-.20,0],M.steel);box(p,'PresserFoot',[.07,.01,.07],[.20,.055,0],M.steel);}
  else{box(h,'ToolBody',[.17,.18,.14],[0,0,0],M.blue);cyl(h,'ToolRam',.025,.18,[0,-.17,0],M.steel);box(h,'WorkingInsert',[.065,.045,.08],[0,-.27,0],M.dark);}
  translated(h,1,-.17);return;
 }
 if(k==='rollbank'){
  if(tag==='flatthread'){
   box(p,'ThreadRollingBed',[.84,.05,.64],[0,.04,0],M.dark);for(const z of [-.23,.23])profile(p,.76,[0,.12,z],'x',.04);
   box(p,'FixedThreadDie',[.64,.20,.07],[0,.25,-.10],M.steel);const slide=move('ReciprocatingDie',[-.15,.25,.10]);box(slide,'MovingThreadDie',[.64,.20,.07],[0,0,0],M.steel);
   for(let j=-5;j<=5;j++){const a=box(p,'FixedDieGroove',[.006,.17,.006],[j*.05,.25,-.061],M.dark,.001);a.rotation.z=.18;const b=box(slide,'MovingDieGroove',[.006,.17,.006],[j*.05,0,-.039],M.dark,.001);b.rotation.z=.18;}
   box(p,'SlideDriveMount',[.12,.18,.2],[-.36,.16,.1],M.blue);translated(slide,0,.28);return;
  }
  const rows=['openmill','tripleroll','quadplaner','draftroll','levelroll','straightroll'].includes(tag)?3:2;
  for(let j=0;j<rows;j++){const x=(j-(rows-1)/2)*.24,g=group(p,'RollTool_'+i+'_'+j,[x,.22+(j%2)*.12,0]);cyl(g,'RollBody',tag==='openmill'?.13:.095,.68,[0,0,0],M.steel,'z');for(const z of [-.39,.39]){bearing(p,x,g.position.y,z);pedestal(p,x,z,g.position.y-.07);}motor(p,[x,g.position.y,-.6]);rotated(g,[0,0,1],j%2?-2*PI:2*PI);
   if(['threadroll','flatthread','embossroll','seamroll','forgeroll','hullroll','compactroll'].includes(tag))for(let t=-5;t<=5;t++)ring(g,'ProcessGroove',.099,.004,[0,0,t*.05],M.dark,'z');
   if(['planer','jointer','quadplaner','opener','doffer','ripsaw','veneerknife'].includes(tag))for(let t=0;t<6;t++){const a=t*PI/3,b=box(g,'RotorKnife',[.018,.026,.64],[.097*Math.sin(a),.097*Math.cos(a),0],M.dark,.001);b.rotation.z=-a;}
  }return;
 }
 if(k==='bandsaw'||k==='beltgrind'){
  stand();box(p,'WheelSupportBeam',[.78,.06,.065],[0,.47,-.34],M.al);const band=move('BandWheels',[0,.48,0]);for(const x of [-.28,.28]){box(p,'BearingMount',[.10,.09,.28],[x,.45,-.20],M.steel);bearing(p,x,.48,-.07);const w=group(band,'BandWheel_'+i+'_'+x,[x,0,0]);wheel(w,.17);rotated(w);}
  for(const y of [-.17,.17])box(band,k==='bandsaw'?'ContinuousSawBlade':'AbrasiveBelt',[.56,.010,.095],[0,y,0],M.dark,.001);for(const x of [-.28,.28])ring(band,'BandReturn',.17,.014,[x,0,0],M.rubber);box(p,'WorkRest',[.55,.028,.30],[0,.32,.18],M.al);motor(p,[-.28,.48,-.25]);return;
 }
 if(k==='centerless'||k==='doublegrind'){
  box(p,'GrindingBase',[.85,.12,.7],[0,.08,0],M.dark);for(const x of [-.24,.24]){box(p,'WheelStand',[.15,.28,.22],[x,.25,-.12],M.blue);const w=group(p,'AbrasiveWheel_'+i+'_'+x,[x,.42,0]);wheel(w,x<0?.20:.13);rotated(w,[0,0,1],x<0?2*PI:-2*PI);motor(p,[x,.42,-.24]);}box(p,'WorkBlade',[.06,.20,.4],[0,.23,.04],M.steel);return;
 }
 if(k==='wireedm'){
  fixedGantry(p,.85);box(p,'DielectricTank',[.83,.17,.75],[0,.09,0],M.blue);const upper=move('WireGuide',[0,.64,0]);box(upper,'GuideBlock',[.10,.12,.10],[0,0,0],M.al);rod(upper,'ElectrodeWire',[0,-.03,0],[0,-.55,0],.0015,M.amber);box(p,'LowerWireGuide',[.1,.06,.1],[0,.07,0],M.al);translated(upper,0,.20);cyl(p,'WireDrive',.06,.12,[-.28,.74,-.32],M.dark,'z');return;
 }
 if(k==='bath'||k==='basket'){
  if(k==='bath'){for(const z of [-.34,.34])box(p,'BasinSide',[.86,.42,.025],[0,.23,z],M.steel);for(const x of [-.43,.43])box(p,'BasinEnd',[.025,.42,.70],[x,.23,0],M.steel);box(p,'BasinFloor',[.86,.025,.70],[0,.025,0],M.steel);for(const z of [-.36,.36])profile(p,.9,[0,.46,z],'x',.028);port(.43,.10,0);const v=move('DrainValve',[.43,.13,.10]);box(v,'ValveLever',[.10,.014,.025],[0,0,0],M.amber);rotated(v,[0,0,1],PI/2);if(['anodebath','platebath'].includes(tag))for(const x of [-.27,.27])box(p,'Busbar',[.035,.035,.74],[x,.47,0],M.amber);}
  else{fixedGantry(p,.90);const b=move('PerforatedBasket',[0,.42,0]);box(b,'BasketBottom',[.60,.015,.47],[0,0,0],M.steel);for(const z of [-.24,.24])for(let j=-3;j<=3;j++)rod(b,'BasketWire',[j*.09,0,z],[j*.09,.22,z],.004);rod(b,'LiftBail',[-.25,.25,0],[.25,.25,0],.007);cyl(p,'LiftCylinder',.035,.24,[0,.78,0],M.blue);rod(b,'LiftRod',[0,.23,0],[0,.50,0],.012);translated(b,1,-.20);}return;
 }
 if(k==='bender'||k==='punchindex'||k==='lap'||k==='tilter'){
  if(k==='tilter'){const cradle=move('Trunnion',[0,.50,0]);for(const z of [-.36,.36]){pedestal(p,0,z,.44);bearing(p,0,.50,z);}box(cradle,'TiltingCradle',[.70,.025,.60],[0,-.12,0],M.blue);for(const x of [-.33,.33])box(cradle,'CradleSide',[.025,.28,.60],[x,0,0],M.al);rotated(cradle,[0,0,1],PI*.75);motor(p,[0,.50,-.6]);}
  else{const g=move('RotaryTool',[0,.14,0]);cyl(g,'ToolPlaten',.35,.06,[0,0,0],k==='lap'?M.dark:M.steel);if(k==='bender')for(const x of [0,.23])cyl(g,'BendingDie',x===0?.11:.055,.13,[x,.09,0],M.steel);if(k==='punchindex')for(let j=0;j<8;j++){const a=j*PI/4;cyl(g,'TurretTool',.035,.14,[.24*Math.sin(a),.09,.24*Math.cos(a)],M.dark);}rotated(g,[0,1,0],k==='lap'?2*PI:PI/2);motor(p,[0,.13,-.47]);}return;
 }
 if(k==='spraytool'||k==='sensor'||k==='fan'||k==='heater'){
  fixedGantry(p,.65);
  if(k==='spraytool'){const s=move('NozzleManifold',[0,.50,0]);cyl(s,'Manifold',.026,.64,[0,0,0],M.al,'z');for(let j=-2;j<=2;j++)cyl(s,'Nozzle',.009,.075,[0,-.045,j*.12],M.amber);translated(s,0,.14);
   if(tag==='powderspray'||(tag==='blast'&&!c.mechanisms.includes('chamber'))){
    for(const x of [-.44,.44])box(p,'SprayBoothSide',[.025,.78,.82],[x,.42,0],M.white);box(p,'SprayBoothBack',[.88,.78,.025],[0,.42,-.41],M.blue);box(p,'SprayBoothRoof',[.91,.025,.84],[0,.82,0],M.white);box(p,'CatchTray',[.84,.035,.76],[0,.035,0],M.steel);box(p,'ExtractionPlenum',[.50,.26,.12],[0,.23,-.48],M.dark);for(let j=-3;j<=3;j++)box(p,'ExtractionGrille',[.40,.009,.006],[0,.17+j*.028,-.409],M.al,.001);
   }
  }
  else if(k==='fan'){const fan=move('DuctFan',[0,.38,0]);for(let j=0;j<6;j++){const b=box(fan,'Blade',[.29,.04,.05],[0,0,0],M.dark);b.rotation.z=j*PI/6;}ring(p,'FanDuct',.19,.025,[0,.38,0],M.al);rotated(fan);}
  else if(k==='heater'){for(let j=-2;j<=2;j++)box(p,'HeatingElement',[.55,.018,.04],[0,.30,j*.10],M.amber);const v=move('CoolingFan',[0,.55,0]);wheel(v,.10);rotated(v);}
  else{if(['metalloop','eddycoil','magnetyoke','lightcurtain'].includes(tag)){for(const z of [-.32,.32])box(p,'SensorColumn',[.055,.46,.055],[0,.27,z],M.blue);box(p,'SensorBridge',[.06,.05,.69],[0,.51,0],M.dark);}else{box(p,'SensorHead',[.13,.12,.13],[0,.45,0],M.blue);cyl(p,'OpticOrProbe',.02,.05,[0,.37,0],M.lens);box(p,'Readout',[.20,.12,.035],[.25,.25,.25],M.dark);} }return;
 }
 if(k==='feederbank'||k==='spindlebank'||k==='dualreel'){
  if(k==='dualreel'){for(const x of [-.23,.23]){for(const z of [-.30,.30])profile(p,.48,[x,.24,z]);const r=group(p,'PayoffMandrel_'+i+'_'+x,[x,.47,0]);cyl(r,'Mandrel',.035,.70,[0,0,0],M.steel,'z');for(const z of [-.24,.24])cyl(r,'Flange',.18,.02,[0,0,z],M.blue,'z');rotated(r);}}
  else if(k==='feederbank'){box(p,'FeederMount',[.82,.03,.68],[0,.12,0],M.dark);for(let j=-3;j<=3;j++){box(p,'FeedRail',[.65,.05,.045],[0,.17,j*.08],M.al);cyl(p,'EmptyReelHub',.05,.028,[-.28,.29,j*.08],M.dark,'z');}const shuttle=move('FeederIndex',[.1,.21,0]);box(shuttle,'IndexComb',[.07,.04,.61],[0,0,0],M.blue);translated(shuttle,0,.04);}
  else{for(let j=-2;j<=2;j++){const r=group(p,'WindingSpindle_'+i+'_'+j,[j*.16,.28,0]);cyl(r,'BareBobbin',.035,.36,[0,0,0],M.steel);for(const y of [-.17,.17])cyl(r,'BobbinFlange',.07,.014,[0,y,0],M.blue);rotated(r,[0,1,0]);pedestal(p,j*.16,0,.10);}box(p,'SpindleDrive',[.86,.07,.20],[0,.065,0],M.dark);}return;
 }
 if(k==='enclosure'){
  const shield=tag==='xray',h=tag==='autoclave'?.85:.78;
  for(const x of [-.43,.43])box(p,'EnclosureSide',[.035,h,.77],[x,h/2+.03,0],M.white);box(p,'ClosedBack',[.86,h,.035],[0,h/2+.03,-.39],M.blue);box(p,'ClosedRoof',[.90,.035,.82],[0,h+.045,0],M.white);
  const door=move('ProcessDoor',[-.42,h/2+.03,.40]);box(door,'DoorFrame',[.82,h-.05,.025],[.41,0,0],shield?M.steel:M.glass);box(door,'DoorGrip',[.025,.16,.035],[.73,0,.03],M.dark);rotated(door,[0,1,0],-PI*.48,'load');
  const bed=move('InternalCarrier',[0,.16,0]);box(bed,'ProcessShelf',[.63,.028,.57],[0,0,0],M.steel);translated(bed,0,.10);
  if(shield){box(p,'XraySource',[.14,.18,.14],[0,.59,0],M.dark);box(p,'Detector',[.32,.025,.30],[0,.08,0],M.dark);}else for(const y of [.32,.49])box(p,'InternalRack',[.64,.018,.58],[0,y,0],M.al);
  if(['climate','chiller','desiccant'].includes(tag))for(let j=0;j<7;j++)box(p,'AirGrille',[.25,.008,.004],[0,.14+j*.025,-.412],M.dark,.001);return;
 }
 if(k==='conveyor'||k==='transfer'||k==='balltable'||k==='gate'){
  if(k==='gate'){box(p,'ChuteBed',[.84,.02,.64],[0,.08,0],M.al);for(const z of [-.32,.32])box(p,'ChuteRail',[.84,.13,.025],[0,.15,z],M.blue);const g=move('Escapement',[0,.17,0]);box(g,'GatePlate',[.04,.18,.60],[0,0,0],M.dark);rotated(g,[0,0,1],PI*.40);return;}
  if(k==='balltable'){box(p,'BallDeck',[.84,.03,.70],[0,.07,0],M.al);for(let a=-3;a<=3;a++)for(let b=-2;b<=2;b++)cyl(p,'TransferBall',.018,.025,[a*.11,.096,b*.12],M.steel);return;}
  const belt=move('TransportFrame',[0,.12,0]);if(tag==='inclinefeed')belt.rotation.z=.24;
  for(const z of [-.35,.35])box(belt,'TransportRail',[.88,.08,.035],[0,0,z],M.blue);for(let j=-3;j<=3;j++)cyl(belt,'TransportRoller',.045,.65,[j*.12,.025,0],M.steel,'z');
  const drive=group(belt,'TransportDrive_'+i);for(let j=-3;j<=3;j++)for(const z of tag==='chainfeed'||tag==='clipchain'?[-.22,.22]:[0])box(drive,'TransportLink',[.045,.012,tag==='meshfeed'?.60:.055],[j*.12,.08,z],M.dark,.001);translated(drive,0,.10);motor(p,[-.35,.14,-.55]);
  if(k==='transfer')rotated(belt,[0,1,0],tag==='rollerturn'?PI/2:0);if(tag==='chainlift')translated(belt,1,.14);if(tag==='extendfeed')translated(belt,0,.18);if(tag==='crossbelt'){const b=group(belt,'CrossBelt_'+i);box(b,'CrossTrack',[.35,.02,.69],[0,.09,0],M.belt);translated(b,2,.12);}return;
 }
 if(k==='magazine'||k==='elevator'||k==='bowlfeed'){
  if(tag==='spiralfeed'){
   profile(p,1.23,[0,.62,0],'y',.08);for(const x of [-.32,.32])box(p,'SpiralBaseFoot',[.16,.018,.30],[x,.012,0],M.dark);
   const segments=72,turns=2,point=(u,r)=>{const a=u*2*PI*turns;return [r*Math.cos(a),.14+u*.93,r*Math.sin(a)];};
   for(let j=0;j<segments;j++){const a=j/segments,b=(j+1)/segments,m=(a+b)/2,q=m*2*PI*turns,xyz=point(m,.29),deck=box(p,'HelicalTrackSegment',[.10,.018,.16],xyz,M.belt,.001);deck.rotation.y=-q-PI/2;for(const r of [.21,.37])rod(p,'HelicalSideGuide',point(a,r).map((v,k)=>k===1?v+.065:v),point(b,r).map((v,k)=>k===1?v+.065:v),.006,M.steel);if(j%9===0)rod(p,'TrackSpoke',[0,xyz[1]-.03,0],xyz,.012,M.al);}
   const drive=move('SpiralDrivePulley',[.29,1.11,0]);cyl(drive,'DrivePulley',.045,.16,[0,0,0],M.dark,'z');rotated(drive);motor(p,[.29,1.11,-.25]);box(p,'DriveBridge',[.42,.04,.25],[.16,1.05,-.12],M.al);return;
  }
  if(k==='bowlfeed'){cyl(p,'BowlBase',.32,.09,[0,.09,0],M.dark);const b=move('FeedBowl',[0,.20,0]);ring(b,'BowlWall',.31,.035,[0,.10,0],M.steel,'y');for(let j=0;j<3;j++)ring(b,'SpiralTrack',.16+j*.05,.012,[0,j*.035,0],M.al,'y');box(b,'ExitTrack',[.30,.025,.07],[.29,.09,0],M.al);translated(b,1,.006);return;}
  for(const x of [-.34,.34])for(const z of [-.28,.28])profile(p,1.20,[x,.6,z]);
  if(k==='magazine'){for(const y of [.15,.40,.65,.90])box(p,'MagazineLedge',[.70,.015,.57],[0,y,0],M.blue);const f=move('MagazineLift',[0,.09,0]);box(f,'LiftFork',[.58,.03,.48],[0,0,0],M.steel);translated(f,1,.65);}
  else{const b=move('ElevatorCarrier',[0,.20,0]);for(let j=0;j<5;j++){box(b,'CarrierStep',[.52,.035,.25],[0,j*.16,0],M.blue);if(tag==='bucketlift')box(b,'BucketLip',[.52,.10,.02],[0,j*.16+.05,.12],M.al);}translated(b,1,.16);motor(p,[0,1.12,-.40]);}return;
 }
 if(k==='tensile'||k==='shaker'){
  if(k==='tensile'){for(const x of [-.28,.28]){cyl(p,'TestColumn',.027,1.15,[x,.58,0],M.steel);cyl(p,'TestLeadScrew',.012,1.15,[x,.58,-.10],M.dark);}box(p,'FixedCrosshead',[.72,.07,.27],[0,1.17,0],M.blue);const b=move('TestCrosshead',[0,.70,0]);box(b,'MovingCrosshead',[.72,.07,.25],[0,0,0],M.al);for(const q of [-1,1])box(q<0?p:b,'SpecimenGrip',[.10,.15,.10],[0,q<0?.18:-.16,0],M.dark);translated(b,1,.10);}
  else{cyl(p,'ShakerBody',.30,.42,[0,.24,0],M.blue);const b=move('Armature',[0,.48,0]);box(b,'SpecimenTable',[.64,.035,.57],[0,0,0],M.steel);for(const x of [-.22,.22])for(const z of [-.2,.2])bolt(b,[x,.025,z]);translated(b,1,.008);}return;
 }
 if(k==='horizontalmold'||k==='raminject'||k==='heatedmold'||k==='formvac'){
  if(k==='horizontalmold'){for(const y of [.17,.57])for(const z of [-.26,.26])rod(p,'TieBar',[-.40,y,z],[.4,y,z],.022);box(p,'FixedPlaten',[.08,.67,.69],[.36,.36,0],M.blue);const b=move('ClampPlaten',[-.23,.36,0]);box(b,'MovingPlaten',[.08,.65,.67],[0,0,0],M.al);box(b,'MouldHalf',[.15,.34,.37],[.11,0,0],M.dark);box(p,'FixedMould',[.11,.34,.37],[.24,.36,0],M.dark);translated(b,0,.20);}
  else if(k==='formvac'){box(p,'VacuumBox',[.76,.15,.65],[0,.10,0],M.blue);const b=move('SheetClamp',[0,.36,0]);for(const z of [-.3,.3])box(b,'SheetFrame',[.78,.025,.025],[0,0,z],M.al);for(const x of [-.38,.38])box(b,'FrameCross',[.025,.025,.60],[x,0,0],M.al);cyl(p,'ToolMould',.15,.12,[0,.23,0],M.steel,'y',.10);translated(b,1,-.10);}
  else if(k==='raminject'){cyl(p,'InjectionCylinder',.085,.62,[0,.39,0],M.al,'x');const b=move('InjectionRam',[-.30,.39,0]);cyl(b,'Plunger',.035,.40,[0,0,0],M.steel,'x');translated(b,0,.14);cyl(p,'ChargePot',.10,.23,[.18,.53,0],M.dark);}
  else{for(const x of [-.3,.3])for(const z of [-.28,.28])profile(p,.83,[x,.42,z]);box(p,'HeatedBed',[.70,.09,.63],[0,.10,0],M.dark);const b=move('HeatedPlaten',[0,.57,0]);box(b,'HotPlaten',[.69,.08,.61],[0,0,0],M.amber);cyl(p,'PressCylinder',.07,.24,[0,.77,0],M.al);translated(b,1,-.32);}return;
 }
 if(k==='dieblock'||k==='waterpellet'||k==='wave'||k==='gridknife'){
  box(p,'ToolBlock',[.42,.31,.45],[0,.20,0],M.al);port(-.25,.23,0);
  if(k==='gridknife'){for(let j=-3;j<=3;j++){box(p,'GridBlade',[.49,.03,.006],[0,.38,j*.07],M.dark,.001);box(p,'CrossBlade',[.006,.03,.49],[j*.07,.38,0],M.dark,.001);}const b=move('GridPusher',[0,.58,0]);box(b,'PusherPlate',[.47,.04,.47],[0,0,0],M.blue);translated(b,1,-.15);}
  else{const b=move('ToolGate',[.24,.23,0]);cyl(b,'DieRing',tag==='bubbledie'?.17:.095,.10,[0,0,0],M.dark,'x');if(tag==='flatdie')box(b,'SlitDie',[.10,.035,.64],[0,0,0],M.dark);if(tag==='stuffnozzle')cyl(b,'FillingNozzle',.025,.28,[.17,0,0],M.steel,'x');rotated(b,[1,0,0],PI/6);}return;
 }
 if(k==='printer'){
  for(const x of [-.38,.38])for(const z of [-.32,.32])profile(p,.95,[x,.48,z]);for(const z of [-.32,.32])profile(p,.77,[0,.95,z],'x');const gantry=move('PrintGantry',[0,.77,0]);profile(gantry,.73,[0,0,0],'z',.045);const head=group(gantry,'PrintHead_'+i);box(head,'HeadHousing',[.12,.12,.12],[0,-.09,0],M.dark);cyl(head,'NozzleOrOptic',.009,.06,[0,-.18,0],M.amber);const bed=move('BuildPlatform',[0,.30,0]);box(bed,'BuildPlate',[.64,.025,.56],[0,0,0],M.dark);translated(gantry,0,.22);translated(head,2,.20);translated(bed,1,-.08);
  if(tag==='sla')box(p,'ResinVatTooling',[.66,.07,.58],[0,.13,0],M.al);if(tag==='powderbed'||tag==='binderjet'){box(p,'PowderBedTooling',[.69,.15,.61],[0,.14,0],M.steel);box(gantry,'RecoaterBlade',[.028,.09,.60],[.15,-.28,0],M.al);}return;
 }
 if(k==='tankpair'||k==='dualscrew'||k==='dualdose'){
  for(const z of [-.22,.22]){cyl(p,'DualBody',.12,.45,[0,.37,z],M.steel,k==='dualscrew'?'x':'y');pedestal(p,0,z,.12);const b=group(p,'MeterDrive_'+i+'_'+z,[0,.64,z]);cyl(b,'DriveShaft',.018,.19,[0,0,0],M.steel);rotated(b,[0,1,0]);motor(p,[0,.7,z]);}return;
 }
 if(k==='tumble'||k==='mixer'||k==='rotorstator'||k==='banbury'||k==='decanter'){
  if(k==='tumble'){stand();const b=move('MixVessel',[0,.52,0]);if(tag==='vblend'){for(const x of [-.14,.14]){const a=cyl(b,'VesselLeg',.13,.56,[x,0,0],M.steel);a.rotation.z=x<0?-.5:.5;}}else{cyl(b,'UpperCone',.24,.31,[0,.15,0],M.steel,'y',.04);cyl(b,'LowerCone',.04,.31,[0,-.15,0],M.steel,'y',.24);}for(const z of [-.36,.36])bearing(p,0,.52,z);rotated(b,[0,0,1]);motor(p,[0,.52,-.58]);}
  else if(k==='decanter'){stand();const b=move('DecanterBowl',[0,.39,0]);cyl(b,'CylindricalBowl',.17,.57,[-.1,0,0],M.steel,'x');cyl(b,'ConicalBowl',.07,.28,[.32,0,0],M.steel,'x',.17);rotated(b,[1,0,0]);motor(p,[-.55,.39,0],'x');}
  else{box(p,'MixerTrough',[.78,.27,.65],[0,.20,0],M.steel);const b=move('MixingRotor',[0,.40,0]);cyl(b,'RotorShaft',.025,.70,[0,0,0],M.steel,'x');for(let j=0;j<5;j++){const a=j*PI*.4,g=group(b,'MixBlade_'+i+'_'+j,[-.28+j*.14,0,0]);box(g,'MixingBlade',[.045,.28,.035],[0,0,0],M.dark);g.rotation.x=a;}rotated(b,[1,0,0]);motor(p,[-.52,.40,0],'x');if(k==='rotorstator')ring(p,'Stator',.17,.023,[0,.40,0],M.al,'x');}return;
 }
 if(k==='tower'||k==='column'||k==='vacuumreceiver'||k==='weighhopper'||k==='filterbank'||k==='plateheater'||k==='holdtube'||k==='valveblock'||k==='aspirator'){
  stand();
  if(k==='plateheater'){for(let j=0;j<14;j++)box(p,'HeatExchangePlate',[.018,.55,.47],[-.27+j*.04,.42,0],j%2?M.al:M.steel);for(const z of [-.3,.3])rod(p,'ClampingTie',[-.35,.2,z],[.35,.2,z],.012);port(.3,.60,.25);port(-.3,.20,.25);}
  else if(k==='holdtube'){for(let j=0;j<6;j++){const y=.14+j*.11;rod(p,'HoldingTube',[-.32,y,0],[.32,y,0],.025,M.steel);if(j<5)rod(p,'ReturnBend',[j%2?.32:-.32,y,0],[j%2?.32:-.32,y+.11,0],.025,M.steel);}}
  else if(k==='valveblock'){box(p,'Manifold',[.70,.13,.14],[0,.32,0],M.al);for(let j=-2;j<=2;j++){cyl(p,'Solenoid',.025,.10,[j*.13,.42,0],M.dark);port(j*.13,.30,.10);}const b=move('ValveIndicator',[0,.50,0]);box(b,'Indicator',[.05,.02,.025],[0,0,0],M.amber);rotated(b,[0,1,0],PI/2);return;}
  else if(k==='filterbank'){const number=tag==='membrane'?4:tag==='baghouse'?6:2;for(let j=0;j<number;j++){const x=(j-(number-1)/2)*.12;cyl(p,'FilterHousing',.05,.65,[x,.47,0],M.steel);cyl(p,'HousingCap',.06,.03,[x,.81,0],M.blue);}rod(p,'InletHeader',[-.39,.14,0],[.39,.14,0],.025);rod(p,'OutletHeader',[-.39,.84,0],[.39,.84,0],.025);}
  else{const r=k==='column'?.13:.27,h=k==='column'?1.10:.62;cyl(p,'ProcessShell',r,h,[0,h/2+.18,0],M.steel);cyl(p,'ConeBottom',.055,.23,[0,.13,0],M.steel,'y',r);ring(p,'ServiceFlange',r+.007,.012,[0,h+.18,0],M.al,'y');port(0,.38,r+.02);if(k==='weighhopper')for(const z of [-.3,.3])box(p,'LoadCell',[.15,.025,.05],[0,.11,z],M.dark);if(k==='tower')rod(p,'CycloneInlet',[-.34,.7,0],[0,.7,0],.055);}
  const v=move('ProcessValve',[.30,.16,.18]);box(v,'ValveHandle',[.12,.016,.03],[0,0,0],M.amber);rotated(v,[0,1,0],PI/2);return;
 }
 if(k==='knitcylinder'||k==='needlebed'||k==='textilecarriage'||k==='rapier'||k==='rackturn'){
  if(k==='knitcylinder'){const b=move('NeedleCylinder',[0,.28,0]);cyl(b,'CylinderTool',.27,.30,[0,0,0],M.al);for(let j=0;j<40;j++){const a=j*PI/20;rod(b,'LatchNeedle',[.274*Math.sin(a),0,.274*Math.cos(a)],[.274*Math.sin(a),.22,.274*Math.cos(a)],.002,M.steel);}rotated(b,[0,1,0]);}
  else if(k==='rackturn'){const b=move('RackCarrier',[0,.1,0]);for(const y of [0,.2,.4,.6])box(b,'TrayShelf',[.60,.015,.50],[0,y,0],M.al);for(const x of [-.28,.28])profile(b,.65,[x,.32,0]);rotated(b,[0,1,0]);}
  else{for(const z of [-.30,.30])box(p,'NeedleRail',[.78,.04,.07],[0,.28,z],M.blue);for(let j=-12;j<=12;j++)rod(p,'Needle',[j*.028,.30,-.28],[j*.028,.31,.28],.002);const b=move('TextileCarriage',[-.25,.38,0]);box(b,'Carriage',[.17,.12,.54],[0,0,0],M.dark);translated(b,0,.50);if(k==='rapier')rod(b,'Rapier',[-.22,-.05,0],[.30,-.05,0],.008,M.steel);}return;
 }
 throw Error('No authored assembly '+k+' / '+tag);
}
