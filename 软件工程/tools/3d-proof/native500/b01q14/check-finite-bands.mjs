import * as T from 'three';import assert from 'node:assert/strict';import fs from 'node:fs/promises';
import{carton,V,manufactureState,allParts,closedLayers,gluePolygon,targetGluePoint,wallBounds}from '../../../../modules/media/native-equipment-500/reviews/B01-Q14/source/lock-bottom-kinematics.mjs';
import{finitePaperPoint,finiteWallPoint,finiteBottomBasePoint,finiteEarPoint,scoredBands,bandPoint,bandCellPoses,bandVolume,developedArea,bandCells}from '../../../../modules/media/native-equipment-500/reviews/B01-Q14/source/finite-scored-bands.mjs';
const rows=[];let maxVolumeRelativeError=0,maxActualSeamGap=0;
const transformed=(p,v)=>V(v).multiply(V(p.scale)).applyQuaternion(new T.Quaternion(...p.q)).add(V(p.p));
function seam(a,b,n){const gap=a.distanceTo(b);maxActualSeamGap=Math.max(maxActualSeamGap,gap);near(a,b,n);}
const near=(a,b,n,t=1e-10)=>assert.ok(a.distanceTo(b)<t,n+': '+a.distanceTo(b));
for(let k=0;k<=100;k++){
 const u=k/100,s=manufactureState(u),bands=scoredBands(u);let actual=0,expected=0;
 for(const b of bands){
  actual+=bandVolume(b);expected+=b.length*b.width*carton.paperThickness;
  for(const v of[b.axisMin,(b.axisMin+b.axisMax)/2,b.axisMax]){
   let root,end;
   if(b.name.startsWith('Body')){const index=Number(b.name.slice(-1)),p=b.origin.clone().addScaledVector(b.axis,v);root=p;end=finiteWallPoint(index,p,s);}
   else if(b.name.startsWith('Bottom')){const index=Number(b.name.slice(-1)),p=b.origin.clone().addScaledVector(b.axis,v);root=finiteWallPoint(index,p,s);end=finiteWallPoint(index,finiteBottomBasePoint(index,p,s),s);}
   else{const index=Number(b.name.slice(-1)),p=b.origin.clone().addScaledVector(b.axis,v);root=finiteWallPoint(index,finiteBottomBasePoint(index,p,s),s);end=finiteWallPoint(index,finiteBottomBasePoint(index,finiteEarPoint(index,p,s),s),s);}
   near(bandPoint(b,0,v),root,b.name+' attaches start');near(bandPoint(b,b.length,v),end,b.name+' attaches far face');
  }
  let previous;
  for(let i=0;i<bandCells;i++){
   const poses=bandCellPoses(b,i);for(const p of poses){assert.ok([...p.p,...p.q,...p.scale].every(Number.isFinite));assert.ok(p.scale.every(n=>n>=-1e-13));}
   const [box,right,left,tail]=poses,segments=[];
   if(box.scale[0]>1e-14){
    for(const z of[-.5,.5])for(const sign of[-1,1]){const wedge=sign<0?left:right;seam(transformed(box,[sign*.5,-.5,z]),transformed(wedge,[0,0,z]),'actual box/wedge bottom');seam(transformed(box,[sign*.5,.5,z]),transformed(wedge,[0,1,z]),'actual box/wedge top');}
    segments.push({start:z=>[transformed(left,[0,1,z]),transformed(left,[-1,0,z])],end:z=>[transformed(right,[0,1,z]),transformed(right,[1,0,z])]});
   }
   if(tail.scale[0]>1e-14)segments.push({start:z=>[transformed(tail,[-.5,.5,z]),transformed(tail,[-.5,-.5,z])],end:z=>[transformed(tail,[.5,.5,z]),transformed(tail,[.5,-.5,z])]});
   for(const segment of segments){if(previous)for(const z of[-.5,.5]){const a=previous.end(z),c=segment.start(z);seam(a[0],c[0],'actual neighboring inner faces');seam(a[1],c[1],'actual neighboring outer faces');}previous=segment;}
  }

 }
 const coreVolume=(developedArea()-scoredBands(0).reduce((a,b)=>a+b.length*b.width,0))*carton.paperThickness,rel=Math.abs(actual-expected)/(expected+coreVolume);maxVolumeRelativeError=Math.max(maxVolumeRelativeError,rel);assert.ok(rel<2e-5,'tessellated total paper volume drifts');rows.push({u,scored_bands:bands.length,all_band_endpoints_connected:true,actual_total_paper_volume_m3:actual+coreVolume,nominal_total_paper_volume_m3:expected+coreVolume,relative_error:rel});
}
const final=allParts().map(part=>{const p=finitePaperPoint(part,part.polygon[0],1);assert.ok(Math.abs(p.y-closedLayers[part.name])<1e-12,part.name+' wrong stack level');return{name:part.name,actual_center_y_m:p.y};});
const bonds=[];for(const index of[0,2]){const part={kind:'bottom',index,piece:'ear'},o=finitePaperPoint(part,V([0,0,0]),1),normal=finitePaperPoint(part,V([0,1,0]),1).sub(o).normalize();for(const p of gluePolygon(index)){const a=finitePaperPoint(part,p,1),b=finitePaperPoint({kind:'bottom',index:index+1,piece:'main'},targetGluePoint(index,p),1),g=b.clone().sub(a).dot(normal)-carton.paperThickness;assert.ok(Math.abs(g-carton.glueThickness)<1e-12);bonds.push({index,face_gap_m:g});}}
const out={passed:true,scope:'analytic finite bands and rigid core attachments through manufacture; not yet whole-blank collision or Native acceptance',manufacturing_samples:101,rows,max_total_paper_volume_relative_error:maxVolumeRelativeError,max_actual_prism_seam_gap_m:maxActualSeamGap,developed_area_before_relief_cuts_m2:developedArea(),scored_band_mesh_parts:8*bandCells*4,final,bonds,separate_gate_refs:{flat_stock:'finite-blank-collision-checks.json',manufacturing_solids:'finite-manufacturing-collision-checks.json',product_GLB:'scenarios/媒体表现/native500-20261024/b01-q14-finite-component-checks.json'},Native:'not_run',device_admitted:false};await fs.writeFile('modules/media/native-equipment-500/reviews/B01-Q14/finite-band-construction-checks.json',JSON.stringify(out,null,2)+'\n');console.log(JSON.stringify({finite_band_endpoints_connected:true,samples:101,stock_area_before_cuts_m2:developedArea(),max_total_volume_relative_error:maxVolumeRelativeError,closed_glue_gap_m:.0001,collision_gate:'see_separate_receipts',device_admitted:false}));
