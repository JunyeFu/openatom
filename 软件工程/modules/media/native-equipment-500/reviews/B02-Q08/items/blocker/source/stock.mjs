import * as T from 'three';
import {geometry} from './geometry.mjs';
import {specs,forming,ramp} from './batch-spec.mjs';

// Rigid panels explain the operation; business quantities are never inferred from this mesh.
export function panelPlan(id){
 const s=specs[id],parts=[],add=(name,L,W,pose,kind='paper')=>parts.push({name,L,W,t:s.t,pose,kind});
 if(s.kind==='wrap'){
  const widths=[s.W,s.H,s.W,s.H,s.lap],flat=widths.reduce((a,b)=>a+b,0);
  for(let j=0;j<widths.length;j++)add('WrapPanel_'+j,s.L,widths[j],(ph,u)=>{
   const f=forming(ph,u),a=f*Math.PI/2,start=ph==='load'?-(flat/2)+(flat-s.W)/2*u:-s.W/2;
   let y=0,z=start;for(let k=0;k<j;k++){y+=widths[k]*Math.sin(k*a);z+=widths[k]*Math.cos(k*a);}
   return {p:[0,y+widths[j]/2*Math.sin(j*a)+(j===4?s.t*f:0),z+widths[j]/2*Math.cos(j*a)],axis:'x',a:-j*a};
  });
 }else if(s.kind==='bottom'){
  const B=s.L-s.flap;
  add('BagBody',B,s.W,(ph,u)=>({p:[-s.flap/2+(ph==='unload'?s.flap/2*u:0),0,0]}),'double');
  add('BottomFold',s.flap,s.W,(ph,u)=>{const f=forming(ph,u),a=Math.PI*f;return{p:[s.L/2-s.flap+s.flap/2*Math.cos(a)+(ph==='unload'?s.flap/2*u:0),s.flap/2*Math.sin(a)+(.004+2*s.t)*f,0],axis:'z',a};},'double');
 }else if(s.kind==='envelope'){
  add('EnvelopeFace',s.L,s.W,()=>({p:[0,0,0]}));
  for(const sign of[-1,1])add('SideFlap_'+sign,s.side,s.W,(ph,u)=>{const f=ph==='load'?0:ph==='unload'?1:ramp(u,.22,.42),a=Math.PI*f;return{p:[sign*(s.L/2+s.side/2*Math.cos(a)),s.side/2*Math.sin(a)+2*s.t*f,0],axis:'z',a:sign*a};});
  add('EnvelopeBack',s.L,s.W,(ph,u)=>{const f=ph==='load'?0:ph==='unload'?1:ramp(u,.46,.70),a=Math.PI*f;return{p:[0,s.W/2*Math.sin(a)+3*s.t*f,s.W/2+s.W/2*Math.cos(a)],axis:'x',a:-a};});
  add('UnsealedLid',s.L*.96,s.lid,()=>({p:[0,0,-s.W/2-s.lid/2]}));
 }else if(s.kind==='heatseal'){
  add('MailerLower',s.L,s.W,()=>({p:[0,0,0]}));
  add('MailerUpper',s.L,s.W,(ph,u)=>({p:[0,.006*(1-forming(ph,u))+s.t*forming(ph,u),0]}));
  add('ReleaseFlap',s.L*.10,s.W,()=>({p:[-s.L*.55,s.t,0]}));
 }else{
  add('LidFloor',s.L,s.W,()=>({p:[0,0,0]}));
  for(const sign of[-1,1]){
   add('LidEnd_'+sign,s.H,s.W,(ph,u)=>{const a=forming(ph,u)*Math.PI/2;return{p:[sign*(s.L/2+s.H/2*Math.cos(a)),s.t+s.H/2*Math.sin(a),0],axis:'z',a:sign*a};});
   add('LidSide_'+sign,s.L,s.H,(ph,u)=>{const a=forming(ph,u)*Math.PI/2;return{p:[0,s.t+s.H/2*Math.sin(a),sign*(s.W/2+s.H/2*Math.cos(a))],axis:'x',a:-sign*a};});
  }
  for(const x of[-1,1])for(const z of[-1,1])add('CornerEar_'+x+'_'+z,s.H,.008,(ph,u)=>{const f=forming(ph,u),a=Math.PI/2*f;return{p:[x*(s.L/2+s.H/2*Math.cos(a)+.0009*f),s.t+s.H/2*Math.sin(a),z*(s.W/2+.004-.008*f)],axis:'z',a:x*a};},'ear');
 }
 return parts;
}

