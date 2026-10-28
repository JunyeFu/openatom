// Original CC0 geometry repairs. Existing phase names, holder anchors and units remain intact.
export function revise(root,family,animations,h){
  const {T,group,box,cyl,rod,ring,bolt,cable,mesh,M,simple}=h,pi=Math.PI;
  const find=n=>root.getObjectByName(n);
  function remove(n){const o=find(n);if(o)o.removeFromParent();}
  function boredPlate(w,height,thickness,holes,up=false){
    const s=new T.Shape();s.moveTo(-w/2,-height/2);s.lineTo(w/2,-height/2);s.lineTo(w/2,height/2);s.lineTo(-w/2,height/2);s.closePath();
    for(const [x,y,r] of holes){const p=new T.Path();p.absarc(x,y,r,0,pi*2,true);s.holes.push(p);}
    const g=new T.ExtrudeGeometry(s,{depth:thickness,bevelEnabled:false,curveSegments:simple?12:40,steps:1});g.translate(0,0,-thickness/2);if(up)g.rotateX(-pi/2);return g;
  }
  function hollow(radius,inner,length){return new T.LatheGeometry([[inner,-length/2],[radius,-length/2],[radius,length/2],[inner,length/2],[inner,-length/2]].map(v=>new T.Vector2(...v)),simple?20:64);}
  function labelPad(p,n,pos){box(p,n,[.038,.001,.018],pos,M.orange,.001);}
  if(['assembly','rework','packing','inspection'].includes(family)){
    const frame=find('BenchFrame');
    // The original mains cable terminated freely. The terminal now belongs to the lower leg.
    box(frame,'LegMainsInlet',[.055,.065,.050],[-.51,.31,-.25],M.dark,.004);
    cyl(frame,'MainsStrainRelief',.010,.024,[-.49,.325,-.25],M.rubber,'x');
    for(const z of [-.273,-.227])bolt(frame,'MainsInletFix'+z,[-.54,.31,z],'x',.003,-1);
    // A bonded tray and attached service manifold, rather than disconnected decorative tubes.
    box(frame,'PneumaticManifold',[.16,.045,.05],[-.30,.64,-.30],M.al,.004);
    for(let i=0;i<3;i++){
      cyl(frame,'ManifoldOutlet'+i,.006,.018,[-.35+i*.05,.67,-.30],M.steel);
      box(frame,'ValveLabel'+i,[.018,.001,.013],[-.35+i*.05,.663,-.274],M.blue,.001);
    }
    box(frame,'AirInletBracket',[.030,.050,.045],[-.58,.50,-.30],M.steel,.002);
    cyl(frame,'AirQuickCoupler',.009,.040,[-.60,.50,-.30],M.steel,'x');
    cable(frame,'ManifoldFeed',[[-.40,.64,-.30],[-.46,.64,-.30],[-.55,.54,-.30],[-.58,.50,-.30]],.004);
    for(const x of [-.48,.48])for(const z of [-.35,.35])bolt(frame,'LowerShelfFix'+x+'_'+z,[x,.276,z*.86],'y',.003);
    if(!simple){
      // A visible, attached edge gasket and inset fasteners help resolve sheet thickness at close range.
      for(const z of [-.402,.402])box(frame,'TopEdgeBand'+z,[1.28,.008,.006],[0,.737,z],M.dark,.001);
      for(const x of [-.61,.61])for(const z of [-.28,.28])bolt(frame,'WorktopFix'+x+'_'+z,[x,.771,z],'y',.003);
    }
  }
  if(['assembly','rework'].includes(family)){
    for(const x of [-.435,.435]){
      box(root,'BallScrewEndSupport'+x,[.042,.065,.07],[x,1.465,-.226],M.dark,.004);
      cyl(root,'EndBearingCover'+x,.018,.009,[x+Math.sign(x)*.026,1.465,-.226],M.steel,'x');
      for(const y of [1.44,1.49])bolt(root,'EndSupportFix'+x+'_'+y,[x,y,-.187],'z',.003);
    }
    // Staging pads stay inside the existing envelope; no extra conveyor or stock is invented.
    for(const x of [-.41,.41]){
      box(root,'StagingTray'+x,[.25,.012,.17],[x,.778,.19],M.dark,.003);
      for(const z of [.11,.27])box(root,'StagingLip'+x+'_'+z,[.25,.016,.008],[x,.789,z],M.al,.001);
    }
    const spindle=find('Z_Spindle');
    ring(spindle,'ColletLockRing',.0155,.002,[0,1.102,.037],M.dark,'y');
    for(let i=0;i<8&&!simple;i++){
      const a=i*pi/4;box(spindle,'ColletSlot'+i,[.0007,.027,.0007],[.0148*Math.cos(a),1.077,.037+.0148*Math.sin(a)],M.dark,.0001);
    }
    if(family==='rework'){
      // Connect the existing extraction hose end to a supported filter unit.
      box(root,'ExtractionFilter',[.18,.29,.17],[-.46,.425,.25],M.blue,.008);
      cyl(root,'ExtractionInlet',.021,.024,[-.553,.40,.28],M.dark,'x');
      cable(root,'ExtractionEndCoupler',[[-.52,.40,.28],[-.54,.40,.28],[-.557,.40,.28]],.017);
      for(let i=0;i<6;i++)box(root,'FilterExhaustSlot'+i,[.12,.005,.003],[-.46,.35+i*.018,.338],M.dark,.001);
      for(const x of [-.51,-.41])bolt(root,'FilterFoot'+x,[x,.279,.25],'y',.004);
    }
  }
  if(family==='packing'){
    // The old rod stayed fixed while CartonPusher moved 39 mm. Make its extension explicit.
    remove('PusherRod');const extension=group(root,'PusherRodExtension',[.35,.84,0]);
    cyl(extension,'PusherRodChrome',.007,.1,[-.05,0,0],M.steel,'x');extension.scale.x=.54;
    const strokes={load:[.54,.93],auto:[.93,.93],unload:[.93,.54]};
    animations=animations.map(a=>new T.AnimationClip(a.name,a.duration,[...a.tracks,new T.VectorKeyframeTrack('PusherRodExtension.scale',[0,1],strokes[a.name].flatMap(x=>[x,1,1]))]));
    for(const x of [.37,.49]){
      box(root,'PusherBodyClamp'+x,[.024,.065,.074],[x,.84,0],M.dark,.003);
      box(root,'PusherClampFoot'+x,[.055,.012,.090],[x,.776,0],M.steel,.002);
      box(root,'PusherClampRiser'+x,[.024,.0255,.072],[x,.79475,0],M.dark,.002);
      for(const z of [-.033,.033])bolt(root,'CylinderMountFix'+x+'_'+z,[x,.782,z],'y',.003);
    }
    const gate=find('PackingGate');
    for(const x of [-.23,.23]){
      box(gate,'GateStiffener'+x,[.018,.025,.32],[x,.018,.185],M.al,.002);
      for(const z of [.05,.32])bolt(gate,'GateStiffenerFix'+x+'_'+z,[x,.032,z],'y',.003);
    }
    labelPad(root,'CartonAlignmentMark',[-.21,.799,.16]);
  }
  if(family==='robot'){
    for(const x of [-.24,.24])for(const z of [-.20,.20])ring(root,'BaseAnchorWasher'+x+'_'+z,.018,.003,[x,.069,z],M.al,'y');
    const wrist=find('J6_ToolRoll');
    box(wrist,'ToolAirDistribution',[.040,.031,.036],[.14,.048,-.085],M.dark,.003);
    if(!simple){for(let i=0;i<12;i++){const a=i*pi/6;cyl(wrist,'ToolFlangeSocket'+i,.002,.002,[.042*Math.cos(a),.018,.042*Math.sin(a)],M.dark);}}
  }
  if(family==='conveyor'){
    const deck=find('ConveyorDeck');
    for(const x of [-.45,0,.45]){
      cyl(deck,'ReturnIdler'+x,.014,.44,[x,.636,0],M.dark,'z');
      for(const z of [-.24,.24])box(deck,'IdlerBearing'+x+'_'+z,[.035,.040,.025],[x,.636,z],M.al,.002);
    }
    for(const x of [-.68,.68])for(const z of [-.294,.294]){
      cyl(deck,'TensionAdjustment'+x+'_'+z,.006,.062,[x,.66,z],M.steel,'x');
      ring(deck,'AdjustLocknut'+x+'_'+z,.009,.002,[x,.66,z],M.dark,'x');
    }
    cable(deck,'OutSensorSupply',[[.63,.73,-.25],[.63,.61,-.27],[.36,.51,-.27],[.36,.51,.27]],.004);
    box(deck,'MotorCableGland',[.024,.025,.024],[-.73,.63,.42],M.dark,.002);
    cable(deck,'DriveCable',[[-.73,.63,.42],[-.69,.57,.42],[-.45,.53,.30],[.29,.53,.29]],.005);
  }
  if(family==='inspection'){
    box(root,'FocusUpperStop',[.085,.024,.060],[0,1.459,-.19],M.dark,.003);
    box(root,'FocusLowerStop',[.085,.024,.060],[0,1.075,-.19],M.dark,.003);
    cyl(root,'FocusAdjustScrew',.006,.36,[.057,1.27,-.17],M.steel);
    cyl(root,'FocusAdjustKnob',.024,.022,[.057,1.47,-.17],M.dark);
    const head=find('OpticalHead');
    box(head,'LensMountPlate',[.11,.014,.10],[0,1.285,.016],M.al,.002);
    for(const x of [-.04,.04])bolt(head,'LensPlateFix'+x,[x,1.293,.016],'y',.0025);
  }
  if(family==='sealer'){
    const head=find('TapeHead');
    // Service fasteners and a supported blade guard remain fixed to the moving cassette.
    for(const x of [-.145,.145])for(const z of [-.067,.067])bolt(head,'CassetteServiceFix'+x+'_'+z,[x,1.102,z],'y',.003);
    box(head,'CutterGuard',[.075,.046,.105],[.11,.989,0],M.orange,.004);
    box(head,'CutterGuardBracket',[.085,.014,.125],[.11,1.018,0],M.steel,.002);
    for(const z of [-.050,.050])bolt(head,'GuardMount'+z,[.11,1.028,z],'y',.003);
    if(!simple)for(let i=0;i<5;i++)box(head,'GuardVent'+i,[.035,.003,.001],[.11,.977+i*.006,.053],M.dark,.0002);
  }
  if(family==='shell'||family==='control-box'){
    // Cut actual material from the front wall and mounting ears, not black dots on solid faces.
    find('HousingWallZ0.086').geometry=boredPlate(.24,.057,.008,[[-.05,-.0065,.006],[.05,-.0065,.006]]);
    for(const x of [-.07,.07]){
      find('MountLug'+x).geometry=boredPlate(.022,.032,.006,[[0,-.005,.003]],true);remove('LugHole'+x);
    }
    for(const x of [-.05,.05]){
      find('CableGland'+x).geometry=hollow(.011,.006,.016);remove('ConnectorBore'+x);
    }
  }
  if(family==='pcb'||family==='control-box'){
    find('FR4Board').geometry=boredPlate(.216,.154,.002,[-.103,.103].flatMap(x=>[-.073,.073].map(z=>[x,z,.0027])),true);
    for(const x of [-.103,.103])for(const z of [-.073,.073])remove('ThroughHole_'+x+'_'+z);
    const pcb=find('PCBAssembly');
    for(const x of [-.103,.103])for(const z of [-.073,.073]){
      const pad=find('PlatedHole_'+x+'_'+z),copper=pad.material;
      pad.geometry=hollow(.004,.0027,.00008);pad.rotation.set(0,0,0);pad.position.y=.00204;
      mesh(pcb,'PlatedBarrel'+x+'_'+z,hollow(.0027,.00255,.002),copper,[x,.001,z]);
      mesh(pcb,'BackPlatedPad'+x+'_'+z,hollow(.004,.0027,.00008),copper,[x,-.00004,z]);
    }
    if(!simple){
      const copper=new T.MeshStandardMaterial({name:'Copper finish',color:'#b48b49',metalness:.82,roughness:.32});
      for(let i=0;i<8;i++){
        const z=-.056+i*.016;
        box(pcb,'BackCopperTrace'+i,[.14,.00010,.00065],[-.009,-.00006,z],copper,.0001);
        box(pcb,'BackTraceBranch'+i,[.00065,.00010,.009],[.06,-.00006,z+.0042],copper,.0001);
      }
    }
  }
  if(family==='fastener'){
    remove('HelicalThread');
    find('ScrewShaft').geometry=new T.LatheGeometry([[0,0],[.0011,0],[.0012,.001],[.0012,.030],[0,.030]].map(v=>new T.Vector2(...v)),simple?20:48);
    if(!simple){
      // A helical V-section ridge: 0.5 mm pitch, 3 mm major diameter. Not a round wire coil.
      const turns=54,steps=turns*40,pitch=.0005,section=[[.00119,-pitch*.42],[.0015,-pitch*.09],[.0015,pitch*.09],[.00119,pitch*.42]],pos=[],uv=[],idx=[];
      for(let i=0;i<=steps;i++){const a=i/steps*turns*pi*2,y=.001+i/steps*turns*pitch;for(const [r,dy]of section){pos.push(r*Math.cos(a),y+dy,r*Math.sin(a));uv.push(i/steps,(dy/pitch+.5));}}
      for(let i=0;i<steps;i++)for(let k=0;k<3;k++){const a=i*4+k,b=a+4;idx.push(a,b,a+1,b,b+1,a+1);}
      const g=new T.BufferGeometry();g.setAttribute('position',new T.Float32BufferAttribute(pos,3));g.setAttribute('uv',new T.Float32BufferAttribute(uv,2));g.setIndex(idx);g.computeVertexNormals();mesh(root,'MachinedVThread',g,M.steel);
    }
  }
  if(family==='carton'){
    // Scores belong to their individual flap, so a folded pose does not leave floating creases.
    for(const n of ['MajorFlapFront','MajorFlapRear']){
      const flap=find(n),sign=n.endsWith('Front')?-1:1;
      box(flap,'FoldScore'+n,[.466,.0002,.0012],[0,.0011,sign*.003],M.wood2,.00005);
    }
    for(const n of ['MinorFlapLeft','MinorFlapRight']){
      const flap=find(n),sign=n.endsWith('Right')?-1:1;
      box(flap,'FoldScore'+n,[.0012,.0002,.312],[sign*.003,.0011,0],M.wood2,.00005);
    }
  }
  if(family==='pallet'){
    // Underside fastening joins bottom runners to blocks; no baked cartons are added.
    for(const x of [-.50,0,.50])for(const z of [-.32,0,.32])for(const dx of [-.045,.045])cyl(root,'UndersideNail'+x+'_'+z+'_'+dx,.002,.0006,[x+dx,.00035,z],M.dark);
  }
  if(family==='sealing-tape'){
    const core=find('HollowCardboardCore');core.material=M.card;
    // Both wound faces receive lamination detail, not only the front display face.
    if(!simple)for(let i=0;i<8;i++)ring(root,'RearTapeLayer'+i,.045+i*.0045,.00016,[0,.080,-.02425],M.wood2,'z');
  }
  return animations;
}
