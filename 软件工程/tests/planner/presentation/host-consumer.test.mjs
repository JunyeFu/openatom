import assert from 'node:assert/strict';
import {test,after} from 'node:test';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {Quaternion} from 'three';
import {root,out,json,bytes} from './runtime.mjs';
import {decodedImageCount} from './pixel-decoder.mjs';
const {inspectGlb,decodeGlb}=await import(new URL('glb.mjs',out));
const {bindScene}=await import(new URL('scene.mjs',out));
const {projectPhase}=await import(new URL('phase.mjs',out));
const {activeClip,declaredProgress}=await import(new URL('clips.mjs',out));
const {assetKey}=await import(new URL('types.mjs',out));
const latest=await json('.work/delivery/host-runtime/delivery/latest-runtime-result.json'),runtime=await json(latest.result);
const models=new Map(),checks=[];
const near=(actual,expected,label)=>assert.ok(Math.abs(actual-expected)<3e-5,`${label}: ${actual} != ${expected}`);
const reference=e=>({id:e.id,version:e.version});
const key=r=>r.id+'@'+r.version;
const mediaFolder=r=>'.work/delivery/host-media/'+r.id+'/'+r.version+'/';
async function model(ref){
 const name=key(ref);if(models.has(name))return models.get(name);
 const pack=await json(mediaFolder(ref)+'module.json'),p=pack.definition.payload.data;
 assert.equal(pack.definition.id,ref.id);assert.equal(pack.definition.version,ref.version);
 const asset=p.source.data,binary=await bytes(mediaFolder(ref)+asset.path),doc=inspectGlb(binary),decoded=await decodeGlb(binary);
 const rig=await bindScene(p,async a=>bytes(mediaFolder(ref)+a.path),{assetKey:assetKey(asset),decoded});
 const base=new Map([...rig.nodes].map(([index,node])=>[index,{translation:node.position.toArray(),rotation:node.quaternion.toArray(),scale:node.scale.toArray()}]));
 const value={pack,p,binary,doc,decoded,rig,base};models.set(name,value);return value;
}
function accessor(m,index){
 const a=m.doc.accessors[index],view=m.doc.bufferViews[a.bufferView],n={SCALAR:1,VEC3:3,VEC4:4}[a.type];
 assert.equal(a.componentType,5126,'animation accessor is a real float buffer');assert.ok(n);
 const jsonLength=new DataView(m.binary).getUint32(12,true),start=20+jsonLength+8+(view.byteOffset??0)+(a.byteOffset??0),stride=view.byteStride??n*4,v=new DataView(m.binary);
 return Array.from({length:a.count},(_,i)=>Array.from({length:n},(_,j)=>v.getFloat32(start+i*stride+j*4,true)));
}
// Independent buffer-level sampler: does not call bindScene/Mixer/KeyframeTrack.
function expectedTransforms(m,clip){
 const expected=new Map([...m.base].map(([id,x])=>[id,structuredClone(x)]));
 if(!clip)return expected;
 assert.equal(clip.clip.source.kind,'model_animation');
 const animation=m.doc.animations[clip.clip.source.data.animation_index];
 const durations=animation.samplers.map(s=>accessor(m,s.input).at(-1)[0]);
 const at=Math.max(...durations)*clip.progress;
 for(const channel of animation.channels){
  const sampler=animation.samplers[channel.sampler],times=accessor(m,sampler.input).map(x=>x[0]),values=accessor(m,sampler.output);
  assert.ok(['LINEAR','STEP'].includes(sampler.interpolation??'LINEAR'),'this real fixture uses the checked finite interpolation subset');
  let lo=0;while(lo+1<times.length&&times[lo+1]<=at)lo++;
  const hi=Math.min(lo+1,times.length-1),u=sampler.interpolation==='STEP'||lo===hi?0:Math.max(0,Math.min(1,(at-times[lo])/(times[hi]-times[lo])));
  const value=channel.target.path==='rotation'?new Quaternion(...values[lo]).slerp(new Quaternion(...values[hi]),u).toArray():values[lo].map((x,i)=>x+(values[hi][i]-x)*u);
  expected.get(channel.target.node)[channel.target.path]=value;
 }
 return expected;
}
let checkedNodes=0,checkedPoints=0;
function assertRig(m,point,label){
 const clip=activeClip(m.p,point),expected=expectedTransforms(m,clip);m.rig.apply(point,clip);
 for(const [id,node]of m.rig.nodes){
  const x=expected.get(id);for(const [path,actual]of [['translation',node.position.toArray()],['scale',node.scale.toArray()]])actual.forEach((v,i)=>near(v,x[path][i],`${label}/${id}/${path}/${i}`));
  const q=x.rotation,dot=node.quaternion.x*q[0]+node.quaternion.y*q[1]+node.quaternion.z*q[2]+node.quaternion.w*q[3];near(Math.abs(dot),1,`${label}/${id}/rotation`);
  const parent=m.p.nodes[id].parent_index;if(parent!=null)assert.equal(node.parent,m.rig.nodes.get(parent));
  checkedNodes++;
 }
 checkedPoints++;
}
test('actual Host bytes for all 28 selected packages keep exact mixed versions, geometry, hierarchy, textures and clip/node mappings',async()=>{
 const registration=await json('modules/media/equipment-quality-v041/registration.json'),inventory=await json('modules/media/equipment-quality-v041/inventory.json');
 assert.equal(registration.register_order.length,28);assert.equal(inventory.entries.length,28);
 for(const entry of registration.register_order){
  const inv=inventory.entries.find(i=>i.id===entry.id&&i.version===entry.version);assert.ok(inv);
  const family=inv.family,expected=['assembly','rework','packing','inspection'].includes(family)?'0.4.1':'0.4.0';assert.equal(entry.version,expected);
  const m=await model(reference(entry));assert.equal(m.rig.nodes.size,m.p.nodes.length);
  assert.equal(m.doc.nodes.length,m.p.nodes.length);
  for(const node of m.p.nodes){assert.equal(node.name,m.doc.nodes[node.geometry.data.node_index].name);assert.equal(node.geometry.kind,'imported_node');}
  let textures=0;m.decoded.gltf.scene.traverse(o=>{if(o.isMesh)for(const mat of Array.isArray(o.material)?o.material:[o.material])for(const x of Object.values(mat))if(x?.isTexture){assert.ok(x.image.width>0&&x.image.data.length>0);textures++;}});
  const usedMaterials=new Set(m.doc.meshes.flatMap(mesh=>mesh.primitives.map(p=>p.material)).filter(x=>x!=null));
  const textured=[...usedMaterials].some(i=>/Texture"/.test(JSON.stringify(m.doc.materials[i])));
  if(textured)assert.ok(textures>0,'used embedded material pixels decoded');
  else assert.equal(textures,0,'untextured lightweight material remains explicitly untextured');
  if(inv.suffix==='detailed')assert.deepEqual(m.pack.dependencies,[{id:entry.id+'.simple',version:entry.version}]);
  checks.push({module_ref:reference(entry),canonical_nodes:m.rig.nodes.size,texture_bindings:textures,pixel_decoder:'Pillow RGBA via explicit headless adapter; no GPU verdict'});
 }
});
test('all declared clips and reverse seeks match independent exported GLB transform samples in both LODs',async()=>{
 for(const [name,m]of models){
  if(!m.p.timeline.length){m.rig.apply(null,null);continue;}
  for(const phase of new Set(m.p.timeline.map(c=>c.phase_ref)))for(const u of [0,.25,.5,.75,1,.5,0])assertRig(m,declaredProgress({phase_ref:phase,view_u:u}),`declared/${name}/${phase}/${u}`);
 }
});
test('actual Host historical work/wait points drive both real LODs without mutating jobs, items, events or holders',async()=>{
 const input=await json('.work/delivery/inputs/engine-input.json'),phase=await json('packages/commands/tests/fixtures/production/p1-phase-calendar.json');
 for(const entry of runtime.cases){
  const record=await json(entry.timepoints),actual=await json(entry.saved_project.replace(/saved-project$/,'actual-run.json')),run=actual.run,before=JSON.stringify({run,record});
  const definitions=entry.label.startsWith('phase-')?phase.definitions:input.definitions;
  for(const point of record.points)for(const job of point.jobs){
   const device=job.execution_ref.data.device_id,family={ASM01:'assembly',R01:'robot',CV01:'conveyor',V01:'inspection',RW01:'rework',CV02:'conveyor',PK01:'packing',CASE01:'sealer',CNC:'assembly'}[device];assert.ok(family);
   const operation=definitions.find(d=>d.id===job.operation_ref.id&&d.version===job.operation_ref.version).payload.data;
   const frame={run,snapshot:{run_id:run.id,at_ms:point.at_ms,next_index:null,objects:[...point.jobs.map(data=>({kind:'job',data})),...point.items.map(data=>({kind:'item',data}))]},job_id:job.id,operation};
   const projected=projectPhase(frame);assert.equal(projected.known,true);assert.ok(projected.progress!=null,`${entry.label}/${device}/${point.at_ms}`);
   if(job.state==='waiting_output'||job.state==='completed')assert.equal(projected.terminal_pose,true);
   for(const suffix of ['','.simple']){const version=['assembly','rework','packing','inspection'].includes(family)?'0.4.1':'0.4.0';assertRig(await model({id:'p302.line-grade.'+family+suffix,version}),projected,`${entry.label}/${device}/${point.at_ms}/${suffix}`);}
  }
  assert.equal(JSON.stringify({run,record}),before);
 }
});
after(async()=>{
 await mkdir(new URL('.work/delivery/consumer-results/',root),{recursive:true});
 await writeFile(new URL('.work/delivery/consumer-results/actual-models.json',root),JSON.stringify({checks,decoded_images:decodedImageCount(),checked_pose_points:checkedPoints,checked_canonical_nodes:checkedNodes,independent_buffer_sampler:true,GUI:false,visual_acceptance:false},null,2));
 for(const m of models.values())m.rig.dispose();
});
