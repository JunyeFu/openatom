import * as T from 'three';
import {geometry} from './geometry.mjs';
import {specs,book,saddle,wire,stack,workY,stagingX,seat,caseFold,cutDone,wasteSeat,ramp} from './batch-spec.mjs';

export function rectangle(L,W,r=0){const s=new T.Shape();if(!r){s.moveTo(-L/2,-W/2);s.lineTo(L/2,-W/2);s.lineTo(L/2,W/2);s.lineTo(-L/2,W/2);s.closePath();return s;}s.moveTo(-L/2+r,-W/2);s.lineTo(L/2-r,-W/2);s.absarc(L/2-r,-W/2+r,r,-Math.PI/2,0);s.lineTo(L/2,W/2-r);s.absarc(L/2-r,W/2-r,r,0,Math.PI/2);s.lineTo(-L/2+r,W/2);s.absarc(-L/2+r,W/2-r,r,Math.PI/2,Math.PI);s.lineTo(-L/2,-W/2+r);s.absarc(-L/2+r,-W/2+r,r,Math.PI,Math.PI*1.5);s.closePath();return s;}
export function bore(s,x,z,r){const p=new T.Path();p.absarc(x,z,r,0,Math.PI*2,true);s.holes.push(p);return s;}
export function slab(shape,H){const g=new T.ExtrudeGeometry(shape,{depth:H,bevelEnabled:false,curveSegments:12});g.rotateX(Math.PI/2);g.translate(0,H,0);return g;}
export function cornerOffcut(sx,sz){const R=stack.cornerR,points=[[0,R],[R,R],[R,0],...Array.from({length:9},(_,i)=>[R*Math.cos(i*Math.PI/16),R*Math.sin(i*Math.PI/16)])];const s=new T.Shape();for(const[p,i]of points.map((p,i)=>[p,i])){const x=sx*(stack.L/2-R+p[0]),z=sz*(stack.W/2-R+p[1]);if(i===0)s.moveTo(x,z);else s.lineTo(x,z);}s.closePath();return s;}

