"""Author the L03-specific 0.2.0 media without changing frozen 0.1.0 packages."""
import json
from pathlib import Path
import shutil

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
OLD=HERE.parent/'p302-line-grade/installable'
families=['assembly','robot','conveyor','inspection','rework','packing','sealer','pallet','control-box','carton','shell','pcb','fastener','sealing-tape']
source=(OLD/'robot/source/generate.mjs').read_text(encoding='utf-8')

# The fixed contact poses are solved for the actual L03 pickup and belt-entry points.
# This is an authored two-link contact trajectory, not an industrial reachability claim.
a=source.index('  // Fixed illustrative contact pose:')
b=source.index('\n}\nfunction conveyor',a)
source=source[:a]+'''  const L1=1.05,L2=1.115;
  function contact(x,y,z){
    const radius=Math.hypot(x,z),height=y+.326-.49;
    const c=(radius*radius+height*height-L1*L1-L2*L2)/(2*L1*L2);
    if(Math.abs(c)>1)throw Error('Declared L03 contact is outside authored arm geometry');
    const elbow=Math.acos(c),shoulder=Math.atan2(radius,height)-Math.atan2(L2*Math.sin(elbow),L1+L2*Math.cos(elbow));
    const yaw=Math.atan2(z,-x);
    return [yaw,shoulder,elbow,0,pi-shoulder-elbow,yaw+pi];
  }
  const pickup=contact(-1.7,.8375,-.9),place=contact(.95,.8375,-.9);
  const raisedPick=contact(-1.7,1.10,-.9),raisedPlace=contact(.95,1.10,-.9);
  const keys=[raisedPick,pickup,pickup,place,place,raisedPick],times=[0,2,3,8,9,12];
  const names=['J1_BaseYaw','J2_Shoulder','J3_Elbow','J4_ForearmRoll','J5_WristPitch','J6_ToolRoll'];
  const axes=[[0,1,0],[0,0,1],[0,0,1],[0,1,0],[0,0,1],[0,1,0]];
  names.forEach((n,i)=>root.getObjectByName(n).quaternion.copy(Q(axes[i],keys[0][i])));
  function phase(name,left,right,jawA,jawB){
    return clip(name,[...names.map((n,i)=>quatTrack(n,[0,1],axes[i],[left[i],right[i]])),
      poses('JawLeft',[0,1],[[-jawA,.08,0],[-jawB,.08,0]]),poses('JawRight',[0,1],[[jawA,.08,0],[jawB,.08,0]])]);
  }
  return [phase('approach',raisedPick,pickup,.165,.165),phase('grip',pickup,pickup,.165,.143),
    phase('loaded_move',pickup,place,.143,.143),phase('release',place,place,.143,.165),
    phase('return',place,raisedPick,.165,.165)];
''' + source[b:]
a=source.index('function robot(');b=source.index('function conveyor(',a)
r=source[a:b].replace("shell(j2,'UpperArmShell',.58,","shell(j2,'UpperArmShell',1.05,")
r=r.replace("'J3_Elbow',[0,.58,0]","'J3_Elbow',[0,1.05,0]")
r=r.replace("shell(j3,'ForearmShell',.53,","shell(j3,'ForearmShell',.98,")
r=r.replace("'J4_ForearmRoll',[0,.53,0]","'J4_ForearmRoll',[0,.98,0]")
source=source[:a]+r+source[b:]

