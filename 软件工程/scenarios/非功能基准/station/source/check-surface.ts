import {readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
import {Box3,DoubleSide,Raycaster,Vector2,Vector3} from 'three';
import {decodeGlb,inspectGlb} from '../../../../packages/presentation/src/glb';
import {bindScene} from '../../../../packages/presentation/src/scene';
import {assetKey} from '../../../../packages/presentation/src/types';

const root=resolve(process.argv[2]),output=resolve(process.argv[3]);
// Installed test adapter decodes real embedded PNGs, without claiming GPU upload.
await import(process.argv[4]);
const read=async(p:string)=>JSON.parse(await readFile(resolve(root,p),'utf8'));
const previous=await read('candidate-1.0.4/station.module-document.json');
const document=await read('candidate-1.0.5/station.module-document.json');
const {presentation:before,...oldBusiness}=previous.data.definition.payload.data;
const {presentation:p,...business}=document.data.definition.payload.data;
assert.deepEqual(business,oldBusiness,'all Engine/port/configuration inputs preserved');
const bytes=await readFile(resolve(root,'candidate-1.0.5',p.source.data.path));
const buffer=bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength);
const glb:any=inspectGlb(buffer);
assert.equal(glb.images.length,6);
assert(glb.materials.some((m:any)=>m.name==='B60_NAMEPLATE'&&m.pbrMetallicRoughness.baseColorTexture));
assert(glb.materials.some((m:any)=>m.name==='B60_SCREEN'&&m.pbrMetallicRoughness.baseColorTexture));
assert(glb.materials.some((m:any)=>m.normalTexture));
const decoded=await decodeGlb(buffer);
const rig=await bindScene(p,async()=>buffer,{assetKey:assetKey(p.source.data),decoded});
const previousBytes=await readFile(resolve(root,'candidate-1.0.4',before.source.data.path));
const previousBuffer=previousBytes.buffer.slice(previousBytes.byteOffset,previousBytes.byteOffset+previousBytes.byteLength);
const previousDecoded=await decodeGlb(previousBuffer);
const previousRig=await bindScene(before,async()=>previousBuffer,{assetKey:assetKey(before.source.data),decoded:previousDecoded});
assert.equal(document.data.definition.version,'1.0.5');
assert.equal(document.data.dependencies[0].version,'1.0.5');
assert.equal(business.behavior.inline.identity_policy,'preserve');
assert.equal(business.behavior.inline.phases[0].duration.value,2000);
assert.equal(business.behavior.inline.phases[0].interruption_policy,'non_preemptive');
assert.deepEqual(p.timeline,before.timeline,'original two-second cycle timeline retained');
for(const action of before.actions){
 const after=p.actions.find((v:any)=>v.id===action.id);
 assert(after,action.id);assert.deepEqual(after.keys,action.keys,'motion keys retained');
 assert.equal(p.nodes[after.node_index].name,before.nodes[action.node_index].name,'same named motion target');
}
for(const name of ['control_display','B60_original_identification']){
 const mesh:any=decoded.gltf.scene.getObjectByName(name);
 mesh.material.map.updateMatrix();
 const mappedTop=mesh.material.map.transformUv(new Vector2(.5,1));
 const mappedBottom=mesh.material.map.transformUv(new Vector2(.5,0));
 assert(Math.abs(mappedTop.x-.5)<1e-7&&Math.abs(mappedTop.y)<1e-7,'label top must sample top image row');
 assert(Math.abs(mappedBottom.x-.5)<1e-7&&Math.abs(mappedBottom.y-1)<1e-7,'label bottom must sample bottom image row');
}
const index=(name:string)=>{const i=p.nodes.findIndex((n:any)=>n.name===name);assert(i>=0,name);return i;};
const center=(name:string)=>rig.nodes.get(index(name))!.getWorldPosition(new Vector3());
const boundsOf=(name:string)=>{const mesh:any=rig.nodes.get(index(name))!;mesh.geometry.computeBoundingBox();return mesh.geometry.boundingBox.clone().applyMatrix4(mesh.matrixWorld);};
const close=(a:number,b:number,label:string)=>assert(Math.abs(a-b)<1e-7,`${label}: ${a} / ${b}`);
const domain=(x:number,y:number,z:number)=>new Vector3(x,z,-y);
// Interior witnesses check actual decoded triangles, including T-slot profiles.
// AABB overlap alone would not prove these mounting contacts.
function inside(name:string,point:Vector3){
 const mesh:any=rig.nodes.get(index(name))!;
 const materials=Array.isArray(mesh.material)?mesh.material:[mesh.material];
 const sides=materials.map((m:any)=>m.side);materials.forEach((m:any)=>m.side=DoubleSide);
 const ray=new Raycaster(point,new Vector3(.791,.327,.516).normalize());
 const distances=ray.intersectObject(mesh,false).map(v=>v.distance).filter(v=>v>1e-8).sort((a,b)=>a-b);
 materials.forEach((m:any,i:number)=>m.side=sides[i]);
 const unique=distances.filter((d,i)=>i===0||Math.abs(d-distances[i-1])>1e-7);
 return unique.length%2===1;
}
const contacts:Array<{a:string;b:string;domain_point:number[]}>=[];
function seated(a:string,b:string,xyz:number[]){
 const point=domain(xyz[0],xyz[1],xyz[2]);
 assert(inside(a,point)&&inside(b,point),`actual triangle contact witness ${a} / ${b} at ${xyz}`);
 contacts.push({a,b,domain_point:xyz});
}
for(const sign of [-1,1]){
 const x=sign*.86;
 seated(`gantry_base_mount_${x}`,`upper_length_rail_${sign*.79}`,[sign*.83,.52,1.095]);
 seated(`gantry_base_mount_${x}`,`gantry_vertical_${x}`,[sign*.90,.55,1.16]);
 seated(`front_guard_base_mount_${x}`,'upper_cross_rail_-0.79',[sign*.83,-.83,1.095]);
 seated(`front_guard_base_mount_${x}`,`front_guard_post_${x}`,[sign*.88,-.88,1.16]);
}
seated('stack_light_mount_bracket','gantry_crossbeam',[.72,.59,2.59]);
seated('stack_light_mount_bracket','stack_light_stem',[.724,.68,2.585]);
for(const y of [-.71,.71]){
 seated(`sensor_cable_${y}`,`sensor_mount_${y}`,[-.69,y,1.31]);
 seated(`sensor_cable_${y}`,`photo_sensor_body_${y}`,[-.69,y,1.322]);
 close(boundsOf(`sensor_cable_${y}`).min.y,.88,'retained cable lower endpoint');
}
for(const sign of [-1,1]){
 seated('fixture_fixed_crossmember',`upper_length_rail_${sign*.79}`,[sign*.80,0,1.07]);
 const x=sign*.125;
 seated(`fixture_base_fixing_stud_${x}`,'fixture_fixed_pedestal',[x,0,1.1675]);
 seated(`fixture_base_fixing_stud_${x}`,'workpiece_fixture_anchor',[x,0,1.19]);
 seated(`fixture_base_fixing_head_${x}`,'workpiece_fixture_anchor',[x,0,1.214]);
}
seated('fixture_fixed_crossmember','fixture_fixed_pedestal',[0,0,1.095]);
seated('fixture_fixed_pedestal','workpiece_fixture_anchor',[0,0,1.167]);
for(const support of ['fixture_fixed_crossmember','fixture_fixed_pedestal'])for(const roller of ['roller_4','roller_5'])
 assert(!boundsOf(support).intersectsBox(boundsOf(roller)),`${support} clears ${roller}`);
