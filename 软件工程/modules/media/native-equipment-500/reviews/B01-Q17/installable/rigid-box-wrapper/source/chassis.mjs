import * as T from 'three';
import{geometry,V,clip}from './geometry.mjs';
import{specs,samples,seat,inputPort,outputPort,outputY,auxPort,auxSeat,stagingX,workY,ramp,opening,minorFold,majorFold,tapeFraction,glueFraction,pressFraction,carton as c,blank,flatCase}from './chassis-spec.mjs';
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
 box(root,'AirPreparationMount',[.20,.25,.015],[.95,.61,.80]);box(root,'AirRegulator',[.18,.10,.06],[.95,.61,.84],m.dark);for(const x of[.90,1.00])cyl(root,'AirBowl_'+x,.023,.13,[x,.505,.84],m.lens);cable(root,'CabinetServiceBundle',[[.78,.70,-.61],[.45,.70,-.69],[-.55,.70,-.69],[1.10,1.55,-.54]]);
 cable(root,'HMIServiceBundle',[[1.16,1.00,.94],[1.16,.35,.94],[.94,.35,-.58]]);
 const cargo=group(root,'PaperSeat');pos(cargo,(ph,u)=>seat(ph,u));group(root,'ProductStaging',[stagingX,outputY(spec.id),0]);group(root,'WorkingAuxiliary',auxSeat());for(const[role,p]of[['input',inputPort()],['output',outputPort(spec.id)],...(spec.auxiliary?[['auxiliary',auxPort()]]:[])])group(root,'Port_'+role,p);group(root,'OperatorSeat',[1.1,0,1.18]);
 function gantry(prefix,height=1.75){for(const x of[-.62,.62]){profile(frame,prefix+'SupportCross_'+x,1.50,[x,.73,0],'z');for(const z of[-.54,.54]){profile(root,prefix+'Upright_'+x+'_'+z,height-.77,[x,(height+.77)/2,z]);box(root,prefix+'Shoe_'+x+'_'+z,[.12,.015,.12],[x,.7675,z],m.dark);bolt(root,prefix+'Bolt_'+x+'_'+z,[x+.035,.778,z]);}}for(const z of[-.54,.54])profile(root,prefix+'Top_'+z,1.32,[0,height,z],'x');for(const x of[-.62,.62])profile(root,prefix+'Bridge_'+x,1.14,[x,height,0],'z');}
 // Supported guards leave working parts readable from the front and side.
 for(const z of[-.69,.69]){for(const x of[-.64,.64]){profile(frame,'GuardCross_'+x+'_'+z,1.50,[x,.73,0],'z');box(root,'GuardShoe_'+x+'_'+z,[.08,.010,.08],[x,.765,z],m.dark);profile(root,'GuardPost_'+x+'_'+z,.88,[x,1.21,z]);}box(root,'GuardRail_'+z,[1.34,.03,.04],[0,1.66,z],m.blue);box(root,'GuardPanel_'+z,[1.22,.70,.006],[0,1.26,z],m.glass,false);}

 root.updateMatrixWorld(true);return{root,animations:Object.entries(tracks).map(([name,t])=>clip(name,t)),moving_names:moving,service_circuits:[],focus:'PaperSeat',cargo:'PaperSeat',anchors:{product_staging:'ProductStaging',aux_seat:'WorkingAuxiliary'},envelope:[3.6,2.5],workY};
}