# Real material structures, including an unsealed/ sealed visual state from the
# authoritative package attribute. No material quantity is inferred from size.
source=source.replace('function gauge(root,type){','function originalGauge(root,type){')
material_models='''
function gauge(root,type){
  if(type==='control-box'){originalGauge(root,type);return [];}
  if(type==='shell'){
    box(root,'HousingBase',[.24,.008,.18],[0,.004,0],M.al);
    for(const z of [-.086,.086])box(root,'HousingWallZ'+z,[.24,.057,.008],[0,.0365,z],M.white);
    for(const x of [-.116,.116])box(root,'HousingWallX'+x,[.008,.057,.18],[x,.0365,0],M.white);
    for(const x of [-.103,.103])for(const z of [-.073,.073])cyl(root,'PCBStandoff'+seq++,.004,.018,[x,.017,z],M.al);
  }else if(type==='pcb'){
    box(root,'CircuitBoard',[.216,.002,.154],[0,.001,0],M.belt);
    for(let i=0;i<4;i++){box(root,'IntegratedCircuit'+i,[.025,.007,.024],[-.065+i*.044,.0055,0],M.dark);
      for(const z of [-.016,.016])for(let j=0;j<4;j++)box(root,'Lead'+seq++,[.002,.001,.007],[-.074+i*.044+j*.006,.003,z],M.steel,.0002);}
    for(const x of [-.103,.103])for(const z of [-.073,.073])ring(root,'MountingPad'+seq++,.004,.001,[x,.002,z],M.al,'y');
  }else if(type==='fastener'){
    cyl(root,'ScrewShaft',.0015,.034,[0,.017,0],M.steel);cyl(root,'SocketHead',.0045,.003,[0,.0355,0],M.dark);
    cyl(root,'HexSocket',.0018,.0005,[0,.0371,0],M.rubber);
    if(!simple){const ps=Array.from({length:129},(_,i)=>{const u=i/128,a=u*pi*16;return V([.0016*Math.cos(a),.001+.032*u,.0016*Math.sin(a)]);});
      mesh(root,'ThreadHelix',new T.TubeGeometry(new T.CatmullRomCurve3(ps),128,.00025,5,false),M.steel);}
  }else if(type==='sealing-tape'){
    const roll=mesh(root,'RepresentativeTapeRoll',new T.TorusGeometry(.080,.035,simple?8:16,simple?24:64),M.card,[0,.035,0]);roll.rotation.x=pi/2;
    ring(root,'CardboardCore',.044,.003,[0,.035,0],M.wood2,'y');
  }else if(type==='carton'){
    box(root,'CartonBottom',[.49,.003,.34],[0,.0015,0],M.card,.0005);
    for(const x of [-.2435,.2435])box(root,'CartonWallX'+x,[.003,.356,.34],[x,.181,0],M.card,.0005);
    for(const z of [-.1685,.1685])box(root,'CartonWallZ'+z,[.484,.356,.003],[0,.181,z],M.card,.0005);
    for(const z of [-.0845,.0845])box(root,'FoldedUnsealedFlap'+z,[.484,.002,.169],[0,.359,z],M.card,.0003);
    box(root,'FlapMeetingLine',[.484,.0003,.001],[0,.3596,0],M.dark,.00005);
    const tape=box(root,'ClosureTape',[.49,.0006,.046],[0,.3597,0],M.wood2,.0001);tape.scale.setScalar(.000001);
    plate(root,'PackageLabel',[.03,.19,.1705],.105,.045);group(root,'GaugeSeat');
    return [clip('open',[new T.VectorKeyframeTrack('ClosureTape.scale',[0,1],[.000001,.000001,.000001,.000001,.000001,.000001])]),
            clip('sealed',[new T.VectorKeyframeTrack('ClosureTape.scale',[0,1],[1,1,1,1,1,1])])];
  }else throw Error('Undeclared material model: '+type);
  group(root,'GaugeSeat');return [];
}
'''
source=source.replace('const pose=o=>',material_models+'\nconst pose=o=>')

# Carton pack fixture and sealing head are physically authored for the original
# .49 x .34 footprint and .36 high package, with unchanged table/roller height.
a=source.index("  if(variant==='packing'){");b=source.index('\n  const work=',a)
r=source[a:b]
for old,new in [('[.4,.018,.31]','[.56,.018,.42]'),('[-.14,.14]','[-.19,.19]'),('[.37,.05,.012]','[.53,.05,.012]'),
                ('.205','.290'),('.166','.251'),('[.33,.84,0]','[.43,.84,0]'),('[.22,.84,0]','[.30,.84,0]'),
                ('[.27,.84,0]','[.37,.84,0]'),('1.025','1.165'),('-.13','-.185'),('[.36,.014,.26]','[.53,.014,.37]'),
                ('[0,0,.13]','[0,0,.185]'),('[-.25,.25]','[-.32,.32]'),(',.56,',',.70,'),(',.36,[x,.95',',.50,[x,1.02'),('1.13','1.27')]:r=r.replace(old,new)