function pages(api,parent,name,L,W,H,shape=null,upright=false,lineInset=0){
 const g=api.group(parent,name);if(shape)api.mesh(g,name+'Paper',slab(shape,H),[0,0,0],[api.m.paper,api.m.edge]);else api.box(g,name+'Paper',[L,H,W],[0,H/2,0],api.m.paper,false);
 for(let i=1;i<=8;i++)for(const sign of[-1,1]){
  if(upright)api.box(g,name+'Leaves_'+i+'_'+sign,[.00012,H,.00010],[sign*(L/2+.00004),H/2,-W/2+W*i/9],api.m.edge,false);
  else api.box(g,name+'Leaves_'+i+'_'+sign,[L-2*lineInset,.00012,.00010],[0,H*i/9,sign*(W/2+.00004)],api.m.edge,false);
 }
 return g;
}
export function casePanels(api,parent){
 const back=api.group(parent,'CaseBack'),spine=api.group(parent,'CaseSpine'),front=api.group(parent,'CaseFront');
 for(const[o,W]of[[back,book.boardW],[spine,book.spine],[front,book.boardW]])api.box(o,o.name+'Board',[book.caseL,book.boardT,W],[0,book.boardT/2,0],api.m.blue,false);
 return[{o:back,pose:()=>({p:[0,0,book.boardW/2]})},{o:spine,pose:(ph,u)=>{const a=caseFold(ph,u)*Math.PI/2;return{p:[0,book.spine/2*Math.sin(a),-book.spine/2*Math.cos(a)],r:['x',a]};}},{o:front,pose:(ph,u)=>{const a=caseFold(ph,u)*Math.PI/2;return{p:[0,book.spine*Math.sin(a)+book.boardW/2*Math.sin(2*a),-book.spine*Math.cos(a)-book.boardW/2*Math.cos(2*a)],r:['x',2*a]};}}];
}
function staple(api,parent,name,at,formed){
 const o=api.group(parent,name,at),R=.00035;
 if(!formed)api.rod(o,name+'Straight',[0,0,-.021],[0,0,.021],R,api.m.steel);
 else{api.rod(o,name+'Crown',[0,0,-.006],[0,0,.006],R,api.m.steel);for(const z of[-1,1]){api.rod(o,name+'Leg'+z,[0,0,z*.006],[0,-.008,z*.006],R,api.m.steel);api.rod(o,name+'Clinched'+z,[0,-.008,z*.006],[0,-.003,z*.001],R,api.m.steel);}}
 return o;
}
export function stock(id,role){
 const s=specs[id],api=geometry(false),root=new T.Group();root.name='StockRoot';api.m.paper.color.set('#ded8c6');api.m.edge.color.set('#b1a58e');const parts=[],add=(o,pose,times)=>parts.push({o,pose,times});
 const phase=(ph,u,active)=>active?[ph,u]:['load',0];
 if(role==='blocker')api.box(root,'OutputOccupant',[.10,.04,.10],[0,.02,0],api.m.amber,false);
 else if(role==='auxiliary'&&s.glue_ml){const H=s.glue_ml*1e-6/(Math.PI*.007**2);api.cyl(root,'MeteredGlue',.007,H,[0,H/2,0],api.m.glue);api.cyl(root,'GlueDoseLid',.008,.002,[0,H+.001,0],api.m.dark);}
 else if(role==='auxiliary'&&s.kind==='stitch'){
  for(let i=0;i<2;i++){
   const raw=staple(api,root,'WireSegment_'+i,[0,0,0],false),bound=staple(api,root,'FormedStaple_'+i,[0,0,0],true);
   add(raw,(ph,u,active)=>({p:[0,.00035,(i-.5)*.018],s:[1,1,1].map(()=>Number(!active||ph==='load'||ph==='auto'&&u<.35))}));
   add(bound,(ph,u,active)=>{const p=seat(id,ph,u),a=[saddle.staples[i],workY+saddle.apex+.001,0],anchor=[.90,workY,-.52];return{p:a.map((v,k)=>v+(ph==='unload'?p[k]-[0,workY,0][k]:0)-anchor[k]),s:[1,1,1].map(()=>Number(active&&(ph==='unload'||ph==='auto'&&u>=.35)))};},[0,.3499,.35,.55,.8,1]);
  }
 }else if(role==='component'&&s.kind==='case'){
  const carrier=api.group(root,'CaseCarrier');root.add(carrier);for(const p of casePanels(api,carrier))add(p.o,(ph,u,active)=>p.pose(...phase(ph,u,active)));
  add(carrier,(ph,u,active)=>{if(!active)return{p:[0,0,0]};const k=ph==='load'?ramp(u,0,.35):1;return{p:[-1+k+(ph==='unload'?stagingX*u:0),0,.52*(1-k)-(ph==='unload'?book.boardW/2*u:0)]};});
 }else if((role==='component'||role==='output')&&s.kind==='wire'){
  const carrier=api.group(root,'TwinWireCarrier');
  for(const[x,i]of wire.holes.map((x,i)=>[x,i]))for(const pair of[-1,1])for(let j=0;j<wire.segments;j++){
   const piece=api.cyl(carrier,'TwinWire_'+i+'_'+pair+'_'+j,wire.rodR,1,[0,0,0],api.m.steel);
   add(piece,(ph,u,active)=>{const f=role==='output'?1:active?(ph==='unload'?1:ph==='auto'?ramp(u,.22,.70):0):0,a=Math.PI/3*(1-f)+.02*f,span=(Math.PI*2-2*a)/wire.segments,t=a+(j+.5)*span;return{p:[x+pair*.0007,wire.cy+wire.r*Math.sin(t),wire.cz+wire.r*Math.cos(t)],r:['x',-t],s:[1,2*wire.r*Math.sin(span/2),1]};});
  }
  api.rod(carrier,'ContinuousTwinWireBackbone',[-.092,wire.cy,wire.cz-wire.r],[.092,wire.cy,wire.cz-wire.r],wire.rodR,api.m.steel);
  if(role==='component')add(carrier,(ph,u,active)=>{if(!active)return{p:[0,0,0]};if(ph==='load'){const k=ramp(u,.55,.85);return{p:[-1+k,.06*(ramp(u,.50,.55)-ramp(u,.85,1)),.52+(-.04-.52)*k]};}return{p:[ph==='unload'?stagingX*u:0,0,ph==='auto'?-.04*(1-ramp(u,0,.15)):0]};});
  if(role==='output')pages(api,root,'BoundPages',wire.L,wire.W,wire.H,wire.holes.reduce((s,x)=>bore(s,x,-.064,wire.holeR),rectangle(wire.L,wire.W))).position.y=wire.pageLift;
 }else if(role==='waste'){
  if(s.kind==='drill'){for(let i=0;i<3;i++)api.cyl(root,'CollectedPaperCore_'+i,.0023,stack.H,[(i-1)*.012,stack.H/2,0],api.m.paper);for(let i=0;i<8;i++)api.box(root,'DrillDust_'+i,[.002,.0005,.003],[((i*3)%7-3)*.004,.00025,((i*5)%7-3)*.004],api.m.edge,false);}
  else for(const sx of[-1,1])for(const sz of[-1,1]){const g=slab(cornerOffcut(sx,sz),stack.H);g.translate(-sx*(stack.L/2-.005)+sx*.018,0,-sz*(stack.W/2-.005)+sz*.018);api.mesh(root,'CollectedCorner_'+sx+'_'+sz,g,[0,0,0],api.m.paper);}
 }else if(s.kind==='case'||s.kind==='glue'){
  const body=pages(api,root,'BookBlock',book.L,s.kind==='glue'?book.H:book.W,s.kind==='glue'?book.W:book.H,null,s.kind==='glue');
  if(s.kind==='case'){
   add(body,(ph,u,active)=>{const f=role==='output'?1:active?(ph==='load'?ramp(u,.45,1):1):0;return{p:[0,book.boardT*f+(ph==='load'&&active?.08*(ramp(u,.45,.60)-ramp(u,.90,1)):0),book.caseBlockZ*f-(ph==='unload'?book.boardW/2*u:0)]};});
   if(role==='output'){const carrier=api.group(root,'BoundCover');for(const p of casePanels(api,carrier))add(p.o,()=>p.pose('unload',1));carrier.position.z=-book.boardW/2;}
  }else{
   add(body,(ph,u)=>{const f=role==='output'?1:ph==='unload'?ramp(u,0,.40):0,a=-Math.PI/2*f;return{p:[0,book.H/2*Math.sin(Math.abs(a)),book.W/2*Math.sin(Math.abs(a))],r:['x',a]};});
   const film=api.group(body,'SpineAdhesive');api.box(film,'CoatedSpine',[.190,.00015,book.H],[0,.000075,0],api.m.glue,false);
   add(film,(ph,u,active)=>({p:[0,0,0],s:[1,1,1].map(()=>Number(role==='output'||active&&(ph==='unload'||ph==='auto'&&u>=.30)))}),[0,.2999,.30,.8,1]);
  }
 }else if(s.kind==='stitch'){
  for(const sign of[-1,1]){const g=api.group(root,'SignatureWing_'+sign,[0,saddle.apex,0]);api.box(g,'GatheredLeaves_'+sign,[saddle.L,saddle.t,saddle.W],[0,saddle.t/2,sign*saddle.W/2],api.m.paper,false);g.rotation.x=sign*Math.PI/4;}
  if(role==='output')for(let i=0;i<2;i++)staple(api,root,'FinishedStaple_'+i,[saddle.staples[i],saddle.apex+.001,0],true);
 }else if(s.kind==='wire'){
  const body=pages(api,root,'PrepunchedPages',wire.L,wire.W,wire.H,wire.holes.reduce((s,x)=>bore(s,x,-.064,wire.holeR),rectangle(wire.L,wire.W)));
  add(body,(ph,u,active)=>({p:[0,active?(ph==='load'?wire.pageLift*ramp(u,.20,.50):wire.pageLift):0,0]}));
 }else{
  const solid=pages(api,root,'SquareStack',stack.L,stack.W,stack.H),shape=s.kind==='drill'?stack.holes.reduce((g,x)=>bore(g,x,stack.holeZ,stack.holeR),rectangle(stack.L,stack.W)):rectangle(stack.L,stack.W,stack.cornerR),machined=pages(api,root,'MachinedStack',stack.L,stack.W,stack.H,shape,false,s.kind==='round'?stack.cornerR:0);
  for(const[o,inverse]of[[solid,true],[machined,false]])add(o,(ph,u,active)=>{const on=role==='output'||active&&cutDone(ph,u);return{p:[0,0,0],s:[1,1,1].map(()=>Number(inverse?!on:on))};},[0,.5499,.55,.8,1]);
  if(role==='input'){
   const chips=api.group(root,'SeparatingCuttings');
   if(s.kind==='drill')for(let i=0;i<3;i++)api.cyl(chips,'CutCore_'+i,.0023,stack.H,[stack.holes[i],stack.H/2,stack.holeZ],api.m.paper);
   else for(const sx of[-1,1])for(const sz of[-1,1])api.mesh(chips,'CutCorner_'+sx+'_'+sz,slab(cornerOffcut(sx,sz),stack.H),[0,0,0],api.m.paper);
   add(chips,(ph,u,active)=>{const on=active&&cutDone(ph,u),f=ph==='unload'?1:ph==='auto'?ramp(u,.62,.80):0,base=seat(id,ph,u),dest=wasteSeat(),lat=ramp(f,.45,.80),y=-.26*ramp(f,0,.45)-.09*ramp(f,.80,1);return{p:[dest[0]*lat-(ph==='unload'?base[0]:0),y,dest[2]*lat],s:[1,1,1].map(()=>Number(on))};},[0,.5499,.55,.62,.68,.70,.764,.80,1]);
  }
 }
 return{root,parts};
}
