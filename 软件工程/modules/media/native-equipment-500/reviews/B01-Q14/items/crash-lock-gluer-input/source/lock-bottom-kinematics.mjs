// Original neutral-surface linkage derivation. Finite paper/adhesive compensation is a separate pending gate.
import * as T from 'three';
export const carton={side:.100,height:.240,minorDepth:.040,majorDeep:.060,majorShallow:.040,tab:.020,paperThickness:.0005,glueThickness:.0001,glueInsetScale:.75,seamWidth:.014};
export const V=a=>new T.Vector3(...a),clamp=(x,a=0,b=1)=>Math.min(b,Math.max(a,x));
const RX=(p,a)=>p.clone().applyAxisAngle(new T.Vector3(1,0,0),a),RZ=(p,a)=>p.clone().applyAxisAngle(new T.Vector3(0,0,1),a),diagAxis=new T.Vector3(1,0,1).normalize();
export function erectionState(u){
 const alpha=-Math.PI/2*clamp(u),beta=-alpha;
 return {alpha,gamma:Math.PI-Math.asin(clamp(Math.tan(beta/2))),diagonal:Math.acos(clamp(1-2*Math.cos(beta),-1,1))};
}
// Body panels in developed strip order 0..3. Minor-bottom panels are 0 and 2; major-bottom panels are 1 and 3.
// The side seam is a fifth panel attached to panel0, not a separately invented Item.
export function wallPoint(index,p,alpha){
 const w=carton.side;
 if(index===0)return RX(p.clone().sub(V([0,0,-w])),Math.PI+alpha).add(V([0,0,-w]));
 if(index===1)return p.clone();
 if(index===2)return RX(p,alpha);
 if(index===3)return RX(p.clone().sub(V([0,0,w])),Math.PI).add(V([0,-w*Math.sin(alpha),w*Math.cos(alpha)]));
 if(index===4)return RX(p.clone().sub(V([0,0,-2*w])),Math.PI).add(V([0,-w*Math.sin(alpha),w*Math.cos(alpha)-w]));
 throw Error('Unknown body panel '+index);
}
export function manufactureState(u){
 return {gamma:Math.PI*clamp((u-.06)/.14),diagonal:Math.PI*clamp((u-.22)/.12),left:Math.PI*clamp((u-.40)/.18),right:Math.PI*clamp((u-.64)/.16),seamGlue:clamp((u-.35)/.04),bottomGlue:clamp((u-.35)/.04)};
}
export function manufacturingWallPoint(index,p,s){
 const w=carton.side;
 if(index===0||index===4)return RX(p.clone().sub(V([0,0,-w])),s.left).add(V([0,0,-w]));
 if(index===3)return RX(p.clone().sub(V([0,0,w])),s.right).add(V([0,0,w]));
 return p.clone();
}
export const wallBounds=index=>index===4?[-2*carton.side-carton.tab,-2*carton.side]:[(index-2)*carton.side,(index-1)*carton.side];
export function bodyPolygon(index){const[z0,z1]=wallBounds(index);return [[0,0,z0],[carton.height,0,z0],[carton.height,0,z1],[0,0,z1]].map(V);}
export function bottomPolygons(index){
 const[z0,z1]=wallBounds(index),d=carton.minorDepth;
 if(index%2===0)return {main:[[0,0,z0],[-d,0,z0],[-d,0,z1-d],[0,0,z1]].map(V),ear:[[0,0,z1],[-d,0,z1-d],[-d,0,z1]].map(V)};
 const a=carton.majorDeep,b=carton.majorShallow,mid=(z0+z1)/2;
 return {main:[[0,0,z0],[-a,0,z0],[-a,0,mid],[-b,0,mid],[-b,0,z1],[0,0,z1]].map(V)};
}
export function bottomPoint(index,part,p,state,wall=wallPoint){
 const zR=wallBounds(index)[1],a=V([0,0,zR]);
 const q=part==='ear'?p.clone().sub(a).applyAxisAngle(diagAxis,state.diagonal).add(a):p.clone();
 return wall(index,RZ(q,-state.gamma),state.alpha);
}
export function manufacturingBottomPoint(index,part,p,s){return bottomPoint(index,part,p,{...s,alpha:0},(i,p)=>manufacturingWallPoint(i,p,s));}
export function gluePolygon(index){
 if(index!==0&&index!==2)throw Error('Glue ears belong to panels0 and2');
 const tri=bottomPolygons(index).ear,c=tri.reduce((a,p)=>a.add(p),new T.Vector3()).multiplyScalar(1/3);
 return tri.map(p=>p.clone().sub(c).multiplyScalar(carton.glueInsetScale).add(c));
}
export function targetGluePoint(index,p){const zR=wallBounds(index)[1];return V([p.z-zR,0,zR-p.x]);}
export function polygonArea(poly){let sum=new T.Vector3();for(let i=1;i<poly.length-1;i++)sum.add(poly[i].clone().sub(poly[0]).cross(poly[i+1].clone().sub(poly[0])));return sum.length()/2;}
export const glueArea=()=>2*polygonArea(gluePolygon(0))+carton.height*carton.seamWidth;
export const stockArea=()=>[0,1,2,3,4].reduce((a,i)=>a+polygonArea(bodyPolygon(i))+(i===4?0:Object.values(bottomPolygons(i)).reduce((s,p)=>s+polygonArea(p),0)),0);
export function paperPoint(part,p,mode,u){
 if(mode==='erection'){const s=erectionState(u);return part.kind==='body'?wallPoint(part.index,p,s.alpha):bottomPoint(part.index,part.piece,p,s);}
 const s=manufactureState(u);return part.kind==='body'?manufacturingWallPoint(part.index,p,s):manufacturingBottomPoint(part.index,part.piece,p,s);
}
export function paperNormal(part,mode,u){
 const samples=[V([0,0,0]),V([1,0,0]),V([0,0,1])],s=mode==='erection'?erectionState(u):manufactureState(u);
 const f=p=>part.kind==='body'?(mode==='erection'?wallPoint(part.index,p,s.alpha):manufacturingWallPoint(part.index,p,s)):(mode==='erection'?bottomPoint(part.index,part.piece,p,s):manufacturingBottomPoint(part.index,part.piece,p,s));
 const [a,b,c]=samples.map(f);return c.sub(a).cross(b.sub(a)).normalize();
}
export function allParts(){
 return [0,1,2,3,4].flatMap(index=>[{kind:'body',index,name:'BodyPanel'+index,polygon:bodyPolygon(index)},...(index===4?[]:Object.entries(bottomPolygons(index)).map(([piece,polygon])=>({kind:'bottom',index,piece,name:'Bottom'+index+'_'+piece,polygon}))) ]);
}
// Closed-pose construction proposal. These offsets clear solid plies and adhesive, but do not yet supply the crease-band manufacturing paths.
export const closedLayers={BodyPanel0:.0042,BodyPanel1:0,BodyPanel2:0,BodyPanel3:.0048,BodyPanel4:.0042,Bottom0_main:.0030,Bottom0_ear:.0018,Bottom1_main:.0012,Bottom2_main:.0012,Bottom2_ear:.0024,Bottom3_main:.0030};
export const finiteConstruction={
 status:'closed_pose_proposal_only',
 body_creases:[{between:[0,1],neutral_radius_m:.0021,half_turn_material_allowance_m:Math.PI*.0021},{between:[2,3],neutral_radius_m:.0024,half_turn_material_allowance_m:Math.PI*.0024}],
 bottom_creases:[0,1,2,3].map(index=>({index,neutral_radius_m:index===3?.0009:.0006,half_turn_material_allowance_m:Math.PI*(index===3?.0009:.0006)})),
 diagonal_creases:[0,2].map(index=>({index,neutral_radius_m:.0006,half_turn_material_allowance_m:Math.PI*.0006})),
 required:'Add these material allowances to the authored blank and represent actual finite hinge strips throughout manufacture; offsetting disconnected rigid faces is not an accepted model.'
};
