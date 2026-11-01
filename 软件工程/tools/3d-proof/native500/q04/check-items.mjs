import fs from 'node:fs/promises';import path from 'node:path';import assert from 'node:assert/strict';import {spawnSync} from 'node:child_process';
import {buildItem} from '../../../../modules/media/native-equipment-500/reviews/B02-Q04/source/item-build.mjs';
import {specs,filmHeight} from '../../../../modules/media/native-equipment-500/reviews/B02-Q04/source/batch-spec.mjs';
import * as T from 'three';
const home='modules/media/native-equipment-500/reviews/B02-Q04',items=JSON.parse(await fs.readFile(home+'/items/inventory.json','utf8')).metadata,checks=[];
for(const item of items){
 const dir=home+'/items/'+item.id,bytes=await fs.readFile(dir+'/paper.glb'),j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12)));
 assert.equal(new Set(j.nodes.map(n=>n.name)).size,j.nodes.length);
 for(const a of j.animations??[]){const channels=new Set();for(const c of a.channels){const key=c.target.node+':'+c.target.path;assert.ok(!channels.has(key));channels.add(key);}}
 const scratch='.work/native500-quality-20261025/q04/items-rebuild/'+item.id;await fs.mkdir(scratch,{recursive:true});await fs.cp(dir+'/source',scratch+'/source',{recursive:true});await fs.copyFile(dir+'/LICENSE.txt',scratch+'/LICENSE.txt');
 const run=spawnSync(process.execPath,['source/item-build.mjs'],{cwd:path.resolve(scratch),windowsHide:true,encoding:'utf8'});assert.equal(run.status,0,run.stderr);assert.ok((await fs.readFile(scratch+'/paper.glb')).equals(bytes),'independent Item source');
 const b=buildItem(item),mixer=new T.AnimationMixer(b.root);
 for(const clip of b.clips){const action=mixer.clipAction(clip);action.play();for(const u of [0,.25,.399,.4,.401,.50,.75,1,.25,0]){action.time=u;mixer.update(0);b.root.updateMatrixWorld(true);if(item.device==='blind-embosser'&&item.role==='input'){const a=b.root.getObjectByName('ActualFlatSheet').scale.x,c=b.root.getObjectByName('ActualEmbossedSheet').scale.x;assert.ok([0,1].includes(a)&&[0,1].includes(c)&&a+c===1,'paper states must not shrink or duplicate during crease switching');}b.root.traverse(o=>{assert.ok(o.matrixWorld.elements.every(Number.isFinite),`${item.id} / ${clip.name} / ${u} / ${o.name} / ${o.position.toArray()}`);});}action.stop();}
 checks.push({id:item.id,bytes:bytes.length,unique_nodes:true,unique_channels:true,own_source_rebuild_same_bytes:true,forward_reverse_finite_pose:true});
}
const recipes=[];
for(const [id,s]of Object.entries(specs)){const primary=items.find(i=>i.device===id&&i.role==='input');assert.equal(primary.attributes.area_m2.value,s.size[0]*s.size[2]);
 if(s.auxiliary&&s.mode!=='foil'){const dose=items.find(i=>i.device===id&&i.role==='auxiliary');assert.equal(dose.attributes.declared_dose_volume_m3.value,s.volume_m3);assert.ok(Math.abs(s.mark[0]*s.mark[1]*filmHeight(s)-s.volume_m3)<1e-15);}
 if(s.mode==='foil'){const dose=items.find(i=>i.device===id&&i.role==='auxiliary'),spent=items.find(i=>i.device===id&&i.role==='spent_backing');assert.equal(dose.attributes.carrier_volume_m3.value,spent.attributes.carrier_volume_m3.value);const out=items.find(i=>i.device===id&&i.role==='output');assert.equal(out.attributes.applied_layer_volume_m3.value,dose.attributes.transfer_volume_m3.value);}
 recipes.push({id,primary_piece_count:1,nominal_paper_area_m2:s.size[0]*s.size[2],material_recipe_consistent:true});}
await fs.writeFile(home+'/items/source-checks.json',JSON.stringify({passed:true,models:items.length,checks,recipes,Native_acceptance:'pending'},null,2)+'\n');console.log('Twelve actual Item GLBs: independent source rebuild, finite forward/reverse poses, nonshrinking emboss state, coating/foil/carrier recipes PASS.');
