import * as T from 'three';
import{geometry,clip,V}from './geometry.mjs';
import{carton as c,opening,minorFold,majorFold,tapeFraction,glueFraction,blank,flatCase,samples}from './batch-spec.mjs';
export function casePanels(theta){
 const A=V([-c.length/2,c.base,-c.width/2]),B=A.clone().add(V([c.length,0,0])),D=A.clone().add(V([c.width*Math.cos(theta),0,c.width*Math.sin(theta)])),C=B.clone().add(D.clone().sub(A));
 return[{name:'Front',a:A,b:B,inward:[0,0,1],minor:false},{name:'Right',a:B,b:C,inward:[-Math.sin(theta),0,Math.cos(theta)],minor:true},{name:'Back',a:C,b:D,inward:[0,0,-1],minor:false},{name:'Left',a:D,b:A,inward:[Math.sin(theta),0,-Math.cos(theta)],minor:true}];
}
// Upright RSC: four linked walls, four bottom flaps, four open upper flaps.
export function buildCarton({formed=false,simple=false}={}){
 const{m,group,box,cyl}=geometry(simple),root=new T.Group();root.name='RSCCase';const panels=[];
 for(const info of casePanels(c.theta0)){
  const len=info.a.distanceTo(info.b),g=group(root,info.name+'Wall');box(g,info.name+'Board',[len-c.thickness,c.height,c.thickness],[0,c.height/2,0],m.paper,false);
  for(const y of[.001,c.height-.001])box(g,info.name+'ScoreLine',[len-c.thickness,.0015,c.thickness+.0001],[0,y,0],m.edge,false);
  const bottom=group(g,info.name+'BottomFold',[0,info.minor?.0032:0,0]);box(bottom,info.name+'BottomWing',[len-c.thickness,c.flapDepth,c.thickness],[0,-c.flapDepth/2,0],m.paper,false);
  const top=group(g,info.name+'TopFold',[0,c.height,0]);box(top,info.name+'TopWing',[len-c.thickness,c.flapDepth,c.thickness],[0,c.flapDepth/2,0],m.paper,false);
  panels.push({info,g,bottom,top});
 }
 function apply(u){const theta=formed?Math.PI/2:opening(u),states=casePanels(theta);for(let i=0;i<panels.length;i++){const{g,bottom}=panels[i],p=states[i],dir=p.b.clone().sub(p.a).normalize(),normal=dir.clone().cross(V([0,1,0]));g.position.copy(p.a).add(p.b).multiplyScalar(.5);g.quaternion.setFromRotationMatrix(new T.Matrix4().makeBasis(dir,V([0,1,0]),normal));bottom.rotation.x=-(formed?Math.PI/2:p.minor?minorFold(u):majorFold(u));}root.updateMatrixWorld(true);}
 const tracks=[];apply(0);for(const{g,bottom}of panels){for(const key of['position','quaternion'])tracks.push(key==='position'?new T.VectorKeyframeTrack(g.name+'.'+key,samples,samples.flatMap(u=>{apply(u);return g.position.toArray();})):new T.QuaternionKeyframeTrack(g.name+'.'+key,samples,samples.flatMap(u=>{apply(u);return g.quaternion.toArray();})));tracks.push(new T.QuaternionKeyframeTrack(bottom.name+'.quaternion',samples,samples.flatMap(u=>{apply(u);return bottom.quaternion.toArray();})));}apply(0);
 return{root,tracks,apply};
}
export function buildStock(id,{role='input',simple=false}={}){
 const{m,group,box,cyl}=geometry(simple),root=new T.Group();root.name='StockRoot';let paper,glue,tape;
 if(id==='carton-erector'){
  paper=buildCarton({formed:role==='output',simple});root.add(paper.root);if(role==='output')paper.root.position.y=-(c.base-c.thickness/2);
 }else if(id==='adhesive-dispenser'){
  box(root,'DieCutSideSeamBlank',[blank.length,blank.thickness,blank.width],[0,blank.thickness/2,0],m.paper,false);
  for(const x of[-.16,.16])box(root,'Crease'+x,[.001,.0001,blank.width],[x,blank.thickness+.00005,0],m.edge,false);
  glue=box(root,'ActualSideSeamGlueBead',[blank.beadLength,blank.beadHeight,blank.beadWidth],[0,blank.thickness+blank.beadHeight/2,blank.beadZ],m.glue,false);if(role==='input')glue.scale.x=0;
 }else{
  for(const sign of[-1,1])box(root,'FlatCasePly'+sign,[flatCase.length,.003,flatCase.width],[0,.0015+(sign>0?.0032:0),0],m.paper,false);
  box(root,'FreshSideSeamGlue',[.40,.0002,.020],[0,.0031,.135],m.glue,false);
  for(const x of[-.10,.10])box(root,'FlatCaseFold'+x,[.001,.0001,flatCase.width],[x,flatCase.thickness+.00005,0],m.edge,false);
 }
 return{root,paper,glue};
}
export function tapePieces(parent,api,fraction=1,offsetY=0){
 const{box,m}=api,segments=[{name:'LeadingTapeLeg',length:.06,at:[-c.length/2-c.thickness/2,.149,0],size:[.0001,.06,.048]},{name:'BottomTape',length:.36,at:[0,c.base-c.thickness/2-.00005,0],size:[.36,.0001,.048]},{name:'TrailingTapeLeg',length:.06,at:[c.length/2+c.thickness/2,.149,0],size:[.0001,.06,.048]}];
 return segments.map(s=>{const at=[...s.at];at[1]+=offsetY;const o=box(parent,s.name,s.size,at,m.tape,false);o.scale.setScalar(fraction);return{s,o};});
}