const workpiece=new Box3(domain(-.09,.09,1.215),domain(.09,-.09,1.315));
for(const x of [-.125,.125])assert(!boundsOf(`fixture_base_fixing_head_${x}`).intersectsBox(workpiece),'fixing head clears retained workpiece');
const fixedNames=p.nodes.filter((n:any)=>/base_mount_|stack_light_mount_bracket|fixture_fixed_|sensor_cable_|fixture_base_fixing_/.test(n.name)).map((n:any)=>n.name);
const fixedMatrices=new Map(fixedNames.map((name:string)=>[name,rig.nodes.get(index(name))!.matrixWorld.toArray()]));
const samples=[];
for(const progress of [0,.15,.3,.5,.65,.8,1,.65,.3,0]){
 rig.apply({known:true,progress,display_phase_id:'cycle'} as any,{clip:p.timeline[0],progress} as any);
  previousRig.apply({known:true,progress,display_phase_id:'cycle'} as any,{clip:before.timeline[0],progress} as any);
 const head=center('ram_tool_head'),fixture=center('workpiece_fixture_anchor'),carriage=center('cycle_ram_carriage');
 assert(Math.abs(head.x-fixture.x)<1e-7&&Math.abs(head.z-fixture.z)<1e-7,`tool/fixture horizontal alignment progress=${progress} head=${head.toArray()} fixture=${fixture.toArray()}`);
 for(const x of [-.36,.36]){
  const bearing=center(`guide_bearing_${x}`);
  assert(Math.abs(bearing.y-carriage.y-.01)<1e-7,'bearing follows carriage');
 }
 const rod=boundsOf('ram_actuator_rod'),body=boundsOf('cycle_ram_carriage');
 assert(Math.abs(rod.min.y-body.max.y)<1e-7,'actuator contacts moving carriage');
 assert(Math.abs(rod.max.y-2.19)<1e-7,'actuator upper end remains in housing');
  if(progress===.3||progress===.5||progress===.65){
  const workpieceTop=fixture.y+.075+.1/2;
  assert(Math.abs(boundsOf('ram_tool_head').min.y-workpieceTop)<1e-7,'full stroke contacts declared workpiece');
 }
 const bounds=new Box3().setFromObject(rig.root),size=bounds.getSize(new Vector3());
 assert(size.x<=2+1e-6&&size.z<=2+1e-6&&size.y<=3+1e-6,`2x2x3m envelope: ${size.toArray()}`);
 assert(Math.abs(bounds.min.y)<1e-7,'feet on ground');
  const oldBounds=new Box3().setFromObject(previousRig.root);
  for(const key of ['x','y','z'] as const){close(bounds.min[key],oldBounds.min[key],'same envelope minimum');close(bounds.max[key],oldBounds.max[key],'same envelope maximum');}
  for(const role of Object.keys(before.anchor_roles))assert(rig.anchor(p.anchor_roles[role])!.distanceTo(previousRig.anchor(before.anchor_roles[role])!)<1e-7,'same role anchor');
  for(const port of Object.keys(before.port_anchors))assert(rig.anchor(p.port_anchors[port])!.distanceTo(previousRig.anchor(before.port_anchors[port])!)<1e-7,'same port anchor');
  for(const name of ['cycle_ram_carriage','ram_tool_head','ram_tool_shaft','guide_bearing_-0.36','guide_bearing_0.36','ram_actuator_rod']){
   const oldIndex=before.nodes.findIndex((n:any)=>n.name===name),oldNode=previousRig.nodes.get(oldIndex)!;
   assert(center(name).distanceTo(oldNode.getWorldPosition(new Vector3()))<1e-7,'retained motion '+name);
  }
  for(const name of fixedNames)assert.deepEqual(rig.nodes.get(index(name))!.matrixWorld.toArray(),fixedMatrices.get(name),'mount/cable remains fixed '+name);
 samples.push({progress,head:head.toArray(),carriage:carriage.toArray(),bounds:{min:bounds.min.toArray(),max:bounds.max.toArray()}});
}
assert.deepEqual(samples[0].head,samples[samples.length-1].head,'reverse seek restores head');
const touching=(a:string,b:string)=>assert(boundsOf(a).expandByScalar(1e-7).intersectsBox(boundsOf(b)),`${a} must connect to ${b}`);
for(const side of [-.52,.52])for(const z of [.62,.74]){
 const mount=`service_handle_mount_${side}_${z}`;
 touching(mount,`service_door_handle_${side}`);
 touching(mount,`side_service_door_${side}`);
}
for(const y of [-.71,.71])touching(`sensor_cable_retainer_${y}`,`sensor_cable_${y}`);
const consoleBounds=boundsOf('control_console');
assert(boundsOf('control_display').max.z>consoleBounds.max.z+.001,'display face is visibly forward of console surface');
assert(boundsOf('emergency_stop').max.z>consoleBounds.max.z+.001,'button protrudes from console front');
assert(boundsOf('B60_original_identification').min.z>boundsOf('side_service_door_-0.52').max.z+.001,'nameplate stays in front of service door');
assert.equal(p.actions.length,before.actions.length);
assert.equal(p.poses.length,before.poses.length);
for(const role of Object.keys(before.anchor_roles))assert(p.anchor_roles[role]);
const simple=await read('candidate-1.0.5/simple.module-document.json');
const oldSimple=await read('candidate-1.0.4/simple.module-document.json');
assert.deepEqual(simple.data.definition.payload,oldSimple.data.definition.payload,'same simplified geometry/actions/anchors');
assert.equal(simple.data.definition.version,'1.0.5');
assert.equal(p.lod.simplified.data.version,'1.0.5');
rig.dispose();decoded.dispose();previousRig.dispose();previousDecoded.dispose();
await writeFile(output,JSON.stringify({result:'passed',version:'1.0.5',baseline:'p201.b60.station@1.0.4',evidence_kind:'current production inspectGlb/decodeGlb/bindScene with installed Pillow pixel adapter; actual decoded-triangle interior witnesses; no GPU or Native claim',embedded_images:glb.images.length,repair_contacts:contacts,samples,business_unchanged:true,simplified_payload_unchanged:true,anchors_and_ports_unchanged:true,exact_envelope_unchanged:true,static_mounts_fixed:true,fixture_support_clears_rollers:true,fixture_fixings_clear_workpiece:true,static_attachment_checks:10,display_button_nameplate_surface_offsets:true,formal_MQ_acceptance:false,Native_acceptance:false,benchmark_ready:false},null,2)+'\n');
console.log(JSON.stringify({result:'passed',samples:samples.length,embedded_images:glb.images.length,Native_acceptance:false}));
