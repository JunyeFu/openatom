//! Independent numeric expectations from the complete P2-01 contract.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile,mkdir,writeFile} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import {Vector3} from 'three';
const root=new URL('../../../',import.meta.url);
const output=new URL('.work/whole-packages/window5-planner-acceptance/layout.mjs',root);
await mkdir(new URL('.',output),{recursive:true});
await writeFile(output,stripTypeScriptTypes(await readFile(new URL('apps/planner/src/scene/layout.ts',root),'utf8')));
const {identityPose,poseMatrix,deviceHallPosition,withHallPosition,rotatePoseZ}=await import(output.href);
const close=(actual,expected)=>actual.forEach((n,i)=>assert.ok(Math.abs(n-expected[i])<1e-10,`${actual} != ${expected}`));
function setup(){const project={lines:[{id:'L',transform:rotatePoseZ({...identityPose(),translation:{x:10,y:20,z:1}},90)}]};const device={line_id:'L',pose:{...identityPose(),translation:{x:2,y:3,z:4}}};return {project,device};}
test('actual layout composes rotated line then rotated hall in declared Z-up meters',()=>{
 const {project,device}=setup();const position=deviceHallPosition(project,device);close(position.toArray(),[7,22,5]);
 const hall=rotatePoseZ({...identityPose(),translation:{x:100,y:200,z:5}},90);
 close(position.applyMatrix4(poseMatrix(hall)).toArray(),[78,207,10]);
});
test('group root movement applies once while member local pose and input identity remain exact',()=>{
 const {project,device}=setup();const original=JSON.stringify(device);
 project.lines[0].transform.translation={x:18,y:16,z:3};
 close(deviceHallPosition(project,device).toArray(),[15,18,7]);assert.equal(JSON.stringify(device),original);
});
test('numeric hall-coordinate edit inverts a rotated group and does not mutate input',()=>{
 const {project,device}=setup();const saved=JSON.stringify({project,device});
 const changed=withHallPosition(project,device,new Vector3(8,24,8));
 close(Object.values(changed.pose.translation),[4,2,7]);close(deviceHallPosition(project,changed).toArray(),[8,24,8]);
 assert.equal(JSON.stringify({project,device}),saved);
});
test('Z rotation preserves existing tilt rather than discarding its other orientation',()=>{
 const p={...identityPose(),rotation:{x:Math.SQRT1_2,y:0,z:0,w:Math.SQRT1_2}};
 const result=rotatePoseZ(p,180);close(Object.values(result.rotation),[0,Math.SQRT1_2,Math.SQRT1_2,0]);
 assert.deepEqual(p.rotation,{x:Math.SQRT1_2,y:0,z:0,w:Math.SQRT1_2});
});
