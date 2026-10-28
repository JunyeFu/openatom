import * as T from 'three';
import{geometry}from './geometry.mjs';
import{specs,formed,ramp,stagingX}from './batch-spec.mjs';
export function slab(shape,H){const g=new T.ExtrudeGeometry(shape,{depth:H,bevelEnabled:false,curveSegments:20});g.rotateX(Math.PI/2);g.translate(0,H,0);return g;}
export function sidewall(s){
 const slant=Math.hypot(s.H,s.rt-s.rb),r0=slant*s.rb/(s.rt-s.rb),r1=r0+slant,a=2*Math.PI*(s.rt-s.rb)/slant,points=[];
 for(let i=0;i<=32;i++){const t=-a/2+a*i/32;points.push([r1*Math.cos(t)-(r0+r1)/2,r1*Math.sin(t)]);}
 for(let i=32;i>=0;i--){const t=-a/2+a*i/32;points.push([r0*Math.cos(t)-(r0+r1)/2,r0*Math.sin(t)]);}
 const shape=new T.Shape();points.forEach((p,i)=>i?shape.lineTo(...p):shape.moveTo(...p));shape.closePath();return shape;
}
export function vessel(s,solid=false){
 const p=solid?[[0,0],[s.rb-s.t,0],[s.rt-s.t,s.H-s.t],[s.rt-s.t,s.H+.025],[0,s.H+.025]]:
 [[s.rb,0],[s.rt,s.H],[s.rt-s.t,s.H],[s.rb-s.t,s.t],[s.rb,0]];
 return new T.LatheGeometry(p.map(v=>new T.Vector2(...v)),48);
}
export function plate(s,tool=false){
 const profile=tool?[[0,s.t+.001],[s.rb,s.t+.001],[s.rt-.003,s.H+.001],[s.rt-.003,s.H+.035],[0,s.H+.035]]:
 [[0,0],[s.rb,0],[s.rt,s.H],[s.rt-.001,s.H+s.t],[s.rb,s.t],[0,s.t]];
 return new T.LatheGeometry(profile.map(p=>new T.Vector2(...p)),48);
}
export function stock(id,role){
 const s=specs[id],api=geometry(false),root=new T.Group();root.name='StockRoot';api.m.paper.color.set('#e8e1cf');api.m.edge.color.set('#b5a88d');const parts=[],add=(o,pose,times)=>parts.push({o,pose,times});
 if(role==='blocker')api.box(root,'OutputOccupant',[.1,.04,.1],[0,.02,0],api.m.amber,false);
 else if(role==='component'){
  const disc=api.cyl(root,'IndependentCoatedBottom',s.rb,s.t,[0,s.t/2,0],api.m.paper);
  add(disc,(ph,u,active)=>{const k=active?(ph==='load'?ramp(u,0,.75):1):0;return{p:[active?(ph==='unload'?stagingX*u:-1+k):0,s.t/2,active?.52*(1-k):0]};});
 }else{
  const blank=api.group(root,'UnformedBlank'),body=api.group(root,'FormedContainer');
  if(s.kind==='plate'){
   api.cyl(blank,'RoundPaperBlank',s.rt,s.t,[0,s.H+s.t/2,0],api.m.paper);
   api.mesh(body,'PressedOpenPlate',plate(s),[0,0,0],api.m.paper);
  }else{
   api.mesh(blank,'PrecutCoatedSidewall',slab(sidewall(s),s.t),[0,0,0],api.m.paper);
   api.mesh(body,'OpenTaperedPaperWall',vessel(s),[0,0,0],api.m.paper);
   const rim=api.mesh(body,'RolledTopRim',new T.TorusGeometry(s.rt-s.t/2,.0012,6,48),[0,s.H,0],api.m.paper);rim.rotation.x=Math.PI/2;
   if(role==='output')api.cyl(body,'SealedPaperBottom',s.rb-s.t,s.t,[0,s.t/2,0],api.m.paper);
  }
  for(const[o,inverse]of[[blank,true],[body,false]])add(o,(ph,u,active)=>{const on=role==='output'||active&&(s.kind==='plate'?ph==='unload'||ph==='auto'&&u>=.30:formed(ph,u));return{p:[0,!inverse&&role==='input'&&s.kind==='plate'&&active?(ph==='auto'?s.H*(1-ramp(u,.30,.45)):0):0,0],s:[1,1,1].map(()=>Number(inverse?!on:on))};},[0,.2999,.30,.4499,.45,1]);
 }
 return{root,parts};
}