source=source[:a]+r+source[b:]
a=source.index('function sealer(');b=source.index('function pallet(',a)
r=source[a:b].replace('-.176,.176','-.231,.231').replace('[.43,.64,.176]','[.43,.64,.231]')
r=r.replace('*.232','*.287').replace("'TapeHead',[0,0,0]","'TapeHead',[0,.14,0]")
r=r.replace("'LiftColumn'+z,.82,[0,1.1,z]","'LiftColumn'+z,.96,[0,1.17,z]")
r=r.replace("'HeadLiftScrew'+z,.009,.75,[.065,1.1,z]","'HeadLiftScrew'+z,.009,.89,[.065,1.17,z]")
r=r.replace("'TopBridge',.84,[0,1.51,0]","'TopBridge',.84,[0,1.65,0]")
source=source[:a]+r+source[b:]
source=source.replace("group(root,'CartonSeat',[0,.72,0])","group(root,'CartonSeat',[-.65,.72,0])")

# A conveyor cargo anchor follows the same declared mechanical animation.
source=source.replace("const marker=group(root,'BeltMarker'", "group(root,'BeltCargo',[-.65,.805-.081,0]);const marker=group(root,'BeltMarker'")
source=source.replace("clip('auto',[poses('BeltMarker'", "clip('auto',[poses('BeltCargo',[0,1],[[-.65,.724,0],[.65,.724,0]]),poses('BeltMarker'")

# Explicit authored logical ports are not inferred by consumers from node names.
extra='''
const portPoints={
 assembly:{shell:[-.43,.805,.22],pcb:[-.43,.805,-.20],fastener:[.43,.805,.22],assembled:[0,.805,0]},
 rework:{input:[0,.805,0],output:[0,.805,0]},
 packing:{product:[-.32,.798,0],carton:[0,.798,0],packed:[0,.798,0]},
 inspection:{input:[0,.805,0],passed:[.48,.805,.0],rework:[0,.805,-.36],quarantine:[-.48,.805,.0]},
 sealer:{loaded:[-.65,.798,0],sealed:[.65,.798,0]},
 conveyor:{input:[-.65,.805,0],output:[.65,.805,0]},
 robot:{pickup:[-1.7,.805,-.9],place:[.95,.805,-.9]},pallet:{input:[0,.1448,0],output:[0,.1448,0]}
};
function authoredPorts(root,family){
  for(const [id,point]of Object.entries(portPoints[family]??{}))group(root,'L03_Port_'+id,point);
  if(family==='sealer')group(root.getObjectByName('TapeRoll'),'L03_Port_tape',[0,0,0]);
  group(root,'L03_Operator',[0,0,1.02]);
}
'''
source=source.replace('const builders={',extra+'\nconst builders={')
source=source.replace("if(suffix==='fit'){const holder", "authoredPorts(root,family);if(suffix==='fit'){const holder")
old="['load','auto','unload','cycle','working'].map((phase_ref,i)=>({id:'line-grade-'+phase_ref,phase_ref,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i<3?i:1}},transition:'hold'}))"
new="j.animations.map((anim,i)=>({id:'l03-'+anim.name,phase_ref:anim.name,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:i}},transition:'hold'})).concat(['robot','carton'].includes(family)?[]:['cycle','working'].map(phase_ref=>({id:'l03-'+phase_ref,phase_ref,start_progress:0,end_progress:1,source:{kind:'model_animation',data:{asset_ref:asset,animation_index:1}},transition:'hold'})))"
assert old in source;source=source.replace(old,new)
source=source.replace("anchor_roles:{operator:{kind:'viewport',data:{x:.8,y:.8}}},port_anchors:{input:anchor(seat),output:anchor(family==='conveyor'?'TailRoller':seat)}", "anchor_roles:{...Object.fromEntries(Object.keys(portPoints[family]??{}).concat(family==='sealer'?['tape']:[]).map(id=>[id,anchor('L03_Port_'+id)])),operator:anchor('L03_Operator'),cargo:anchor(family==='conveyor'?'BeltCargo':seat)},port_anchors:Object.fromEntries(Object.keys(portPoints[family]??{}).concat(family==='sealer'?['tape']:[]).map(id=>[id,anchor('L03_Port_'+id)]))")
# Item models have no operator declaration or device logical port set.
source=source.replace("authoredPorts(root,family);if", "authoredPorts(root,family);if")
source=source.replace("const animations=builders[family]?builders[family](root):(gauge(root,family),[]);", "let animations=builders[family]?builders[family](root):gauge(root,family);if(family==='sealer')animations=animations.map((a,i)=>{const xs=[[-.65,-.455],[-.455,.035],[.035,.65]][i];return new T.AnimationClip(['infeed','seal','outfeed'][i],a.duration,[...a.tracks,poses('CartonSeat',[0,1],xs.map(x=>[x,.72,0]))]);});")
source=source.replace("if(!builders[family]){value.timeline=[];", "if(!builders[family]&&family!=='carton'){value.timeline=[];")
source=source.replace("['control-box','carton'].includes(family)?'GaugeSeat'", "['control-box','carton','shell','pcb','fastener','sealing-tape'].includes(family)?'GaugeSeat'")

