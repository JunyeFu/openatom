import fs from 'node:fs/promises';import assert from 'node:assert/strict';import * as T from 'three';import {buildItem} from '../../../../modules/media/native-equipment-500/reviews/B01-Q09/source/item-build.mjs';
const home='modules/media/native-equipment-500/reviews/B01-Q09',receipt=JSON.parse(await fs.readFile(home+'/items/source-checks.json','utf8')),checks=[];
for(const [role,expected]of[['input',.19984336],['output',.1600032],['scrap',.00996004]]){
 const{root}=buildItem({device:'blank-stripper',role,id:'blank-stripper-'+role});root.updateMatrixWorld(true);let area=0;
 root.traverse(o=>{if(!o.isMesh)return;const p=o.geometry.attributes.position,idx=o.geometry.index,n=idx?.count??p.count;for(let i=0;i<n;i+=3){const v=[0,1,2].map(j=>new T.Vector3().fromBufferAttribute(p,idx?idx.getX(i+j):i+j).applyMatrix4(o.matrixWorld));if(v.every(x=>Math.abs(x.y-.003)<1e-8)){const cross=v[1].clone().sub(v[0]).cross(v[2].clone().sub(v[0]));area+=cross.length()/2;}}});assert.ok(Math.abs(area-expected)<1e-8,role+' mesh area '+area);checks.push({role,actual_top_triangle_area_m2:area,expected_area_m2:expected});
}
receipt.actual_mesh_areas=checks;await fs.writeFile(home+'/items/source-checks.json',JSON.stringify(receipt,null,2)+'\n');console.log('Actual precut/tabbed blank/corner mesh projected areas .19984336/.1600032/.00996004 m2 PASS.');
