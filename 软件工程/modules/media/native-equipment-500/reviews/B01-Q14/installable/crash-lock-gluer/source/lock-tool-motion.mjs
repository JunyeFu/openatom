import * as T from 'three';
import {carton,V,manufactureState}from './lock-bottom-kinematics.mjs';import{finitePaperPoint,scoredBands}from './finite-scored-bands.mjs';import{paperShiftX,paperDatumY,processU,clamp,specs}from './batch-spec.mjs';
const s=specs['crash-lock-gluer'],axisD=V([1,0,1]).normalize(),axisWorld=V([-1,0,1]).normalize(),Y=V([0,1,0]);
const lerp=(a,b,t)=>a.clone().lerp(b,clamp(t)),RZ=a=>new T.Quaternion().setFromAxisAngle(V([0,0,1]),a),Qdiag=a=>RZ(-Math.PI).multiply(new T.Quaternion().setFromAxisAngle(axisD,a));
function poseAt(part,local,u){const a=finitePaperPoint(part,V([0,0,0]),processU('auto',u)),basis=[V([1,0,0]),V([0,1,0]),V([0,0,1])].map(v=>finitePaperPoint(part,v,processU('auto',u)).sub(a)),q=new T.Quaternion().setFromRotationMatrix(new T.Matrix4().makeBasis(...basis));return{point:finitePaperPoint(part,local,processU('auto',u)).add(V([paperShiftX,paperDatumY(s),0])),q};}
export const bottomContact=index=>{const z=(index-2)*carton.side+.025;return{part:{kind:'bottom',index,piece:'main'},point:V([-([.010,.050,.010,.050][index]),0,z])};};
export const hookMotorZ=index=>[-.28,-.58,.28,.58][index];
export const hookSpoonZ=index=>poseAt(bottomContact(index).part,bottomContact(index).point,.06).point.z-hookMotorZ(index);
export function bottomActive(index,u){const c=bottomContact(index),p=poseAt(c.part,c.point.clone().add(V([0,-carton.paperThickness/2-.001,0])),u);return{c:p.point.clone().sub(V([0,.046,hookSpoonZ(index)]).applyQuaternion(p.q)),q:p.q,tip:p.point};}
export function bottomHookState(index,phase,u){
 const start=bottomActive(index,.06),end=bottomActive(index,.20),park=V([index%2===0?-.90:-1.00,index%2===0?1.12:.96,start.c.z]),returnY=index%2===0?1.12:.96,approachX=index%2===0?-.99:-1.02;if(phase!=='auto')return{c:park,q:RZ(Math.PI/2),parked:true};let c,q;
 if(u<.02){c=park.clone();c.x=park.x+(approachX-park.x)*clamp(u/.02);q=RZ(Math.PI/2*(1-clamp(u/.02)));}
 else if(u<.03){c=park.clone();c.x=approachX;c.y=park.y+(.78-park.y)*clamp((u-.02)/.01);q=RZ(0);}
 else if(u<.05){c=lerp(V([approachX,.78,park.z]),V([start.c.x,.78,start.c.z]),(u-.03)/.02);q=RZ(0);}
 else if(u<.06){c=lerp(V([start.c.x,.78,start.c.z]),start.c,(u-.05)/.01);q=RZ(0);}
 else if(u<=.21)return{...bottomActive(index,Math.min(u,.20)),contact:true};
 else if(u<.22){c=end.c.clone().add(V([-.12*clamp((u-.21)/.01),0,0]));q=RZ(-Math.PI);}
 else if(u<.24){c=end.c.clone().add(V([-.12,0,0]));c.y=end.c.y+(returnY-end.c.y)*clamp((u-.22)/.02);q=RZ(-Math.PI);}
 else if(u<.25){c=lerp(V([end.c.x-.12,returnY,end.c.z]),V([park.x,returnY,park.z]),(u-.24)/.01);q=RZ(-Math.PI);}
 else if(u<.26){c=V([park.x,returnY+(park.y-returnY)*clamp((u-.25)/.01),park.z]);q=RZ(-Math.PI-Math.PI/2*clamp((u-.25)/.01));}
 else return{c:park,q:RZ(Math.PI/2),parked:true};
 return{c,q,tip:V([0,.046,hookSpoonZ(index)]).applyQuaternion(q).add(c),contact:false};
}
export const diagonalContact=index=>{const z=(index-1)*carton.side;return{part:{kind:'bottom',index,piece:'ear'},point:V([-2*carton.minorDepth/3,0,z-2*carton.minorDepth/3+.0015])};};
export function diagonalActive(index,u){const c=diagonalContact(index),p=poseAt(c.part,c.point.clone().add(V([0,-carton.paperThickness/2-.0002,0])),u);return{c:p.point.clone().sub(Y.clone().multiplyScalar(.05).sub(axisD.clone().multiplyScalar(.15)).applyQuaternion(p.q)),q:p.q,tip:p.point};}
export function diagonalToolState(index,phase,u){
 const start=diagonalActive(index,.28),end=diagonalActive(index,.42),park=V([-.65,.79,index===0?-.20:.23]);if(phase!=='auto')return{c:park,q:Qdiag(Math.PI/2),parked:true};let c,q;
 if(u<.26)return{c:park,q:Qdiag(Math.PI/2),parked:true};
 if(u<.265){c=park;q=Qdiag(Math.PI/2*(1-clamp((u-.26)/.005)));}
 else if(u<.27){c=park.clone();c.y=park.y+(start.c.y+.03-park.y)*clamp((u-.265)/.005);q=Qdiag(0);}
 else if(u<.275){c=lerp(V([park.x,start.c.y+.03,park.z]),start.c.clone().add(V([0,.03,0])),(u-.27)/.005);q=Qdiag(0);}
 else if(u<.28){c=start.c.clone().add(V([0,.03*(1-clamp((u-.275)/.005)),0]));q=Qdiag(0);}
 else if(u<=.42)return{...diagonalActive(index,u),contact:true};
 else if(u<.44){c=end.c.clone().addScaledVector(axisWorld,.12*clamp((u-.42)/.02));q=Qdiag(Math.PI);}
 else if(u<.455){c=end.c.clone().addScaledVector(axisWorld,.12);c.y=end.c.y+(.94-end.c.y)*clamp((u-.44)/.015);q=Qdiag(Math.PI);}
 else if(u<.47){c=lerp(end.c.clone().addScaledVector(axisWorld,.12).setY(.94),V([end.c.x-axisWorld.x*-.12,.94,park.z]),(u-.455)/.015);q=Qdiag(Math.PI);}
 else if(u<.49){c=lerp(V([end.c.x+axisWorld.x*.12,.94,park.z]),V([park.x,.94,park.z]),(u-.47)/.02);q=Qdiag(Math.PI);}
 else if(u<.50){c=V([park.x,.94,park.z]);q=Qdiag(Math.PI-Math.PI/2*clamp((u-.49)/.01));}
 else if(u<.51){c=V([park.x,.94+(.79-.94)*clamp((u-.50)/.01),park.z]);q=Qdiag(Math.PI/2);}
 else return{c:park,q:Qdiag(Math.PI/2),parked:true};
 return{c,q,tip:Y.clone().multiplyScalar(.05).sub(axisD.clone().multiplyScalar(.15)).applyQuaternion(q).add(c),contact:false};
}
export const toolTimes=[...new Set([0,1,.02,.03,.05,.06,.20,.205,.21,.215,.22,.23,.28,.30,.33,.34,.35,.38,.39,.40,.58,.585,.595,.60,.62,.64,.80,.805,.825,.84,.88,.90,.94,.95,...Array.from({length:513},(_,i)=>i/512)])].sort((a,b)=>a-b);
export const retainBottom=index=>{const[z0]=[(index-2)*carton.side],p=poseAt({kind:'bottom',index,piece:'main'},V([-.030,-carton.paperThickness/2,z0+(index===2?.050:.025)]),.20);return p.point;};
export const bottomRetainerY=(index,phase,u)=>{const y=retainBottom(index).y;return phase==='auto'?1.10-(1.10-y)*clamp((u-.201)/.008)+(1.10-y)*clamp((u-.49)/.03):1.10;};
export const upperPadY=(phase,u)=>phase==='auto'?1.12-(1.12-.9030)*clamp((u-.42)/.019)+(1.12-.9030)*clamp((u-.49)/.03):1.12;

export const toolKeyTimes=kind=>{const ranges=kind==='bottom'?[[0,.02],[.06,.20],[.25,.26]]:[[.26,.265],[.28,.42],[.49,.50]];return[...new Set([...samplesForTools(),...ranges.flatMap(([a,b])=>Array.from({length:257},(_,i)=>a+(b-a)*i/256))])].sort((a,b)=>a-b);};
const samplesForTools=()=>[0,1,.02,.03,.05,.06,.20,.21,.22,.24,.25,.26,.265,.27,.275,.28,.42,.44,.455,.47,.49,.50,.51];
