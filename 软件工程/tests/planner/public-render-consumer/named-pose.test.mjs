import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
import {bindScene} from '../../../scenarios/Agent共建/runtime-public-render-consumer/regression/old-media-pure/shared/scene.mjs';
import {activeClip,declaredProgress} from '../../../scenarios/Agent共建/runtime-public-render-consumer/regression/old-media-pure/shared/clips.mjs';
const base=JSON.parse(await readFile(new URL('../../../scenarios/媒体表现/runtime-node-inspector/inline-native.json',import.meta.url),'utf8'));
const pose=x=>({translation:{x,y:0,z:0},rotation:{x:0,y:0,z:0,w:1},scale:{x:1,y:1,z:1},unit:'meter',coordinate_frame:'right_handed_y_up'});
test('explicit named pose reuses canonical native clip transforms, restores untouched base nodes and refreshes world matrices',async()=>{
 const p={...base,nodes:base.nodes.map((n,index)=>({...n,transform:pose(index)})),poses:[{id:'open',node_transforms:[{node_index:0,pose:pose(3)}]},{id:'closed',node_transforms:[{node_index:1,pose:pose(-2)}]}],timeline:[{id:'original-canonical',phase_ref:'auto',start_progress:0,end_progress:1,source:{kind:'native_pose',data:'open'},transition:'hold'}],actions:[]};
 const original=structuredClone(p),rig=await bindScene(p,()=>{throw new Error('native declaration has no byte source');}),point=declaredProgress({phase_ref:'auto',view_u:.5});
 const positions=()=>[...rig.nodes].map(([id,node])=>[id,...node.position.toArray(),...node.matrixWorld.elements]);
 rig.apply(point,activeClip(p,point));const clip=positions();rig.applyNamedPose('closed');assert.equal(rig.nodes.get(0).position.x,0);assert.equal(rig.nodes.get(1).position.x,-2);rig.applyNamedPose('open');assert.deepEqual(positions(),clip);
 rig.apply(null,null);assert.equal(rig.nodes.get(0).position.x,0);assert.equal(rig.nodes.get(1).position.x,1);assert.throws(()=>rig.applyNamedPose('not-declared'),/命名姿态/);assert.deepEqual(p,original);rig.dispose();
});