export function stock(id,role){
 const s=specs[id],api=geometry(false),root=new T.Group();root.name='StockRoot';const parts=[];
 if(role==='blocker')api.box(root,'OutputOccupant',[.10,.04,.10],[0,.02,0],api.m.amber,false);
 else if(role==='auxiliary'){
  const H=s.glue_ml*1e-6/(Math.PI*.007**2);api.cyl(root,'MeteredAdhesive',.007,H,[0,H/2,0],api.m.glue);api.cyl(root,'DoseLid',.008,.002,[0,H+.001,0],api.m.dark);
 }else{
  for(const p of panelPlan(id)){
   const o=api.group(root,p.name),material=p.kind==='ear'?api.m.edge:api.m.paper;
   api.box(o,p.name+'Paper',[p.L,p.t,p.W],[0,p.t/2,0],material,false);
   if(p.kind==='double'){
    api.box(o,p.name+'BackPaper',[p.L,p.t,p.W],[0,.002+p.t/2,0],material,false);
    for(const sign of[-1,1])api.box(o,p.name+'ClosedSide_'+sign,[p.L,.002,p.t],[0,.001,sign*(p.W-p.t)/2],api.m.edge,false);
   }
   parts.push({...p,o,pose:(ph,u)=>({...p.pose(ph,u),...(id==='paper-bag-tuber'&&role==='input'?{s:[1,1,1].map(()=>Number(ph!=='load'||u>=.35))}:{})}),times:id==='paper-bag-tuber'&&role==='input'?[0,.12,.22,.3499,.35,.46,.58,.70,.80,.86,.94,1]:undefined});
  }
  if(s.kind==='heatseal'){
   for(const sign of[-1,1]){const o=api.group(root,'SealBand_'+sign);api.box(o,'SealedLongEdge_'+sign,[s.L-.005,.00015,.004],[0,0,0],api.m.tape,false);parts.push({o,pose:(ph,u)=>({p:[0,s.t+.0001+s.t*forming(ph,u),sign*(s.W/2-.002)]})});}
   const o=api.group(root,'EndSealBand');api.box(o,'SealedEndEdge',[.004,.00015,s.W],[0,0,0],api.m.tape,false);parts.push({o,pose:(ph,u)=>({p:[s.L/2-.002,s.t+.0001+s.t*forming(ph,u),0]})});
  }
 }
 if(id==='paper-bag-tuber'&&role==='input'){const o=api.group(root,'MeteredWebRoll'),shape=new T.Shape();shape.absarc(0,0,.019,0,Math.PI*2,false);const hole=new T.Path();hole.absarc(0,0,.015,0,Math.PI*2,true);shape.holes.push(hole);const g=new T.ExtrudeGeometry(shape,{depth:s.L,bevelEnabled:false,curveSegments:24});g.rotateY(Math.PI/2);g.translate(-s.L/2,0,0);api.mesh(o,'RolledPaperAnnulus',g,[0,0,0],api.m.paper);parts.push({o,pose:(ph,u)=>({p:[0,.019,.26],s:[1,1,1].map(()=>Number(ph==='load'&&u<.35))}),times:[0,.3499,.35,1]});}
 const apply=(ph,u)=>{for(const p of parts){const pose=p.pose(role==='output'?'unload':ph,role==='output'?1:u);p.o.position.fromArray(pose.p);if(pose.s)p.o.scale.fromArray(pose.s);p.o.rotation.set(0,0,0);if(pose.axis)p.o.rotation[pose.axis]=pose.a;}};
 apply('load',0);return{root,parts,apply};
}