registration=[]
for family in families:
    for suffix in ['simple','detailed']:
        template=family if family in ['assembly','robot','conveyor','inspection','rework','packing','sealer','pallet','control-box','carton'] else 'control-box'
        old=OLD/(template+('-simple' if suffix=='simple' else ''))
        out=HERE/'installable'/(family+('-simple' if suffix=='simple' else ''))
        (out/'source').mkdir(parents=True,exist_ok=True)
        for f in ['nameplate.png','LICENSE.txt']:shutil.copyfile(old/f,out/f)
        for f in ['quality-checklist.json','quality-amendments.json']:
            text=(old/'source'/f).read_text(encoding='utf-8').replace('320x230x220mm carton top touches pressure roller at 940mm','490x340x360mm package top touches actual TapeContact at1158mm')
            text=text.replace('line-grade-pre-export-','l03-exact0.2-pre-export-').replace('Root still owns actual layout placement, real item attachment, industrial validation and Native adoption.','Current actual Host owns original layout/phase/holder and fresh-reopen evidence; Native and industrial/human acceptance remain separate.')
            text=text.replace('"carton_top_m": 1.018','"carton_top_m": 1.158').replace('one scalar shoulder angle for a fixed elbow','authored two-link shoulder/elbow contact poses')
            (out/'source'/f).write_text(text,encoding='utf-8')
        p=json.loads((old/'source/parameters.json').read_text(encoding='utf-8'))
        p['version']='0.2.0';p['carton_gauge']=dict(x=.49,y=.36,z=.34)
        p['export_filter']=dict(family=family,suffix=suffix)
        p['l03_variant']='Original L03 contact/phase/port/pack envelope; legacy0.1 immutable'
        (out/'source/parameters.json').write_text(json.dumps(p,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
        (out/'source/generate.mjs').write_text(source,encoding='utf-8')
        registration.append(dict(id='p302.line-grade.'+family+('.simple' if suffix=='simple' else ''),version='0.2.0',directory=str(out.relative_to(ROOT)).replace('\\','/')))
registration.sort(key=lambda r:('.simple' not in r['id'],r['id']))
(HERE/'registration.json').write_text(json.dumps(dict(register_order=registration),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Prepared28 new exact0.2 media sources; frozen0.1 untouched. Run each source/generate.mjs to export.')
