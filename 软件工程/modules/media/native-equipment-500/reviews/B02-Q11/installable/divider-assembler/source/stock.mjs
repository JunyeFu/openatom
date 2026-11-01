import * as T from 'three';
import{geometry}from './geometry.mjs';
import{specs,seat,stagingX,workY,wasteSeat,ramp,done,pileLift}from './batch-spec.mjs';
export function rectangle(L,W){const s=new T.Shape();s.moveTo(-L/2,-W/2);s.lineTo(L/2,-W/2);s.lineTo(L/2,W/2);s.lineTo(-L/2,W/2);s.closePath();return s;}
export function slab(s,H){const g=new T.ExtrudeGeometry(s,{depth:H,bevelEnabled:false,curveSegments:12});g.rotateX(Math.PI/2);g.translate(0,H,0);return g;}
export function notched(lower=false){const pts=[[-.15,-.04],[.15,-.04],[.15,.04],[.0515,.04],[.0515,0],[.0485,0],[.0485,.04],[-.0485,.04],[-.0485,0],[-.0515,0],[-.0515,.04],[-.15,.04]],s=new T.Shape();pts.forEach(([x,z],i)=>i?s.lineTo(x,lower?-z:z):s.moveTo(x,lower?-z:z));s.closePath();return s;}
export function bag(api,p){const g=api.group(p,'PaperBagBody');for(const z of[-.04,.04])api.box(g,'BagWideWall_'+z,[.18,.20,.0005],[0,.10,z],api.m.paper,false);for(const x of[-.09,.09])api.box(g,'BagSideWall_'+x,[.0005,.20,.08],[x,.10,0],api.m.paper,false);api.box(g,'BagBottom',[.18,.0005,.08],[0,.00025,0],api.m.paper,false);return g;}
function handles(api,p){const g=api.group(p,'PrefabricatedHandles');for(const sign of[-1,1]){const h=api.group(g,'Handle_'+sign,[0,0,sign*.04075]);api.cable(h,'PaperRope_'+sign,[[-.055,.020,0],[-.055,.065,0],[0,.100,0],[.055,.065,0],[.055,.020,0]],.0025).material=api.m.paper;for(const x of[-.055,.055])api.box(h,'HandlePatch_'+sign+'_'+x,[.020,.025,.001],[x,.0125,0],api.m.paper,false);}return g;}
function pallet(api,p){const g=api.group(p,'BoardPallet');for(const x of[-.12,0,.12])api.box(g,'PalletRunner_'+x,[.04,.05,.24],[x,.025,0],api.m.edge);for(let i=0;i<7;i++)api.box(g,'PalletDeck_'+i,[.045,.020,.28],[-.15+i*.05,.060,0],api.m.paper,false);return g;}
function boards(api,p,lower=false,cross=false){const g=api.group(p,cross?'CrossPartitionPair':'LongPartitionPair');for(const sign of[-1,1]){const holder=api.group(g,(cross?'CrossPartition_':'LongPartition_')+sign,cross?[sign*.05,0,0]:[0,0,sign*.05]);holder.rotation.y=cross?Math.PI/2:0;const b=api.mesh(holder,(cross?'CrossSlottedBoard_':'LongSlottedBoard_')+sign,slab(notched(lower),.002),[0,.04,0],api.m.paper);b.rotation.x=-Math.PI/2;}return g;}
function rope(api,p){const g=api.group(p,'TwistedPaperCord');for(let j=0;j<3;j++){const pts=Array.from({length:97},(_,i)=>{const f=i/96,a=2*Math.PI*(8*f+j/3);return new T.Vector3(-.15+.30*f,.00345+.00185*Math.cos(a),.00185*Math.sin(a));});api.mesh(g,'PaperStrand_'+j,new T.TubeGeometry(new T.CatmullRomCurve3(pts),96,.0016,6,false),[0,0,0],api.m.paper);}return g;}
function bale(api,p){const g=api.group(p,'CompressedPaperBale');api.box(g,'BaleBody',[.28,.12,.20],[0,.06,0],api.m.paper);for(let i=1;i<8;i++)api.box(g,'CompressedEdge_'+i,[.28,.0006,.2002],[0,i*.015,0],api.m.edge,false);return g;}
function straps(api,p,wrapped){const g=api.group(p,wrapped?'WrappedStraps':'StraightStraps');for(const x of[-.07,.07]){if(!wrapped){const roll=api.mesh(g,'CoiledStrap_'+x,new T.TorusGeometry(.030,.003,6,24),[x,.003,0],api.m.dark);roll.rotation.x=Math.PI/2;}else for(const[y,z,h,d]of[[0,0,.0007,.202],[.12,0,.0007,.202],[.06,-.10,.12,.0007],[.06,.10,.12,.0007]])api.box(g,'BaleStrap_'+x+'_'+y+'_'+z,[.008,h,d],[x,y,z],api.m.dark,false);}return g;}
export function stock(id,role){
 const s=specs[id],api=geometry(false),root=new T.Group();root.name='StockRoot';const parts=[],add=(o,pose,times)=>parts.push({o,pose,times}),visible=(o,fn,inverse=false,times=[0,.5499,.55,.6499,.65,1])=>add(o,(ph,u,active)=>({s:[1,1,1].map(()=>Number(inverse?!fn(ph,u,active):fn(ph,u,active)))}),times);
 if(role==='blocker')api.box(root,'OutputOccupant',[.10,.04,.10],[0,.02,0],api.m.amber,false);
 else if(role==='auxiliary'&&s.glue_ml){const H=s.glue_ml*1e-6/(Math.PI*.007**2);api.cyl(root,'MeteredGlue',.007,H,[0,H/2,0],api.m.glue);}
 else if(role==='auxiliary'){
  const raw=straps(api,root,false),wrap=straps(api,root,true);visible(raw,(ph,u,a)=>a&&done(ph,u,.65),true);visible(wrap,(ph,u,a)=>a&&done(ph,u,.65));
  add(wrap,(ph,u)=>({p:[(ph==='unload'?stagingX*u:0)-.90,0,.52]}));
 }else if(role==='component'){
  let object;if(s.kind==='handle')object=handles(api,root);else if(s.kind==='divider')object=boards(api,root,true,true);else object=pallet(api,root);
  add(object,(ph,u,active)=>{if(!active)return{p:[0,0,0]};if(ph==='load'){const k=ramp(u,.20,.80);return{p:[-1+k,s.kind==='handle'?.175*ramp(u,.20,.80):s.kind==='divider'?.12*ramp(u,.80,1):0,.52*(1-k)]};}return{p:[ph==='unload'?stagingX*u:0,s.kind==='handle'?.175:s.kind==='divider'&&ph==='auto'?.12*(1-ramp(u,.15,.55)):0,0]};});
 }else if(role==='waste'){for(const x of[-.016,.016])api.box(root,'SlotOffcut_'+x,[.003,.002,.04],[x,.001,0],api.m.paper,false);}
 else if(s.kind==='handle'){
  bag(api,root);const patches=api.group(root,'AdhesivePatches');for(const z of[-.0403,.0403])for(const x of[-.055,.055])api.box(patches,'AppliedGlue_'+x+'_'+z,[.020,.025,.0002],[x,.1875,z],api.m.glue,false);visible(patches,(ph,u,a)=>role==='output'||a&&done(ph,u,.35),false,[0,.3499,.35,1]);if(role==='output')handles(api,root).position.y=.175;
 }else if(s.kind==='rope'){
  const raw=api.group(root,'PaperStripLot');for(let i=0;i<3;i++)api.box(raw,'PaperStrip_'+i,[.36,.0004,.006],[0,.0002,(i-1)*.008],api.m.paper,false);const formed=rope(api,root);
  for(const obj of[raw,formed])add(obj,(ph,u,active)=>({p:[0,role==='output'?0:active?(ph==='load'?.03*ramp(u,.75,1):ph==='auto'?.03:.03*(1-ramp(u,0,.30))):0,0],s:[1,1,1].map(()=>Number(obj===raw?!(role==='output'||active&&done(ph,u)):(role==='output'||active&&done(ph,u))))}));
 }else if(s.kind==='slot'){
  const raw=api.mesh(root,'UncutPartition',slab(rectangle(.30,.08),.002),[0,0,0],api.m.paper),cut=api.mesh(root,'SlottedPartition',slab(notched(),.002),[0,0,0],api.m.paper);visible(raw,(ph,u,a)=>role==='output'||a&&done(ph,u),true);visible(cut,(ph,u,a)=>role==='output'||a&&done(ph,u));
  if(role==='input'){const chips=api.group(root,'SeparatingSlotOffcuts');for(const x of[-.05,.05])api.box(chips,'Cutout_'+x,[.003,.002,.04],[x,.001,.02],api.m.paper,false);add(chips,(ph,u,a)=>{const f=ph==='unload'?1:ph==='auto'?ramp(u,.55,.80):0,k=ramp(f,.45,.80),b=seat(id,ph,u);return{p:[.2*k-(ph==='unload'?b[0]:0),-.26*ramp(f,0,.45)-.09*ramp(f,.80,1),-.5*k],s:[1,1,1].map(()=>Number(a&&done(ph,u)))};},[0,.5499,.55,.65,.75,.80,1]);}
 }else if(s.kind==='divider'){
  boards(api,root);if(role==='output')boards(api,root,true,true);
 }else if(s.kind==='baler'){
  const raw=api.group(root,'FourLoosePaperCharges');for(let j=0;j<4;j++)for(let i=0;i<8;i++){const o=api.box(raw,'LooseSheet_'+j+'_'+i,[.12,.002,.09],[(j%2-.5)*.14,.006+i*.033,(Math.floor(j/2)-.5)*.10],api.m.paper,false);o.rotation.y=(i%3-1)*.15;}const block=bale(api,root);visible(block,(ph,u,a)=>role==='output'||a&&done(ph,u));add(raw,(ph,u,a)=>({s:(role==='output'||a&&done(ph,u))?[0,0,0]:[1,1-.5*(a&&ph==='auto'?ramp(u,.15,.55):0),1]}));if(role==='output')straps(api,root,true);
 }else{
  const stack=api.group(root,'TenBoardStack');for(let i=0;i<10;i++)api.box(stack,'CorrugatedBoard_'+i,[.30,.0038,.22],[0,.002+i*.004,0],i%2?api.m.paper:api.m.edge,false);add(stack,(ph,u)=>({p:[0,role==='output'?.07:pileLift(ph,u),0]}));if(role==='output')pallet(api,root);
 }
 return{root,parts};
}
