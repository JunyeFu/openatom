import * as T from 'three';
import{carton,V,allParts}from './lock-bottom-kinematics.mjs';
import{finitePaperPoint,scoredBands,bandCellPoses,bandCells,finiteCoreAnchor}from './finite-scored-bands.mjs';
export function buildPaper(reference=u=>u,remap=u=>u){
const root=new T.Group();root.name='FiniteScoredCrashLockMaterial';
const kraft=new T.MeshStandardMaterial({name:'Original finite paperboard',color:'#cbb17d',roughness:.84}),creaseMat=new T.MeshStandardMaterial({name:'Original scored bands',color:'#c3a76e',roughness:.87}),parts=allParts(),tracks=[],names=[];
const boxGeometry=new T.BoxGeometry(1,1,1),triGeometry=sign=>{const sh=new T.Shape();sh.moveTo(0,0);sh.lineTo(sign,0);sh.lineTo(0,1);sh.closePath();const g=new T.ExtrudeGeometry(sh,{depth:1,bevelEnabled:false});g.translate(0,0,-.5);return g;},leftGeometry=triGeometry(-1),rightGeometry=triGeometry(1);
const ranges={bottom:[.06,.20],ear:[.22,.34],left:[.40,.58],right:[.64,.80]},timeKeys=stages=>[...new Set([0,1,...stages.flatMap(stage=>Array.from({length:193},(_,i)=>ranges[stage][0]+(ranges[stage][1]-ranges[stage][0])*i/192))])].sort((a,b)=>a-b);
const keyTimes=ss=>timeKeys(ss).map(remap);
const stages=(kind,index,piece)=>kind==='body'?(index===0||index===4?['left']:index===3?['right']:[]):['bottom',...(piece==='ear'?['ear']:[]),...(index===0?['left']:index===3?['right']:[])];
function coreState(part,u){const anchor=finiteCoreAnchor(part),a=finitePaperPoint(part,anchor,u),x=finitePaperPoint(part,anchor.clone().add(V([1,0,0])),u).sub(a),y=finitePaperPoint(part,anchor.clone().add(V([0,1,0])),u).sub(a),z=finitePaperPoint(part,anchor.clone().add(V([0,0,1])),u).sub(a);return{p:a.toArray(),q:new T.Quaternion().setFromRotationMatrix(new T.Matrix4().makeBasis(x,y,z)).toArray(),scale:[1,1,1]};}
function add(name,geometry,material,keys,states,scaled){const o=new T.Mesh(geometry,material);o.name=name;const first=states[0];o.position.fromArray(first.p);o.quaternion.fromArray(first.q);o.scale.fromArray(first.scale);root.add(o);names.push(name);tracks.push(new T.VectorKeyframeTrack(name+'.position',keys,states.flatMap(s=>s.p)));tracks.push(new T.QuaternionKeyframeTrack(name+'.quaternion',keys,states.flatMap(s=>s.q)));if(scaled)tracks.push(new T.VectorKeyframeTrack(name+'.scale',keys,states.flatMap(s=>s.scale)));}
for(const part of parts){const sh=new T.Shape();sh.moveTo(part.polygon[0].x,part.polygon[0].z);for(const p of part.polygon.slice(1))sh.lineTo(p.x,p.z);sh.closePath();const g=new T.ExtrudeGeometry(sh,{depth:carton.paperThickness,bevelEnabled:false});g.rotateX(Math.PI/2);g.translate(0,carton.paperThickness/2,0);const anchor=finiteCoreAnchor(part);g.translate(-anchor.x,-anchor.y,-anchor.z);const keys=keyTimes(stages(part.kind,part.index,part.piece));add(part.name,g,kraft,keys,keys.map(u=>coreState(part,reference(u))),false);}
for(const initialBand of scoredBands(0)){
 const name=initialBand.name,index=Number(name.slice(-1)),bandKind=name.startsWith('Body')?'body':'bottom',piece=name.startsWith('Ear')?'ear':'main',keys=keyTimes(stages(bandKind,index,piece)),states=keys.map(u=>{const b=scoredBands(reference(u)).find(b=>b.name===name);return Array.from({length:bandCells},(_,i)=>bandCellPoses(b,i));});
 for(let i=0;i<bandCells;i++)for(let j=0;j<4;j++){const suffix=['ArcCore','ArcRight','ArcLeft','StraightTail'][j],g=j===1?rightGeometry:j===2?leftGeometry:boxGeometry;add(name+'_'+i+'_'+suffix,g,creaseMat,keys,states.map(s=>s[i][j]),true);}
}
return{root,tracks};
}
