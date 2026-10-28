// Original finite scored bands for the actual manufacturing path. No morph targets or skins.
import * as T from 'three';
import {carton,V,manufactureState,wallBounds,allParts,polygonArea} from './lock-bottom-kinematics.mjs';
const Y=V([0,1,0]),X=V([1,0,0]),Z=V([0,0,1]);
const bodyRadius={0:.0021,3:.0024},bottomRadius=index=>index===3?.0009:.0006,diagonalRadius=.0006;
export const bandCells=64;
export function foldPath(D,N,r,angle,s){const a=Math.min(angle,s/r),tail=Math.max(0,s-r*angle);return D.clone().multiplyScalar(r*Math.sin(a)+tail*Math.cos(angle)).addScaledVector(N,r*(1-Math.cos(a))+tail*Math.sin(angle));}
export function finiteFold(p,origin,D,N,r,angle){const axis=D.clone().cross(N).normalize();return p.clone().sub(origin).applyAxisAngle(axis,angle).add(origin).add(foldPath(D,N,r,angle,Math.PI*r));}
export function finiteWallPoint(index,p,s){
 if(index===0||index===4)return finiteFold(p,V([0,0,-carton.side]),Z.clone().negate(),Y,bodyRadius[0],s.left);
 if(index===3)return finiteFold(p,V([0,0,carton.side]),Z,Y,bodyRadius[3],s.right);
 return p.clone();
}
export function finiteBottomBasePoint(index,p,s){return finiteFold(p,V([0,0,0]),X.clone().negate(),Y,bottomRadius(index),s.gamma);}
export function finiteEarPoint(index,p,s){return finiteFold(p,V([0,0,wallBounds(index)[1]]),V([-1,0,1]).normalize(),Y.clone().negate(),diagonalRadius,s.diagonal);}
export function finitePaperPoint(part,p,u){const s=manufactureState(u);if(part.kind==='body')return finiteWallPoint(part.index,p,s);return finiteWallPoint(part.index,finiteBottomBasePoint(part.index,part.piece==='ear'?finiteEarPoint(part.index,p,s):p,s),s);}
const identity=p=>p.clone();
export function scoredBands(u){
 const s=manufactureState(u),bands=[];
 for(const index of[0,3]){const side=index===0?-1:1;bands.push({name:'BodyScore'+index,r:bodyRadius[index],angle:index===0?s.left:s.right,origin:V([0,0,side*carton.side]),D:Z.clone().multiplyScalar(side),N:Y.clone(),axisMin:index===0?0:-carton.height,axisMax:index===0?carton.height:0,parent:identity});}
 for(const index of[0,1,2,3]){const[z0,z1]=wallBounds(index);bands.push({name:'BottomScore'+index,r:bottomRadius(index),angle:s.gamma,origin:V([0,0,0]),D:X.clone().negate(),N:Y.clone(),axisMin:-z1,axisMax:-z0,parent:p=>finiteWallPoint(index,p,s)});}
 for(const index of[0,2])bands.push({name:'EarScore'+index,r:diagonalRadius,angle:s.diagonal,origin:V([0,0,wallBounds(index)[1]]),D:V([-1,0,1]).normalize(),N:Y.clone().negate(),axisMin:-Math.SQRT2*carton.minorDepth,axisMax:0,parent:p=>finiteWallPoint(index,finiteBottomBasePoint(index,p,s),s)});
 return bands.map(b=>({...b,axis:b.D.clone().cross(b.N).normalize(),length:Math.PI*b.r,width:b.axisMax-b.axisMin}));
}
export function bandPoint(b,s,v){return b.parent(b.origin.clone().add(foldPath(b.D,b.N,b.r,b.angle,s)).addScaledVector(b.axis,v));}
function frame(b,a){const t=b.D.clone().multiplyScalar(Math.cos(a)).addScaledVector(b.N,Math.sin(a)),n=b.D.clone().multiplyScalar(-Math.sin(a)).addScaledVector(b.N,Math.cos(a)),p0=b.parent(V([0,0,0]));return[t,n,b.axis].map(v=>b.parent(v).sub(p0).normalize());}
export function bandCellPoses(b,i,cells=bandCells){
 const s0=b.length*i/cells,s1=b.length*(i+1)/cells,a0=Math.min(b.angle,s0/b.r),a1=Math.min(b.angle,s1/b.r),mid=(a0+a1)/2,half=(a1-a0)/2,t=carton.paperThickness,w=b.width,axisMid=(b.axisMin+b.axisMax)/2;
 const C=bandPoint(b,b.r*mid,axisMid),[Tangent,Normal,Axis]=frame(b,mid),q=new T.Quaternion().setFromRotationMatrix(new T.Matrix4().makeBasis(Tangent,Normal,Axis)).toArray(),cy=b.r*(1-Math.cos(half)),H=t*Math.cos(half),inner=(b.r-t/2)*Math.sin(half),D=t*Math.sin(half);
 const state=(name,p,size)=>({name,p:p.toArray(),q,scale:size});
 const arc=[state('ArcCore',C.clone().addScaledVector(Normal,cy),[2*inner,H,w]),state('ArcRight',C.clone().addScaledVector(Tangent,inner).addScaledVector(Normal,cy-H/2),[D,H,w]),state('ArcLeft',C.clone().addScaledVector(Tangent,-inner).addScaledVector(Normal,cy-H/2),[D,H,w])];
 const start=Math.max(s0,b.r*b.angle),tail=Math.max(0,s1-start),[TT,NN,AA]=frame(b,b.angle),qt=new T.Quaternion().setFromRotationMatrix(new T.Matrix4().makeBasis(TT,NN,AA)).toArray(),tailState={name:'StraightTail',p:bandPoint(b,tail>0?(start+s1)/2:s1,axisMid).toArray(),q:qt,scale:[tail,t,w]};
 return[...arc,tailState];
}
export function bandVolume(b,cells=bandCells){let v=0;for(let i=0;i<cells;i++){const p=bandCellPoses(b,i,cells);for(const a of p)v+=a.scale[0]*a.scale[1]*a.scale[2]*(a.name==='ArcRight'||a.name==='ArcLeft'?.5:1);}return v;}
export function developedArea(){return allParts().reduce((a,p)=>a+polygonArea(p.polygon),0)+scoredBands(0).reduce((a,b)=>a+b.width*b.length,0);}
export const manufacturingTimes=[...new Set([0,1,.06,.20,.22,.34,.35,.39,.40,.58,.64,.80,...Array.from({length:257},(_,i)=>i/256)])].sort((a,b)=>a-b);
export function finiteCoreAnchor(part){if(part.index===0||part.index===4)return V([0,0,-carton.side]);if(part.index===3||part.piece==='ear')return V([0,0,carton.side]);return V([0,0,0]);}
