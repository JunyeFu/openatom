import * as T from 'three';import assert from 'node:assert/strict';import fs from 'node:fs/promises';
import {carton,V,erectionState,wallPoint,wallBounds,manufactureState,manufacturingWallPoint,bottomPoint,manufacturingBottomPoint,bottomPolygons,gluePolygon,targetGluePoint,stockArea,glueArea,allParts,paperPoint,paperNormal,polygonArea} from '../../../../modules/media/native-equipment-500/reviews/B01-Q14/source/lock-bottom-kinematics.mjs';
const home='modules/media/native-equipment-500/reviews/B01-Q14',checks=[],finite=[];let maxGap=0,maxClosure=0;
for(let i=0;i<=80;i++){
 const u=i/80,s=erectionState(u),bonds=[];
 for(const index of[0,2])for(const p of gluePolygon(index)){
  const a=bottomPoint(index,'ear',p,s),q=targetGluePoint(index,p),b=bottomPoint(index+1,'main',q,s),gap=a.distanceTo(b);maxGap=Math.max(maxGap,gap);assert.ok(gap<1e-8,`ear ${index} bond opens at${u}: ${gap}`);bonds.push({index,neutral_gap_m:gap});
 }
 for(const [a,za,b,zb]of[[0,-carton.side,1,-carton.side],[1,0,2,0],[2,carton.side,3,carton.side],[3,2*carton.side,4,-2*carton.side],[4,-2*carton.side,0,-2*carton.side]]){
  for(const x of[0,carton.height]){const q=wallPoint(a,V([x,0,za]),s.alpha),r=wallPoint(b,V([x,0,zb]),s.alpha),gap=q.distanceTo(r);maxClosure=Math.max(maxClosure,gap);assert.ok(gap<1e-10,'body ring seam opens');}
 }
 for(const p of allParts())for(const v of p.polygon)assert.ok(paperPoint(p,v,'erection',u).toArray().every(Number.isFinite));
 checks.push({u,alpha:s.alpha,bottom_angle:s.gamma,diagonal_angle:s.diagonal,bonds,body_ring_closed:true});
 // Real thickness requires an explicit inter-layer offset; coincidence of neutral surfaces is not a valid adhesive joint.
 for(const index of[0,2]){const p=gluePolygon(index)[0],q=targetGluePoint(index,p),a=bottomPoint(index,'ear',p,s),b=bottomPoint(index+1,'main',q,s),normal=paperNormal({kind:'bottom',index,piece:'ear'},'erection',u),signed=b.clone().sub(a).dot(normal),required=carton.paperThickness+carton.glueThickness;finite.push({u,index,actual_neutral_separation_m:signed,required_neutral_separation_m:required,missing_allowance_m:required-signed,passed:Math.abs(signed-required)<1e-8});}
}
const start=manufactureState(0),end=manufactureState(1);
for(const part of allParts())for(const p of part.polygon){const a=paperPoint(part,p,'manufacture',0);assert.ok(a.distanceTo(p)<1e-12);const b=paperPoint(part,p,'manufacture',1),c=paperPoint(part,p,'erection',0);assert.ok(b.distanceTo(c)<1e-12,'manufactured flat state differs from erection start');}
assert.ok(Math.abs(stockArea()-.1188)<1e-12);assert.ok(Math.abs(glueArea()*.0001-.000000426)<1e-15);
// The two complementary stepped major flaps must cover, rather than leave a hole in, the100x100mm erected floor.
const endState=erectionState(1),polys=[1,3].map(index=>bottomPolygons(index).main.map(p=>bottomPoint(index,'main',p,endState))),inside=(p,poly)=>{let yes=false;for(let i=0,j=poly.length-1;i<poly.length;j=i++){const a=poly[i],b=poly[j];if((a.y>p.y)!==(b.y>p.y)&&p.z<(b.z-a.z)*(p.y-a.y)/(b.y-a.y)+a.z)yes=!yes;}return yes;};
for(let y=0;y<100;y++)for(let z=0;z<100;z++){const p=V([0,(y+.5)*.001,-.1+(z+.5)*.001]);assert.ok(polys.filter(poly=>inside(p,poly)).length===1,'floor has a gap or duplicate at '+p.toArray());}
const json=(f,v)=>fs.writeFile(home+'/'+f,JSON.stringify(v,null,2)+'\n');
await json('neutral-linkage-checks.json',{passed:true,scope:'original zero-thickness neutral-surface linkage only; not device acceptance',opening_samples:81,checks,max_bond_gap_m:maxGap,max_body_closure_gap_m:maxClosure,stock_area_m2:stockArea(),glue_volume_m3:glueArea()*carton.glueThickness,manufacturing_to_opening_state_continuous:true,erected_floor_coverage_samples:10000,finite_paper_and_glue_gate:'failed_pending_compensation',Native:'not_run'});
await json('finite-layer-checks.json',{passed:false,scope:'observed deficit of uncompensated diagnostic solids; never counted as supported',checks:finite,reason:'Bonded neutral planes coincide; two0.5mm paper plies plus0.1mm adhesive need0.6mm center separation. Crease/stack compensation and collision-free manufacturing path remain required.'});
console.log(JSON.stringify({neutral_linkage_passed:true,opening_samples:81,max_bond_gap_m:maxGap,stock_area_m2:stockArea(),glue_mL:glueArea()*carton.glueThickness*1e6,finite_layer_gate_passed:false,device_admitted:false}));
