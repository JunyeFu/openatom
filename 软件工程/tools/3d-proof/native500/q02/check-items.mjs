import fs from 'node:fs/promises';import path from 'node:path';import assert from 'node:assert/strict';import {spawnSync} from 'node:child_process';
import {buildItem,tangent} from '../../../../modules/media/native-equipment-500/reviews/B02-Q02/source/item-build.mjs';
import {paper,radius} from '../../../../modules/media/native-equipment-500/reviews/B02-Q02/source/batch-spec.mjs';
import * as T from 'three';
const home='modules/media/native-equipment-500/reviews/B02-Q02',items=JSON.parse(await fs.readFile(home+'/items/inventory.json','utf8')).metadata,checks=[];
for(const item of items){
 const dir=home+'/items/'+item.id,bytes=await fs.readFile(dir+'/paper.glb'),j=JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12)));
 assert.equal(new Set(j.nodes.map(n=>n.name)).size,j.nodes.length);
 for(const a of j.animations??[]){const channels=new Set();for(const c of a.channels){const key=c.target.node+':'+c.target.path;assert.ok(!channels.has(key));channels.add(key);}}
 const scratch='.work/native500-quality-20261025/q02/items-rebuild/'+item.id;await fs.mkdir(scratch,{recursive:true});await fs.cp(dir+'/source',scratch+'/source',{recursive:true});await fs.copyFile(dir+'/LICENSE.txt',scratch+'/LICENSE.txt');
 const run=spawnSync(process.execPath,['source/item-build.mjs'],{cwd:path.resolve(scratch),windowsHide:true,encoding:'utf8'});assert.equal(run.status,0,run.stderr);assert.ok((await fs.readFile(scratch+'/paper.glb')).equals(bytes),'independent Item source');
 const b=buildItem(item),mixer=new T.AnimationMixer(b.root);
 for(const clip of b.clips){const action=mixer.clipAction(clip);action.play();for(const u of [0,.25,.50,.75,1,.25,0]){action.time=u;mixer.update(0);b.root.updateMatrixWorld(true);b.root.traverse(o=>{assert.ok(o.matrixWorld.elements.every(Number.isFinite),`${item.id} / ${clip.name} / ${u} / ${o.name} / ${o.position.toArray()}`);});}action.stop();}
 checks.push({id:item.id,bytes:bytes.length,unique_nodes:true,unique_channels:true,own_source_rebuild_same_bytes:true,forward_reverse_finite_pose:true});
}
let pairs=0;
for(const u of [0,.25,.5,.75,1])for(const [a,ra,b,rb]of [[[-1.05,1.27],radius(1-u),[-.25,.95],.05],[[.25,.95],.05,[1.05,1.27],radius(u)]]){
 const [p,q]=tangent(a,ra,b,rb),v=[q[0]-p[0],q[1]-p[1]];assert.ok(Math.abs(v[0]*(p[0]-a[0])+v[1]*(p[1]-a[1]))<1e-9);assert.ok(Math.abs(v[0]*(q[0]-b[0])+v[1]*(q[1]-b[1]))<1e-9);pairs++;
}
await fs.writeFile(home+'/items/source-checks.json',JSON.stringify({passed:true,models:items.length,checks,tangent_pairs:pairs,area_m2:paper.length*paper.width,Native_acceptance:'pending'},null,2)+'\n');console.log('Eight actual-paper GLBs: own-source rebuild, reverse seek, finite poses and changing-radius paper tangents PASS.');
