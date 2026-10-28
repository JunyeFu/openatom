import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
const root=path.resolve(import.meta.dirname,'../../..'),base=path.join(root,'.work/full-closeout-20261021/delivery');
const read=async p=>JSON.parse(await fs.readFile(path.join(base,p),'utf8'));
const result=await read('render/result.json'),expected=await read('render/expectations.json');
assert.equal(result.passed,true);assert.equal(result.frames.length,expected.expected_screens);
const frames=result.frames;
function frame(label,t,lod){const x=frames.find(x=>x.case===label&&x.at_ms===t&&x.lod===lod);assert.ok(x,label+'/'+t+'/'+lod);return x;}
const checks=[];
for(const lod of expected.lods){
 const nodes=(label,t,device)=>frame(label,t,lod).localNodes.filter(n=>n.device===device);
 const hold=(label,start,end,device)=>{const a=nodes(label,start,device),b=nodes(label,end,device);assert.ok(a.length);assert.deepEqual(a,b);checks.push({label,lod,start,end,device,canonical_nodes:a.length,pose_held:true});};
 hold('phase-resource-wait',35000,39999,'CNC');
 hold('phase-interrupted',20000,30000,'CNC');hold('phase-interrupted',30000,39999,'CNC');
 hold('l03-robot-output-wait',48000,60000,'R01');hold('l03-robot-output-wait',60000,360000,'R01');
 hold('l03-pallet-output-wait',90000,360000,'CASE01');
 const point=(label,t,device)=>frame(label,t,lod).active.find(x=>x.id===device).point;
 assert.equal(point('phase-resource-wait',35000,'CNC').display_phase_id,'auto');
 assert.equal(point('phase-resource-wait',35000,'CNC').progress,1);
 assert.equal(point('phase-resource-wait',40000,'CNC').display_phase_id,'unload');
 assert.equal(point('phase-resource-wait',40000,'CNC').progress,0);
 assert.equal(point('phase-interrupted',20000,'CNC').effective_ms,15000);
 assert.equal(point('phase-interrupted',39999,'CNC').progress,.5);
 assert.equal(point('l03-robot-output-wait',48000,'R01').job.state,'waiting_output');
 assert.equal(point('l03-pallet-output-wait',90000,'CASE01').job.state,'waiting_output');
 for(const t of expected.old_exact_carton_times)assert.ok(frame('l03-complete-old-0.3',t,lod).run_id);
}
for(const frame of frames){
 assert.equal(frame.actual_items,frame.regionSummary.actual_ids.length);
 assert.equal(frame.regionSummary.individual_meshes,0);
 assert.equal(new Set(frame.rendered_ids).size,frame.rendered_ids.length);
 assert.ok((await fs.stat(path.join(base,frame.image))).size>1000);
}
// The historical carton has fixed folded panels, not the newer four-hinge rig.
const cartonNodes=async version=>{
 const dir=path.join(root,'.work/delivery/host-media/p302.line-grade.carton',version);
 const pkg=JSON.parse(await fs.readFile(path.join(dir,'module.json'),'utf8'));
 const bytes=await fs.readFile(path.join(dir,pkg.definition.payload.data.source.data.path));
 return JSON.parse(bytes.subarray(20,20+bytes.readUInt32LE(12)).toString()).nodes.map(n=>n.name);
};
const articulated=['MajorFlapRear','MajorFlapFront','MinorFlapLeft','MinorFlapRight'];
const old=await cartonNodes('0.3.0'),current=await cartonNodes('0.4.0');
assert.ok(articulated.every(n=>!old.includes(n)&&current.includes(n)));
await fs.writeFile(path.join(base,'render/history-independent-comparison.json'),JSON.stringify({
 passed:true,frames:frames.length,checks,region_identity_projection_checked:true,
 old_carton_not_replaced_with_new_hinges:true,Native:false,NFR:false},null,2));
console.log(JSON.stringify({passed:true,frames:frames.length,wait_pose_pairs:checks.length,old_carton_preserved:true}));
